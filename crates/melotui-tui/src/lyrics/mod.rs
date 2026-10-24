//! Lyrics in scripts the terminal grid cannot draw.
//!
//! A terminal is a grid of equal cells, and it lays text out one grapheme
//! cluster per cell. That works for Latin and CJK. It cannot work for
//! Devanagari, where a conjunct is a single cluster but anything up to three
//! cells wide, vowel signs are drawn before the consonant they follow, and the
//! headline runs unbroken across a word. kitty gives every Hindi cluster
//! exactly one cell — `स्ते` included — while ratatui, counting code points,
//! gives the same text more: one line of lyrics measured 27 columns by
//! ratatui's count and 22 by kitty's, and the difference came out as text
//! drawn over itself. No whole number of cells is right either: padding each
//! cluster to whole cells, as kitty's text sizing protocol (OSC 66) allows,
//! keeps the grid aligned but breaks the headline at every cluster.
//!
//! Right-to-left scripts fail differently. The terminal lays every line out
//! left to right, and few terminals reorder anything, so Urdu came out
//! backwards with its letters unjoined.
//!
//! So lyrics containing any such script are drawn by a real text engine —
//! cosmic-text, shaping with a port of HarfBuzz and doing its own bidi — into
//! an image of the whole panel, shown through the terminal's graphics
//! protocol, where the grid no longer constrains anything. The same goes for
//! lyrics in any script once they are zoomed (`ui.lyrics_zoom`), since only an
//! image can draw text larger than the terminal's; and one image for the
//! panel, rather than one a line, lets lines sit a fraction of a row apart, so
//! the panel fits as many as its height allows.
//!
//! - kitty's protocol (kitty, Ghostty) and iTerm2's (WezTerm, iTerm2, Rio,
//!   mintty, VS Code, Warp) keep the image's transparency, so the lyrics sit
//!   on the terminal's own background.
//! - Sixel (Windows Terminal, foot, Konsole, xterm) has none, so the text is
//!   blended onto the background colour the terminal reports — and if it
//!   reports none, lyrics stay text.
//! - Anything else, or `ui.lyrics_shaping` turned off, and lyrics stay text,
//!   with right-to-left lines put into visual order for the terminal.

pub mod bidi;
mod kitty;
mod script;
mod shaper;

use std::collections::HashMap;
use std::io::Write;

use image::{DynamicImage, Rgba, RgbaImage};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui_image::picker::{Picker, ProtocolType};
use ratatui_image::protocol::Protocol;
use ratatui_image::{Image, Resize};

use kitty::KittyImage;
pub use script::needs_shaping;
use shaper::Shaper;

/// How the panel's image reaches the terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Route {
    /// kitty's protocol, sent as PNG by [`kitty`].
    Kitty,
    /// iTerm2's protocol, which keeps transparency.
    Transparent,
    /// Sixel, onto this background colour.
    Opaque([u8; 3]),
}

/// What the panel's image was drawn from: when any of it changes, the image
/// is drawn again.
#[derive(Clone, PartialEq, Eq, Hash)]
struct PanelKey {
    /// Each line shown: its text, colour, and the slot it starts in.
    lines: Vec<(String, [u8; 3], usize)>,
    columns: u16,
    rows: u16,
    cell: (u16, u16),
    slots: usize,
    route: Route,
}

/// What a line's height in slots depends on.
#[derive(Clone, PartialEq, Eq, Hash)]
struct CountKey {
    text: String,
    width: u32,
    /// The pitch's bits, so it can be hashed.
    pitch: u32,
}

/// Line counts remembered before the cache is started afresh; a few songs'
/// worth.
const MAX_COUNTS: usize = 2048;

enum Shown {
    Kitty(KittyImage),
    Other(Protocol),
}

/// The panel as last drawn.
struct Panel {
    key: PanelKey,
    shown: Shown,
    /// The image's id in kitty's store, so it can be freed when it goes.
    kitty_id: Option<u32>,
}

