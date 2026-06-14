use serde::Deserialize;
use std::path::PathBuf;

/// Application configuration loaded from `~/.config/melotui/config.toml`.
///
/// All fields are optional — missing values use sensible defaults.
/// Example config:
///
/// ```toml
/// [library]
/// music_directories = ["~/Music", "/mnt/data/audio"]
///
/// [ui]
/// sidebar_width = 28
/// scrolloff = 8
///
/// [audio]
/// default_speed = 1.0
/// ```
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct AppConfig {
    pub ui: UiConfig,
    pub library: LibraryConfig,
    pub audio: AudioConfig,
    #[serde(skip)]
    pub theme_overrides: std::collections::HashMap<String, String>,
    #[serde(skip)]
    pub keymap_overrides:
        std::collections::HashMap<String, std::collections::HashMap<String, String>>,
    #[serde(skip)]
    pub startup_errors: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    /// Sidebar width in columns (28-80)
    pub sidebar_width: u16,
    /// Scroll offset — lines to keep visible above/below cursor
    pub scrolloff: usize,
    /// Color theme ID (e.g. "catppuccin_mocha", "dracula")
    pub theme: String,
    /// Notification duration in seconds
    pub notification_duration_secs: u64,
    /// Maximum cover art height percentage in sidebar (e.g. 40 means 40%)
    pub art_max_height_percent: u16,
    /// Debounce time in ms before search triggers
    pub search_debounce_ms: u64,
    /// Target frames per second for rendering
    pub fps_limit: u64,
    /// Max visible items in playlist picker
    pub playlist_picker_max_visible: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct LibraryConfig {
    /// Directories to scan for music. If empty, falls back to `dirs::audio_dir()`.
    pub music_directories: Vec<String>,
    /// Custom database path. If None, falls back to `dirs::data_dir()/melotui/library.db`.
    pub db_path: Option<String>,
    /// Max resolution for cover art downscaling
    pub max_cover_resolution: u32,
    /// Max visible items in the playlist picker viewport
    pub picker_viewport_size: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct AudioConfig {
    /// Default playback speed (0.5–2.0)
    pub default_speed: f32,
    /// Default bass level (0.1–5.0)
    pub default_bass: f32,
    /// Default treble level (0.0–5.0)
    pub default_treble: f32,
    /// Default reverb level (0.0–1.0)
    pub default_reverb: f32,
    /// Default echo level (0.0–0.9)
    pub default_echo: f32,
    /// Seek step in seconds for arrow keys
    pub seek_step: i64,
    /// Volume step amount (0.0 - 1.0)
    pub volume_step: f32,
    /// Speed step amount (0.0 - 2.0)
    pub speed_step: f32,
    /// FFT size for the audio visualizer
    pub fft_size: usize,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            sidebar_width: 28,
            scrolloff: 8,
            theme: "default".into(),
            notification_duration_secs: 2,
            art_max_height_percent: 40,
            search_debounce_ms: 500,
            fps_limit: 60,
            playlist_picker_max_visible: 10,
        }
    }
}

impl Default for LibraryConfig {
    fn default() -> Self {
        Self {
            music_directories: Vec::new(),
            db_path: None,
            max_cover_resolution: 512,
            picker_viewport_size: 20,
        }
    }
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            default_speed: 1.0,
            default_bass: 1.0,
            default_treble: 1.0,
            default_reverb: 0.0,
            default_echo: 0.0,
            seek_step: 5,
            volume_step: 0.05,
            speed_step: 0.1,
            fft_size: 4096,
        }
    }
}

