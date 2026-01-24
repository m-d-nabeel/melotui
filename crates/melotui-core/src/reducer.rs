use crate::{
    AppMsg, AppState, action,
    message::{
        audio::AudioMsg, download::DownloadMsg, library::LibraryMsg, navigation::NavigationMsg,
        playback::PlaybackMsg, ui::UIMsg,
    },
};

pub fn reduce(state: &mut AppState, msg: AppMsg) -> Vec<AppMsg> {
    match msg {
        AppMsg::RequestQuit => action::library_action::handle_request_quit(state),
        AppMsg::Noop => vec![],
        AppMsg::Navigation(nav_msg) => handle_navigation_message(state, nav_msg),
        AppMsg::Playback(playback_msg) => handle_playback_message(state, playback_msg),
        AppMsg::Audio(audio_msg) => handle_audio_message(state, audio_msg),
        AppMsg::UI(ui_msg) => handle_ui_message(state, ui_msg),
        AppMsg::Library(library_msg) => handle_library_message(state, library_msg),
        AppMsg::Download(download_msg) => handle_download_message(state, download_msg),
    }
}

fn handle_navigation_message(state: &mut AppState, msg: NavigationMsg) -> Vec<AppMsg> {
    match msg {
        NavigationMsg::SwitchToPlaylists => action::nav_action::handle_switch_to_playlists(state),
        NavigationMsg::SwitchToTracks => action::nav_action::handle_switch_to_tracks(state),
        NavigationMsg::ToggleTrackInfo => action::nav_action::handle_toggle_track_info(state),
        NavigationMsg::ToggleLibraryQueue => action::nav_action::handle_toggle_library_queue(state),
        NavigationMsg::MoveUp => action::nav_action::handle_move_up(state),
        NavigationMsg::MoveDown => action::nav_action::handle_move_down(state),
        NavigationMsg::ToggleArt => action::playback_action::handle_toggle_art(state),
        NavigationMsg::ToggleTreeExpand => action::nav_action::handle_toggle_tree_expand(state),
    }
}

fn handle_playback_message(state: &mut AppState, msg: PlaybackMsg) -> Vec<AppMsg> {
    match msg {
        PlaybackMsg::RequestPlaySelected => {
            action::playback_action::handle_request_play_selected(state)
        }
        PlaybackMsg::RequestTogglePlayback => {
            action::playback_action::handle_request_toggle_playback(state)
        }
        PlaybackMsg::RequestStop => action::playback_action::handle_request_stop(state),
        PlaybackMsg::RequestSeek { offset_seconds } => {
            action::playback_action::handle_request_seek(state, offset_seconds)
        }
        PlaybackMsg::DispatchPlay { path } => {
            action::playback_action::handle_dispatch_play(state, path)
        }
        PlaybackMsg::DispatchPause => action::playback_action::handle_dispatch_pause(state),
        PlaybackMsg::DispatchResume => action::playback_action::handle_dispatch_resume(state),
        PlaybackMsg::DispatchStop => action::playback_action::handle_dispatch_stop(state),
        PlaybackMsg::DispatchSeek { position_ms } => {
            action::playback_action::handle_dispatch_seek(state, position_ms)
        }
        PlaybackMsg::DispatchSetVisualizer { spectrum_style } => {
            action::ui_action::handle_dispatch_set_visualizer(state, spectrum_style)
        }
        PlaybackMsg::AudioProgress { position_ms } => {
            action::playback_action::handle_audio_progress(state, position_ms)
        }
        PlaybackMsg::AudioStarted { path } => {
            action::playback_action::handle_audio_started(state, path)
        }
        PlaybackMsg::AudioPaused => action::playback_action::handle_audio_paused(state),
        PlaybackMsg::AudioResumed => action::playback_action::handle_audio_resumed(state),
        PlaybackMsg::AudioFinished => action::playback_action::handle_audio_finished(state),
        PlaybackMsg::AudioError { error } => {
            action::playback_action::handle_audio_error(state, error)
        }
    }
}

