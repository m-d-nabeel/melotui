mod album_art_view;
mod spectrum_visualizer_view;
use melotui_core::{
    AppState,
    state::{SpectrumStyle, VisualizerMode},
};
use ratatui::{Frame, layout::Rect};

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    match state.session.visualizer_mode {
        VisualizerMode::Spectrum => {
            match state.session.spectrum_style {
                SpectrumStyle::Cava => spectrum_visualizer_view::draw_cava(frame, area, state),
                SpectrumStyle::Default => {
                    spectrum_visualizer_view::draw_default(frame, area, state)
                }
                SpectrumStyle::Bar => spectrum_visualizer_view::draw_bar(frame, area, state),
                SpectrumStyle::Particle => {
                    spectrum_visualizer_view::draw_particle(frame, area, state)
                }
            };
        }
        VisualizerMode::AlbumArt => {
            album_art_view::draw(frame, area, state);
        }
    }
}
