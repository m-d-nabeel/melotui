pub mod app_state;
pub mod art_converter;
pub mod command;
pub mod library;
pub mod message;
pub mod reducer;
pub mod state_registry;
pub mod visualizer_interface;

pub use app_state::AppState;
pub use command::Command;
pub use library::{Library, Playlist};
pub use message::AppMsg;
pub use state_registry::UIMode;

/// Helper to ensure cursors don't go out of bounds (e.g. after a rescan or delete)
pub fn reconcile_nav_state(state: &mut AppState) {
    let playlist_count = state.library.playlists.len();

    if playlist_count == 0 {
        state.nav.library.selected_playlist_idx = 0;
        state.nav.library.selected_tracks_curs.clear();
    } else {
        if state.nav.library.selected_playlist_idx >= playlist_count {
            state.nav.library.selected_playlist_idx = playlist_count - 1;
        }
        // Resize cursor vector if playlists changed
        state
            .nav
            .library
            .selected_tracks_curs
            .resize(playlist_count, 0);

        for (i, playlist) in state.library.playlists.iter().enumerate() {
            let track_count = playlist.tracks.len();
            if track_count == 0 {
                state.nav.library.selected_tracks_curs[i] = 0;
            } else if state.nav.library.selected_tracks_curs[i] >= track_count {
                state.nav.library.selected_tracks_curs[i] = track_count - 1;
            }
        }
    }

    // 2. Reconcile Queue
    let queue_len = state.playback_queue.tracks.len();
    if queue_len == 0 {
        state.nav.queue.selected_idx = 0;
    } else if state.nav.queue.selected_idx >= queue_len {
        state.nav.queue.selected_idx = queue_len - 1;
    }
}
