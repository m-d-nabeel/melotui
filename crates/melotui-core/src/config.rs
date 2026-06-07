use serde::Deserialize;
use std::path::PathBuf;

/// Application configuration loaded from `~/.config/melotui/config.toml`.
///
/// All fields are optional — missing values use sensible defaults.
/// Example config:
///
/// ```toml
/// [ui]
/// sidebar_width = 28
/// scrolloff = 8
/// ```
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub ui: UiConfig,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    /// Sidebar width in columns (15-80)
    pub sidebar_width: u16,
    /// Scroll offset — lines to keep visible above/below cursor
    pub scrolloff: usize,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            ui: UiConfig::default(),
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            sidebar_width: 28,
            scrolloff: 8,
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
        self.ui.sidebar_width = self.ui.sidebar_width.clamp(15, 80);
        self.ui.scrolloff = self.ui.scrolloff.clamp(0, 20);
        self
    }

    fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("melotui")
            .join("config.toml")
    }
}
