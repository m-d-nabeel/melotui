//! Text shaped and rasterised by a real text engine, for the scripts a
//! terminal grid cannot draw.

use std::sync::{Arc, Mutex, OnceLock};

use cosmic_text::{
    Align, Attrs, Buffer, Color, Family, FontSystem, Metrics, Shaping, SwashCache, Weight, Wrap,
};
use image::{Rgba, RgbaImage};

/// Font size as a share of the line height. Devanagari carries marks well
/// above its headline and below its baseline, and Urdu's descenders run deep;
/// at this size both fit inside the line.
const FONT_TO_ROW: f32 = 0.62;
/// The furthest a line is shrunk to fit the panel; one that would need more is
/// wrapped onto further lines instead.
const MIN_FIT: f32 = 0.8;
/// Share of the panel's width a line is fitted to, so it does not end up
/// touching both sides.
const FIT_MARGIN: f32 = 0.96;

struct Engine {
    fonts: FontSystem,
    glyphs: SwashCache,
}

/// Shapes and rasterises lines once the system's fonts are loaded.
///
/// Loading them takes about a quarter of a second for a few hundred faces, so
/// it happens on a thread of its own, and only once some lyrics actually need
/// it; until then the lyrics are drawn as text.
pub struct Shaper {
    engine: Arc<OnceLock<Mutex<Engine>>>,
    loading: bool,
}

impl Shaper {
    pub fn new() -> Self {
        Self {
            engine: Arc::new(OnceLock::new()),
            loading: false,
        }
    }

    /// Whether the fonts are in. The first call starts loading them.
    pub fn ready(&mut self) -> bool {
        if self.engine.get().is_some() {
            return true;
        }
        if !self.loading {
            self.loading = true;
            let engine = Arc::clone(&self.engine);
            // If the thread cannot be started the fonts never arrive, and the
            // lyrics simply stay text.
            let _ = std::thread::Builder::new()
                .name("lyrics-fonts".into())
                .spawn(move || {
                    let _ = engine.set(Mutex::new(Engine {
                        fonts: FontSystem::new(),
                        glyphs: SwashCache::new(),
                    }));
                });
        }
        false
    }

    /// How many rows of `cell_h` pixels `text` takes, laid out `width` pixels
    /// wide at `zoom` times the text size, or `None` while the fonts are still
    /// loading.
    pub fn rows(&self, text: &str, width: u32, cell_h: u32, zoom: f32) -> Option<u16> {
        let mut engine = self.engine.get()?.lock().ok()?;
        let buffer = lay_out(&mut engine.fonts, text, width, cell_h, zoom);
        Some(rows_of(&buffer, zoom))
    }

    /// `text` in `color` at `zoom` times the text size, shaped and centred in
    /// an image `width` pixels wide on a transparent background, with the
    /// number of `cell_h`-pixel rows it fills — or `None` while the fonts are
    /// still loading.
    pub fn render(
        &self,
        text: &str,
        color: [u8; 3],
        width: u32,
        cell_h: u32,
        zoom: f32,
    ) -> Option<(RgbaImage, u16)> {
        let mut engine = self.engine.get()?.lock().ok()?;
        let Engine { fonts, glyphs } = &mut *engine;
        Some(rasterise(fonts, glyphs, text, color, width, cell_h, zoom))
    }

