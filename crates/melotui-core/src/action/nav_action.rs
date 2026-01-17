use crate::{
    AppMsg, AppState, Library,
    state::{LeftPanelMode, LibraryViewDepth},
    utils::{SizeMode, bytes_to_ascii},
};

pub fn handle_toggle_library_queue(state: &mut AppState) -> Vec<AppMsg> {
    tracing::info!("Toggle Library Queue");

    state.session.nav.active_panel = match state.session.nav.active_panel {
        LeftPanelMode::Library => {
            if !state.queue().is_empty() {
                state.session.nav.queue.selected_idx = 0;
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
    state.session.nav.active_panel = LeftPanelMode::Library;
    state.session.nav.library.depth = LibraryViewDepth::Playlists;
    vec![]
}

pub fn handle_switch_to_tracks(state: &mut AppState) -> Vec<AppMsg> {
    tracing::debug!("Switch to Tracks");
    state.last_nav_time = state.time_elapsed;
    state.session.nav.active_panel = LeftPanelMode::Library;
    state.session.nav.library.depth = LibraryViewDepth::Tracks;
    vec![]
}

pub fn handle_switch_to_track_info(state: &mut AppState) -> Vec<AppMsg> {
    tracing::debug!("Switch to Track Info");
    state.last_nav_time = state.time_elapsed;
    state.session.nav.active_panel = LeftPanelMode::Library;
    state.session.nav.library.depth = LibraryViewDepth::TrackInfo;

    state.track_info_art = None;

    let p_idx = state.selected_playlist_index();
    if let Some(playlist) = state.library.playlists.get(p_idx) {
        let t_idx = state.selected_track_index_for_playlist(p_idx);

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

    match state.session.nav.active_panel {
        LeftPanelMode::Library => match state.session.nav.library.depth {
            LibraryViewDepth::Playlists => {
                if state.selected_playlist_index() > 0 {
                    state.session.nav.library.selected_playlist_idx -= 1;
                }
            }
            LibraryViewDepth::Tracks => {
                let p_idx = state.selected_playlist_index();
                if p_idx < state.session.nav.library.selected_tracks_curs.len()
                    && state.session.nav.library.selected_tracks_curs[p_idx] > 0
                {
                    state.session.nav.library.selected_tracks_curs[p_idx] -= 1;
                }
            }
            LibraryViewDepth::TrackInfo => {}
        },
        LeftPanelMode::Queue => {
            if state.selected_queue_index() > 0 {
                state.session.nav.queue.selected_idx -= 1;
            }
        }
    }
    vec![]
}

pub fn handle_move_down(state: &mut AppState) -> Vec<AppMsg> {
    state.last_nav_time = state.time_elapsed;

    match state.session.nav.active_panel {
        LeftPanelMode::Library => match state.session.nav.library.depth {
            LibraryViewDepth::Playlists => {
                if state.selected_playlist_index() + 1 < state.library.playlists.len() {
                    state.session.nav.library.selected_playlist_idx += 1;
                }
            }
            LibraryViewDepth::Tracks => {
                let p_idx = state.selected_playlist_index();
                if let Some(playlist) = state.library.playlists.get(p_idx)
                    && state.selected_track_index_for_playlist(p_idx) + 1 < playlist.tracks.len()
                {
                    state.session.nav.library.selected_tracks_curs[p_idx] += 1;
                }
            }
            LibraryViewDepth::TrackInfo => {}
        },
        LeftPanelMode::Queue => {
            let max_len = state.queue().len();
            if max_len > 0 && state.selected_queue_index() + 1 < max_len {
                state.session.nav.queue.selected_idx += 1;
            }
        }
    }
    vec![]
}
