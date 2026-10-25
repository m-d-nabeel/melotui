//! Images shown through kitty's graphics protocol, as unicode placeholders.
//!
//! ratatui-image does this too, but sends every image as raw pixels. A lyrics
//! panel is one image the size of the panel, sent again each time the line
//! being sung changes; raw, that is megabytes a line in the full lyrics view,
//! where as PNG — text on a transparent background — it is a few tens of
//! kilobytes.

use std::cell::RefCell;
use std::io::Cursor;

use base64::Engine;
use image::RgbaImage;
use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use ratatui::{
    buffer::{Buffer, CellDiffOption},
    layout::Rect,
    style::Color,
    widgets::Widget,
};

/// An image kitty holds under `id`, shown `cols` × `rows` cells large wherever
/// it is drawn.
pub struct KittyImage {
    id: u32,
    cols: u16,
    rows: u16,
    /// The escape that sends it, until it has been sent.
    transmit: RefCell<Option<String>>,
}

impl KittyImage {
    /// `image`, to be stored by kitty as image `id` (below 2^24, so the id
    /// fits the placeholder's colour alone) and shown `cols` × `rows` cells
    /// large.
    pub fn new(image: &RgbaImage, id: u32, cols: u16, rows: u16, is_tmux: bool) -> Option<Self> {
        let mut png = Vec::new();
        image
            .write_with_encoder(PngEncoder::new_with_quality(
                Cursor::new(&mut png),
                CompressionType::Fast,
                FilterType::Adaptive,
            ))
            .ok()?;
        Some(Self {
            id,
            cols,
            rows,
            transmit: RefCell::new(Some(transmit(&png, id, cols, rows, is_tmux))),
        })
    }
}

/// The escape that stores `png` as image `id` with a virtual placement
/// `cols` × `rows` cells large, for unicode placeholders to show.
fn transmit(png: &[u8], id: u32, cols: u16, rows: u16, is_tmux: bool) -> String {
    // Payloads are sent at most 4096 base64 characters at a time.
    const CHUNK: usize = 4096 / 4 * 3;
    let chunks: Vec<&[u8]> = png.chunks(CHUNK).collect();
    let mut out = String::new();
    for (i, chunk) in chunks.iter().enumerate() {
        let more = u8::from(i + 1 < chunks.len());
        let keys = if i == 0 {
            format!("a=T,U=1,f=100,t=d,i={id},c={cols},r={rows},q=2,m={more}")
        } else {
            format!("q=2,m={more}")
        };
        let payload = base64::engine::general_purpose::STANDARD.encode(chunk);
        out.push_str(&wrap(&format!("\x1b_G{keys};{payload}\x1b\\"), is_tmux));
    }
    out
}

/// The escape that frees image `id` from kitty's memory.
pub fn delete(id: u32, is_tmux: bool) -> String {
    wrap(&format!("\x1b_Ga=d,d=I,i={id},q=2\x1b\\"), is_tmux)
}

/// `escape`, passed through tmux to the terminal outside it if need be.
fn wrap(escape: &str, is_tmux: bool) -> String {
    if is_tmux {
        format!("\x1bPtmux;{}\x1b\\", escape.replace('\x1b', "\x1b\x1b"))
    } else {
        escape.to_string()
    }
}

impl Widget for &KittyImage {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let [_, r, g, b] = self.id.to_be_bytes();
        let id_colour = Color::Rgb(r, g, b);
        for y in 0..area.height.min(self.rows) {
            for x in 0..area.width.min(self.cols) {
                let Some(cell) = buf.cell_mut((area.left() + x, area.top() + y)) else {
                    continue;
                };
                let mut symbol = if (x, y) == (0, 0) {
                    self.transmit.borrow_mut().take().unwrap_or_default()
                } else {
                    String::new()
                };
                push_placeholder(&mut symbol, diacritic(y), x, None);
                cell.set_symbol(&symbol).set_fg(id_colour);
                cell.set_diff_option(one_column(&symbol));
            }
        }
    }
}

/// Adds the placeholder for column `col` of the image row `row` (its
/// diacritic) to `symbol`, with the diacritic for the top byte of the id if
/// it has one.
///
/// Each cell names its own row and column rather than leaving them to follow
/// on from the cell to its left, so a cell stands alone: a popup drawn over
/// part of an image, or taken away again, changes just the cells it covers,
/// which ratatui sends like any others. Past the last numbered column a
/// placeholder has to follow on after all.
fn push_placeholder(symbol: &mut String, row: char, col: u16, id_top: Option<char>) {
    symbol.push('\u{10EEEE}');
    if let Some(&col) = DIACRITICS.get(usize::from(col)) {
        symbol.push(row);
        symbol.push(col);
        symbol.extend(id_top);
    }
}