impl AppConfig {
    /// Load config from the standard path, falling back to defaults.
    ///
    /// Config path: `~/.config/melotui/config.toml`
    /// If the file doesn't exist or can't be parsed, returns defaults
    /// with a log warning.
    pub fn load() -> Self {
        let path = Self::config_path();

        match std::fs::read_to_string(&path) {
            Ok(contents) => match toml::from_str::<AppConfig>(&contents) {
                Ok(mut config) => {
                    tracing::info!("Loaded config from {:?}", path);
                    // Attempt to load external theme
                    config.load_external_theme();
                    config.load_external_keymaps();
                    config.validated()
                }
                Err(e) => {
                    tracing::warn!("Failed to parse config {:?}: {}", path, e);
                    let mut config = Self::default();
                    config.startup_errors.push(format!("Config Error: {}", e));
                    config.load_external_theme();
                    config.load_external_keymaps();
                    config.validated()
                }
            },
            Err(_) => {
                tracing::debug!("No config file at {:?}, using defaults", path);
                let mut config = Self::default();
                config.load_external_theme();
                config.load_external_keymaps();
                config.validated()
            }
        }
    }

    fn load_external_theme(&mut self) {
        let theme_name = &self.ui.theme;
        if theme_name == "default" || theme_name == "catppuccin_mocha" {
            return; // Use embedded default
        }

        if let Some(mut theme_path) = dirs::config_dir() {
            theme_path.push("melotui");
            theme_path.push("themes");
            theme_path.push(format!("{}.toml", theme_name));

            if let Ok(contents) = std::fs::read_to_string(&theme_path) {
                if let Ok(overrides) = toml::from_str(&contents) {
                    self.theme_overrides = overrides;
                    tracing::info!("Loaded theme overrides from {:?}", theme_path);
                } else if let Err(e) =
                    toml::from_str::<std::collections::HashMap<String, String>>(&contents)
                {
                    let err_msg = format!("Failed to parse theme file {:?}: {}", theme_path, e);
                    tracing::warn!("{}", err_msg);
                    self.startup_errors.push(err_msg);
                }
            } else {
                tracing::warn!("Theme file not found: {:?}", theme_path);
            }
        }
    }

    fn load_external_keymaps(&mut self) {
        if let Some(mut keymaps_path) = dirs::config_dir() {
            keymaps_path.push("melotui");
            keymaps_path.push("keymaps.toml");

            if let Ok(contents) = std::fs::read_to_string(&keymaps_path) {
                if let Ok(overrides) = toml::from_str(&contents) {
                    self.keymap_overrides = overrides;
                    tracing::info!("Loaded keymap overrides from {:?}", keymaps_path);
                } else if let Err(e) = toml::from_str::<
                    std::collections::HashMap<String, std::collections::HashMap<String, String>>,
                >(&contents)
                {
                    let err_msg = format!("Failed to parse keymaps file {:?}: {}", keymaps_path, e);
                    tracing::warn!("{}", err_msg);
                    self.startup_errors.push(err_msg);
                }
            }
        }
    }

    /// Run simple validation, capping or normalizing values. to valid ranges.
    fn validated(mut self) -> Self {
        self.ui.sidebar_width = self.ui.sidebar_width.clamp(28, 80);
        self.ui.scrolloff = self.ui.scrolloff.clamp(0, 20);
        self.audio.default_speed = self.audio.default_speed.clamp(0.5, 2.0);
        self.audio.default_bass = self.audio.default_bass.clamp(0.1, 5.0);
        self.audio.default_treble = self.audio.default_treble.clamp(0.0, 5.0);
        self.audio.default_reverb = self.audio.default_reverb.clamp(0.0, 1.0);
        self.audio.default_echo = self.audio.default_echo.clamp(0.0, 0.9);
        self
    }

    /// Resolve music directories from config, falling back to OS audio dir.
    ///
    /// Handles tilde (`~`) expansion in paths.
    pub fn resolve_music_dirs(&self) -> Vec<PathBuf> {
        if self.library.music_directories.is_empty() {
            // Fall back to OS audio directory
            match dirs::audio_dir() {
                Some(dir) => vec![dir],
                None => {
                    tracing::warn!("No music directories configured and OS audio dir not found");
                    vec![]
                }
            }
        } else {
            self.library
                .music_directories
                .iter()
                .map(|s| expand_tilde(s))
                .collect()
        }
    }

