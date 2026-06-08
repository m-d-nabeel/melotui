use ratatui::style::Color;

/// Centralized color theme for the TUI.
///
/// Every widget references this instead of defining inline colors.
/// The theme lives in `AppState` so it can be changed at runtime
/// (e.g. via `:theme dracula`).
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