/// A cell carrying the image itself is escape codes, which ratatui would
/// measure as dozens of columns and skip that far past.
fn one_column(symbol: &str) -> CellDiffOption {
    if symbol.contains('\x1b') {
        CellDiffOption::ForcedWidth(std::num::NonZeroU16::MIN)
    } else {
        CellDiffOption::None
    }
}

/// Spreads each kitty image row that ratatui-image wrote into its first cell
/// over the cells of the row, one placeholder each, the way [`KittyImage`]
/// draws. ratatui-image puts the whole row in the first cell and leaves the
/// rest to be skipped, so when that cell was sent it painted over anything
/// drawn on top of the row, and when a popup went away the skipped cells it
/// had covered were never sent again and it stayed on screen. Called after
/// the panels are drawn and before anything goes over them.
pub(crate) fn spread_rows(buf: &mut Buffer) {
    let area = buf.area;
    for y in area.top()..area.bottom() {
        let mut x = area.left();
        while x < area.right() {
            let Some(row) = ImageRow::parse(buf[(x, y)].symbol()) else {
                x += 1;
                continue;
            };
            for i in 0..row.width {
                let Some(cell) = buf.cell_mut((x + i, y)) else {
                    break;
                };
                let mut symbol = if i == 0 {
                    row.transmit.clone()
                } else {
                    String::new()
                };
                push_placeholder(&mut symbol, row.row, i, row.id_top);
                cell.set_symbol(&symbol).set_fg(row.id_colour);
                #[allow(deprecated)]
                cell.set_skip(false);
                cell.set_diff_option(one_column(&symbol));
            }
            x += row.width.max(1);
        }
    }
}

/// An image row as ratatui-image writes it into one cell: maybe the image
/// itself, then `ESC [ s`, the id as a colour, a placeholder carrying the
/// row, column 0 and the id's top byte, the rest of the row's placeholders,
/// and cursor movement back.
struct ImageRow {
    transmit: String,
    id_colour: Color,
    row: char,
    id_top: Option<char>,
    width: u16,
}

impl ImageRow {
    fn parse(symbol: &str) -> Option<Self> {
        const START: &str = "\x1b[s\x1b[38;2;";
        let at = symbol.find(START)?;
        let rest = &symbol[at + START.len()..];
        let (rgb, rest) = rest.split_once('m')?;
        let mut rgb = rgb.split(';').map(|n| n.parse::<u8>().ok());
        let (Some(Some(r)), Some(Some(g)), Some(Some(b)), None) =
            (rgb.next(), rgb.next(), rgb.next(), rgb.next())
        else {
            return None;
        };
        let mut chars = rest.chars().peekable();
        if chars.next()? != '\u{10EEEE}' {
            return None;
        }
        let row = chars.next()?;
        let _col = chars.next()?;
        let id_top = chars.next_if(|&c| c != '\u{10EEEE}' && c != '\x1b');
        let mut width: u16 = 1;
        while chars.next_if_eq(&'\u{10EEEE}').is_some() {
            width += 1;
        }
        Some(Self {
            transmit: symbol[..at].to_string(),
            id_colour: Color::Rgb(r, g, b),
            row,
            id_top,
            width,
        })
    }
}

fn diacritic(n: u16) -> char {
    DIACRITICS
        .get(usize::from(n))
        .copied()
        .unwrap_or(DIACRITICS[0])
}

