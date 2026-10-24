//! Text shaped and rasterised by a real text engine, for the scripts a
//! terminal grid cannot draw.

use std::sync::{Arc, Mutex, OnceLock};

use cosmic_text::{
    Align, Attrs, Buffer, Color, Family, FontSystem, Metrics, Shaping, SwashCache, Weight, Wrap,
};
use image::{Rgba, RgbaImage};

/// Font size as a share of the row height. Devanagari carries marks well above
/// its headline and below its baseline, and Urdu's descenders run deep; at
/// this size both fit inside a single terminal row.
const FONT_TO_ROW: f32 = 0.62;
/// The furthest a line is shrunk to fit the panel before it is left to run
/// past the edges.
const MIN_FIT: f32 = 0.7;
/// Share of the panel a shrunk line is fitted to, so it does not end up
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

    /// `text` in `color`, shaped and centred in an image `width` × `height`
    /// pixels on a transparent background, or `None` while the fonts are still
    /// loading.
    pub fn render(&self, text: &str, color: [u8; 3], width: u32, height: u32) -> Option<RgbaImage> {
        let mut engine = self.engine.get()?.lock().ok()?;
        let Engine { fonts, glyphs } = &mut *engine;
        Some(rasterise(fonts, glyphs, text, color, width, height))
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

fn rasterise(
    fonts: &mut FontSystem,
    glyphs: &mut SwashCache,
    text: &str,
    color: [u8; 3],
    width: u32,
    height: u32,
) -> RgbaImage {
    let row = height as f32;
    let attrs = Attrs::new().family(Family::SansSerif).weight(Weight::BOLD);
    let layout = |fonts: &mut FontSystem, scale: f32, width: Option<f32>, align: Option<Align>| {
        let mut buffer = Buffer::new(fonts, Metrics::new(row * FONT_TO_ROW * scale, row));
        buffer.set_wrap(Wrap::None);
        buffer.set_size(width, Some(row));
        buffer.set_text(text, &attrs, Shaping::Advanced, align);
        buffer.shape_until_scroll(fonts, false);
        buffer
    };

    // Measured unconstrained first, then shrunk if it would not fit the panel.
    let natural = layout(fonts, 1.0, None, None)
        .layout_runs()
        .map(|run| run.line_w)
        .fold(0.0, f32::max);
    let room = width as f32 * FIT_MARGIN;
    let scale = if natural > room {
        (room / natural).max(MIN_FIT)
    } else {
        1.0
    };
    let mut buffer = layout(fonts, scale, Some(width as f32), Some(Align::Center));

    let mut image = RgbaImage::new(width, height);
    let [r, g, b] = color;
    buffer.draw(fonts, glyphs, Color::rgb(r, g, b), |x, y, w, h, c| {
        let coverage = u16::from(c.a());
        if coverage == 0 {
            return;
        }
        for py in y.max(0)..(y + h as i32).min(height as i32) {
            for px in x.max(0)..(x + w as i32).min(width as i32) {
                let pixel = image.get_pixel_mut(px as u32, py as u32);
                // One colour throughout, so only the coverage accumulates.
                let alpha = 255 - (255 - u16::from(pixel[3])) * (255 - coverage) / 255;
                *pixel = Rgba([r, g, b, alpha as u8]);
            }
        }
    });
    image
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

    #[test]
    fn a_line_is_drawn_centred_in_its_colour_on_a_transparent_background() {
        let Some(s) = shaper() else { return };
        let image = s
            .render("कभी कभी मेरे दिल में", [137, 180, 250], 600, 40)
            .unwrap();
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
            let (l, r) = ink(&s.render(text, [255, 255, 255], 400, 40).unwrap()).unwrap();
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
        let text = "तुम्हारी क्षत्रिय हिन्दी प्रेम नमस्ते तुम्हारी";
        let Some(s) = shaper() else { return };
        let image = s.render(text, [255, 255, 255], 260, 40).unwrap();
        let (left, right) = ink(&image).unwrap();
        assert!(
            left > 0 && right < 259,
            "ran off the edges: ink from {left} to {right}"
        );
    }
}
