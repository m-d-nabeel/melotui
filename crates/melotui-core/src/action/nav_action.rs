use crate::{
    AppMsg, AppState, Library,
    art_converter::{SizeMode, bytes_to_ascii},
    state::{LeftPanelMode, LibraryViewDepth},
};

pub fn handle_toggle_library_queue(state: &mut AppState) -> Vec<AppMsg> {
    tracing::info!("Toggle Library Queue");
    state.nav.active_panel = match state.nav.active_panel {
        LeftPanelMode::Library => {
            state.nav.active_panel = LeftPanelMode::Queue;
            // FIX: When switching TO Queue, default to Index 1 if playing
            let playing_from_queue = state
                .playback
                .current
                .as_ref()
                .is_some_and(|c| c.context == crate::state::PlaybackContext::Queue);

            if playing_from_queue && state.playback_queue.tracks.len() > 1 {
                state.nav.queue.selected_idx = 1;
            }
            LeftPanelMode::Queue
        }
        LeftPanelMode::Queue => LeftPanelMode::Library,
    };
    vec![]
}

pub fn handle_switch_to_playlists(state: &mut AppState) -> Vec<AppMsg> {
    tracing::debug!("Switch to Playlist Mode");
    state.last_nav_time = state.time_elapsed;
    state.nav.active_panel = LeftPanelMode::Library;
    state.nav.library.depth = LibraryViewDepth::Playlists;
    vec![]
}

pub fn handle_switch_to_tracks(state: &mut AppState) -> Vec<AppMsg> {
    tracing::debug!("Switch to Tracks");
    state.last_nav_time = state.time_elapsed;
    state.nav.active_panel = LeftPanelMode::Library;
    state.nav.library.depth = LibraryViewDepth::Tracks;
    vec![]
}

pub fn handle_switch_to_track_info(state: &mut AppState) -> Vec<AppMsg> {
    tracing::debug!("Switch to Track Info");
    state.last_nav_time = state.time_elapsed;
    state.nav.active_panel = LeftPanelMode::Library;
    state.nav.library.depth = LibraryViewDepth::TrackInfo;

    state.track_info_art = None;
    let p_idx = state.nav.library.selected_playlist_idx;
    if let Some(playlist) = state.library.playlists.get(p_idx) {
        let t_idx = state.nav.library.selected_tracks_curs[p_idx];
        if let Some(track) = playlist.tracks.get(t_idx)
            && track.has_cover
            && let Some((data, _)) = Library::get_album_art(&track.file_path)
            && let Ok(ascii) = bytes_to_ascii(
                &data,
                SizeMode::Cover {
                    width: 32,
                    height: 15,
                },
            )
        {
            state.track_info_art = Some(ascii);
        }
    }
    vec![]
}

pub fn handle_move_up(state: &mut AppState) -> Vec<AppMsg> {
    state.last_nav_time = state.time_elapsed;
    match state.nav.active_panel {
        LeftPanelMode::Library => match state.nav.library.depth {
            LibraryViewDepth::Playlists => {
                if state.nav.library.selected_playlist_idx > 0 {
                    state.nav.library.selected_playlist_idx -= 1;
                }
            }
            LibraryViewDepth::Tracks => {
                let p = state.nav.library.selected_playlist_idx;
                if p < state.nav.library.selected_tracks_curs.len()
                    && state.nav.library.selected_tracks_curs[p] > 0
                {
                    state.nav.library.selected_tracks_curs[p] -= 1;
                }
            }
            LibraryViewDepth::TrackInfo => {}
        },
        LeftPanelMode::Queue => {
            let playing_from_queue = state
                .playback
                .current
                .as_ref()
                .is_some_and(|c| c.context == crate::state::PlaybackContext::Queue);

            let min_index = if playing_from_queue && state.playback_queue.tracks.len() > 1 {
                1
            } else {
                0
            };

            if state.nav.queue.selected_idx > min_index {
                state.nav.queue.selected_idx -= 1;
            }
        }
    }
    vec![]
}

pub fn handle_move_down(state: &mut AppState) -> Vec<AppMsg> {
    state.last_nav_time = state.time_elapsed;
    match state.nav.active_panel {
        LeftPanelMode::Library => match state.nav.library.depth {
            LibraryViewDepth::Playlists => {
                if state.nav.library.selected_playlist_idx + 1 < state.library.playlists.len() {
                    state.nav.library.selected_playlist_idx += 1;
                }
            }
            LibraryViewDepth::Tracks => {
                let p = state.nav.library.selected_playlist_idx;
                if let Some(playlist) = state.library.playlists.get(p)
                    && state.nav.library.selected_tracks_curs[p] + 1 < playlist.tracks.len()
                {
                    state.nav.library.selected_tracks_curs[p] += 1;
                }
            }
            LibraryViewDepth::TrackInfo => {}
        },
        LeftPanelMode::Queue => {
            let max_len = state.playback_queue.tracks.len();
            if max_len > 0 && state.nav.queue.selected_idx + 1 < max_len {
                state.nav.queue.selected_idx += 1;
            }
        }
    }
    vec![]
}
