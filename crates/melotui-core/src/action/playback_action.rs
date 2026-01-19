use rand::Rng;
use std::time::{Duration, Instant};

use crate::{
    AppMsg, AppState,
    domain::TrackId,
    message::playback::PlaybackMsg,
    navigation::ids::{CategoryId, SidebarNodeId},
    playback::state::{PlaybackStatus, RepeatMode},
    state::{Notification, NotificationKind, VisualizerMode},
};

pub fn handle_request_play_selected(state: &mut AppState) -> Vec<AppMsg> {
    let mut target_track_id: Option<TrackId> = None;
    let mut from_queue = false;
    let mut new_context: Option<(Vec<TrackId>, usize)> = None;
    let mut source_name = String::new();

    // ─── 1. HANDLE QUEUE SELECTION (Non-Destructive) ─────────────────────────
    if state.is_queue_focused() {
        if let Some(qid) = state.session.nav.queue.selected_queue_id {
            // Find the track but DO NOT remove it
            if let Some(entry) = state.queue().iter().find(|e| e.id == qid) {
                target_track_id = Some(entry.track_id);
                // Set the specific queue row as active
                state.session.playback.active_queue_id = Some(qid);
                from_queue = true;
            }
        }
    }
    // ─── 2. HANDLE LIBRARY SELECTION ─────────────────────────────────────────
    else {
        let selected_node = state.session.nav.library.selected_id.clone();
        if let Some(node) = selected_node {
            match node {
                SidebarNodeId::Track { id, .. } => {
                    target_track_id = Some(id);
                    source_name = "Single Track".to_string();
                }
                SidebarNodeId::Playlist(pid) => {
                    if let Some(playlist) = state.library.get_playlist(pid)
                        && !playlist.tracks.is_empty()
                    {
                        target_track_id = Some(playlist.tracks[0]);
                        new_context = Some((playlist.tracks.clone(), 0));
                        source_name = playlist.name.clone();
                    }
                }
                SidebarNodeId::Category(CategoryId::AllSongs) => {
                    let all_ids: Vec<TrackId> = state.library.tracks.keys().copied().collect();
                    if !all_ids.is_empty() {
                        target_track_id = Some(all_ids[0]);
                        new_context = Some((all_ids, 0));
                        source_name = "All Songs".to_string();
                    }
                }
                _ => {}
            }
        }
    }

    // ─── 3. DISPATCH ─────────────────────────────────────────────────────────
    if let Some(id) = target_track_id {
        if let Some((ctx_queue, ctx_idx)) = new_context {
            state.session.playback.context_queue = ctx_queue;
            state.session.playback.context_idx = ctx_idx;
            state.session.playback.source_name = source_name;
        }

        // If playing from library, clear queue state to avoid confusion
        if !from_queue {
            state.session.playback.active_queue_id = None;
        }

        start_playback(state, id, from_queue)
    } else {
        vec![]
    }
}

pub fn handle_audio_finished(state: &mut AppState) -> Vec<AppMsg> {
    state.session.playback.status = PlaybackStatus::Stopped;
    state.session.playback.progress_ms = 0;

    // 1. Repeat Track Logic
    if state.session.repeat_mode == RepeatMode::Track
        && let Some(id) = state.session.playback.current
    {
        return start_playback(state, id, state.is_playing_from_queue());
    }

    // 2. Queue Logic (Pop-After-Play)
    if let Some(finished_qid) = state.session.playback.active_queue_id {
        let queue_items = &mut state.session.playback_queue.items;

        // Find index of finished item
        if let Some(idx) = queue_items.iter().position(|e| e.id == finished_qid) {
            // Check for next item BEFORE removing
            let next_entry = if idx + 1 < queue_items.len() {
                Some(queue_items[idx + 1].clone())
            } else {
                None
            };

            // POP: Remove the finished song
            queue_items.remove(idx);

            // Play next if exists
            if let Some(entry) = next_entry {
                state.session.playback.active_queue_id = Some(entry.id);
                return start_playback(state, entry.track_id, true);
            } else {
                state.session.playback.active_queue_id = None;
            }
        } else {
            state.session.playback.active_queue_id = None;
        }
    }
    // Fallback: Legacy/Empty Queue
    else if state.is_playing_from_queue()
        && !state.queue().is_empty()
        && let Some(entry) = state.session.playback_queue.items.pop_front()
    {
        state.session.playback.active_queue_id = Some(entry.id);
        return start_playback(state, entry.track_id, true);
    }

    // 3. Context Queue (Playlist) Fallback
    if let Some(id) = next_context_id(state) {
        return start_playback(state, id, false);
    }

    vec![]
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
            if let Some(id) = state.session.playback.current
                && let Some(meta) = state.library.get_track(id)
            {
                state.session.playback.status = PlaybackStatus::Playing;
                return vec![AppMsg::Playback(PlaybackMsg::DispatchPlay {
                    path: meta.file_path.clone(),
                })];
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
    let style = state.session.spectrum_style.clone();
    vec![AppMsg::Playback(PlaybackMsg::DispatchSetVisualizer {
        spectrum_style: active.then_some(style),
    })]
}

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

fn start_playback(state: &mut AppState, id: TrackId, from_queue: bool) -> Vec<AppMsg> {
    let meta = match state.library.get_track(id) {
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
