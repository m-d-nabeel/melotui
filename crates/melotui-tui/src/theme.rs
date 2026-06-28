use ratatui::style::Color;

/// Centralized color theme for the TUI.
///
/// Every widget references this instead of defining inline colors.
/// The theme lives in `AppState` so it can be changed at runtime
/// (e.g. via config.toml `theme = "dracula"`).
pub fn get_theme(state: &melotui_core::AppState) -> &'static Theme {
    static THEME: std::sync::OnceLock<Theme> = std::sync::OnceLock::new();

    THEME.get_or_init(|| {
        let mut t = Theme::default();
        t.apply_overrides(state.theme_overrides());
        t
    })
}
#[derive(Debug, Clone)]
pub struct Theme {
    // ── Panel chrome ──────────────────────────────────────
    pub border_active: Color,
    pub border_inactive: Color,

    // ── Backgrounds ───────────────────────────────────────
    pub bg_selected: Color,
    pub bg_overlay: Color,
    pub bg_input: Color,

    // ── Text hierarchy ────────────────────────────────────
    pub fg_primary: Color,
    pub fg_secondary: Color,
    pub fg_muted: Color,

    // ── Tree depth hierarchy ──────────────────────────────
    pub fg_tree_root: Color,
    pub fg_tree_folder: Color,
    pub fg_tree_subfolder: Color,
    pub fg_tree_file: Color,

    // ── Semantic accents ──────────────────────────────────
    pub accent_primary: Color,
    pub accent_playing: Color,
    pub accent_ancestor: Color,
    pub accent_info: Color,
    pub accent_warning: Color,
    pub accent_error: Color,

    // ── Playback status ───────────────────────────────────
    pub status_playing: Color,
    pub status_paused: Color,
    pub status_stopped: Color,

    // ── Category icons ────────────────────────────────────
    pub icon_folder: Color,
    pub icon_playlist: Color,
    pub icon_category: Color,

    // ── Mixer sliders ─────────────────────────────────────
    pub slider_volume: Color,
    pub slider_speed: Color,
    pub slider_bass: Color,
    pub slider_treble: Color,
    pub slider_reverb: Color,
    pub slider_echo: Color,
    pub slider_track: Color,
    pub slider_pan: Color,
    pub slider_lowpass: Color,
    pub slider_highpass: Color,

    // ── DSP Studio ────────────────────────────────────────
    pub dsp_section_border: Color,
    pub dsp_selected_bg: Color,
    pub eq_bar_fill: Color,
    pub eq_bar_empty: Color,
    pub eq_bar_selected: Color,
    pub eq_zero_line: Color,

    // ── Help view ─────────────────────────────────────────
    pub help_key: Color,
    pub help_header: Color,
    pub help_text: Color,
    pub help_subtext: Color,
    pub help_overlay_bg: Color,
    pub help_border: Color,

    // ── Track info badges ─────────────────────────────────
    pub badge_lossless: Color,
    pub badge_lossy: Color,
    pub badge_detail_bg: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self::catppuccin_mocha()
    }
}

impl Theme {
    /// Applies arbitrary user overrides from the config map.
    pub fn apply_overrides(&mut self, overrides: &std::collections::HashMap<String, String>) {
        use std::str::FromStr;
        for (key, val) in overrides {
            if let Ok(color) = Color::from_str(val) {
                match key.as_str() {
                    "border_active" => self.border_active = color,
                    "border_inactive" => self.border_inactive = color,
                    "bg_selected" => self.bg_selected = color,
                    "bg_overlay" => self.bg_overlay = color,
                    "bg_input" => self.bg_input = color,
                    "fg_primary" => self.fg_primary = color,
                    "fg_secondary" => self.fg_secondary = color,
                    "fg_muted" => self.fg_muted = color,
                    "fg_tree_root" => self.fg_tree_root = color,
                    "fg_tree_folder" => self.fg_tree_folder = color,
                    "fg_tree_subfolder" => self.fg_tree_subfolder = color,
                    "fg_tree_file" => self.fg_tree_file = color,
                    "accent_primary" => self.accent_primary = color,
                    "accent_playing" => self.accent_playing = color,
                    "accent_ancestor" => self.accent_ancestor = color,
                    "accent_info" => self.accent_info = color,
                    "accent_warning" => self.accent_warning = color,
                    "accent_error" => self.accent_error = color,
                    "status_playing" => self.status_playing = color,
                    "status_paused" => self.status_paused = color,
                    "status_stopped" => self.status_stopped = color,
                    "icon_folder" => self.icon_folder = color,
                    "icon_playlist" => self.icon_playlist = color,
                    "icon_category" => self.icon_category = color,
                    "slider_volume" => self.slider_volume = color,
                    "slider_speed" => self.slider_speed = color,
                    "slider_bass" => self.slider_bass = color,
                    "slider_treble" => self.slider_treble = color,
                    "slider_reverb" => self.slider_reverb = color,
                    "slider_echo" => self.slider_echo = color,
                    "slider_track" => self.slider_track = color,
                    "slider_pan" => self.slider_pan = color,
                    "slider_lowpass" => self.slider_lowpass = color,
                    "slider_highpass" => self.slider_highpass = color,
                    "dsp_section_border" => self.dsp_section_border = color,
                    "dsp_selected_bg" => self.dsp_selected_bg = color,
                    "eq_bar_fill" => self.eq_bar_fill = color,
                    "eq_bar_empty" => self.eq_bar_empty = color,
                    "eq_bar_selected" => self.eq_bar_selected = color,
                    "eq_zero_line" => self.eq_zero_line = color,
                    "help_key" => self.help_key = color,
                    "help_header" => self.help_header = color,
                    "help_text" => self.help_text = color,
                    "help_subtext" => self.help_subtext = color,
                    "help_overlay_bg" => self.help_overlay_bg = color,
                    "help_border" => self.help_border = color,
                    "badge_lossless" => self.badge_lossless = color,
                    "badge_lossy" => self.badge_lossy = color,
                    "badge_detail_bg" => self.badge_detail_bg = color,
                    _ => {}
                }
            }
        }
    }

