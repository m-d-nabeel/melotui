use rand::Rng;
use std::time::{Duration, Instant};

use crate::{
    AppMsg, AppState,
    domain::TrackId,
    message::playback::PlaybackMsg,
    playback::state::{PlaybackStatus, RepeatMode},
    state::{Notification, NotificationKind, VisualizerMode},
};

pub fn handle_request_play_selected(state: &mut AppState) -> Vec<AppMsg> {
    let mut selected_id: Option<TrackId> = None;
    let mut from_queue = false;

    // ---- QUEUE ----
    if state.is_queue_focused() {
        selected_id = state
            .session
            .playback_queue
            .selected_track_id(&state.session.nav.queue);

        if let Some(_) = selected_id {
            from_queue = true;
            let idx = state.session.nav.queue.selected_idx;
            state.session.playback_queue.items.remove(idx);
            state.reconcile_nav();
        }
    }
    // ---- LIBRARY ----
    else if state.is_library_focused() {
        selected_id = state
            .library
            .get_selected_track(&state.session.nav.library)
            .map(|t| t.id);

        if let Some(id) = selected_id {
            let playlist_idx = state.selected_playlist_index();
            if let Some(pl) = state.library.playlists.get(playlist_idx) {
                state.session.playback.context_queue = pl.tracks.iter().map(|t| t.id).collect();
                state.session.playback.source_name = pl.name.clone();
                state.session.playback.context_idx =
                    pl.tracks.iter().position(|t| t.id == id).unwrap_or(0);
            }
        }
    }

    match selected_id {
        Some(id) => start_playback(state, id, from_queue),
        None => vec![],
    }
}

pub fn handle_request_toggle_playback(state: &mut AppState) -> Vec<AppMsg> {
    match state.playback_status() {
        PlaybackStatus::Playing => {
            state.session.playback.status = PlaybackStatus::Paused;
            vec![AppMsg::Playback(PlaybackMsg::DispatchPause)]
        }
        PlaybackStatus::Paused => {
            if state.session.playback.current.is_some() {
                state.session.playback.status = PlaybackStatus::Playing;
                vec![AppMsg::Playback(PlaybackMsg::DispatchResume)]
            } else {
                vec![]
            }
        }
        PlaybackStatus::Stopped => {
            if let Some(id) = state.session.playback.current {
                if let Some(meta) = state.track_meta(id) {
                    state.session.playback.status = PlaybackStatus::Playing;
                    return vec![AppMsg::Playback(PlaybackMsg::DispatchPlay {
                        path: meta.file_path.clone(),
                    })];
                }
            }
            vec![]
        }
    }
}

pub fn handle_request_stop(state: &mut AppState) -> Vec<AppMsg> {
    state.session.playback.status = PlaybackStatus::Stopped;
    state.session.playback.progress_ms = 0;
    vec![AppMsg::Playback(PlaybackMsg::DispatchStop)]
}

pub fn handle_request_seek(state: &mut AppState, offset_seconds: i64) -> Vec<AppMsg> {
    if !state.is_playing() && !state.is_paused() {
        return vec![];
    }

    if let Some(last) = state.session.last_seek_time
        && last.elapsed() < Duration::from_millis(200)
    {
        return vec![];
    }

    state.session.last_seek_time = Some(Instant::now());

    let cur = state.progress_ms() as i64;
    let dur = state.total_duration_ms() as i64;
    let new_ms = (cur + offset_seconds * 1000).clamp(0, dur);

    state.session.playback.progress_ms = new_ms as u64;

    vec![AppMsg::Playback(PlaybackMsg::DispatchSeek {
        position_ms: new_ms as u64,
    })]
}

pub fn handle_audio_progress(state: &mut AppState, position_ms: u64) -> Vec<AppMsg> {
    if state.is_playing() {
        state.session.playback.progress_ms = position_ms;
    }
    vec![]
}

