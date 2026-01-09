use crate::{AppMsg, AppState, action};

pub fn reduce(state: &mut AppState, msg: AppMsg) -> Vec<AppMsg> {
    match msg {
        // ───── Lifecycle ─────
        AppMsg::RequestQuit => action::library_action::handle_request_quit(state),
        AppMsg::Noop => vec![],

        // ───── Navigation ─────
        AppMsg::SwitchToPlaylists => action::nav_action::handle_switch_to_playlists(state),
        AppMsg::SwitchToTracks => action::nav_action::handle_switch_to_tracks(state),
        AppMsg::SwitchToTrackInfo => action::nav_action::handle_switch_to_track_info(state),
        AppMsg::ToggleLibraryQueue => action::nav_action::handle_toggle_library_queue(state),
        AppMsg::MoveUp => action::nav_action::handle_move_up(state),
        AppMsg::MoveDown => action::nav_action::handle_move_down(state),

        // ───── Queue & Repeat ─────
        AppMsg::AddToQueue(track) => action::library_action::handle_add_to_queue(state, track),
        AppMsg::PlayNext(track) => action::library_action::handle_play_next(state, track),
        AppMsg::ToggleRepeatMode => action::library_action::handle_toggle_repeat_mode(state),
        AppMsg::ToggleShuffle => action::library_action::handle_toggle_shuffle(state),
        AppMsg::RemoveFromQueue => action::library_action::handle_remove_from_queue(state),
        AppMsg::ClearQueue => action::library_action::handle_clear_queue(state),

        // ───── Library ─────
        AppMsg::RequestRescanLibrary => {
            action::library_action::handle_request_rescan_library(state)
        }

        // ───── Playback ─────
        AppMsg::RequestPlaySelected => action::playback_action::handle_request_play_selected(state),
        AppMsg::RequestTogglePlayback => {
            action::playback_action::handle_request_toggle_playback(state)
        }
        AppMsg::RequestStop => action::playback_action::handle_request_stop(state),
        AppMsg::RequestSeek { offset_seconds } => {
            action::playback_action::handle_request_seek(state, offset_seconds)
        }
        AppMsg::DispatchPlay { path } => action::playback_action::handle_dispatch_play(state, path),
        AppMsg::DispatchPause => action::playback_action::handle_dispatch_pause(state),
        AppMsg::DispatchResume => action::playback_action::handle_dispatch_resume(state),
        AppMsg::DispatchStop => action::playback_action::handle_dispatch_stop(state),
        AppMsg::DispatchSeek { position_ms } => {
            action::playback_action::handle_dispatch_seek(state, position_ms)
        }
        AppMsg::AudioProgress { position_ms } => {
            action::playback_action::handle_audio_progress(state, position_ms)
        }
        AppMsg::AudioStarted { path } => action::playback_action::handle_audio_started(state, path),
        AppMsg::AudioPaused => action::playback_action::handle_audio_paused(state),
        AppMsg::AudioResumed => action::playback_action::handle_audio_resumed(state),
        AppMsg::AudioFinished => action::playback_action::handle_audio_finished(state),
        AppMsg::AudioError { error } => action::playback_action::handle_audio_error(state, error),

        // ───── Audio Controls ─────
        AppMsg::AdjustVolume(delta) => action::audio_action::handle_adjust_volume(state, delta),
        AppMsg::AdjustSpeed(delta) => action::audio_action::handle_adjust_speed(state, delta),
        AppMsg::AdjustBass(delta) => action::audio_action::handle_adjust_bass(state, delta),
        AppMsg::AdjustTreble(delta) => action::audio_action::handle_adjust_treble(state, delta),
        AppMsg::AdjustReverb(delta) => action::audio_action::handle_adjust_reverb(state, delta),
        AppMsg::AdjustEcho(delta) => action::audio_action::handle_adjust_echo(state, delta),
        AppMsg::DispatchSetVolume(volume) => {
            action::audio_action::handle_dispatch_set_volume(state, volume)
        }
        AppMsg::DispatchSetSpeed(speed) => {
            action::audio_action::handle_dispatch_set_speed(state, speed)
        }
        AppMsg::DispatchSetBass(bass) => {
            action::audio_action::handle_dispatch_set_bass(state, bass)
        }
        AppMsg::DispatchSetTreble(treble) => {
            action::audio_action::handle_dispatch_set_treble(state, treble)
        }
        AppMsg::DispatchSetReverb(reverb) => {
            action::audio_action::handle_dispatch_set_reverb(state, reverb)
        }
        AppMsg::DispatchSetEcho(echo) => {
            action::audio_action::handle_dispatch_set_echo(state, echo)
        }

        // ───── Command bar ─────
        AppMsg::EnterCommandMode => action::ui_action::handle_enter_command_mode(state),
        AppMsg::ExitCommandMode => action::ui_action::handle_exit_command_mode(state),
        AppMsg::UpdateCommand(c) => action::ui_action::handle_update_command(state, c),
        AppMsg::BackspaceCommand => action::ui_action::handle_backspace_command(state),
        AppMsg::SubmitCommand(input) => action::ui_action::handle_submit_command(state, input),

        // ───── Downloads ─────
        AppMsg::RequestDownload { query } => {
            action::download_action::handle_request_download(state, query)
        }
        AppMsg::BeginNextDownload => action::download_action::handle_begin_next_download(state),
        AppMsg::DispatchDownload {
            query,
            download_path,
        } => action::download_action::handle_dispatch_download(state, query, download_path),
        AppMsg::DownloadProgress { percent } => {
            action::download_action::handle_download_progress(state, percent)
        }
        AppMsg::DownloadFinished => action::download_action::handle_download_finished(state),
        AppMsg::DownloadFailed { error } => {
            action::download_action::handle_download_failed(state, error)
        }
        AppMsg::RequestCancelDownload => {
            action::download_action::handle_request_cancel_download(state)
        }

        // ───── UI ─────
        AppMsg::ToggleArt => action::playback_action::handle_toggle_art(state),
        AppMsg::ToggleHelp => action::ui_action::handle_toggle_help(state),
        AppMsg::DispatchSetVisualizer { active } => {
            action::ui_action::handle_dispatch_set_visualizer(state, active)
        }
    }
}