fn handle_audio_message(state: &mut AppState, msg: AudioMsg) -> Vec<AppMsg> {
    match msg {
        AudioMsg::AdjustVolume(delta) => action::audio_action::handle_adjust_volume(state, delta),
        AudioMsg::AdjustSpeed(delta) => action::audio_action::handle_adjust_speed(state, delta),
        AudioMsg::AdjustBass(delta) => action::audio_action::handle_adjust_bass(state, delta),
        AudioMsg::AdjustTreble(delta) => action::audio_action::handle_adjust_treble(state, delta),
        AudioMsg::AdjustReverb(delta) => action::audio_action::handle_adjust_reverb(state, delta),
        AudioMsg::AdjustEcho(delta) => action::audio_action::handle_adjust_echo(state, delta),
        AudioMsg::DispatchSetVolume(volume) => {
            action::audio_action::handle_dispatch_set_volume(state, volume)
        }
        AudioMsg::DispatchSetSpeed(speed) => {
            action::audio_action::handle_dispatch_set_speed(state, speed)
        }
        AudioMsg::DispatchSetBass(bass) => {
            action::audio_action::handle_dispatch_set_bass(state, bass)
        }
        AudioMsg::DispatchSetTreble(treble) => {
            action::audio_action::handle_dispatch_set_treble(state, treble)
        }
        AudioMsg::DispatchSetReverb(reverb) => {
            action::audio_action::handle_dispatch_set_reverb(state, reverb)
        }
        AudioMsg::DispatchSetEcho(echo) => {
            action::audio_action::handle_dispatch_set_echo(state, echo)
        }
    }
}

fn handle_ui_message(state: &mut AppState, msg: UIMsg) -> Vec<AppMsg> {
    match msg {
        UIMsg::EnterCommandMode => action::ui_action::handle_enter_command_mode(state),
        UIMsg::ExitCommandMode => action::ui_action::handle_exit_command_mode(state),
        UIMsg::UpdateCommand(c) => action::ui_action::handle_update_command(state, c),
        UIMsg::BackspaceCommand => action::ui_action::handle_backspace_command(state),
        UIMsg::SubmitCommand(input) => action::ui_action::handle_submit_command(state, input),
        UIMsg::ToggleHelp => action::ui_action::handle_toggle_help(state),
    }
}

fn handle_library_message(state: &mut AppState, msg: LibraryMsg) -> Vec<AppMsg> {
    match msg {
        LibraryMsg::AddToQueue(track) => action::library_action::handle_add_to_queue(state, track),
        LibraryMsg::PlayNext(track) => action::library_action::handle_play_next(state, track),
        LibraryMsg::ToggleRepeatMode => action::library_action::handle_toggle_repeat_mode(state),
        LibraryMsg::ToggleShuffle => action::library_action::handle_toggle_shuffle(state),
        LibraryMsg::RemoveFromQueue => action::library_action::handle_remove_from_queue(state),
        LibraryMsg::ClearQueue => action::library_action::handle_clear_queue(state),
        LibraryMsg::RequestRescanLibrary => {
            action::library_action::handle_request_rescan_library(state)
        }
    }
}

fn handle_download_message(state: &mut AppState, msg: DownloadMsg) -> Vec<AppMsg> {
    match msg {
        DownloadMsg::RequestDownload { query } => {
            action::download_action::handle_request_download(state, query)
        }
        DownloadMsg::BeginNextDownload => {
            action::download_action::handle_begin_next_download(state)
        }
        DownloadMsg::DispatchDownload {
            query,
            download_path,
        } => action::download_action::handle_dispatch_download(state, query, download_path),
        DownloadMsg::DownloadProgress { percent } => {
            action::download_action::handle_download_progress(state, percent)
        }
        DownloadMsg::DownloadFinished => action::download_action::handle_download_finished(state),
        DownloadMsg::DownloadFailed { error } => {
            action::download_action::handle_download_failed(state, error)
        }
        DownloadMsg::RequestCancelDownload => {
            action::download_action::handle_request_cancel_download(state)
        }
    }
}
