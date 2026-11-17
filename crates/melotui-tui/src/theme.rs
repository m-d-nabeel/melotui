//! The colours everything is drawn in.
//!
//! A theme starts from a palette of sixteen colours — a background, the
//! surfaces drawn on it, three levels of text, an accent and eight hues —
//! and every colour the widgets use is worked out from those
//! ([`Theme::from_palette`]), so a theme that gives only the palette is
//! whole. Any of those colours can then be given as well, by name.
//!
//! Built in are fifteen, their palettes as each one's own project gives
//! them. A file in the themes folder is a theme of its own, or changes the
//! built-in one whose name it has, or the one it names as its `base`.

use std::str::FromStr;

use ratatui::style::Color;

use melotui_core::theme::ThemeChoice;

/// The theme `state` is in: worked out once for each theme, and kept.
pub fn get_theme(state: &melotui_core::AppState) -> &'static Theme {
    use std::sync::Mutex;
    // Each theme is worked out once and kept for as long as the program
    // runs, so that what is drawn can hold on to it: one for each theme
    // switched to, which is never many.
    static THEMES: Mutex<Vec<(u64, bool, &'static Theme)>> = Mutex::new(Vec::new());
    let choice = state.theme();
    let background = state.theme_background();
    let mut themes = THEMES.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(&(_, _, theme)) = themes
        .iter()
        .find(|(generation, painted, _)| *generation == choice.generation && *painted == background)
    {
        return theme;
    }
    let (mut theme, _) = build(choice);
    if !background {
        theme.bg_main = Color::Reset;
    }
    let theme: &'static Theme = Box::leak(Box::new(theme));
    themes.push((choice.generation, background, theme));
    theme
}

/// The base colours of a theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    /// What everything is drawn on; `reset` for the terminal's own.
    pub background: Color,
    /// Panels drawn over the rest: pickers, dialogs, the command bar.
    pub surface: Color,
    /// What is selected is drawn on.
    pub surface_high: Color,
    /// Borders of what does not have the keys, and slider tracks.
    pub border: Color,
    pub foreground: Color,
    pub subtext: Color,
    pub muted: Color,
    /// What has the keys, and what is selected.
    pub accent: Color,
    pub red: Color,
    pub orange: Color,
    pub yellow: Color,
    pub green: Color,
    pub cyan: Color,
    pub blue: Color,
    pub purple: Color,
    pub pink: Color,
}

/// The names the palette's colours are given by in a theme file.
pub const PALETTE_KEYS: &[&str] = &[
    "background",
    "surface",
    "surface_high",
    "border",
    "foreground",
    "subtext",
    "muted",
    "accent",
    "red",
    "orange",
    "yellow",
    "green",
    "cyan",
    "blue",
    "purple",
    "pink",
];

impl Palette {
    fn colour_mut(&mut self, key: &str) -> Option<&mut Color> {
        Some(match key {
            "background" => &mut self.background,
            "surface" => &mut self.surface,
            "surface_high" => &mut self.surface_high,
            "border" => &mut self.border,
            "foreground" => &mut self.foreground,
            "subtext" => &mut self.subtext,
            "muted" => &mut self.muted,
            "accent" => &mut self.accent,
            "red" => &mut self.red,
            "orange" => &mut self.orange,
            "yellow" => &mut self.yellow,
            "green" => &mut self.green,
            "cyan" => &mut self.cyan,
            "blue" => &mut self.blue,
            "purple" => &mut self.purple,
            "pink" => &mut self.pink,
            _ => return None,
        })
    }

    /// The darkest of what is drawn on: the background, or where that is
    /// the terminal's, the surfaces.
    pub fn ground(&self) -> Color {
        if self.background == Color::Reset {
            self.surface
        } else {
            self.background
        }
    }
}

