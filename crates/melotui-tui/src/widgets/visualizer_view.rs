use melotui_core::{AppState, state_registry::VisualizerMode};
use ratatui::{Frame, layout::Rect, style::Color};

use crate::widgets::{album_art_view, spectrum_visualizer_view};

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    match state.visualizer_mode {
        VisualizerMode::Spectrum => {
            // spectrum_visualizer_view::draw_cava(frame, area, state);
            spectrum_visualizer_view::draw_4(frame, area, state);
        }
        VisualizerMode::AlbumArt => {
            album_art_view::draw(frame, area, state);
        }
    }
}

/// Helper: HSL to Ratatui Color
pub fn hsl_to_rgb(h: f64, s: f64, l: f64) -> Color {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;

    let (r, g, b) = match h {
        v if v < 60.0 => (c, x, 0.0),
        v if v < 120.0 => (x, c, 0.0),
        v if v < 180.0 => (0.0, c, x),
        v if v < 240.0 => (0.0, x, c),
        v if v < 300.0 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    Color::Rgb(
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
    )
}