    /// Catppuccin Mocha — the default theme.
    ///
    /// Derived from existing colors already used in tree.rs and help_view.rs.
    pub fn catppuccin_mocha() -> Self {
        Self {
            // Panel chrome
            border_active: Color::Rgb(137, 180, 250), // Blue
            border_inactive: Color::Rgb(69, 71, 90),  // Surface1

            // Backgrounds
            bg_selected: Color::Rgb(40, 44, 52), // Custom dark selection
            bg_overlay: Color::Rgb(24, 24, 37),  // Mantle
            bg_input: Color::Rgb(20, 20, 25),    // Deep dark

            // Text hierarchy
            fg_primary: Color::Rgb(205, 214, 244),   // Text
            fg_secondary: Color::Rgb(186, 194, 222), // Subtext1
            fg_muted: Color::Rgb(108, 112, 134),     // Overlay0

            // Tree depth hierarchy
            fg_tree_root: Color::Rgb(205, 214, 244),   // Text
            fg_tree_folder: Color::Rgb(186, 194, 222), // Subtext1
            fg_tree_subfolder: Color::Rgb(166, 173, 200), // Subtext0
            fg_tree_file: Color::Rgb(147, 153, 178),   // Overlay2

            // Semantic accents
            accent_primary: Color::Rgb(137, 180, 250), // Blue
            accent_playing: Color::Rgb(166, 227, 161), // Green
            accent_ancestor: Color::Rgb(148, 226, 213), // Teal
            accent_info: Color::Rgb(137, 180, 250),    // Blue
            accent_warning: Color::Rgb(249, 226, 175), // Yellow
            accent_error: Color::Rgb(243, 139, 168),   // Red

            // Playback status
            status_playing: Color::Rgb(166, 227, 161), // Green
            status_paused: Color::Rgb(249, 226, 175),  // Yellow
            status_stopped: Color::Rgb(243, 139, 168), // Red

            // Category icons
            icon_folder: Color::Rgb(215, 153, 33),   // Gold
            icon_playlist: Color::Rgb(177, 98, 134), // Purple
            icon_category: Color::Rgb(97, 175, 239), // Soft Blue

            // Mixer sliders
            slider_volume: Color::Rgb(166, 227, 161), // Green
            slider_speed: Color::Rgb(137, 180, 250),  // Blue
            slider_bass: Color::Rgb(249, 226, 175),   // Yellow
            slider_treble: Color::Rgb(148, 226, 213), // Teal
            slider_reverb: Color::Rgb(203, 166, 247), // Mauve
            slider_echo: Color::Rgb(243, 139, 168),   // Red
            slider_track: Color::Rgb(69, 71, 90),     // Surface1
            slider_pan: Color::Rgb(245, 194, 231),     // Pink
            slider_lowpass: Color::Rgb(180, 190, 254),  // Lavender
            slider_highpass: Color::Rgb(250, 179, 135), // Peach

            // DSP Studio
            dsp_section_border: Color::Rgb(88, 91, 112), // Surface2
            dsp_selected_bg: Color::Rgb(49, 50, 68),     // Surface0
            eq_bar_fill: Color::Rgb(166, 227, 161),      // Green
            eq_bar_empty: Color::Rgb(49, 50, 68),        // Surface0
            eq_bar_selected: Color::Rgb(137, 180, 250),  // Blue
            eq_zero_line: Color::Rgb(69, 71, 90),        // Surface1

            // Help view
            help_key: Color::Rgb(243, 139, 168),     // Pink
            help_header: Color::Rgb(166, 227, 161),  // Green
            help_text: Color::Rgb(205, 214, 244),    // Text
            help_subtext: Color::Rgb(166, 173, 200), // Subtext0
            help_overlay_bg: Color::Rgb(69, 71, 90), // Surface1
            help_border: Color::Rgb(88, 91, 112),    // Surface2

            // Track info badges
            badge_lossless: Color::Rgb(166, 227, 161), // Green
            badge_lossy: Color::Rgb(137, 180, 250),    // Blue
            badge_detail_bg: Color::Rgb(69, 71, 90),   // Surface1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_apply_overrides() {
        let mut theme = Theme::default();
        let mut overrides = HashMap::new();
        overrides.insert("fg_tree_root".to_string(), "#ff0000".to_string());
        overrides.insert("bg_selected".to_string(), "#00ff00".to_string());
        overrides.insert("invalid_color".to_string(), "not_a_color".to_string());

        theme.apply_overrides(&overrides);

        assert_eq!(theme.fg_tree_root, Color::Rgb(255, 0, 0));
        assert_eq!(theme.bg_selected, Color::Rgb(0, 255, 0));
    }
}
