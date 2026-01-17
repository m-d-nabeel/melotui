use crate::widgets::library_view::{self, queue_view};
use melotui_core::AppState;
use ratatui::{Frame, layout::Rect};

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    if state.is_library_focused() {
        library_view::draw(frame, area, state);
    } else if state.is_queue_focused() {
        if state.is_queue_info_view() {
            library_view::track_details_view::draw(frame, area, state);
        } else {
            queue_view::draw(frame, area, state);
        }
    }
}
