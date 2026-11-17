//! Which colour theme is in use, and the file it was read from: what the
//! colours themselves are is the TUI's to work out (`melotui_tui::theme`).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The themes melotui has built in, by the names they are chosen by.
pub const BUILT_IN: &[&str] = &[
    "catppuccin_mocha",
    "catppuccin_macchiato",
    "catppuccin_frappe",
    "catppuccin_latte",
    "dracula",
    "everforest_dark",
    "gruvbox_dark",
    "gruvbox_light",
    "kanagawa",
    "nord",
    "one_dark",
    "rose_pine",
    "solarized_dark",
    "solarized_light",
    "tokyo_night",
];

/// Other names some of them go by.
const ALIASES: &[(&str, &str)] = &[
    ("default", "catppuccin_mocha"),
    ("gruvbox", "gruvbox_dark"),
    ("onedark", "one_dark"),
    ("everforest", "everforest_dark"),
    ("solarized", "solarized_dark"),
];

/// The built-in theme `name` is, if it is one, under any of its names.
pub fn built_in(name: &str) -> Option<&'static str> {
    let name = ALIASES
        .iter()
        .find(|(alias, _)| *alias == name)
        .map_or(name, |&(_, to)| to);
    BUILT_IN.iter().copied().find(|&built| built == name)
}

/// The theme in use: its name, and what its file in the themes folder says,
/// if it has one — on its own, or over the built-in theme of that name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeChoice {
    pub name: String,
    pub file: Option<HashMap<String, String>>,
    /// Counted up each time the theme changes, for what is worked out from
    /// it to be worked out again.
    pub generation: u64,
}

impl Default for ThemeChoice {
    /// The default theme, with no file.
    fn default() -> Self {
        Self {
            name: "default".into(),
            file: None,
            generation: 0,
        }
    }
}

impl ThemeChoice {
    /// The theme `name`, its file read from the themes folder.
    pub fn load(name: &str) -> Result<Self, String> {
        load_from(&themes_dir(), name).map(|file| Self {
            name: name.to_string(),
            file,
            generation: 0,
        })
    }
}

/// Where theme files are kept: `themes` in the config folder.
pub fn themes_dir() -> PathBuf {
    crate::config::AppConfig::config_dir().join("themes")
}

/// What the file for theme `name` in `dir` says, `None` if there is none
/// and it is built in. An error if it is neither, or its file does not read
/// as a list of names and colours.
pub fn load_from(dir: &Path, name: &str) -> Result<Option<HashMap<String, String>>, String> {
    // A name, not a way out of the folder.
    if name.is_empty() || name.contains(['/', '\\']) || name.starts_with('.') {
        return Err(format!("`{name}` is not a theme's name"));
    }
    let path = dir.join(format!("{name}.toml"));
    match std::fs::read_to_string(&path) {
        Ok(contents) => toml::from_str::<HashMap<String, String>>(&contents)
            .map(Some)
            .map_err(|e| format!("The theme file {} does not read: {e}", path.display())),
        Err(_) if built_in(name).is_some() => Ok(None),
        Err(_) => Err(format!(
            "No theme called `{name}`: it is not built in, and there is no {}",
            path.display()
        )),
    }
}

/// Every theme there is to choose: the built-in ones and those in `dir`.
pub fn available_in(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = BUILT_IN.iter().map(|name| name.to_string()).collect();
    if let Ok(entries) = std::fs::read_dir(dir) {
        names.extend(entries.filter_map(|entry| {
            let path = entry.ok()?.path();
            (path.extension()? == "toml").then_some(())?;
            Some(path.file_stem()?.to_str()?.to_string())
        }));
    }
    names.sort();
    names.dedup();
    names
}

/// Every theme there is to choose, from the themes folder too.
pub fn available() -> Vec<String> {
    available_in(&themes_dir())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_built_in_theme_is_found_by_any_of_its_names() {
        assert_eq!(built_in("nord"), Some("nord"));
        assert_eq!(built_in("default"), Some("catppuccin_mocha"));
        assert_eq!(built_in("gruvbox"), Some("gruvbox_dark"));
        assert_eq!(built_in("onedark"), Some("one_dark"));
        assert_eq!(built_in("cyberpunk"), None);
    }

    #[test]
    fn a_theme_is_built_in_or_read_from_its_file_or_said_to_be_missing() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load_from(dir.path(), "nord"), Ok(None));
        assert_eq!(load_from(dir.path(), "default"), Ok(None));

        std::fs::write(
            dir.path().join("mine.toml"),
            "base = \"nord\"\naccent = \"#ff8800\"\n",
        )
        .unwrap();
        let file = load_from(dir.path(), "mine").unwrap().unwrap();
        assert_eq!(file["base"], "nord");
        assert_eq!(file["accent"], "#ff8800");

        // Over a built-in one, by its name.
        std::fs::write(dir.path().join("nord.toml"), "accent = \"red\"\n").unwrap();
        assert_eq!(
            load_from(dir.path(), "nord").unwrap().unwrap()["accent"],
            "red"
        );

        let missing = load_from(dir.path(), "nothing").unwrap_err();
        assert!(
            missing.starts_with("No theme called `nothing`"),
            "{missing}"
        );

        std::fs::write(dir.path().join("broken.toml"), "accent = 3\n").unwrap();
        let broken = load_from(dir.path(), "broken").unwrap_err();
        assert!(broken.contains("does not read"), "{broken}");
    }

    #[test]
    fn a_theme_name_cannot_reach_outside_the_themes_folder() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["../config", "a/b", "a\\b", ".hidden", ""] {
            assert!(load_from(dir.path(), name).is_err(), "{name}");
        }
    }

    #[test]
    fn the_themes_to_choose_are_the_built_in_ones_and_the_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("cyberpunk.toml"), "").unwrap();
        std::fs::write(dir.path().join("nord.toml"), "").unwrap();
        std::fs::write(dir.path().join("notes.txt"), "").unwrap();
        let names = available_in(dir.path());
        assert!(names.contains(&"cyberpunk".to_string()));
        assert_eq!(names.iter().filter(|n| *n == "nord").count(), 1);
        assert!(!names.iter().any(|n| n.contains("notes")));
        assert_eq!(names.len(), BUILT_IN.len() + 1);
    }
}
