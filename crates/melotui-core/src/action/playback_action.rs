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

    // ─── 1. HANDLE QUEUE SELECTION ───────────────────────────────────────────
    if state.is_queue_focused() {
        if let Some(id) = state.session.nav.queue.selected_track_id {
            // Remove from queue logic (mirrors library_action)
            let queue_items = &mut state.session.playback_queue.items;
            if let Some(idx) = queue_items.iter().position(|&x| x == id) {
                queue_items.remove(idx);

                // Update selection to next item if possible
                if !queue_items.is_empty() {
                    let new_sel = if idx < queue_items.len() {
                        idx
                    } else {
                        queue_items.len() - 1
                    };
                    state.session.nav.queue.selected_track_id = queue_items.get(new_sel).copied();
                } else {
                    state.session.nav.queue.selected_track_id = None;
                }
            }

            target_track_id = Some(id);
            from_queue = true;
        }
    }
    // ─── 2. HANDLE LIBRARY / SIDEBAR SELECTION ───────────────────────────────
    else {
        // We clone the ID to avoid borrowing issues while mutating state later
        let selected_node = state.session.nav.library.selected_id.clone();

        if let Some(node) = selected_node {
            match node {
                // A. User selected a specific Track leaf
                SidebarNodeId::Track(id) => {
                    target_track_id = Some(id);
                    // TODO: In the future, we could infer the parent context
                    // (e.g. if inside a playlist) to populate the context queue.
                    // For now, playing a track directly plays just that track.
                    source_name = "Single Track".to_string();
                }

                // B. User selected a Playlist Node -> Play whole playlist
                SidebarNodeId::Playlist(pid) => {
                    if let Some(playlist) = state.library.get_playlist(pid)
                        && !playlist.tracks.is_empty()
                    {
                        target_track_id = Some(playlist.tracks[0]);
                        new_context = Some((playlist.tracks.clone(), 0));
                        source_name = playlist.name.clone();
                    }
                }

                // C. User selected "All Songs" -> Play all
                SidebarNodeId::Category(CategoryId::AllSongs) => {
                    // Collect all IDs (sorted by internal map order, or specific sort later)
                    let all_ids: Vec<TrackId> = state.library.tracks.keys().copied().collect();
                    if !all_ids.is_empty() {
                        target_track_id = Some(all_ids[0]);
                        new_context = Some((all_ids, 0));
                        source_name = "All Songs".to_string();
                    }
                }

                // D. Folders / Artists (Stub)
                _ => {}
            }
        }
    }

    // ─── 3. DISPATCH ─────────────────────────────────────────────────────────
    if let Some(id) = target_track_id {
        // Apply new context if we started a playlist/category
        if let Some((ctx_queue, ctx_idx)) = new_context {
            state.session.playback.context_queue = ctx_queue;
            state.session.playback.context_idx = ctx_idx;
            state.session.playback.source_name = source_name;
        }

        start_playback(state, id, from_queue)
    } else {
        vec![]
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
                // Fix: Use library.get_track instead of helper
                if let Some(meta) = state.library.get_track(id) {
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

    // Debounce seek
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

    // 1. Repeat Track Logic
    if state.session.repeat_mode == RepeatMode::Track
        && let Some(id) = state.session.playback.current
    {
        return start_playback(state, id, false);
    }

    // 2. Play Once Logic
    if state.session.repeat_mode == RepeatMode::PlayOnce {
        return vec![];
    }

    // 3. User Queue Priority
    if let Some(id) = state.session.playback_queue.items.pop_front() {
        // No nav reconciliation needed in new architecture
        return start_playback(state, id, true);
    }

    // 4. Context Queue (Playlist) Fallback
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
    // Clone style to avoid borrow issues if necessary, usually Copy is fine or Clone
    let style = state.session.spectrum_style.clone();

    vec![AppMsg::Playback(PlaybackMsg::DispatchSetVisualizer {
        spectrum_style: active.then_some(style),
    })]
}

// ---- Dispatch no-ops (Event Loop bridging) ----
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

// ---- Helpers ----------------------------------------------------------------

fn start_playback(state: &mut AppState, id: TrackId, from_queue: bool) -> Vec<AppMsg> {
    // FIX: Access library directly, no helper wrappers
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
