use crate::{
    AppMsg, AppState,
    action::{
        audio_action, cover_action::handle_cover_message, download_action, library_action,
        nav_action, playback_action, ui_action,
    },
    message::{
        audio::AudioMsg, database::DatabaseMsg, download::DownloadMsg, library::LibraryMsg,
        navigation::NavigationMsg, playback::PlaybackMsg, ui::UIMsg,
    },
};

pub fn reduce(state: &mut AppState, msg: AppMsg) -> Vec<AppMsg> {
    match msg {
        AppMsg::RequestQuit => library_action::handle_request_quit(state),
        AppMsg::Noop => vec![],
        AppMsg::Navigation(nav_msg) => handle_navigation_message(state, nav_msg),
        AppMsg::Playback(playback_msg) => handle_playback_message(state, playback_msg),
        AppMsg::Audio(audio_msg) => handle_audio_message(state, audio_msg),
        AppMsg::UI(ui_msg) => handle_ui_message(state, ui_msg),
        AppMsg::Library(library_msg) => handle_library_message(state, library_msg),
        AppMsg::Download(download_msg) => handle_download_message(state, download_msg),
        AppMsg::Cover(cover_msg) => handle_cover_message(state, cover_msg),
        AppMsg::Database(database_msg) => handle_database_message(state, database_msg),
    }
}

fn handle_navigation_message(state: &mut AppState, msg: NavigationMsg) -> Vec<AppMsg> {
    match msg {
        NavigationMsg::SwitchToPlaylists => nav_action::handle_switch_to_playlists(state),
        NavigationMsg::SwitchToTracks => nav_action::handle_switch_to_tracks(state),
        NavigationMsg::ToggleTrackInfo => nav_action::handle_toggle_track_info(state),
        NavigationMsg::ToggleLibraryQueue => nav_action::handle_toggle_library_queue(state),
        NavigationMsg::MoveUp => nav_action::handle_move_up(state),
        NavigationMsg::MoveDown => nav_action::handle_move_down(state),
        NavigationMsg::ToggleArt => playback_action::handle_toggle_art(state),
        NavigationMsg::ToggleTreeExpand => nav_action::handle_toggle_tree_expand(state),
    }
}

fn handle_playback_message(state: &mut AppState, msg: PlaybackMsg) -> Vec<AppMsg> {
    match msg {
        PlaybackMsg::RequestPlaySelected => playback_action::handle_request_play_selected(state),
        PlaybackMsg::RequestTogglePlayback => {
            playback_action::handle_request_toggle_playback(state)
        }
        PlaybackMsg::RequestStop => playback_action::handle_request_stop(state),
        PlaybackMsg::RequestSeek { offset_seconds } => {
            playback_action::handle_request_seek(state, offset_seconds)
        }
        PlaybackMsg::DispatchPlay { path } => playback_action::handle_dispatch_play(state, path),
        PlaybackMsg::DispatchPause => playback_action::handle_dispatch_pause(state),
        PlaybackMsg::DispatchResume => playback_action::handle_dispatch_resume(state),
        PlaybackMsg::DispatchStop => playback_action::handle_dispatch_stop(state),
        PlaybackMsg::DispatchSeek { position_ms, epoch } => {
            playback_action::handle_dispatch_seek(state, position_ms, epoch)
        }
        PlaybackMsg::DispatchSetVisualizer { spectrum_style } => {
            ui_action::handle_dispatch_set_visualizer(state, spectrum_style)
        }
        PlaybackMsg::AudioProgress { position_ms, epoch } => {
            playback_action::handle_audio_progress(state, position_ms, epoch)
        }
        PlaybackMsg::AudioStarted { path } => playback_action::handle_audio_started(state, path),
        PlaybackMsg::AudioPaused => playback_action::handle_audio_paused(state),
        PlaybackMsg::AudioResumed => playback_action::handle_audio_resumed(state),
        PlaybackMsg::AudioFinished => playback_action::handle_audio_finished(state),
        PlaybackMsg::AudioError { error } => playback_action::handle_audio_error(state, error),
        PlaybackMsg::RequestPlayNext => playback_action::handle_request_next(state),
        PlaybackMsg::RequestPlayPrev => playback_action::handle_request_prev(state),
    }
}