/// Draws the lyrics panel as an image, and keeps track of it.
pub struct LyricsRenderer {
    enabled: bool,
    shaper: Shaper,
    panel: Option<Panel>,
    counts: HashMap<CountKey, usize>,
    /// kitty images that have left the screen, freed once the frame is drawn.
    retired: Vec<u32>,
    next_id: u32,
    is_tmux: bool,
    /// The terminal's background colour, for Sixel, if it said.
    background: Option<[u8; 3]>,
}

impl LyricsRenderer {
    pub fn new(enabled: bool) -> Self {
        // Image ids are shared with the album art, which picks them at random,
        // so these start somewhere arbitrary too rather than at 1. They stay
        // below 2^24 so a placeholder's colour carries the whole id.
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.subsec_nanos())
            ^ std::process::id().rotate_left(16);
        Self {
            enabled,
            shaper: Shaper::new(),
            panel: None,
            counts: HashMap::new(),
            retired: Vec::new(),
            next_id: (seed & 0xFF_FFFF).max(1),
            is_tmux: std::env::var_os("TMUX").is_some(),
            background: None,
        }
    }

    /// The terminal's background colour, which Sixel needs the text blended
    /// onto. Without it, Sixel terminals get the lyrics as text.
    pub fn set_background(&mut self, background: Option<[u8; 3]>) {
        self.background = background;
    }

    fn route(&self, picker: &Picker) -> Option<Route> {
        match picker.protocol_type() {
            ProtocolType::Kitty => Some(Route::Kitty),
            ProtocolType::Iterm2 => Some(Route::Transparent),
            ProtocolType::Sixel => self.background.map(Route::Opaque),
            ProtocolType::Halfblocks => None,
        }
    }

    /// Whether lyrics made of `lines`, at `zoom` times the text size, should
    /// be drawn as an image: when some script in them needs it, or when they
    /// are zoomed, which only an image can do — and only where the terminal
    /// can show one over its own background. The fonts load the first time
    /// this answers yes; until they are in, it answers no.
    pub fn should_shape(&mut self, picker: &Picker, lines: &[&str], zoom: f32) -> bool {
        let (cell_w, cell_h) = picker.font_size();
        self.enabled
            && self.route(picker).is_some()
            && cell_w > 0
            && cell_h > 0
            && (zoom > 1.0 || lines.iter().any(|line| needs_shaping(line)))
            && self.shaper.ready()
    }

    /// How many slots `text` takes, `width` pixels wide with lines `pitch`
    /// pixels apart.
    fn count(&mut self, text: &str, width: u32, pitch: f32) -> usize {
        if text.trim().is_empty() {
            return 1;
        }
        let key = CountKey {
            text: text.to_owned(),
            width,
            pitch: pitch.to_bits(),
        };
        if let Some(&count) = self.counts.get(&key) {
            return count;
        }
        let count = self.shaper.lines(text, width, pitch).unwrap_or(1);
        if self.counts.len() >= MAX_COUNTS {
            self.counts.clear();
        }
        self.counts.insert(key, count);
        count
    }

    /// Draws `lines` into `area` as one image, line `active` centred, at as
    /// close to `zoom` times the text size as whole lines fill the panel.
    /// `false` if it cannot, and the lyrics should be drawn as text instead.
    pub fn draw(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        picker: &Picker,
        lines: &[(&str, Color)],
        active: Option<usize>,
        zoom: f32,
    ) -> bool {
        let Some(route) = self.route(picker) else {
            return false;
        };
        let (cell_w, cell_h) = picker.font_size();
        if lines.is_empty() || area.is_empty() || cell_w == 0 || cell_h == 0 {
            return false;
        }
        let width = u32::from(area.width) * u32::from(cell_w);
        let height = u32::from(area.height) * u32::from(cell_h);
        let anchor = active.unwrap_or(0).min(lines.len() - 1);

        // As many slots as the zoom asks for, rounded to fill the panel; more,
        // smaller ones if the line being sung would not fit.
        let most = usize::from(area.height) * 2;
        let mut slots = ((f32::from(area.height) / zoom).round() as usize).clamp(1, most);
        let (pitch, placed) = loop {
            let pitch = height as f32 / slots as f32;
            let count = |i: usize| self.count(lines[i].0, width, pitch);
            if let Some(placed) = place(count, lines.len(), anchor, slots) {
                break (pitch, placed);
            }
            if slots >= most {
                return false;
            }
            slots += 1;
        };

        let key = PanelKey {
            lines: placed
                .iter()
                .filter(|(i, _)| !lines[*i].0.trim().is_empty())
                .map(|&(i, slot)| (lines[i].0.to_owned(), rgb(lines[i].1), slot))
                .collect(),
            columns: area.width,
            rows: area.height,
            cell: (cell_w, cell_h),
            slots,
            route,
        };
        if self.panel.as_ref().is_none_or(|panel| panel.key != key) {
            let texts: Vec<(&str, [u8; 3], usize)> = key
                .lines
                .iter()
                .map(|(text, color, slot)| (text.as_str(), *color, *slot))
                .collect();
            let Some(image) = self.shaper.render(&texts, width, height, pitch) else {
                return false;
            };
            let full = Rect::new(0, 0, area.width, area.height);
            let (shown, kitty_id) = match route {
                Route::Kitty => {
                    let id = self.allocate_id();
                    match KittyImage::new(&image, id, area.width, area.height, self.is_tmux) {
                        Some(kitty) => (Shown::Kitty(kitty), Some(id)),
                        None => return false,
                    }
                }
                Route::Transparent => {
                    let image = DynamicImage::ImageRgba8(image);
                    match picker.new_protocol(image, full, Resize::Fit(None)) {
                        Ok(protocol) => (Shown::Other(protocol), None),
                        Err(_) => return false,
                    }
                }
                Route::Opaque(background) => {
                    let image = DynamicImage::ImageRgba8(onto(image, background));
                    match picker.new_protocol(image, full, Resize::Fit(None)) {
                        Ok(protocol) => (Shown::Other(protocol), None),
                        Err(_) => return false,
                    }
                }
            };
            self.clear();
            self.panel = Some(Panel {
                key,
                shown,
                kitty_id,
            });
        }

        match &self.panel.as_ref().expect("just drawn").shown {
            Shown::Kitty(kitty) => frame.render_widget(kitty, area),
            Shown::Other(protocol) => frame.render_widget(Image::new(protocol), area),
        }
        true
    }

    /// Drops the panel's image, so it is drawn afresh next time — for frames
    /// that draw the lyrics as text, and after something covered them.
    pub fn clear(&mut self) {
        if let Some(panel) = self.panel.take() {
            self.retired.extend(panel.kitty_id);
        }
    }

    /// Frees the kitty images that have left the screen.
    ///
    /// Called between frames, when nothing else is being written. Without it,
    /// every panel shown — one each time the line being sung changes — would
    /// stay in kitty's memory until its storage quota pushed it out.
    pub fn flush_retired(&mut self, out: &mut impl Write) -> std::io::Result<()> {
        if self.retired.is_empty() {
            return Ok(());
        }
        for id in self.retired.drain(..) {
            out.write_all(kitty::delete(id, self.is_tmux).as_bytes())?;
        }
        out.flush()
    }

    /// Blocks until the fonts are loaded. For tests.
    #[cfg(test)]
    pub fn wait_for_fonts(&mut self) {
        self.shaper.wait_until_ready();
    }

    /// How many lines the panel shows, or `None` with no image. For tests.
    #[cfg(test)]
    pub fn shown_lines(&self) -> Option<usize> {
        self.panel.as_ref().map(|panel| panel.key.lines.len())
    }

    fn allocate_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id = self.next_id % 0xFF_FFFF + 1;
        id
    }
}