pub fn handle_audio_finished(state: &mut AppState) -> Vec<AppMsg> {
    state.session.playback.status = PlaybackStatus::Stopped;
    state.session.playback.progress_ms = 0;

    // Repeat Track
    if state.session.repeat_mode == RepeatMode::Track {
        if let Some(id) = state.session.playback.current {
            return start_playback(state, id, false);
        }
    }

    // Play Once
    if state.session.repeat_mode == RepeatMode::PlayOnce {
        return vec![];
    }

    // Queue first
    if let Some(id) = state.session.playback_queue.items.pop_front() {
        state.reconcile_nav();
        return start_playback(state, id, true);
    }

    // Context
    if let Some(id) = next_context_id(state) {
        return start_playback(state, id, false);
    }

    vec![]
}

pub fn handle_audio_error(state: &mut AppState, error: String) -> Vec<AppMsg> {
    state.session.playback.status = PlaybackStatus::Stopped;
    state.session.notification.current = Some(Notification {
        kind: NotificationKind::Error,
        message: format!("Audio Error: {}", error),
        progress: None,
        created_at: Instant::now(),
        dismiss_after: Some(Duration::from_secs(4)),
    });
    vec![]
}

pub fn handle_toggle_art(state: &mut AppState) -> Vec<AppMsg> {
    state.session.visualizer_mode = match state.session.visualizer_mode {
        VisualizerMode::Spectrum => VisualizerMode::AlbumArt,
        VisualizerMode::AlbumArt => VisualizerMode::Spectrum,
    };

    let active = state.should_draw_spectrum();
    vec![AppMsg::Playback(PlaybackMsg::DispatchSetVisualizer {
        spectrum_style: active.then(|| state.spectrum_style().clone()),
    })]
}

// ---- Dispatch no-ops ----
pub fn handle_dispatch_play(_: &mut AppState, _: std::path::PathBuf) -> Vec<AppMsg> {
    vec![]
}
pub fn handle_dispatch_pause(_: &mut AppState) -> Vec<AppMsg> {
    vec![]
}
pub fn handle_dispatch_resume(_: &mut AppState) -> Vec<AppMsg> {
    vec![]
}
pub fn handle_dispatch_stop(_: &mut AppState) -> Vec<AppMsg> {
    vec![]
}
pub fn handle_dispatch_seek(_: &mut AppState, _: u64) -> Vec<AppMsg> {
    vec![]
}
pub fn handle_audio_started(_: &mut AppState, _: String) -> Vec<AppMsg> {
    vec![]
}
pub fn handle_audio_paused(_: &mut AppState) -> Vec<AppMsg> {
    vec![]
}
pub fn handle_audio_resumed(_: &mut AppState) -> Vec<AppMsg> {
    vec![]
}

// ---- Helpers ----
fn start_playback(state: &mut AppState, id: TrackId, from_queue: bool) -> Vec<AppMsg> {
    let meta = match state.track_meta(id) {
        Some(m) => m,
        None => return vec![],
    };

    state.session.playback.current = Some(id);
    state.session.playback.playing_from_queue = from_queue;
    state.session.playback.status = PlaybackStatus::Playing;
    state.session.playback.progress_ms = 0;
    state.session.playback.duration_ms = Some(meta.duration_ms);

    state.session.notification.current = Some(Notification::new(
        NotificationKind::Info,
        format!("Playing {}", meta.title),
    ));

    vec![AppMsg::Playback(PlaybackMsg::DispatchPlay {
        path: meta.file_path.clone(),
    })]
}

fn next_context_id(state: &mut AppState) -> Option<TrackId> {
    let ctx = &state.session.playback.context_queue;
    if ctx.is_empty() {
        return None;
    }

    let next = if state.shuffle_active() {
        rand::rng().random_range(0..ctx.len())
    } else {
        let n = state.session.playback.context_idx + 1;
        if n < ctx.len() {
            n
        } else if state.session.repeat_mode == RepeatMode::Playlist {
            0
        } else {
            return None;
        }
    };

    state.session.playback.context_idx = next;
    ctx.get(next).copied()
}