fn handle_audio_message(state: &mut AppState, msg: AudioMsg) -> Vec<AppMsg> {
    match msg {
        AudioMsg::AdjustVolume(delta) => audio_action::handle_adjust_volume(state, delta),
        AudioMsg::AdjustSpeed(delta) => audio_action::handle_adjust_speed(state, delta),
        AudioMsg::AdjustBass(delta) => audio_action::handle_adjust_bass(state, delta),
        AudioMsg::AdjustTreble(delta) => audio_action::handle_adjust_treble(state, delta),
        AudioMsg::AdjustReverb(delta) => audio_action::handle_adjust_reverb(state, delta),
        AudioMsg::AdjustEcho(delta) => audio_action::handle_adjust_echo(state, delta),
        AudioMsg::DispatchSetVolume(volume) => {
            audio_action::handle_dispatch_set_volume(state, volume)
        }
        AudioMsg::DispatchSetSpeed(speed) => audio_action::handle_dispatch_set_speed(state, speed),
        AudioMsg::DispatchSetBass(bass) => audio_action::handle_dispatch_set_bass(state, bass),
        AudioMsg::DispatchSetTreble(treble) => {
            audio_action::handle_dispatch_set_treble(state, treble)
        }
        AudioMsg::DispatchSetReverb(reverb) => {
            audio_action::handle_dispatch_set_reverb(state, reverb)
        }
        AudioMsg::DispatchSetEcho(echo) => audio_action::handle_dispatch_set_echo(state, echo),
    }
}

fn handle_ui_message(state: &mut AppState, msg: UIMsg) -> Vec<AppMsg> {
    match msg {
        UIMsg::EnterCommandMode => ui_action::handle_enter_command_mode(state),
        UIMsg::ExitCommandMode => ui_action::handle_exit_command_mode(state),
        UIMsg::UpdateCommand(c) => ui_action::handle_update_command(state, c),
        UIMsg::BackspaceCommand => ui_action::handle_backspace_command(state),
        UIMsg::SubmitCommand(input) => ui_action::handle_submit_command(state, input),
        UIMsg::OpenHelp => ui_action::handle_open_help(state),
        UIMsg::CloseHelp => ui_action::handle_close_help(state),
        UIMsg::Resize(w, h) => ui_action::handle_resize(state, w, h),
    }
}

fn handle_library_message(state: &mut AppState, msg: LibraryMsg) -> Vec<AppMsg> {
    match msg {
        LibraryMsg::AddToQueue(track) => library_action::handle_add_to_queue(state, track),
        LibraryMsg::PlayNext(track) => library_action::handle_play_next(state, track),
        LibraryMsg::ToggleRepeatMode => library_action::handle_toggle_repeat_mode(state),
        LibraryMsg::ToggleShuffle => library_action::handle_toggle_shuffle(state),
        LibraryMsg::RemoveFromQueue => library_action::handle_remove_from_queue(state),
        LibraryMsg::ClearQueue => library_action::handle_clear_queue(state),
        LibraryMsg::RequestScanLibrary => library_action::handle_request_scan_library(state),
        LibraryMsg::RequestToggleFavorite(id) => {
            library_action::handle_request_toggle_favorite(state, id)
        }
        LibraryMsg::Search { field, query } => library_action::handle_search(state, field, query),
        LibraryMsg::SearchNext => library_action::handle_search_next(state),
        LibraryMsg::SearchPrev => library_action::handle_search_prev(state),
        LibraryMsg::ClearSearch => {
            state.session.nav.library.search = Default::default();
            vec![]
        }
        LibraryMsg::RequestCreatePlaylist(name) => {
            library_action::handle_request_create_playlist(state, name)
        }
        LibraryMsg::RequestDeletePlaylist(name) => {
            library_action::handle_request_delete_playlist(state, name)
        }
        LibraryMsg::RequestAddToPlaylist {
            playlist_id,
            track_id,
        } => library_action::handle_request_add_to_playlist(state, playlist_id, track_id),
        LibraryMsg::RequestRemoveFromPlaylist {
            playlist_id,
            track_id,
        } => library_action::handle_request_remove_from_playlist(state, playlist_id, track_id),
        LibraryMsg::PlaylistCreated { id, name } => {
            state.library.add_playlist(id, name);
            vec![]
        }
    }
}

fn handle_download_message(state: &mut AppState, msg: DownloadMsg) -> Vec<AppMsg> {
    match msg {
        DownloadMsg::RequestDownload { query } => {
            download_action::handle_request_download(state, query)
        }
        DownloadMsg::BeginNextDownload => download_action::handle_begin_next_download(state),
        DownloadMsg::DispatchDownload {
            query,
            download_path,
        } => download_action::handle_dispatch_download(state, query, download_path),
        DownloadMsg::DownloadProgress { percent } => {
            download_action::handle_download_progress(state, percent)
        }
        DownloadMsg::DownloadFinished => download_action::handle_download_finished(state),
        DownloadMsg::DownloadFailed { error } => {
            download_action::handle_download_failed(state, error)
        }
        DownloadMsg::RequestCancelDownload => {
            download_action::handle_request_cancel_download(state)
        }
    }
}

fn handle_database_message(_state: &mut AppState, msg: DatabaseMsg) -> Vec<AppMsg> {
    match msg {
        // dispatch-only — these just forward to IPC, no local state change
        DatabaseMsg::DispatchToggleFavorite(_)
        | DatabaseMsg::DispatchCreatePlaylist(_)
        | DatabaseMsg::DispatchDeletePlaylist(_)
        | DatabaseMsg::DispatchAddToPlaylist { .. }
        | DatabaseMsg::DispatchRemoveFromPlaylist { .. } => vec![],
    }
}
