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
    /// Whether to automatically restore last session on startup
    pub restore_session: bool,
    /// Draw lyrics in scripts the terminal grid cannot render — Hindi and the
    /// other Indic scripts, Urdu, Arabic, Hebrew — with a real text engine, as
    /// images, on terminals with kitty's or iTerm2's graphics protocol (kitty,
    /// Ghostty, WezTerm, iTerm2 and others). Turn off to keep them as
    /// terminal text.
    pub lyrics_shaping: bool,
    /// How large lyrics are drawn in the lyrics panel, as a multiple of the
    /// terminal's text size, from 1 to 4. Larger than 1 needs the same
    /// terminals as `lyrics_shaping`; elsewhere lyrics stay at text size.
    /// `:lyrics zoom <n>` changes it while running.
    pub lyrics_zoom: f32,
    /// The same for the full lyrics view (`:focus lyrics`).
    pub lyrics_zoom_focus: f32,
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
    /// Scan the music folders in the background at start, picking up music
    /// added while melotui was closed. Only new and changed files are read.
    pub scan_on_startup: bool,
    /// Watch the music folders while running, and scan again a few seconds
    /// after they change. Turn off for folders on a slow network share.
    pub watch: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct AudioConfig {
    /// Default playback speed (0.5–2.0)
    pub default_speed: f32,
    /// Default bass level (0.1–5.0)
    pub default_bass: f32,
    /// Default treble level (0.1–5.0)
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
    /// Default pan position (-1.0 left to 1.0 right)
    pub default_pan: f32,
    /// Default low-pass filter cutoff in Hz (200–20000)
    pub default_lowpass: f32,
    /// Default high-pass filter cutoff in Hz (20–5000)
    pub default_highpass: f32,
    /// Output sample rate in Hz. `None` uses the device default.
    ///
    /// Set this to your sound server's native rate (PipeWire: check
    /// `pw-metadata -n settings | grep clock.rate`, usually 48000) to avoid a
    /// pointless resample round-trip — the ALSA "default" PCM commonly reports
    /// 44100 even when the server runs at 48000.
    pub sample_rate: Option<u32>,
    /// Output buffer size in frames. `None` uses cpal's default
    /// (25 ms period / 100 ms buffer).
    ///
    /// This is the dropout safety margin: the device gets two periods of
    /// `buffer_size / 2`, so 8192 @ 48 kHz gives a ~171 ms buffer refilled
    /// ~85 ms at a time. Larger is more robust under load and adds latency;
    /// since melotui is not a live-monitoring tool, robustness wins.
    pub buffer_size: Option<u32>,
    /// The output device to play through, by name or part of it (see
    /// `:device` for the names). Unset follows the system default, and a
    /// device that goes away is replaced by the default until it is back.
    pub output_device: Option<String>,
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
            restore_session: true,
            lyrics_shaping: true,
            lyrics_zoom: 2.0,
            lyrics_zoom_focus: 3.0,
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
            scan_on_startup: true,
            watch: true,
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
            default_pan: 0.0,
            default_lowpass: 20000.0,
            default_highpass: 20.0,
            sample_rate: Some(48_000),
            buffer_size: Some(8192),
            output_device: None,
        }
    }
}

impl AppConfig {
    /// Load config from the standard path, falling back to defaults.
    ///
    /// Config path: `~/.config/melotui/config.toml`
    /// If the file doesn't exist or can't be parsed, returns defaults
    /// with a log warning.
    /// The configuration from its usual place, or the defaults if there is no
    /// file there. A file that does not parse also gives the defaults, with
    /// the error kept in `startup_errors` to be shown once the app is up.
    pub fn load() -> Self {
        let path = Self::config_path();
        match std::fs::read_to_string(&path) {
            Ok(contents) => Self::parse(&path, &contents),
            Err(_) => {
                tracing::debug!("No config file at {:?}, using defaults", path);
                Self::default().finished()
            }
        }
    }

    /// The configuration from `path`, which unlike the usual place has to
    /// exist: it was asked for by name.
    pub fn load_from(path: &std::path::Path) -> std::io::Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        Ok(Self::parse(path, &contents))
    }

    fn parse(path: &std::path::Path, contents: &str) -> Self {
        match toml::from_str::<AppConfig>(contents) {
            Ok(config) => {
                tracing::info!("Loaded config from {:?}", path);
                config.finished()
            }
            Err(e) => {
                tracing::warn!("Failed to parse config {:?}: {}", path, e);
                let mut config = Self::default();
                config.startup_errors.push(format!("Config Error: {}", e));
                config.finished()
            }
        }
    }

    /// With the external theme and keymaps read in, and every value in range.
    fn finished(mut self) -> Self {
        self.load_external_theme();
        self.load_external_keymaps();
        self.validated()
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
        self.ui.lyrics_zoom = clamp_lyrics_zoom(self.ui.lyrics_zoom, 2.0);
        self.ui.lyrics_zoom_focus = clamp_lyrics_zoom(self.ui.lyrics_zoom_focus, 3.0);
        self.audio.default_speed = self.audio.default_speed.clamp(0.5, 2.0);
        self.audio.default_bass = self.audio.default_bass.clamp(0.1, 5.0);
        self.audio.default_treble = self.audio.default_treble.clamp(0.1, 5.0);
        self.audio.default_reverb = self.audio.default_reverb.clamp(0.0, 1.0);
        self.audio.default_echo = self.audio.default_echo.clamp(0.0, 0.9);
        self.audio.default_pan = self.audio.default_pan.clamp(-1.0, 1.0);
        self.audio.default_lowpass = self.audio.default_lowpass.clamp(200.0, 20000.0);
        self.audio.default_highpass = self.audio.default_highpass.clamp(20.0, 5000.0);
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

    /// Where the library database is: the configured path, or the data
    /// directory.
    pub fn db_path(&self) -> PathBuf {
        match &self.library.db_path {
            Some(p) => expand_tilde(p),
            None => Self::data_dir().join("library.db"),
        }
    }

    /// [`Self::db_path`], with the directory it goes in created.
    pub fn resolve_db_path(&self) -> anyhow::Result<PathBuf> {
        let path = self.db_path();
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        Ok(path)
    }

    /// Where the configuration is read from unless another file is named.
    pub fn config_path() -> PathBuf {
        Self::config_dir().join("config.toml")
    }

    /// melotui's configuration directory: the config file, themes, keymaps
    /// and presets.
    pub fn config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("melotui")
    }

    /// melotui's data directory: the library, session, history and logs.
    pub fn data_dir() -> PathBuf {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("melotui")
    }
}

/// The smallest and largest lyrics zoom.
pub const LYRICS_ZOOM: std::ops::RangeInclusive<f32> = 1.0..=4.0;

/// `zoom` within [`LYRICS_ZOOM`], or `fallback` if it is not a number.
pub fn clamp_lyrics_zoom(zoom: f32, fallback: f32) -> f32 {
    if zoom.is_finite() {
        zoom.clamp(*LYRICS_ZOOM.start(), *LYRICS_ZOOM.end())
    } else {
        fallback
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