    /// Resolve database path from config, falling back to OS data dir.
    pub fn resolve_db_path(&self) -> anyhow::Result<PathBuf> {
        match &self.library.db_path {
            Some(p) => Ok(expand_tilde(p)),
            None => {
                let db_root = dirs::data_dir()
                    .ok_or_else(|| anyhow::anyhow!("Could not determine data directory"))?
                    .join("melotui");
                std::fs::create_dir_all(&db_root)?;
                Ok(db_root.join("library.db"))
            }
        }
    }

    fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("melotui")
            .join("config.toml")
    }
}

/// Expand `~` at the start of a path to the user's home directory.
fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/")
        && let Some(home) = dirs::home_dir()
    {
        return home.join(rest);
    }
    if path == "~"
        && let Some(home) = dirs::home_dir()
    {
        return home;
    }
    PathBuf::from(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.ui.sidebar_width, 28);
        assert_eq!(config.ui.scrolloff, 8);
        assert_eq!(config.audio.default_speed, 1.0);
        assert!(config.library.music_directories.is_empty());
        assert!(config.library.db_path.is_none());
    }

    #[test]
    fn test_parse_minimal_toml() {
        let toml_str = r#"
            [ui]
            sidebar_width = 40
        "#;
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.ui.sidebar_width, 40);
        assert_eq!(config.ui.scrolloff, 8); // default
        assert_eq!(config.audio.default_speed, 1.0); // default
    }

    #[test]
    fn test_parse_full_toml() {
        let toml_str = r#"
            [library]
            music_directories = ["/home/user/Music", "/mnt/data/audio"]
            db_path = "/custom/path/library.db"

            [ui]
            sidebar_width = 35
            scrolloff = 4

            [audio]
            default_speed = 1.5
        "#;
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.library.music_directories.len(), 2);
        assert_eq!(
            config.library.db_path.as_deref(),
            Some("/custom/path/library.db")
        );
        assert_eq!(config.ui.sidebar_width, 35);
        assert_eq!(config.ui.scrolloff, 4);
        assert_eq!(config.audio.default_speed, 1.5);
    }

    #[test]
    fn test_validation_clamps() {
        let toml_str = r#"
            [ui]
            sidebar_width = 200
            scrolloff = 50

            [audio]
            default_speed = 10.0
        "#;
        let config: AppConfig = toml::from_str::<AppConfig>(toml_str).unwrap().validated();
        assert_eq!(config.ui.sidebar_width, 80);
        assert_eq!(config.ui.scrolloff, 20);
        assert_eq!(config.audio.default_speed, 2.0);
    }

    #[test]
    fn test_expand_tilde() {
        // Absolute paths pass through unchanged
        let abs = expand_tilde("/home/user/Music");
        assert_eq!(abs, PathBuf::from("/home/user/Music"));

        // Tilde expansion (only verifiable if home_dir exists)
        if let Some(home) = dirs::home_dir() {
            let expanded = expand_tilde("~/Music");
            assert_eq!(expanded, home.join("Music"));
        }
    }

    #[test]
    fn test_resolve_music_dirs_empty_falls_back() {
        let config = AppConfig::default();
        let dirs = config.resolve_music_dirs();
        // Should return the OS audio dir (or empty if not available)
        // We can't assert the exact path, but it shouldn't panic
        assert!(dirs.len() <= 1);
    }

    #[test]
    fn test_resolve_music_dirs_configured() {
        let config = AppConfig {
            library: LibraryConfig {
                music_directories: vec![
                    "/home/user/Music".to_string(),
                    "/mnt/data/audio".to_string(),
                ],
                db_path: None,
                ..Default::default()
            },
            ..Default::default()
        };
        let dirs = config.resolve_music_dirs();
        assert_eq!(dirs.len(), 2);
        assert_eq!(dirs[0], PathBuf::from("/home/user/Music"));
        assert_eq!(dirs[1], PathBuf::from("/mnt/data/audio"));
    }
}
