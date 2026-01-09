use std::time::{Duration, Instant};

use crate::{
    AppMsg, AppState,
    action_handlers::{handler_play_current_selection, handler_playback_ended},
    state::{Notification, NotificationKind, PlaybackStatus, VisualizerMode},
};

pub fn handle_request_play_selected(state: &mut AppState) -> Vec<AppMsg> {
    tracing::info!("Play Requested");
    handler_play_current_selection(state)
}

pub fn handle_request_toggle_playback(state: &mut AppState) -> Vec<AppMsg> {
    match state.playback.status {
        PlaybackStatus::Playing => {
            state.playback.status = PlaybackStatus::Paused;
            vec![AppMsg::DispatchPause]
        }
        PlaybackStatus::Paused => {
            if state.playback.current.is_some() {
                state.playback.status = PlaybackStatus::Playing;
                vec![AppMsg::DispatchResume]
            } else {
                vec![]
            }
        }
        PlaybackStatus::Stopped => {
            if let Some(track_id) = &state.playback.current {
                state.playback.status = PlaybackStatus::Playing;
                vec![AppMsg::DispatchPlay {
                    path: track_id.path.clone(),
                }]
            } else {
                vec![]
            }
        }
    }
}

pub fn handle_request_stop(state: &mut AppState) -> Vec<AppMsg> {
    state.playback.status = PlaybackStatus::Stopped;
    state.playback.progress_ms = 0;
    vec![AppMsg::DispatchStop]
}

pub fn handle_request_seek(state: &mut AppState, _offset_seconds: i64) -> Vec<AppMsg> {
    let _ = state;
    // Placeholder for seek logic
    tracing::error!("Seeking not implemented yet");
    vec![]
}

pub fn handle_audio_progress(state: &mut AppState, position_ms: u64) -> Vec<AppMsg> {
    if state.playback.status == PlaybackStatus::Playing {
        state.playback.progress_ms = position_ms;
    }
    vec![]
}

pub fn handle_audio_finished(state: &mut AppState) -> Vec<AppMsg> {
    tracing::info!("Audio Finished...");
    handler_playback_ended(state)
}

pub fn handle_audio_paused(state: &mut AppState) -> Vec<AppMsg> {
    let _ = state;
    vec![]
}

pub fn handle_audio_resumed(state: &mut AppState) -> Vec<AppMsg> {
    let _ = state;
    vec![]
}

pub fn handle_audio_error(state: &mut AppState, error: String) -> Vec<AppMsg> {
    state.playback.status = PlaybackStatus::Stopped;
    state.notification.current = Some(Notification {
        kind: NotificationKind::Error,
        message: format!("Audio Error: {}", error),
        progress: None,
        created_at: Instant::now(),
        dismiss_after: Some(Duration::from_secs(4)),
    });
    vec![]
}

pub fn handle_toggle_art(state: &mut AppState) -> Vec<AppMsg> {
    state.visualizer_mode = match state.visualizer_mode {
        VisualizerMode::Spectrum => VisualizerMode::AlbumArt,
        VisualizerMode::AlbumArt => VisualizerMode::Spectrum,
    };
    let is_spectrum = state.visualizer_mode == VisualizerMode::Spectrum;
    vec![AppMsg::DispatchSetVisualizer {
        active: is_spectrum,
    }]
}

// Side-effect boundaries that do nothing in reducer
pub fn handle_dispatch_play(_state: &mut AppState, _path: std::path::PathBuf) -> Vec<AppMsg> {
    vec![]
}

pub fn handle_dispatch_pause(_state: &mut AppState) -> Vec<AppMsg> {
    vec![]
}

pub fn handle_dispatch_resume(_state: &mut AppState) -> Vec<AppMsg> {
    vec![]
}

pub fn handle_dispatch_stop(_state: &mut AppState) -> Vec<AppMsg> {
    vec![]
}

pub fn handle_dispatch_seek(_state: &mut AppState, _position_ms: u64) -> Vec<AppMsg> {
    vec![]
}

pub fn handle_audio_started(_state: &mut AppState, _path: String) -> Vec<AppMsg> {
    vec![]
}
