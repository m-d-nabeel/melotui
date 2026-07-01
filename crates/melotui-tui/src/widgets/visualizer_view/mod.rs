mod album_art_view;
mod spectrum_visualizer_view;
use melotui_core::{AppState, state::SpectrumStyle};
use ratatui::{Frame, layout::Rect};

use crate::image_cache::ImageCache;

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState, cache: &mut ImageCache) {
    if state.should_draw_spectrum() {
        match state.spectrum_style() {
            SpectrumStyle::Cava => spectrum_visualizer_view::draw_cava(frame, area, state),
            SpectrumStyle::Default => spectrum_visualizer_view::draw_default(frame, area, state),
            SpectrumStyle::Bar => spectrum_visualizer_view::draw_bar(frame, area, state),
            SpectrumStyle::Particle => spectrum_visualizer_view::draw_particle(frame, area, state),
            SpectrumStyle::Radial => spectrum_visualizer_view::draw_radial(frame, area, state),
            SpectrumStyle::Nova => spectrum_visualizer_view::draw_nova(frame, area, state),
        }
    } else {
        album_art_view::draw(frame, area, state, cache);
    }
}
