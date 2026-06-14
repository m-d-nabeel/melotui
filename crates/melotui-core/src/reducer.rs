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
        PlaybackMsg::RequestSeekRelative(offset) => {
            playback_action::handle_request_seek_relative(state, offset)
        }
        PlaybackMsg::RequestSeekAbsolute(pos) => {
            playback_action::handle_request_seek_absolute(state, pos)
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
        UIMsg::CommandInput(key) => ui_action::handle_command_input(state, key),
        UIMsg::SubmitCommand(input) => ui_action::handle_submit_command(state, input),
        UIMsg::Autocomplete => ui_action::handle_autocomplete(state),
        UIMsg::AutocompleteReverse => ui_action::handle_autocomplete_reverse(state),
        UIMsg::CommandHistoryPrev => ui_action::handle_command_history_prev(state),
        UIMsg::CommandHistoryNext => ui_action::handle_command_history_next(state),
        UIMsg::OpenHelp => ui_action::handle_open_help(state),
        UIMsg::CloseHelp => ui_action::handle_close_help(state),
        UIMsg::Resize(w, h) => ui_action::handle_resize(state, w, h),
        UIMsg::EnterPlaylistPicker => ui_action::handle_enter_playlist_picker(state),
        UIMsg::ExitPlaylistPicker => ui_action::handle_exit_playlist_picker(state),
        UIMsg::SubmitPlaylistPicker => ui_action::handle_submit_playlist_picker(state),
        UIMsg::PlaylistPickerMoveUp => ui_action::handle_playlist_picker_move_up(state),
        UIMsg::PlaylistPickerMoveDown => ui_action::handle_playlist_picker_move_down(state),
        UIMsg::PlaylistPickerUpdateQuery(c) => {
            ui_action::handle_playlist_picker_update_query(state, c)
        }
        UIMsg::PlaylistPickerBackspace => ui_action::handle_playlist_picker_backspace(state),
        UIMsg::DeletePlaylistConfirm => ui_action::handle_delete_playlist_confirm(state),
        UIMsg::ShowConfirm {
            message,
            on_confirm,
        } => ui_action::handle_show_confirm(state, message, on_confirm),
        UIMsg::ConfirmYes => ui_action::handle_confirm_yes(state),
        UIMsg::ConfirmNo => ui_action::handle_confirm_no(state),
        UIMsg::CycleFocusMode => ui_action::handle_cycle_focus_mode(state),
        UIMsg::SetFocusMode(mode) => ui_action::handle_set_focus_mode(state, mode),
        UIMsg::SetSidebarWidth(width) => ui_action::handle_set_sidebar_width(state, width),
        UIMsg::AdjustSidebarWidth(delta) => ui_action::handle_adjust_sidebar_width(state, delta),
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
            if let Some(track_id) = state.session.ui.pending_add_track.take() {
                return vec![AppMsg::Library(LibraryMsg::RequestAddToPlaylist {
                    playlist_id: id,
                    track_id,
                })];
            }
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
        DatabaseMsg::DispatchToggleFavorite(_)
        | DatabaseMsg::DispatchCreatePlaylist(_)
        | DatabaseMsg::DispatchDeletePlaylist(_)
        | DatabaseMsg::DispatchAddToPlaylist { .. }
        | DatabaseMsg::DispatchRemoveFromPlaylist { .. } => vec![],
    }
}
