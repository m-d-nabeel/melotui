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
//! images shown through the terminal's graphics protocol, where the grid no
//! longer constrains anything. The same goes for lyrics in any script once
//! they are zoomed (`ui.lyrics_zoom`), since only an image can draw text
//! larger than the terminal's. Without a graphics protocol that can draw them
//! over the terminal's own background, or with `ui.lyrics_shaping` turned off,
//! lyrics stay text, with right-to-left lines put into visual order for the
//! terminal.

pub mod bidi;
mod script;
mod shaper;

use std::collections::HashMap;
use std::io::Write;

use image::DynamicImage;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui_image::Resize;
use ratatui_image::picker::{Picker, ProtocolType};
use ratatui_image::protocol::Protocol;
use ratatui_image::protocol::kitty::Kitty;

pub use script::needs_shaping;
use shaper::Shaper;

#[derive(Clone, PartialEq, Eq, Hash)]
struct LineKey {
    text: String,
    color: [u8; 3],
    columns: u16,
    cell: (u16, u16),
    /// The zoom's bits, so it can be hashed.
    zoom: u32,
}

/// What a line's height depends on.
#[derive(Clone, PartialEq, Eq, Hash)]
struct HeightKey {
    text: String,
    columns: u16,
    cell: (u16, u16),
    zoom: u32,
}

/// Heights remembered before the cache is started afresh; a few songs' worth.
const MAX_HEIGHTS: usize = 2048;

/// One rendered line, kept for as long as it stays on screen.
struct Rendered {
    protocol: Protocol,
    rows: u16,
    /// The image's id in kitty's store, so it can be freed when it goes.
    kitty_id: Option<u32>,
    frame: u64,
}

/// Draws lyric lines as shaped images, and keeps track of them.
pub struct LyricsRenderer {
    enabled: bool,
    shaper: Shaper,
    lines: HashMap<LineKey, Rendered>,
    heights: HashMap<HeightKey, u16>,
    frame: u64,
    /// kitty images that have left the screen, freed once the frame is drawn.
    retired: Vec<u32>,
    next_id: u32,
    is_tmux: bool,
}

