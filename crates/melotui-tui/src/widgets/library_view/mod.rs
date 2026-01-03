mod playlist_view;
mod queue_header;
mod track_details_view;
mod track_view;

use melotui_core::{AppState, NavMode};
use ratatui::{Frame, layout::Rect};

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    match state.nav.mode {
        NavMode::Playlists => playlist_view::draw(frame, area, state),
        NavMode::Tracks => track_view::draw(frame, area, state),
        NavMode::TrackInfo => track_details_view::draw(frame, area, state),
    }
}
