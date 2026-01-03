use crate::widgets::{library_view, queue_view};
use melotui_core::{AppState, state_registry::LeftPanelMode};
use ratatui::{Frame, layout::Rect};

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    match state.nav.active_panel {
        LeftPanelMode::Library => library_view::draw(frame, area, state),
        LeftPanelMode::Queue => queue_view::draw(frame, area, state),
    }
}