impl Drop for LyricsRenderer {
    fn drop(&mut self) {
        self.clear();
        let mut out = std::io::stdout();
        if std::io::IsTerminal::is_terminal(&out) {
            let _ = self.flush_retired(&mut out);
        }
    }
}

/// Where each line that fits goes, as (line, first slot), with line `anchor`
/// centred, unless that would leave the panel empty above the first line.
/// `count` says how many slots a line takes. Only whole lines are placed;
/// `None` if `anchor` itself does not fit.
fn place(
    mut count: impl FnMut(usize) -> usize,
    total: usize,
    anchor: usize,
    slots: usize,
) -> Option<Vec<(usize, usize)>> {
    let anchor_slots = count(anchor);
    if anchor_slots > slots {
        return None;
    }
    let centred = (slots - anchor_slots) / 2;
    let mut above = 0;
    for i in (0..anchor).rev() {
        above += count(i);
        if above >= centred {
            break;
        }
    }
    let top = centred.min(above);

    let mut placed = vec![(anchor, top)];
    let mut slot = top;
    for i in (0..anchor).rev() {
        let n = count(i);
        if n > slot {
            break;
        }
        slot -= n;
        placed.push((i, slot));
    }
    let mut slot = top + anchor_slots;
    for i in anchor + 1..total {
        let n = count(i);
        if slot + n > slots {
            break;
        }
        placed.push((i, slot));
        slot += n;
    }
    placed.sort_by_key(|&(i, _)| i);
    Some(placed)
}

