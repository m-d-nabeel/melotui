use rand::Rng;
use std::time::{Duration, Instant};

use crate::{
    AppMsg, AppState, Library,
    library::Track,
    message::playback::PlaybackMsg,
    state::{Notification, NotificationKind, PlaybackStatus, RepeatMode, VisualizerMode},
    utils::{SizeMode, bytes_to_ascii},
};

pub fn handle_request_play_selected(state: &mut AppState) -> Vec<AppMsg> {
    play_current_selection(state)
}

fn play_current_selection(state: &mut AppState) -> Vec<AppMsg> {
    let mut track_to_start: Option<Track> = None;
    let mut new_context: Option<(Vec<Track>, String, usize)> = None;
    let mut is_priority = false;

    if state.is_queue_focused() {
        if let Some(track) = state.get_selected_track() {
            track_to_start = Some(track.clone());
            is_priority = true;

            let idx = state.selected_queue_index();
            if idx < state.session.playback_queue.items.len() {
                state.session.playback_queue.items.remove(idx);
            }
            state.reconcile_nav();
        }
    } else if state.is_library_focused()
        && let Some(track) = state.get_selected_track()
    {
        track_to_start = Some(track.clone());
        let p_idx = state.selected_playlist_index();

        let (list, name) = if let Some(p) = state.library.playlists.get(p_idx) {
            (p.tracks.clone(), p.name.clone())
        } else {
            (vec![], "Unknown".to_string())
        };

        let idx = list.iter().position(|t| t.id == track.id).unwrap_or(0);
        new_context = Some((list, name, idx));
    }

    if let Some(track) = track_to_start {
        if let Some((queue, name, idx)) = new_context {
            state.session.playback.context_queue = queue;
            state.session.playback.source_name = name;
            state.session.playback.context_idx = idx;
        }

        start_playback(state, track, is_priority)
    } else {
        vec![]
    }
}

fn start_playback(state: &mut AppState, track: Track, from_queue: bool) -> Vec<AppMsg> {
    let path = track.file_path.clone();
    let title = track.title.clone();
    let has_cover = track.has_cover;

    state.session.playback.current = Some(track);
    state.session.playback.playing_from_queue = from_queue;

    state.session.playback.status = PlaybackStatus::Playing;
    state.session.playback.progress_ms = 0;
    state.session.playback.duration_ms = None;

    state.session.notification.current = Some(Notification {
        kind: NotificationKind::Info,
        message: format!("Playing {}", title),
        progress: None,
        created_at: Instant::now(),
        dismiss_after: Some(Duration::from_secs(2)),
    });

    // Art Loading
    state.current_album_art = None;
    if has_cover
        && let Some((data, _)) = Library::get_album_art(&path)
        && let Ok(ascii) = bytes_to_ascii(&data, SizeMode::Height(30))
    {
        state.current_album_art = Some(ascii);
    }

    vec![AppMsg::Playback(PlaybackMsg::DispatchPlay { path })]
}

fn playback_ended(state: &mut AppState) -> Vec<AppMsg> {
    state.session.playback.status = PlaybackStatus::Stopped;
    state.session.playback.progress_ms = 0;

    // REPEAT TRACK (Override)
    if *state.repeat_mode() == RepeatMode::Track
        && let Some(current) = &state.session.playback.current
    {
        let path = current.file_path.clone();
        state.session.playback.status = PlaybackStatus::Playing;
        return vec![AppMsg::Playback(PlaybackMsg::DispatchPlay { path })];
    }

    // PLAY ONCE (Stop)
    if *state.repeat_mode() == RepeatMode::PlayOnce {
        return vec![];
    }

    // Always consume the User Queue first.
    if let Some(track) = state.session.playback_queue.items.pop_front() {
        state.reconcile_nav();
        return start_playback(state, track, true);
    }

    if let Some(track) = get_next_context_track(state) {
        return start_playback(state, track, false);
    }

    vec![]
}

fn get_next_context_track(state: &mut AppState) -> Option<Track> {
    let len = state.session.playback.context_queue.len();
    if len == 0 {
        return None;
    }

    let next_idx = if state.shuffle_active() {
        if len > 1 {
            rand::rng().random_range(0..len)
        } else {
            0
        }
    } else {
        // Sequential: Pick up exactly where we left off + 1
        let next = state.session.playback.context_idx + 1;
        if next < len {
            next
        } else {
            // Loop Context
            if *state.repeat_mode() == RepeatMode::Playlist {
                0
            } else {
                return None;
            }
        }
    };

    state.session.playback.context_idx = next_idx;
    state.session.playback.context_queue.get(next_idx).cloned()
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
            if let Some(track) = &state.session.playback.current {
                let path = track.file_path.clone();
                state.session.playback.status = PlaybackStatus::Playing;
                vec![AppMsg::Playback(PlaybackMsg::DispatchPlay { path })]
            } else {
                vec![]
            }
        }
    }
}

pub fn handle_request_stop(state: &mut AppState) -> Vec<AppMsg> {
    state.session.playback.status = PlaybackStatus::Stopped;
    state.session.playback.progress_ms = 0;
    vec![AppMsg::Playback(PlaybackMsg::DispatchStop)]
}

pub fn handle_request_seek(_: &mut AppState, _: i64) -> Vec<AppMsg> {
    vec![]
}
pub fn handle_audio_progress(state: &mut AppState, position_ms: u64) -> Vec<AppMsg> {
    if state.is_playing() {
        state.session.playback.progress_ms = position_ms;
    }
    vec![]
}
pub fn handle_audio_finished(state: &mut AppState) -> Vec<AppMsg> {
    tracing::info!("Audio Finished...");
    playback_ended(state)
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
    let is_spectrum = state.should_draw_spectrum();
    vec![AppMsg::Playback(PlaybackMsg::DispatchSetVisualizer {
        spectrum_style: if is_spectrum {
            Some(state.spectrum_style().clone())
        } else {
            None
        },
    })]
}
// Dispatch no-ops
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
