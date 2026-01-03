mod playlist_view;
mod track_details_view;
mod track_view;

use melotui_core::{AppState, state_registry::LibraryViewDepth};
use ratatui::{Frame, layout::Rect, text::Line};

// SIGNATURE CHANGED
pub fn draw(frame: &mut Frame, area: Rect, state: &AppState, title: Line) {
    match state.nav.library.depth {
        // Pass title to Playlist View
        LibraryViewDepth::Playlists => playlist_view::draw(frame, area, state, title),

        // Pass title to Track View
        LibraryViewDepth::Tracks => track_view::draw(frame, area, state, title),

        // Track Info can probably ignore it or use it, let's keep it clean
        LibraryViewDepth::TrackInfo => track_details_view::draw(frame, area, state),
    }
}