/// `image` laid over an opaque `background`.
fn onto(image: RgbaImage, [r, g, b]: [u8; 3]) -> RgbaImage {
    let mut out = RgbaImage::from_pixel(image.width(), image.height(), Rgba([r, g, b, 255]));
    for (src, dst) in image.pixels().zip(out.pixels_mut()) {
        let a = u32::from(src[3]);
        for c in 0..3 {
            dst[c] = ((u32::from(src[c]) * a + u32::from(dst[c]) * (255 - a)) / 255) as u8;
        }
    }
    out
}

/// `color` as RGB. Named and indexed colours are whatever the terminal's
/// palette says they are, which an image cannot ask, so they are taken from
/// xterm's defaults.
fn rgb(color: Color) -> [u8; 3] {
    const BASE: [[u8; 3]; 16] = [
        [0, 0, 0],
        [205, 0, 0],
        [0, 205, 0],
        [205, 205, 0],
        [0, 0, 238],
        [205, 0, 205],
        [0, 205, 205],
        [229, 229, 229],
        [127, 127, 127],
        [255, 0, 0],
        [0, 255, 0],
        [255, 255, 0],
        [92, 92, 255],
        [255, 0, 255],
        [0, 255, 255],
        [255, 255, 255],
    ];
    let indexed = |i: u8| match i {
        0..=15 => BASE[usize::from(i)],
        16..=231 => {
            let level = |v: u8| if v == 0 { 0 } else { 55 + v * 40 };
            let i = i - 16;
            [level(i / 36), level(i / 6 % 6), level(i % 6)]
        }
        _ => {
            let v = 8 + (i - 232) * 10;
            [v, v, v]
        }
    };
    match color {
        Color::Rgb(r, g, b) => [r, g, b],
        Color::Indexed(i) => indexed(i),
        Color::Black => BASE[0],
        Color::Red => BASE[1],
        Color::Green => BASE[2],
        Color::Yellow => BASE[3],
        Color::Blue => BASE[4],
        Color::Magenta => BASE[5],
        Color::Cyan => BASE[6],
        Color::Gray => BASE[7],
        Color::DarkGray => BASE[8],
        Color::LightRed => BASE[9],
        Color::LightGreen => BASE[10],
        Color::LightYellow => BASE[11],
        Color::LightBlue => BASE[12],
        Color::LightMagenta => BASE[13],
        Color::LightCyan => BASE[14],
        Color::White | Color::Reset => BASE[15],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

    fn picker(protocol: ProtocolType) -> Picker {
        let mut picker = Picker::halfblocks();
        picker.set_protocol_type(protocol);
        picker
    }

    fn ready(enabled: bool) -> LyricsRenderer {
        let mut r = LyricsRenderer::new(enabled);
        r.shaper.wait_until_ready();
        r
    }

    const SONG: [(&str, Color); 5] = [
        ("कभी कभी मेरे दिल में", Color::Rgb(108, 112, 134)),
        ("ख़याल आता है", Color::Rgb(137, 180, 250)),
        ("تمہارے بغیر ہم", Color::Rgb(186, 194, 222)),
        ("Never gonna give you up", Color::Rgb(186, 194, 222)),
        ("तुम ही हो", Color::Rgb(186, 194, 222)),
    ];

    /// Draws `lines` into a `rows`-row panel 40 columns wide, returning
    /// whether it was an image, and the frame.
    fn draw(
        r: &mut LyricsRenderer,
        picker: &Picker,
        lines: &[(&str, Color)],
        active: Option<usize>,
        rows: u16,
        zoom: f32,
    ) -> (bool, Buffer) {
        let mut terminal = Terminal::new(TestBackend::new(40, rows)).unwrap();
        let mut drawn = false;
        let frame = terminal
            .draw(|f| drawn = r.draw(f, f.area(), picker, lines, active, zoom))
            .unwrap();
        (drawn, frame.buffer.clone())
    }

    #[test]
    fn which_terminals_get_images() {
        let mut r = ready(true);
        let hindi = ["तुम ही हो"];
        assert!(r.should_shape(&picker(ProtocolType::Kitty), &hindi, 1.0));
        assert!(r.should_shape(&picker(ProtocolType::Iterm2), &hindi, 1.0));
        // Sixel only once the terminal has said what its background is.
        assert!(!r.should_shape(&picker(ProtocolType::Sixel), &hindi, 1.0));
        r.set_background(Some([30, 30, 46]));
        assert!(r.should_shape(&picker(ProtocolType::Sixel), &hindi, 1.0));
        // Half blocks cannot draw text at text size.
        assert!(!r.should_shape(&picker(ProtocolType::Halfblocks), &hindi, 2.0));
    }

    #[test]
    fn what_gets_images() {
        let mut r = ready(true);
        let kitty = picker(ProtocolType::Kitty);
        assert!(r.should_shape(&kitty, &["Never gonna", "तुम ही हो"], 1.0));
        assert!(r.should_shape(&kitty, &["تمہارے بغیر"], 1.0));
        assert!(!r.should_shape(&kitty, &["Never gonna give you up", "君の名は"], 1.0));
        // Zoomed, anything: the terminal cannot enlarge text.
        assert!(r.should_shape(&kitty, &["Never gonna give you up"], 2.0));
        assert!(
            !ready(false).should_shape(&kitty, &["तुम ही हो"], 2.0),
            "turned off"
        );
    }

    #[test]
    fn the_panel_fits_as_many_lines_as_its_height_allows() {
        // Five rows at 2x: two lines two rows tall would leave a row empty;
        // three, 1.67 rows apart, fill it — the line being sung, and one
        // either side of it.
        let mut r = ready(true);
        let (drawn, _) = draw(&mut r, &picker(ProtocolType::Kitty), &SONG, Some(1), 5, 2.0);
        assert!(drawn);
        assert_eq!(r.shown_lines(), Some(3));
        let key = &r.panel.as_ref().unwrap().key;
        let slots: Vec<usize> = key.lines.iter().map(|(_, _, slot)| *slot).collect();
        assert_eq!((key.slots, slots), (3, vec![0, 1, 2]));
        assert_eq!(
            key.lines[1].0, SONG[1].0,
            "the line being sung is in the middle"
        );

        // Twelve rows at 2x: six lines.
        draw(
            &mut r,
            &picker(ProtocolType::Kitty),
            &SONG,
            Some(1),
            12,
            2.0,
        );
        assert_eq!(r.panel.as_ref().unwrap().key.slots, 6);
    }

    #[test]
    fn kitty_gets_one_png_image_and_redraws_only_when_the_lyrics_move_on() {
        let mut r = ready(true);
        let kitty = picker(ProtocolType::Kitty);
        let (_, buffer) = draw(&mut r, &kitty, &SONG, Some(1), 6, 2.0);
        let first = buffer[(0, 0)].symbol();
        assert!(first.contains("f=100"), "not sent as PNG");
        for y in 0..6 {
            assert!(buffer[(0, y)].symbol().contains('\u{10EEEE}'), "row {y}");
            assert!(buffer[(1, y)].skip, "row {y}");
        }
        let id = r.panel.as_ref().unwrap().kitty_id;

        draw(&mut r, &kitty, &SONG, Some(1), 6, 2.0);
        assert_eq!(
            r.panel.as_ref().unwrap().kitty_id,
            id,
            "drawn again unchanged"
        );
        assert!(r.retired.is_empty());

        draw(&mut r, &kitty, &SONG, Some(2), 6, 2.0);
        assert_ne!(r.panel.as_ref().unwrap().kitty_id, id);
        assert_eq!(r.retired, vec![id.unwrap()], "the old image is freed");
        let mut out = Vec::new();
        r.flush_retired(&mut out).unwrap();
        assert_eq!(
            String::from_utf8(out).unwrap().matches("a=d,d=I").count(),
            1
        );
    }

    #[test]
    fn iterm2_and_sixel_get_their_own_protocols() {
        let mut r = ready(true);
        let (drawn, buffer) = draw(
            &mut r,
            &picker(ProtocolType::Iterm2),
            &SONG,
            Some(1),
            6,
            2.0,
        );
        assert!(drawn);
        assert!(
            buffer[(0, 0)].symbol().contains("]1337;File="),
            "{:?}",
            &buffer[(0, 0)].symbol()[..20]
        );

        let sixel = picker(ProtocolType::Sixel);
        let (drawn, _) = draw(&mut r, &sixel, &SONG, Some(1), 6, 2.0);
        assert!(!drawn, "no background colour, no sixel");
        r.set_background(Some([30, 30, 46]));
        let (drawn, buffer) = draw(&mut r, &sixel, &SONG, Some(1), 6, 2.0);
        assert!(drawn);
        assert!(buffer[(0, 0)].symbol().contains("\x1bP"), "not a sixel");
    }

    #[test]
    fn half_blocks_and_impossible_panels_fall_back_to_text() {
        let mut r = ready(true);
        let (drawn, _) = draw(
            &mut r,
            &picker(ProtocolType::Halfblocks),
            &SONG,
            Some(1),
            6,
            2.0,
        );
        assert!(!drawn);
        let (drawn, _) = draw(&mut r, &picker(ProtocolType::Kitty), &[], None, 6, 2.0);
        assert!(!drawn, "no lyrics");
    }

    #[test]
    fn sixel_text_is_blended_onto_the_background() {
        let mut image = RgbaImage::new(2, 1);
        image.put_pixel(0, 0, Rgba([255, 255, 255, 255]));
        image.put_pixel(1, 0, Rgba([255, 255, 255, 51]));
        let out = onto(image, [0, 0, 100]);
        assert_eq!(out.get_pixel(0, 0), &Rgba([255, 255, 255, 255]));
        assert_eq!(out.get_pixel(1, 0), &Rgba([51, 51, 131, 255]));
    }

    #[test]
    fn placing_centres_the_line_being_sung_with_whole_lines_around_it() {
        let ones = |_| 1;
        assert_eq!(place(ones, 10, 5, 3), Some(vec![(4, 0), (5, 1), (6, 2)]));
        // At the start of the song there is nothing above to centre against.
        assert_eq!(place(ones, 10, 0, 3), Some(vec![(0, 0), (1, 1), (2, 2)]));
        // A two-slot line before the one being sung does not fit in one slot.
        let counts = [1, 2, 1, 1];
        assert_eq!(place(|i| counts[i], 4, 2, 3), Some(vec![(2, 1), (3, 2)]));
        // The line being sung does not fit at all.
        assert_eq!(place(|_| 4, 3, 1, 3), None);
    }

    #[test]
    fn named_colours_get_sensible_rgb() {
        assert_eq!(rgb(Color::Rgb(1, 2, 3)), [1, 2, 3]);
        assert_eq!(rgb(Color::Indexed(196)), [255, 0, 0]);
        assert_eq!(rgb(Color::Indexed(244)), [128, 128, 128]);
        assert_eq!(rgb(Color::DarkGray), [127, 127, 127]);
    }
}
