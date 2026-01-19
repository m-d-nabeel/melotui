pub mod queue;
pub mod tree;

use melotui_core::AppState;
use ratatui::{Frame, layout::Rect};

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    if state.is_queue_focused() {
        queue::draw(frame, area, state);
    } else {
        tree::draw(frame, area, state);
    }
}
