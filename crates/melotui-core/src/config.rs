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
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    /// Sidebar width in columns (28-80)
    pub sidebar_width: u16,
    /// Scroll offset — lines to keep visible above/below cursor
    pub scrolloff: usize,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct LibraryConfig {
    /// Directories to scan for music. If empty, falls back to `dirs::audio_dir()`.
    pub music_directories: Vec<String>,
    /// Custom database path. If None, falls back to `dirs::data_dir()/melotui/library.db`.
    pub db_path: Option<String>,
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
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            sidebar_width: 28,
            scrolloff: 8,
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
                Ok(config) => {
                    tracing::info!("Loaded config from {:?}", path);
                    config.validated()
                }
                Err(e) => {
                    tracing::warn!("Failed to parse config {:?}: {}", path, e);
                    Self::default()
                }
            },
            Err(_) => {
                tracing::debug!("No config file at {:?}, using defaults", path);
                Self::default()
            }
        }
    }

    /// Validate and clamp config values to valid ranges.
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
            },
            ..Default::default()
        };
        let dirs = config.resolve_music_dirs();
        assert_eq!(dirs.len(), 2);
        assert_eq!(dirs[0], PathBuf::from("/home/user/Music"));
        assert_eq!(dirs[1], PathBuf::from("/mnt/data/audio"));
    }
}