/// The combining marks that number a placeholder's row and column, from
/// kitty's rowcolumn-diacritics.txt
/// (<https://sw.kovidgoyal.net/kitty/graphics-protocol/#unicode-placeholders>),
/// as ratatui-image (MIT) lists them.
pub(crate) static DIACRITICS: [char; 297] = [
    '\u{305}',
    '\u{30D}',
    '\u{30E}',
    '\u{310}',
    '\u{312}',
    '\u{33D}',
    '\u{33E}',
    '\u{33F}',
    '\u{346}',
    '\u{34A}',
    '\u{34B}',
    '\u{34C}',
    '\u{350}',
    '\u{351}',
    '\u{352}',
    '\u{357}',
    '\u{35B}',
    '\u{363}',
    '\u{364}',
    '\u{365}',
    '\u{366}',
    '\u{367}',
    '\u{368}',
    '\u{369}',
    '\u{36A}',
    '\u{36B}',
    '\u{36C}',
    '\u{36D}',
    '\u{36E}',
    '\u{36F}',
    '\u{483}',
    '\u{484}',
    '\u{485}',
    '\u{486}',
    '\u{487}',
    '\u{592}',
    '\u{593}',
    '\u{594}',
    '\u{595}',
    '\u{597}',
    '\u{598}',
    '\u{599}',
    '\u{59C}',
    '\u{59D}',
    '\u{59E}',
    '\u{59F}',
    '\u{5A0}',
    '\u{5A1}',
    '\u{5A8}',
    '\u{5A9}',
    '\u{5AB}',
    '\u{5AC}',
    '\u{5AF}',
    '\u{5C4}',
    '\u{610}',
    '\u{611}',
    '\u{612}',
    '\u{613}',
    '\u{614}',
    '\u{615}',
    '\u{616}',
    '\u{617}',
    '\u{657}',
    '\u{658}',
    '\u{659}',
    '\u{65A}',
    '\u{65B}',
    '\u{65D}',
    '\u{65E}',
    '\u{6D6}',
    '\u{6D7}',
    '\u{6D8}',
    '\u{6D9}',
    '\u{6DA}',
    '\u{6DB}',
    '\u{6DC}',
    '\u{6DF}',
    '\u{6E0}',
    '\u{6E1}',
    '\u{6E2}',
    '\u{6E4}',
    '\u{6E7}',
    '\u{6E8}',
    '\u{6EB}',
    '\u{6EC}',
    '\u{730}',
    '\u{732}',
    '\u{733}',
    '\u{735}',
    '\u{736}',
    '\u{73A}',
    '\u{73D}',
    '\u{73F}',
    '\u{740}',
    '\u{741}',
    '\u{743}',
    '\u{745}',
    '\u{747}',
    '\u{749}',
    '\u{74A}',
    '\u{7EB}',
    '\u{7EC}',
    '\u{7ED}',
    '\u{7EE}',
    '\u{7EF}',
    '\u{7F0}',
    '\u{7F1}',
    '\u{7F3}',
    '\u{816}',
    '\u{817}',
    '\u{818}',
    '\u{819}',
    '\u{81B}',
    '\u{81C}',
    '\u{81D}',
    '\u{81E}',
    '\u{81F}',
    '\u{820}',
    '\u{821}',
    '\u{822}',
    '\u{823}',
    '\u{825}',
    '\u{826}',
    '\u{827}',
    '\u{829}',
    '\u{82A}',
    '\u{82B}',
    '\u{82C}',
    '\u{82D}',
    '\u{951}',
    '\u{953}',
    '\u{954}',
    '\u{F82}',
    '\u{F83}',
    '\u{F86}',
    '\u{F87}',
    '\u{135D}',
    '\u{135E}',
    '\u{135F}',
    '\u{17DD}',
    '\u{193A}',
    '\u{1A17}',
    '\u{1A75}',
    '\u{1A76}',
    '\u{1A77}',
    '\u{1A78}',
    '\u{1A79}',
    '\u{1A7A}',
    '\u{1A7B}',
    '\u{1A7C}',
    '\u{1B6B}',
    '\u{1B6D}',
    '\u{1B6E}',
    '\u{1B6F}',
    '\u{1B70}',
    '\u{1B71}',
    '\u{1B72}',
    '\u{1B73}',
    '\u{1CD0}',
    '\u{1CD1}',
    '\u{1CD2}',
    '\u{1CDA}',
    '\u{1CDB}',
    '\u{1CE0}',
    '\u{1DC0}',
    '\u{1DC1}',
    '\u{1DC3}',
    '\u{1DC4}',
    '\u{1DC5}',
    '\u{1DC6}',
    '\u{1DC7}',
    '\u{1DC8}',
    '\u{1DC9}',
    '\u{1DCB}',
    '\u{1DCC}',
    '\u{1DD1}',
    '\u{1DD2}',
    '\u{1DD3}',
    '\u{1DD4}',
    '\u{1DD5}',
    '\u{1DD6}',
    '\u{1DD7}',
    '\u{1DD8}',
    '\u{1DD9}',
    '\u{1DDA}',
    '\u{1DDB}',
    '\u{1DDC}',
    '\u{1DDD}',
    '\u{1DDE}',
    '\u{1DDF}',
    '\u{1DE0}',
    '\u{1DE1}',
    '\u{1DE2}',
    '\u{1DE3}',
    '\u{1DE4}',
    '\u{1DE5}',
    '\u{1DE6}',
    '\u{1DFE}',
    '\u{20D0}',
    '\u{20D1}',
    '\u{20D4}',
    '\u{20D5}',
    '\u{20D6}',
    '\u{20D7}',
    '\u{20DB}',
    '\u{20DC}',
    '\u{20E1}',
    '\u{20E7}',
    '\u{20E9}',
    '\u{20F0}',
    '\u{2CEF}',
    '\u{2CF0}',
    '\u{2CF1}',
    '\u{2DE0}',
    '\u{2DE1}',
    '\u{2DE2}',
    '\u{2DE3}',
    '\u{2DE4}',
    '\u{2DE5}',
    '\u{2DE6}',
    '\u{2DE7}',
    '\u{2DE8}',
    '\u{2DE9}',
    '\u{2DEA}',
    '\u{2DEB}',
    '\u{2DEC}',
    '\u{2DED}',
    '\u{2DEE}',
    '\u{2DEF}',
    '\u{2DF0}',
    '\u{2DF1}',
    '\u{2DF2}',
    '\u{2DF3}',
    '\u{2DF4}',
    '\u{2DF5}',
    '\u{2DF6}',
    '\u{2DF7}',
    '\u{2DF8}',
    '\u{2DF9}',
    '\u{2DFA}',
    '\u{2DFB}',
    '\u{2DFC}',
    '\u{2DFD}',
    '\u{2DFE}',
    '\u{2DFF}',
    '\u{A66F}',
    '\u{A67C}',
    '\u{A67D}',
    '\u{A6F0}',
    '\u{A6F1}',
    '\u{A8E0}',
    '\u{A8E1}',
    '\u{A8E2}',
    '\u{A8E3}',
    '\u{A8E4}',
    '\u{A8E5}',
    '\u{A8E6}',
    '\u{A8E7}',
    '\u{A8E8}',
    '\u{A8E9}',
    '\u{A8EA}',
    '\u{A8EB}',
    '\u{A8EC}',
    '\u{A8ED}',
    '\u{A8EE}',
    '\u{A8EF}',
    '\u{A8F0}',
    '\u{A8F1}',
    '\u{AAB0}',
    '\u{AAB2}',
    '\u{AAB3}',
    '\u{AAB7}',
    '\u{AAB8}',
    '\u{AABE}',
    '\u{AABF}',
    '\u{AAC1}',
    '\u{FE20}',
    '\u{FE21}',
    '\u{FE22}',
    '\u{FE23}',
    '\u{FE24}',
    '\u{FE25}',
    '\u{FE26}',
    '\u{10A0F}',
    '\u{10A38}',
    '\u{1D185}',
    '\u{1D186}',
    '\u{1D187}',
    '\u{1D188}',
    '\u{1D189}',
    '\u{1D1AA}',
    '\u{1D1AB}',
    '\u{1D1AC}',
    '\u{1D1AD}',
    '\u{1D242}',
    '\u{1D243}',
    '\u{1D244}',
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_image_is_sent_once_as_png_and_drawn_as_placeholders() {
        let image = RgbaImage::new(40, 20);
        let kitty = KittyImage::new(&image, 0x0A0B0C, 4, 2, false).unwrap();
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 3));
        (&kitty).render(Rect::new(1, 0, 4, 2), &mut buf);
        let first = buf[(1, 0)].symbol().to_string();
        assert!(
            first.starts_with("\x1b_Ga=T,U=1,f=100,t=d,i=658188,c=4,r=2,q=2,m=0;iVBOR"),
            "{first:?}"
        );
        assert!(first.ends_with("\u{10EEEE}\u{305}\u{305}"), "{first:?}");
        // Every cell is its own placeholder, numbered, in the id's colour.
        assert_eq!(buf[(3, 0)].symbol(), "\u{10EEEE}\u{305}\u{30E}");
        assert_eq!(buf[(1, 1)].symbol(), "\u{10EEEE}\u{30D}\u{305}");
        assert_eq!(buf[(4, 1)].fg, Color::Rgb(10, 11, 12));
        assert_eq!(buf[(5, 0)].symbol(), " ", "outside the image");
        assert!(
            (buf[(1, 0)].diff_option == CellDiffOption::ForcedWidth(std::num::NonZeroU16::MIN))
                && (buf[(2, 0)].diff_option == CellDiffOption::None)
        );

        // Drawn again, it is not sent again.
        let mut again = Buffer::empty(Rect::new(0, 0, 10, 3));
        (&kitty).render(Rect::new(1, 0, 4, 2), &mut again);
        assert_eq!(again[(1, 0)].symbol(), "\u{10EEEE}\u{305}\u{305}");
    }

    #[test]
    fn a_large_image_is_sent_in_chunks() {
        let mut image = RgbaImage::new(300, 300);
        for (x, y, p) in image.enumerate_pixels_mut() {
            *p = image::Rgba([(x * 7) as u8, (y * 13) as u8, (x ^ y) as u8, 255]);
        }
        let kitty = KittyImage::new(&image, 7, 30, 15, false).unwrap();
        let sent = kitty.transmit.borrow().clone().unwrap();
        let chunks = sent.matches("\x1b_G").count();
        assert!(chunks > 1);
        assert_eq!(sent.matches("m=1;").count(), chunks - 1);
        assert_eq!(sent.matches("m=0;").count(), 1);
    }
}
