use melotui_core::{
    AppState,
    state_registry::{SpectrumStyle, VisualizerMode},
};
use ratatui::{Frame, layout::Rect};

use crate::widgets::{album_art_view, spectrum_visualizer_view};

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    match state.visualizer_mode {
        VisualizerMode::Spectrum => {
            match state.spectrum_style {
                SpectrumStyle::Cava => spectrum_visualizer_view::draw_cava(frame, area, state),
                SpectrumStyle::Default => {
                    spectrum_visualizer_view::draw_default(frame, area, state)
                }
                SpectrumStyle::Bar => spectrum_visualizer_view::draw_bar(frame, area, state),
            };
        }
        VisualizerMode::AlbumArt => {
            album_art_view::draw(frame, area, state);
        }
    }
}
