mod playlist_view;
mod track_details_view;
mod track_view;

use melotui_core::{AppState, state::LibraryViewDepth};
use ratatui::{Frame, layout::Rect};

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    match state.session.nav.library.depth {
        LibraryViewDepth::Playlists => playlist_view::draw(frame, area, state),
        LibraryViewDepth::Tracks => track_view::draw(frame, area, state),
        LibraryViewDepth::TrackInfo => track_details_view::draw(frame, area, state),
    }
}