/// Declares the theme's colours, each a field and the name it is given by in
/// a theme file.
macro_rules! theme_colours {
    ($($(#[$doc:meta])* $field:ident,)*) => {
        /// Every colour the widgets are drawn in.
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct Theme {
            /// What the rest were worked out from, for what has no colour
            /// of its own.
            pub palette: Palette,
            $($(#[$doc])* pub $field: Color,)*
        }

        /// The names a theme's colours are given by in a theme file.
        pub const THEME_KEYS: &[&str] = &[$(stringify!($field)),*];

        impl Theme {
            fn colour_mut(&mut self, key: &str) -> Option<&mut Color> {
                match key {
                    $(stringify!($field) => Some(&mut self.$field),)*
                    _ => None,
                }
            }
        }
    };
}

theme_colours! {
    // ── Panel chrome
    border_active,
    border_inactive,
    /// Panels that never have the keys: the visualizer, the album art, what
    /// is playing.
    border_panel,
    // ── Backgrounds
    /// What the whole screen is drawn on; `reset` for the terminal's.
    bg_main,
    bg_selected,
    bg_overlay,
    bg_input,
    /// What is behind a popup is darkened to.
    bg_dim,
    // ── Text
    fg_primary,
    fg_secondary,
    fg_muted,
    /// Text on what is selected in the accent colour.
    fg_selected,
    // ── The tree, by depth
    fg_tree_root,
    fg_tree_folder,
    fg_tree_subfolder,
    fg_tree_file,
    // ── Accents
    accent_primary,
    accent_playing,
    accent_ancestor,
    accent_info,
    accent_warning,
    accent_error,
    // ── Playback status
    status_playing,
    status_paused,
    status_stopped,
    // ── Icons in the tree
    icon_folder,
    icon_playlist,
    icon_category,
    // ── Sliders, and the DSP studio's sections after them
    slider_volume,
    slider_speed,
    slider_bass,
    slider_treble,
    slider_reverb,
    slider_echo,
    slider_track,
    slider_pan,
    slider_lowpass,
    slider_highpass,
    // ── DSP studio
    /// The equaliser's border.
    dsp_section_border,
    dsp_selected_bg,
    eq_bar_fill,
    eq_bar_empty,
    eq_bar_selected,
    eq_zero_line,
    // ── Help
    help_key,
    help_header,
    help_text,
    help_subtext,
    help_overlay_bg,
    help_border,
    // ── Track info
    badge_lossless,
    badge_lossy,
    badge_detail_bg,
    // ── The visualizer, from the quietest or lowest to the loudest or
    // highest
    visualizer_low,
    visualizer_mid,
    visualizer_high,
}

impl Default for Theme {
    fn default() -> Self {
        Self::catppuccin_mocha()
    }
}

impl Theme {
    /// Every colour worked out from `p`.
    pub fn from_palette(p: &Palette) -> Self {
        let ground = p.ground();
        Self {
            palette: *p,
            border_active: p.accent,
            border_inactive: p.border,
            border_panel: p.foreground,
            bg_main: p.background,
            bg_selected: p.surface_high,
            bg_overlay: p.surface,
            bg_input: p.surface,
            bg_dim: blend(ground, Color::Rgb(0, 0, 0), 0.55),
            fg_primary: p.foreground,
            fg_secondary: p.subtext,
            fg_muted: p.muted,
            fg_selected: more_legible_on(p.accent, ground, p.foreground),
            fg_tree_root: p.foreground,
            fg_tree_folder: p.subtext,
            fg_tree_subfolder: blend(p.subtext, p.muted, 0.35),
            fg_tree_file: blend(p.subtext, p.muted, 0.65),
            accent_primary: p.accent,
            accent_playing: p.green,
            accent_ancestor: p.cyan,
            accent_info: p.blue,
            accent_warning: p.yellow,
            accent_error: p.red,
            status_playing: p.green,
            status_paused: p.yellow,
            status_stopped: p.red,
            icon_folder: p.yellow,
            icon_playlist: p.purple,
            icon_category: p.blue,
            slider_volume: p.green,
            slider_speed: p.blue,
            slider_bass: p.yellow,
            slider_treble: p.cyan,
            slider_reverb: p.purple,
            slider_echo: p.red,
            slider_track: p.border,
            slider_pan: p.pink,
            slider_lowpass: blend(p.blue, p.purple, 0.5),
            slider_highpass: p.orange,
            dsp_section_border: p.blue,
            dsp_selected_bg: p.surface_high,
            eq_bar_fill: p.green,
            eq_bar_empty: p.border,
            eq_bar_selected: p.accent,
            eq_zero_line: p.border,
            help_key: p.red,
            help_header: p.green,
            help_text: p.foreground,
            help_subtext: p.subtext,
            help_overlay_bg: p.surface,
            help_border: p.border,
            badge_lossless: p.green,
            badge_lossy: p.blue,
            badge_detail_bg: p.border,
            // A little toward the background, and no more: on a light
            // theme further would all but vanish into it.
            visualizer_low: blend(p.blue, ground, 0.25),
            visualizer_mid: p.blue,
            visualizer_high: blend(p.cyan, p.foreground, 0.3),
        }
    }

    /// Catppuccin Mocha, the default: worked out from its palette, and then
    /// the colours melotui has always had where they are not quite that.
    /// It keeps the terminal's background.
    pub fn catppuccin_mocha() -> Self {
        let mut theme = Self::from_palette(&catppuccin(
            None,
            [
                0x181825, 0x313244, 0x45475a, 0xcdd6f4, 0xbac2de, 0x6c7086, 0xf38ba8, 0xfab387,
                0xf9e2af, 0xa6e3a1, 0x94e2d5, 0x89b4fa, 0xcba6f7, 0xf5c2e7,
            ],
        ));
        let rgb = |hex: u32| Color::Rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8);
        // The terminal's own text colour, as these panels always had.
        theme.border_panel = Color::Reset;
        theme.bg_selected = rgb(0x282c34);
        theme.bg_input = rgb(0x141419);
        theme.bg_dim = rgb(0x0a0a0f);
        theme.fg_selected = Color::Black;
        theme.fg_tree_subfolder = rgb(0xa6adc8);
        theme.fg_tree_file = rgb(0x9399b2);
        theme.icon_folder = rgb(0xd79921);
        theme.icon_playlist = rgb(0xb16286);
        theme.icon_category = rgb(0x61afef);
        theme.slider_lowpass = rgb(0xb4befe);
        theme.dsp_section_border = rgb(0x61afef);
        theme.dsp_selected_bg = rgb(0x313244);
        theme.eq_bar_empty = rgb(0x45475a);
        theme.eq_zero_line = rgb(0x45475a);
        theme.help_subtext = rgb(0xa6adc8);
        theme.help_overlay_bg = rgb(0x45475a);
        theme.help_border = rgb(0x585b70);
        theme.badge_detail_bg = rgb(0x45475a);
        theme.visualizer_low = Color::Rgb(40, 90, 210);
        theme.visualizer_mid = Color::Rgb(90, 150, 232);
        theme.visualizer_high = Color::Rgb(140, 210, 255);
        theme
    }

    /// The colour text is drawn in where it has none of its own: the
    /// theme's, where it paints its own background, and otherwise the
    /// terminal's, which goes with the terminal's background.
    pub fn text(&self) -> Color {
        if self.bg_main == Color::Reset {
            Color::Reset
        } else {
            self.fg_primary
        }
    }

    /// The built-in theme `name`, under any of its names.
    pub fn built_in(name: &str) -> Option<Self> {
        let name = melotui_core::theme::built_in(name)?;
        if name == "catppuccin_mocha" {
            return Some(Self::catppuccin_mocha());
        }
        built_in_palette(name).map(|palette| Self::from_palette(&palette))
    }
}

/// The theme `choice` is, and what is wrong with its file: names that are
/// not colours or bases it has, and values that are not colours.
pub fn build(choice: &ThemeChoice) -> (Theme, Vec<String>) {
    let mut problems = Vec::new();
    let Some(file) = &choice.file else {
        return (Theme::built_in(&choice.name).unwrap_or_default(), problems);
    };
    let base = file.get("base");
    let mut theme = match base {
        Some(base) => Theme::built_in(base).unwrap_or_else(|| {
            problems.push(format!("`base`: there is no built-in theme `{base}`"));
            Theme::default()
        }),
        None => Theme::built_in(&choice.name).unwrap_or_default(),
    };

    let mut keys: Vec<&String> = file.keys().filter(|key| *key != "base").collect();
    keys.sort();
    let colour = |key: &str, value: &str, problems: &mut Vec<String>| match Color::from_str(value) {
        Ok(colour) => Some(colour),
        Err(_) => {
            problems.push(format!("`{key}`: `{value}` is not a colour"));
            None
        }
    };

    // The palette first, and everything worked out from it again.
    let mut palette = theme.palette;
    let mut palette_given = false;
    for key in &keys {
        if let Some(slot) = palette.colour_mut(key)
            && let Some(value) = colour(key, &file[*key], &mut problems)
        {
            *slot = value;
            palette_given = true;
        }
    }
    if palette_given {
        theme = Theme::from_palette(&palette);
    }
    // Then each colour given by its own name.
    for key in keys {
        if palette.colour_mut(key).is_some() {
            continue;
        }
        let value = &file[key];
        match theme.colour_mut(key) {
            Some(slot) => {
                if let Some(value) = colour(key, value, &mut problems) {
                    *slot = value;
                }
            }
            None => problems.push(format!("`{key}` is not a colour a theme has")),
        }
    }
    (theme, problems)
}

/// What is wrong with the theme `choice`, for saying so.
pub fn problems(choice: &ThemeChoice) -> Vec<String> {
    build(choice).1
}

// ── Colour arithmetic ─────────────────────────────────────

/// `colour` as red, green and blue: the terminal's named colours as xterm's
/// defaults have them, which is as near as can be known.
fn rgb_of(colour: Color) -> (u8, u8, u8) {
    const NAMED: [(u8, u8, u8); 16] = [
        (0, 0, 0),
        (205, 0, 0),
        (0, 205, 0),
        (205, 205, 0),
        (0, 0, 238),
        (205, 0, 205),
        (0, 205, 205),
        (229, 229, 229),
        (127, 127, 127),
        (255, 0, 0),
        (0, 255, 0),
        (255, 255, 0),
        (92, 92, 255),
        (255, 0, 255),
        (0, 255, 255),
        (255, 255, 255),
    ];
    let indexed = |i: u8| match i {
        0..=15 => NAMED[usize::from(i)],
        16..=231 => {
            let level = |n: u8| if n == 0 { 0 } else { 55 + n * 40 };
            let n = i - 16;
            (level(n / 36), level(n / 6 % 6), level(n % 6))
        }
        _ => {
            let grey = 8 + (i - 232) * 10;
            (grey, grey, grey)
        }
    };
    match colour {
        Color::Rgb(r, g, b) => (r, g, b),
        Color::Indexed(i) => indexed(i),
        Color::Black => NAMED[0],
        Color::Red => NAMED[1],
        Color::Green => NAMED[2],
        Color::Yellow => NAMED[3],
        Color::Blue => NAMED[4],
        Color::Magenta => NAMED[5],
        Color::Cyan => NAMED[6],
        Color::Gray => NAMED[7],
        Color::DarkGray => NAMED[8],
        Color::LightRed => NAMED[9],
        Color::LightGreen => NAMED[10],
        Color::LightYellow => NAMED[11],
        Color::LightBlue => NAMED[12],
        Color::LightMagenta => NAMED[13],
        Color::LightCyan => NAMED[14],
        Color::White => NAMED[15],
        Color::Reset => NAMED[0],
    }
}

/// `a` with `t` of the way to `b` mixed in.
pub fn blend(a: Color, b: Color, t: f32) -> Color {
    let (a, b) = (rgb_of(a), rgb_of(b));
    let mix = |x: u8, y: u8| (f32::from(x) + (f32::from(y) - f32::from(x)) * t).round() as u8;
    Color::Rgb(mix(a.0, b.0), mix(a.1, b.1), mix(a.2, b.2))
}

/// How light `colour` looks: WCAG's relative luminance.
fn luminance(colour: Color) -> f32 {
    let (r, g, b) = rgb_of(colour);
    let linear = |c: u8| {
        let c = f32::from(c) / 255.0;
        if c <= 0.039_28 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b)
}

/// WCAG's contrast ratio between `a` and `b`, from 1 to 21.
pub fn contrast(a: Color, b: Color) -> f32 {
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

/// Whichever of `one` and `other` is easier to read on `on`.
fn more_legible_on(on: Color, one: Color, other: Color) -> Color {
    if contrast(on, one) >= contrast(on, other) {
        one
    } else {
        other
    }
}

// ── The built-in palettes ─────────────────────────────────

const fn hex(rgb: u32) -> Color {
    Color::Rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
}

/// A palette from its colours, in `PALETTE_KEYS` order after the
/// background and accent.
#[allow(clippy::too_many_arguments)]
const fn palette(background: Option<u32>, accent: u32, rest: [u32; 14]) -> Palette {
    Palette {
        background: match background {
            Some(rgb) => hex(rgb),
            None => Color::Reset,
        },
        accent: hex(accent),
        surface: hex(rest[0]),
        surface_high: hex(rest[1]),
        border: hex(rest[2]),
        foreground: hex(rest[3]),
        subtext: hex(rest[4]),
        muted: hex(rest[5]),
        red: hex(rest[6]),
        orange: hex(rest[7]),
        yellow: hex(rest[8]),
        green: hex(rest[9]),
        cyan: hex(rest[10]),
        blue: hex(rest[11]),
        purple: hex(rest[12]),
        pink: hex(rest[13]),
    }
}

/// A Catppuccin flavour: mantle, surface0, surface1, text, subtext1,
/// overlay0, red, peach, yellow, green, teal, blue, mauve, pink — and its
/// blue for the accent.
const fn catppuccin(base: Option<u32>, colours: [u32; 14]) -> Palette {
    palette(base, colours[11], colours)
}

/// The palette of the built-in theme `name`. Each is as its own project
/// gives it: Catppuccin's palette.json, Dracula's and Gruvbox's Vim
/// palettes, Nord's nord.css, Tokyo Night's and Kanagawa's Neovim palettes,
/// Rosé Pine's palette.json, One Dark's and Everforest's Vim palettes, and
/// Solarized's README. Where a theme has no colour for a slot, the colour it
/// maps the terminal's colour of that name to is taken.
fn built_in_palette(name: &str) -> Option<Palette> {
    Some(match name {
        "catppuccin_macchiato" => catppuccin(
            Some(0x24273a),
            [
                0x1e2030, 0x363a4f, 0x494d64, 0xcad3f5, 0xb8c0e0, 0x6e738d, 0xed8796, 0xf5a97f,
                0xeed49f, 0xa6da95, 0x8bd5ca, 0x8aadf4, 0xc6a0f6, 0xf5bde6,
            ],
        ),
        "catppuccin_frappe" => catppuccin(
            Some(0x303446),
            [
                0x292c3c, 0x414559, 0x51576d, 0xc6d0f5, 0xb5bfe2, 0x737994, 0xe78284, 0xef9f76,
                0xe5c890, 0xa6d189, 0x81c8be, 0x8caaee, 0xca9ee6, 0xf4b8e4,
            ],
        ),
        "catppuccin_latte" => catppuccin(
            Some(0xeff1f5),
            [
                0xe6e9ef, 0xccd0da, 0xbcc0cc, 0x4c4f69, 0x5c5f77, 0x9ca0b0, 0xd20f39, 0xfe640b,
                0xdf8e1d, 0x40a02b, 0x179299, 0x1e66f5, 0x8839ef, 0xea76cb,
            ],
        ),
        // bg, bgdark, selection, bglighter, fg, — , comment; no blue of its
        // own, and its terminal blue is its purple.
        "dracula" => {
            let mut p = palette(
                Some(0x282a36),
                0xbd93f9,
                [
                    0x21222c, 0x44475a, 0x424450, 0xf8f8f2, 0xf8f8f2, 0x6272a4, 0xff5555, 0xffb86c,
                    0xf1fa8c, 0x50fa7b, 0x8be9fd, 0xbd93f9, 0xbd93f9, 0xff79c6,
                ],
            );
            p.subtext = blend(p.foreground, p.muted, 0.3);
            p
        }
        // nord0–nord15: the frost's nord8 for the accent.
        "nord" => {
            let mut p = palette(
                Some(0x2e3440),
                0x88c0d0,
                [
                    0x3b4252, 0x434c5e, 0x4c566a, 0xeceff4, 0xd8dee9, 0x4c566a, 0xbf616a, 0xd08770,
                    0xebcb8b, 0xa3be8c, 0x8fbcbb, 0x81a1c1, 0xb48ead, 0xb48ead,
                ],
            );
            // nord3 is its comments' colour, too dark on nord0 for text.
            p.muted = blend(p.border, p.subtext, 0.25);
            p
        }
        "gruvbox_dark" => palette(
            Some(0x282828),
            0xfabd2f,
            [
                0x1d2021, 0x3c3836, 0x504945, 0xebdbb2, 0xbdae93, 0x928374, 0xfb4934, 0xfe8019,
                0xfabd2f, 0xb8bb26, 0x8ec07c, 0x83a598, 0xd3869b, 0xd3869b,
            ],
        ),
        "gruvbox_light" => palette(
            Some(0xfbf1c7),
            0xb57614,
            [
                0xebdbb2, 0xd5c4a1, 0xbdae93, 0x3c3836, 0x665c54, 0x928374, 0x9d0006, 0xaf3a03,
                0xb57614, 0x79740e, 0x427b58, 0x076678, 0x8f3f71, 0x8f3f71,
            ],
        ),
        // Night: bg, bg_dark, bg_highlight, fg_gutter, fg, fg_dark, comment.
        "tokyo_night" => palette(
            Some(0x1a1b26),
            0x7aa2f7,
            [
                0x16161e, 0x292e42, 0x3b4261, 0xc0caf5, 0xa9b1d6, 0x565f89, 0xf7768e, 0xff9e64,
                0xe0af68, 0x9ece6a, 0x7dcfff, 0x7aa2f7, 0x9d7cd8, 0xbb9af7,
            ],
        ),
        // Main: base, surface, overlay, text, subtle, muted; love, gold,
        // rose, pine, foam, iris as its terminal colours have them.
        "rose_pine" => {
            let mut p = palette(
                Some(0x191724),
                0xc4a7e7,
                [
                    0x1f1d2e, 0x26233a, 0x26233a, 0xe0def4, 0x908caa, 0x6e6a86, 0xeb6f92, 0xebbcba,
                    0xf6c177, 0x31748f, 0x9ccfd8, 0x9ccfd8, 0xc4a7e7, 0xebbcba,
                ],
            );
            p.border = blend(p.surface_high, p.muted, 0.4);
            p
        }
        // background, cursor_grey, visual_grey, gutter_fg_grey, foreground,
        // — , comment_grey; dark_yellow for orange.
        "one_dark" => {
            let mut p = palette(
                Some(0x282c34),
                0x61afef,
                [
                    0x2c323c, 0x3e4452, 0x4b5263, 0xabb2bf, 0xabb2bf, 0x5c6370, 0xe06c75, 0xd19a66,
                    0xe5c07b, 0x98c379, 0x56b6c2, 0x61afef, 0xc678dd, 0xc678dd,
                ],
            );
            p.subtext = blend(p.foreground, p.muted, 0.3);
            p
        }
        // Wave: sumiInk3, sumiInk2, waveBlue1, sumiInk6, fujiWhite,
        // oldWhite, fujiGray.
        "kanagawa" => palette(
            Some(0x1f1f28),
            0x7e9cd8,
            [
                0x1a1a22, 0x223249, 0x54546d, 0xdcd7ba, 0xc8c093, 0x727169, 0xe46876, 0xffa066,
                0xe6c384, 0x98bb6c, 0x7aa89f, 0x7e9cd8, 0x957fb8, 0xd27e99,
            ],
        ),
        // Medium dark: bg0, bg_dim, bg2, bg4, fg, grey2, grey0.
        "everforest_dark" => palette(
            Some(0x2d353b),
            0xa7c080,
            [
                0x232a2e, 0x3d484d, 0x4f585e, 0xd3c6aa, 0x9da9a0, 0x7a8478, 0xe67e80, 0xe69875,
                0xdbbc7f, 0xa7c080, 0x83c092, 0x7fbbb3, 0xd699b6, 0xd699b6,
            ],
        ),
        // base03 on, base02 raised, base01 for borders and muted text,
        // base1 and base0 for text.
        "solarized_dark" => palette(
            Some(0x002b36),
            0x268bd2,
            [
                0x073642, 0x073642, 0x586e75, 0x93a1a1, 0x839496, 0x586e75, 0xdc322f, 0xcb4b16,
                0xb58900, 0x859900, 0x2aa198, 0x268bd2, 0x6c71c4, 0xd33682,
            ],
        ),
        // base3 on, base2 raised, base1 for borders and muted text, base01
        // and base00 for text.
        "solarized_light" => palette(
            Some(0xfdf6e3),
            0x268bd2,
            [
                0xeee8d5, 0xeee8d5, 0x93a1a1, 0x586e75, 0x657b83, 0x93a1a1, 0xdc322f, 0xcb4b16,
                0xb58900, 0x859900, 0x2aa198, 0x268bd2, 0x6c71c4, 0xd33682,
            ],
        ),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn choice(name: &str, file: &[(&str, &str)]) -> ThemeChoice {
        ThemeChoice {
            name: name.into(),
            file: (!file.is_empty()).then(|| {
                file.iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect()
            }),
            generation: 0,
        }
    }

    /// The colours melotui was drawn in before themes had palettes.
    fn as_it_was() -> Vec<(&'static str, Color)> {
        let rgb = Color::Rgb;
        vec![
            ("border_active", rgb(137, 180, 250)),
            ("border_inactive", rgb(69, 71, 90)),
            ("bg_selected", rgb(40, 44, 52)),
            ("bg_overlay", rgb(24, 24, 37)),
            ("bg_input", rgb(20, 20, 25)),
            ("fg_primary", rgb(205, 214, 244)),
            ("fg_secondary", rgb(186, 194, 222)),
            ("fg_muted", rgb(108, 112, 134)),
            ("fg_tree_root", rgb(205, 214, 244)),
            ("fg_tree_folder", rgb(186, 194, 222)),
            ("fg_tree_subfolder", rgb(166, 173, 200)),
            ("fg_tree_file", rgb(147, 153, 178)),
            ("accent_primary", rgb(137, 180, 250)),
            ("accent_playing", rgb(166, 227, 161)),
            ("accent_ancestor", rgb(148, 226, 213)),
            ("accent_info", rgb(137, 180, 250)),
            ("accent_warning", rgb(249, 226, 175)),
            ("accent_error", rgb(243, 139, 168)),
            ("status_playing", rgb(166, 227, 161)),
            ("status_paused", rgb(249, 226, 175)),
            ("status_stopped", rgb(243, 139, 168)),
            ("icon_folder", rgb(215, 153, 33)),
            ("icon_playlist", rgb(177, 98, 134)),
            ("icon_category", rgb(97, 175, 239)),
            ("slider_volume", rgb(166, 227, 161)),
            ("slider_speed", rgb(137, 180, 250)),
            ("slider_bass", rgb(249, 226, 175)),
            ("slider_treble", rgb(148, 226, 213)),
            ("slider_reverb", rgb(203, 166, 247)),
            ("slider_echo", rgb(243, 139, 168)),
            ("slider_track", rgb(69, 71, 90)),
            ("slider_pan", rgb(245, 194, 231)),
            ("slider_lowpass", rgb(180, 190, 254)),
            ("slider_highpass", rgb(250, 179, 135)),
            ("dsp_selected_bg", rgb(49, 50, 68)),
            ("eq_bar_fill", rgb(166, 227, 161)),
            ("eq_bar_selected", rgb(137, 180, 250)),
            ("eq_zero_line", rgb(69, 71, 90)),
            ("help_key", rgb(243, 139, 168)),
            ("help_header", rgb(166, 227, 161)),
            ("help_text", rgb(205, 214, 244)),
            ("help_subtext", rgb(166, 173, 200)),
            ("help_overlay_bg", rgb(69, 71, 90)),
            ("help_border", rgb(88, 91, 112)),
            ("badge_lossless", rgb(166, 227, 161)),
            ("badge_lossy", rgb(137, 180, 250)),
            ("badge_detail_bg", rgb(69, 71, 90)),
            // Drawn in these, under other names or none.
            ("bg_main", Color::Reset),
            ("border_panel", Color::Reset),
            ("bg_dim", rgb(10, 10, 15)),
            ("fg_selected", Color::Black),
            ("dsp_section_border", rgb(97, 175, 239)),
            ("eq_bar_empty", rgb(69, 71, 90)),
            ("visualizer_low", rgb(40, 90, 210)),
            ("visualizer_high", rgb(140, 210, 255)),
        ]
    }

    #[test]
    fn the_default_is_drawn_in_the_colours_it_always_was() {
        let mut theme = Theme::default();
        for (key, was) in as_it_was() {
            assert_eq!(*theme.colour_mut(key).unwrap(), was, "{key}");
        }
    }

    #[test]
    fn every_built_in_theme_is_there_and_its_text_can_be_read() {
        let mut unreadable = Vec::new();
        for &name in melotui_core::theme::BUILT_IN {
            let theme = Theme::built_in(name).unwrap_or_else(|| panic!("{name} is missing"));
            let ground = theme.palette.ground();
            // Text on what it is drawn on: WCAG's 4.5 for body text, 3 for
            // what is set apart, like a selection's.
            let mut read = |what: &str, fg: Color, bg: Color, least: f32| {
                let ratio = contrast(fg, bg);
                if ratio < least {
                    unreadable.push(format!("{name}: {what} at {ratio:.2}"));
                }
            };
            read("text", theme.fg_primary, ground, 4.5);
            // Solarized is lower in contrast than WCAG asks by design, and
            // is kept as it is: its text on its own highlight is 4.4.
            let on_selection = if name == "solarized_light" { 4.0 } else { 4.5 };
            read(
                "text on a selection",
                theme.fg_primary,
                theme.bg_selected,
                on_selection,
            );
            read(
                "a selection's text",
                theme.fg_selected,
                theme.accent_primary,
                3.0,
            );
            read("secondary text", theme.fg_secondary, ground, 3.0);
            // The visualizer stands out from what it is drawn on, at its
            // faintest too.
            read("the visualizer's low", theme.visualizer_low, ground, 2.0);
            read("the visualizer's middle", theme.visualizer_mid, ground, 3.0);
            read("the visualizer's high", theme.visualizer_high, ground, 3.0);
        }
        assert!(unreadable.is_empty(), "{unreadable:#?}");
        // And nothing built in that core does not know, or the reverse.
        for name in melotui_core::theme::BUILT_IN {
            assert!(
                *name == "catppuccin_mocha" || built_in_palette(name).is_some(),
                "{name}"
            );
        }
    }

    #[test]
    fn a_palette_alone_makes_a_whole_theme() {
        let (theme, problems) = build(&choice(
            "mine",
            &[
                ("background", "#101010"),
                ("foreground", "#eeeeee"),
                ("accent", "#ff8800"),
                ("green", "#00ff00"),
            ],
        ));
        assert!(problems.is_empty(), "{problems:?}");
        assert_eq!(theme.bg_main, Color::Rgb(16, 16, 16));
        assert_eq!(theme.border_active, Color::Rgb(255, 136, 0));
        assert_eq!(theme.eq_bar_selected, Color::Rgb(255, 136, 0));
        assert_eq!(theme.slider_volume, Color::Rgb(0, 255, 0));
        assert_eq!(theme.status_playing, Color::Rgb(0, 255, 0));
        // What it does not give is the default's palette.
        assert_eq!(theme.accent_error, Theme::default().palette.red);
    }

    #[test]
    fn a_file_named_for_a_built_in_theme_changes_that_one() {
        // Only some colours, as the old theme files give them: the rest are
        // Nord's own, not the default's.
        let (theme, problems) = build(&choice("nord", &[("accent_primary", "#ff0000")]));
        assert!(problems.is_empty(), "{problems:?}");
        let nord = Theme::built_in("nord").unwrap();
        assert_eq!(theme.accent_primary, Color::Rgb(255, 0, 0));
        assert_eq!(theme.slider_pan, nord.slider_pan);
        assert_eq!(theme.eq_bar_fill, nord.eq_bar_fill);
        // And one with another name, on the one it says.
        let (theme, _) = build(&choice("mine", &[("base", "dracula"), ("red", "#ff0000")]));
        assert_eq!(theme.bg_main, Theme::built_in("dracula").unwrap().bg_main);
        assert_eq!(theme.accent_error, Color::Rgb(255, 0, 0));
    }

    #[test]
    fn what_is_wrong_with_a_theme_file_is_said() {
        let (theme, problems) = build(&choice(
            "mine",
            &[
                ("base", "solarized_purple"),
                ("acent", "#ff0000"),
                ("accent", "not a colour"),
                ("fg_primary", "#123456"),
            ],
        ));
        assert_eq!(
            problems,
            [
                "`base`: there is no built-in theme `solarized_purple`",
                "`accent`: `not a colour` is not a colour",
                "`acent` is not a colour a theme has",
            ]
        );
        // What could be taken is.
        assert_eq!(theme.fg_primary, Color::Rgb(0x12, 0x34, 0x56));
    }

    #[test]
    fn colours_mix_and_compare_as_they_should() {
        assert_eq!(
            blend(Color::Rgb(0, 0, 0), Color::Rgb(255, 255, 255), 0.5),
            Color::Rgb(128, 128, 128)
        );
        assert!((contrast(Color::Rgb(0, 0, 0), Color::Rgb(255, 255, 255)) - 21.0).abs() < 0.01);
        assert!((contrast(Color::Red, Color::Red) - 1.0).abs() < 1e-6);
        // Dark text on a light accent, light text on a dark one.
        let (dark, light) = (Color::Rgb(20, 20, 20), Color::Rgb(240, 240, 240));
        assert_eq!(
            more_legible_on(Color::Rgb(250, 220, 120), dark, light),
            dark
        );
        assert_eq!(more_legible_on(Color::Rgb(30, 60, 140), dark, light), light);
    }

    #[test]
    fn every_colour_and_palette_name_can_be_given_in_a_file() {
        let mut theme = Theme::default();
        for key in THEME_KEYS {
            assert!(theme.colour_mut(key).is_some(), "{key}");
        }
        let mut palette = theme.palette;
        for key in PALETTE_KEYS {
            assert!(palette.colour_mut(key).is_some(), "{key}");
            assert!(!THEME_KEYS.contains(key), "{key} names two colours");
        }
    }

    #[test]
    fn the_themes_written_up_are_the_themes_there_are() {
        let written = include_str!("../../../CONFIG.md").replace("\r\n", "\n");
        // The palette example is a whole theme, every colour taken.
        let from = written
            .find("```toml\nbackground")
            .expect("the example is gone")
            + 8;
        let example = &written[from..from + written[from..].find("```").unwrap()];
        let file: std::collections::HashMap<String, String> = toml::from_str(example).unwrap();
        assert_eq!(
            file.len(),
            PALETTE_KEYS.len(),
            "a palette colour is left out"
        );
        let (_, problems) = build(&ThemeChoice {
            name: "mine".into(),
            file: Some(file),
            generation: 0,
        });
        assert!(problems.is_empty(), "{problems:?}");
        // Every colour's name is in the table, and every built-in theme's.
        for key in THEME_KEYS {
            assert!(
                written.contains(&format!("`{key}`")),
                "{key} is not written up"
            );
        }
        for name in melotui_core::theme::BUILT_IN {
            assert!(
                written.contains(&format!("`{name}`")),
                "{name} is not written up"
            );
        }
    }
}