impl LyricsRenderer {
    pub fn new(enabled: bool) -> Self {
        // Image ids are shared with the album art, which picks them at random,
        // so these start somewhere arbitrary too rather than at 1.
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.subsec_nanos())
            ^ std::process::id().rotate_left(16);
        Self {
            enabled,
            shaper: Shaper::new(),
            lines: HashMap::new(),
            heights: HashMap::new(),
            frame: 0,
            retired: Vec::new(),
            next_id: seed | 1,
            is_tmux: std::env::var_os("TMUX").is_some(),
        }
    }

    /// Whether lyrics made of `lines`, at `zoom` times the text size, should
    /// be drawn by the text engine: when some script in them needs it, or
    /// when they are zoomed, which only an image can do.
    ///
    /// Only through a protocol that keeps the image's transparency (kitty's
    /// and iTerm2's), since the lyrics sit on the terminal's own background
    /// and the theme does not say what colour that is. The fonts load the
    /// first time this answers yes; until they are in, it answers no.
    pub fn should_shape(&mut self, picker: &Picker, lines: &[&str], zoom: f32) -> bool {
        let (cell_w, cell_h) = picker.font_size();
        self.enabled
            && matches!(
                picker.protocol_type(),
                ProtocolType::Kitty | ProtocolType::Iterm2
            )
            && cell_w > 0
            && cell_h > 0
            && (zoom > 1.0 || lines.iter().any(|line| needs_shaping(line)))
            && self.shaper.ready()
    }

    /// Starts a frame of shaped lyrics.
    pub fn begin(&mut self) {
        self.frame += 1;
    }

    /// How many rows one line takes, `columns` cells wide at `zoom` times
    /// the text size — more than one when zoomed, or when it has to wrap.
    pub fn rows(&mut self, picker: &Picker, text: &str, columns: u16, zoom: f32) -> Option<u16> {
        let (cell_w, cell_h) = picker.font_size();
        let key = HeightKey {
            text: text.to_owned(),
            columns,
            cell: (cell_w, cell_h),
            zoom: zoom.to_bits(),
        };
        if let Some(&rows) = self.heights.get(&key) {
            return Some(rows);
        }
        let rows = self.shaper.rows(
            text,
            u32::from(columns) * u32::from(cell_w),
            u32::from(cell_h),
            zoom,
        )?;
        if self.heights.len() >= MAX_HEIGHTS {
            self.heights.clear();
        }
        self.heights.insert(key, rows);
        Some(rows)
    }

    /// The image of one line, `columns` cells wide, in `color` at `zoom`
    /// times the text size, with the number of rows it fills. Rendered once
    /// and reused for as long as it stays on screen.
    pub fn line(
        &mut self,
        picker: &Picker,
        text: &str,
        color: Color,
        columns: u16,
        zoom: f32,
    ) -> Option<(&Protocol, u16)> {
        let (cell_w, cell_h) = picker.font_size();
        let key = LineKey {
            text: text.to_owned(),
            color: rgb(color),
            columns,
            cell: (cell_w, cell_h),
            zoom: zoom.to_bits(),
        };
        if !self.lines.contains_key(&key) {
            let (image, rows) = self.shaper.render(
                text,
                key.color,
                u32::from(columns) * u32::from(cell_w),
                u32::from(cell_h),
                zoom,
            )?;
            let image = DynamicImage::ImageRgba8(image);
            let area = Rect::new(0, 0, columns, rows);
            let (protocol, kitty_id) = if picker.protocol_type() == ProtocolType::Kitty {
                let id = self.allocate_id();
                let kitty = Kitty::new(image, area, id, self.is_tmux).ok()?;
                (Protocol::Kitty(kitty), Some(id))
            } else {
                let protocol = picker.new_protocol(image, area, Resize::Fit(None)).ok()?;
                (protocol, None)
            };
            self.lines.insert(
                key.clone(),
                Rendered {
                    protocol,
                    rows,
                    kitty_id,
                    frame: self.frame,
                },
            );
        }
        let rendered = self.lines.get_mut(&key)?;
        rendered.frame = self.frame;
        Some((&rendered.protocol, rendered.rows))
    }

    /// Ends the frame: lines that were not drawn in it are dropped, and their
    /// kitty images freed after the frame.
    pub fn end(&mut self) {
        let frame = self.frame;
        let retired = &mut self.retired;
        self.lines.retain(|_, rendered| {
            let on_screen = rendered.frame == frame;
            if !on_screen {
                retired.extend(rendered.kitty_id);
            }
            on_screen
        });
    }

    /// Drops every rendered line, for frames that draw the lyrics as text.
    pub fn clear(&mut self) {
        self.retired.extend(
            self.lines
                .drain()
                .filter_map(|(_, rendered)| rendered.kitty_id),
        );
    }

    /// Frees the kitty images of lines that have left the screen.
    ///
    /// Called between frames, when nothing else is being written. Without it,
    /// every line of every song shown — each in up to three colours as it is
    /// sung — would stay in kitty's memory until its storage quota pushed it
    /// out.
    pub fn flush_retired(&mut self, out: &mut impl Write) -> std::io::Result<()> {
        if self.retired.is_empty() {
            return Ok(());
        }
        for id in self.retired.drain(..) {
            let delete = format!("\x1b_Ga=d,d=I,i={id},q=2\x1b\\");
            if self.is_tmux {
                write!(
                    out,
                    "\x1bPtmux;{}\x1b\\",
                    delete.replace('\x1b', "\x1b\x1b")
                )?;
            } else {
                out.write_all(delete.as_bytes())?;
            }
        }
        out.flush()
    }

    /// Blocks until the fonts are loaded. For tests.
    #[cfg(test)]
    pub fn wait_for_fonts(&mut self) {
        self.shaper.wait_until_ready();
    }

    /// How many lines are currently rendered. For tests.
    #[cfg(test)]
    pub fn rendered_lines(&self) -> usize {
        self.lines.len()
    }

    fn allocate_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1).max(1);
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

    fn kitty_picker() -> Picker {
        let mut picker = Picker::halfblocks();
        picker.set_protocol_type(ProtocolType::Kitty);
        picker
    }

    fn ready(enabled: bool) -> LyricsRenderer {
        let mut r = LyricsRenderer::new(enabled);
        r.shaper.wait_until_ready();
        r
    }

    #[test]
    fn complex_scripts_are_shaped_and_everything_else_is_left_as_text() {
        let mut r = ready(true);
        let picker = kitty_picker();
        assert!(r.should_shape(&picker, &["Never gonna", "तुम ही हो"], 1.0));
        assert!(r.should_shape(&picker, &["تمہارے بغیر"], 1.0));
        assert!(!r.should_shape(&picker, &["Never gonna give you up", "君の名は"], 1.0));
        // Zoomed, anything is drawn as images: the terminal cannot enlarge text.
        assert!(r.should_shape(&picker, &["Never gonna give you up"], 2.0));
    }

    #[test]
    fn nothing_is_shaped_without_a_protocol_that_can_show_it() {
        let mut r = ready(true);
        // Half blocks cannot draw text at text size, and sixel would put a box
        // in some guessed background colour behind every line.
        for protocol in [ProtocolType::Halfblocks, ProtocolType::Sixel] {
            let mut picker = Picker::halfblocks();
            picker.set_protocol_type(protocol);
            assert!(!r.should_shape(&picker, &["तुम ही हो"], 2.0), "{protocol:?}");
        }
    }

    #[test]
    fn turned_off_means_text() {
        let mut r = ready(false);
        assert!(!r.should_shape(&kitty_picker(), &["तुम ही हो"], 2.0));
    }

    #[test]
    fn a_line_is_rendered_once_and_freed_when_it_leaves_the_screen() {
        let mut r = ready(true);
        let picker = kitty_picker();

        r.begin();
        assert!(
            r.line(&picker, "तुम ही हो", Color::Rgb(137, 180, 250), 40, 1.0)
                .is_some()
        );
        assert!(
            r.line(&picker, "अब तुम ही हो", Color::Rgb(186, 194, 222), 40, 1.0)
                .is_some()
        );
        r.end();
        assert_eq!(r.lines.len(), 2);

        // Next frame: the first line is still there, now sung and dimmed, so it
        // is a new image; the old colour and the other line have gone.
        r.begin();
        assert!(
            r.line(&picker, "तुम ही हो", Color::Rgb(108, 112, 134), 40, 1.0)
                .is_some()
        );
        r.end();
        assert_eq!(r.lines.len(), 1);
        assert_eq!(r.retired.len(), 2, "both images that left are freed");

        let mut out = Vec::new();
        r.flush_retired(&mut out).unwrap();
        let out = String::from_utf8(out).unwrap();
        assert_eq!(out.matches("\x1b_Ga=d,d=I,i=").count(), 2, "{out:?}");
        assert!(r.retired.is_empty());
    }

    #[test]
    fn a_line_on_screen_is_not_rendered_again() {
        let mut r = ready(true);
        let picker = kitty_picker();
        let id_of = |r: &LyricsRenderer| r.lines.values().next().and_then(|l| l.kitty_id);
        r.begin();
        r.line(&picker, "तुम ही हो", Color::Rgb(137, 180, 250), 40, 1.0);
        r.end();
        let first = id_of(&r);
        for _ in 0..3 {
            r.begin();
            r.line(&picker, "तुम ही हो", Color::Rgb(137, 180, 250), 40, 1.0);
            r.end();
        }
        assert_eq!(id_of(&r), first, "re-rendered instead of reused");
        assert!(r.retired.is_empty());
    }

    #[test]
    fn named_colours_get_sensible_rgb() {
        assert_eq!(rgb(Color::Rgb(1, 2, 3)), [1, 2, 3]);
        assert_eq!(rgb(Color::Indexed(196)), [255, 0, 0]);
        assert_eq!(rgb(Color::Indexed(244)), [128, 128, 128]);
        assert_eq!(rgb(Color::DarkGray), [127, 127, 127]);
    }

    #[test]
    fn a_zoomed_line_is_as_many_rows_tall_as_its_zoom() {
        let mut r = ready(true);
        let picker = kitty_picker();
        r.begin();
        let (_, rows) = r
            .line(
                &picker,
                "Never gonna give you up",
                Color::Rgb(137, 180, 250),
                60,
                3.0,
            )
            .unwrap();
        r.end();
        assert_eq!(rows, 3);
        assert_eq!(r.rows(&picker, "Never gonna give you up", 60, 3.0), Some(3));
        assert_eq!(r.rows(&picker, "Never gonna give you up", 60, 1.0), Some(1));
        // Each zoom is an image of its own.
        r.begin();
        r.line(
            &picker,
            "Never gonna give you up",
            Color::Rgb(137, 180, 250),
            60,
            2.0,
        );
        r.end();
        assert_eq!(r.retired.len(), 1, "the 3x image was not freed");
    }
}