    /// Blocks until the fonts are loaded. For tests.
    #[cfg(test)]
    pub fn wait_until_ready(&mut self) {
        while !self.ready() {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    /// Whether some installed font has glyphs for all of `text`. For tests,
    /// which cannot check shaping on a machine with no font for the script.
    #[cfg(test)]
    pub fn covers(&self, text: &str) -> bool {
        let Some(engine) = self.engine.get() else {
            return false;
        };
        let mut engine = engine.lock().unwrap();
        let fonts = &mut engine.fonts;
        let mut buffer = Buffer::new(fonts, Metrics::new(20.0, 24.0));
        buffer.set_text(text, &Attrs::new(), Shaping::Advanced, None);
        buffer.shape_until_scroll(fonts, false);
        buffer
            .layout_runs()
            .all(|run| run.glyphs.iter().all(|glyph| glyph.glyph_id != 0))
    }
}

fn shape(
    fonts: &mut FontSystem,
    text: &str,
    font: f32,
    pitch: f32,
    width: Option<f32>,
    wrap: Wrap,
) -> Buffer {
    let attrs = Attrs::new().family(Family::SansSerif).weight(Weight::BOLD);
    let mut buffer = Buffer::new(fonts, Metrics::new(font, pitch));
    buffer.set_wrap(wrap);
    buffer.set_size(width, None);
    let align = width.map(|_| Align::Center);
    buffer.set_text(text, &attrs, Shaping::Advanced, align);
    buffer.shape_until_scroll(fonts, false);
    buffer
}

/// `text` laid out to fit `width` pixels, one line `zoom` rows of `cell_h`
/// pixels apart: as it is if it fits, shrunk a little if that is enough, and
/// otherwise wrapped onto as many lines as it needs, `FIT_MARGIN` of the
/// width wide and centred.
fn lay_out(fonts: &mut FontSystem, text: &str, width: u32, cell_h: u32, zoom: f32) -> Buffer {
    let pitch = cell_h as f32 * zoom;
    let font = pitch * FONT_TO_ROW;
    let room = width as f32 * FIT_MARGIN;
    let natural = shape(fonts, text, font, pitch, None, Wrap::None)
        .layout_runs()
        .map(|run| run.line_w)
        .fold(0.0, f32::max);
    if natural <= room {
        shape(fonts, text, font, pitch, Some(room), Wrap::None)
    } else if room / natural >= MIN_FIT {
        shape(
            fonts,
            text,
            font * room / natural,
            pitch,
            Some(room),
            Wrap::None,
        )
    } else {
        shape(fonts, text, font, pitch, Some(room), Wrap::WordOrGlyph)
    }
}

/// The whole rows a laid-out line fills.
fn rows_of(buffer: &Buffer, zoom: f32) -> u16 {
    let lines = buffer.layout_runs().count().max(1) as f32;
    // A hair under, so 2 lines at 1.5 is 3 rows and not 4 from rounding.
    ((lines * zoom - 0.01).ceil() as u16).max(1)
}

fn rasterise(
    fonts: &mut FontSystem,
    glyphs: &mut SwashCache,
    text: &str,
    color: [u8; 3],
    width: u32,
    cell_h: u32,
    zoom: f32,
) -> (RgbaImage, u16) {
    let mut buffer = lay_out(fonts, text, width, cell_h, zoom);
    let rows = rows_of(&buffer, zoom);
    let height = u32::from(rows) * cell_h;

    // The lines centred in the rows they fill, and the panel's margin either
    // side of them.
    let lines = buffer.layout_runs().count().max(1) as f32;
    let dy = ((height as f32 - lines * cell_h as f32 * zoom) / 2.0).round() as i32;
    let dx = ((width as f32 * (1.0 - FIT_MARGIN)) / 2.0).round() as i32;

    let mut image = RgbaImage::new(width, height);
    let [r, g, b] = color;
    buffer.draw(fonts, glyphs, Color::rgb(r, g, b), |x, y, w, h, c| {
        let coverage = u16::from(c.a());
        if coverage == 0 {
            return;
        }
        let (x, y) = (x + dx, y + dy);
        for py in y.max(0)..(y + h as i32).min(height as i32) {
            for px in x.max(0)..(x + w as i32).min(width as i32) {
                let pixel = image.get_pixel_mut(px as u32, py as u32);
                // One colour throughout, so only the coverage accumulates.
                let alpha = 255 - (255 - u16::from(pixel[3])) * (255 - coverage) / 255;
                *pixel = Rgba([r, g, b, alpha as u8]);
            }
        }
    });
    (image, rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A loaded shaper, or `None` — and the test skipped — on a machine with
    /// no Devanagari font.
    fn shaper() -> Option<Shaper> {
        let mut s = Shaper::new();
        s.wait_until_ready();
        if s.covers("क्ष") {
            Some(s)
        } else {
            eprintln!("skipped: no Devanagari font installed");
            None
        }
    }

    /// Horizontal extent of the inked pixels, as (first column, last column).
    fn ink(image: &RgbaImage) -> Option<(u32, u32)> {
        let cols: Vec<u32> = (0..image.width())
            .filter(|&x| (0..image.height()).any(|y| image.get_pixel(x, y)[3] > 0))
            .collect();
        Some((*cols.first()?, *cols.last()?))
    }

    /// Vertical extent of the inked pixels, as (first row, last row).
    fn ink_rows(image: &RgbaImage) -> Option<(u32, u32)> {
        let rows: Vec<u32> = (0..image.height())
            .filter(|&y| (0..image.width()).any(|x| image.get_pixel(x, y)[3] > 0))
            .collect();
        Some((*rows.first()?, *rows.last()?))
    }

    #[test]
    fn a_line_is_drawn_centred_in_its_colour_on_a_transparent_background() {
        let Some(s) = shaper() else { return };
        let image = s
            .render("कभी कभी मेरे दिल में", [137, 180, 250], 600, 40, 1.0)
            .unwrap()
            .0;
        assert_eq!((image.width(), image.height()), (600, 40));
        let (left, right) = ink(&image).expect("nothing was drawn");
        let centre = (left + right) / 2;
        assert!(
            (290..=310).contains(&centre),
            "not centred: ink from {left} to {right}"
        );
        assert!(
            image.pixels().any(|p| p[3] == 0),
            "the background is not transparent"
        );
        assert!(
            image
                .pixels()
                .filter(|p| p[3] > 0)
                .all(|p| [p[0], p[1], p[2]] == [137, 180, 250]),
            "drawn in some other colour"
        );
    }

    #[test]
    fn a_conjunct_is_one_shaped_glyph_not_its_letters() {
        // क्ष drawn as consonant, virama, consonant would be three glyphs side
        // by side; shaped, it is a single ligature, and much narrower.
        let Some(s) = shaper() else { return };
        let width_of = |text: &str| {
            let (l, r) = ink(&s.render(text, [255, 255, 255], 400, 40, 1.0).unwrap().0).unwrap();
            r - l
        };
        let shaped = width_of("क्ष");
        let spelled_out = width_of("क ् ष");
        assert!(
            shaped * 2 < spelled_out,
            "क्ष is {shaped}px wide; spelled out it is {spelled_out}px — it was not shaped"
        );
    }

    #[test]
    fn a_line_too_wide_for_the_panel_is_shrunk_to_fit() {
        let text = "तुम्हारी क्षत्रिय हिन्दी प्रेम नमस्ते";
        let Some(s) = shaper() else { return };
        // A panel a tenth narrower than the line comes out at full size: too
        // narrow for it, but well within how far a line may be shrunk. Taken
        // from the line as drawn, since fonts differ in width — Windows'
        // Devanagari font is a good deal wider than Linux's.
        let (l, r) = ink(&s.render(text, [255, 255, 255], 2000, 40, 1.0).unwrap().0).unwrap();
        let width = (r - l) * 9 / 10;
        let (image, rows) = s.render(text, [255, 255, 255], width, 40, 1.0).unwrap();
        assert_eq!(rows, 1, "shrunk, not wrapped");
        let (left, right) = ink(&image).unwrap();
        assert!(
            left > 0 && right < width - 1,
            "ran off the edges of {width}px: ink from {left} to {right}"
        );
    }

    #[test]
    fn a_line_far_too_wide_for_the_panel_wraps_onto_more_rows() {
        let text = "तुम्हारी क्षत्रिय हिन्दी प्रेम नमस्ते तुम्हारी क्षत्रिय हिन्दी प्रेम नमस्ते";
        let Some(s) = shaper() else { return };
        let (l, r) = ink(&s.render(text, [255, 255, 255], 4000, 40, 1.0).unwrap().0).unwrap();
        // A third of its width: far more than shrinking may make up for.
        let width = (r - l) / 3;
        let (image, rows) = s.render(text, [255, 255, 255], width, 40, 1.0).unwrap();
        assert!(rows >= 3, "{rows} rows");
        assert_eq!(image.height(), u32::from(rows) * 40);
        assert_eq!(s.rows(text, width, 40, 1.0), Some(rows));
        let (left, right) = ink(&image).unwrap();
        assert!(left > 0 && right < width - 1, "ink from {left} to {right}");
        let (top, bottom) = ink_rows(&image).unwrap();
        assert!(
            top < 40 && bottom > image.height() - 40,
            "not spread over the rows: ink from {top} to {bottom}"
        );
    }

    #[test]
    fn zoomed_text_is_larger_and_fills_more_rows() {
        let text = "ख़याल आता है";
        let Some(s) = shaper() else { return };
        let (one, one_rows) = s.render(text, [255, 255, 255], 800, 40, 1.0).unwrap();
        let (two, two_rows) = s.render(text, [255, 255, 255], 800, 40, 2.0).unwrap();
        let (_, half_rows) = s.render(text, [255, 255, 255], 800, 40, 1.5).unwrap();
        assert_eq!((one_rows, half_rows, two_rows), (1, 2, 2));
        let width = |image: &RgbaImage| {
            let (l, r) = ink(image).unwrap();
            (r - l) as f32
        };
        let ratio = width(&two) / width(&one);
        assert!(
            (1.8..2.2).contains(&ratio),
            "twice the zoom drew {ratio}x as wide"
        );
        // Centred in its two rows.
        let (top, bottom) = ink_rows(&two).unwrap();
        let middle = (top + bottom) / 2;
        assert!((30..=50).contains(&middle), "ink from {top} to {bottom}");
    }
}
