mod playlist_view;
pub mod queue_view;
pub mod track_details_view;
mod track_view;

use melotui_core::AppState;
use ratatui::{Frame, layout::Rect};

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    if state.is_playlist_view() {
        playlist_view::draw(frame, area, state);
    } else if state.is_track_list_view() {
        track_view::draw(frame, area, state);
    } else if state.is_track_info_view() {
        track_details_view::draw(frame, area, state);
    }
}
