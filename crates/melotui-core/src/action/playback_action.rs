use std::time::{Duration, Instant};

use rand::Rng;

use crate::{
    AppMsg, AppState, Library,
    library::Track,
    message::playback::PlaybackMsg,
    reconcile_nav_state,
    state::{
        ActiveTrack, Notification, NotificationKind, PlaybackContext, PlaybackStatus, RepeatMode,
        VisualizerMode,
    },
    utils::{SizeMode, bytes_to_ascii},
};

pub fn handle_request_play_selected(state: &mut AppState) -> Vec<AppMsg> {
    tracing::info!("Play Requested");
    play_current_selection(state)
}

fn play_current_selection(state: &mut AppState) -> Vec<AppMsg> {
    let selected_track = state.get_selected_track();
    let track_to_play = if state.is_library_focused() {
        if state.is_playlist_view() {
            None
        } else {
            selected_track.as_ref()
        }
    } else if state.is_queue_focused() {
        selected_track.as_ref()
    } else {
        None
    };

    if let Some(track) = track_to_play {
        // Clone necessary fields to release borrow before mutating state
        let track_id = track.id;
        let track_path = track.file_path.clone();
        let track_title = track.title.clone();
        let has_cover = track.has_cover;

        let context = if state.is_library_focused() {
            PlaybackContext::Library {
                playlist_name: state.library.playlists[state.selected_playlist_index()]
                    .name
                    .clone(),
            }
        } else {
            PlaybackContext::Queue
        };

        // A. Update UI State IMMEDIATELY
        state.session.playback.current = Some(ActiveTrack {
            id: track_id,
            context,
            path: track_path.clone(),
        });
        state.session.playback.status = PlaybackStatus::Playing;
        state.session.playback.progress_ms = 0;
        state.session.playback.duration_ms = None;

        state.session.notification.current = Some(Notification {
            kind: NotificationKind::Info,
            message: format!("Playing {}", track_title),
            progress: None,
            created_at: Instant::now(),
            dismiss_after: Some(Duration::from_secs(2)),
        });

        state.current_album_art = None;
        if has_cover
            && let Some((data, _mime)) = Library::get_album_art(&track_path)
            && let Ok(ascii) = bytes_to_ascii(&data, SizeMode::Height(30))
        {
            state.current_album_art = Some(ascii);
        }

        return vec![AppMsg::Playback(PlaybackMsg::DispatchPlay {
            path: track_path,
        })];
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
            if state.get_playing_track().is_some() {
                state.session.playback.status = PlaybackStatus::Playing;
                vec![AppMsg::Playback(PlaybackMsg::DispatchResume)]
            } else {
                vec![]
            }
        }
        PlaybackStatus::Stopped => {
            if let Some(track) = state.get_playing_track() {
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

pub fn handle_request_seek(state: &mut AppState, _offset_seconds: i64) -> Vec<AppMsg> {
    let _ = state;
    // Placeholder for seek logic
    tracing::error!("Seeking not implemented yet");
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

pub fn handle_audio_paused(state: &mut AppState) -> Vec<AppMsg> {
    let _ = state;
    vec![]
}

pub fn handle_audio_resumed(state: &mut AppState) -> Vec<AppMsg> {
    let _ = state;
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
    let is_spectrum = state.should_draw_spectrum();
    vec![AppMsg::Playback(PlaybackMsg::DispatchSetVisualizer {
        active: is_spectrum,
    })]
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

fn playback_ended(state: &mut AppState) -> Vec<AppMsg> {
    state.session.playback.status = PlaybackStatus::Stopped;
    state.session.playback.progress_ms = 0;

    // 1. PRIORITY: Repeat One Track
    // If Repeat is Track, we ignore Shuffle and Queue logic, just loop.
    if *state.repeat_mode() == RepeatMode::Track
        && let Some(current) = state.get_playing_track()
    {
        let path = current.file_path.clone();
        state.session.playback.status = PlaybackStatus::Playing;
        return vec![AppMsg::Playback(PlaybackMsg::DispatchPlay { path })];
    }

    // 2. PRIORITY: Play Single (Stop)
    // If Single mode is on, we stop here.
    if *state.repeat_mode() == RepeatMode::PlayOnce {
        state.session.notification.current = Some(Notification {
            kind: NotificationKind::Info,
            message: "Playback Finished (Single)".into(),
            progress: None,
            created_at: Instant::now(),
            dismiss_after: Some(Duration::from_secs(2)),
        });
        return vec![];
    }

    // 3. LOGIC: Calculate Next Track
    let mut next_track_info: Option<Track> = None;
    let mut next_context: Option<PlaybackContext> = None;

    if let Some(current) = &state.session.playback.current {
        // Helper closure to pick a track index based on mode
        let get_next_index = |current_idx: usize, total_len: usize| -> Option<usize> {
            // A. SHUFFLE MODE
            if state.shuffle_active() {
                if total_len <= 1 {
                    return None;
                }
                let mut rng = rand::rng();
                return Some(rng.random_range(0..total_len));
            }

            // B. SEQUENTIAL MODE
            let next = current_idx + 1;
            if next < total_len {
                return Some(next);
            }

            // C. END OF LIST -> Loop if Repeat Playlist is on
            if *state.repeat_mode() == RepeatMode::Playlist {
                return Some(0);
            }

            // RepeatMode::Off -> Stop at end
            None
        };

        // --- CONTEXT LOGIC ---
        match &current.context {
            PlaybackContext::Queue => {
                // Case A: We are ALREADY in the Queue.
                // Consumer behavior: remove the current track (the one that just finished)
                if let Some(idx) = state
                    .session
                    .playback_queue
                    .tracks
                    .iter()
                    .position(|t| t.file_path == current.path)
                {
                    state.session.playback_queue.tracks.remove(idx);
                    reconcile_nav_state(state);
                }

                next_context = Some(PlaybackContext::Queue);
                // Peek the NEW front (Index 0) to play next
                if !state.session.playback_queue.tracks.is_empty() {
                    next_track_info = state.session.playback_queue.tracks.front().cloned();
                }
            }
            PlaybackContext::Library { playlist_name } => {
                // Case B: We are in the Library.

                // FIX: Check Queue. If exists, Play but DO NOT POP yet.
                if !state.session.playback_queue.tracks.is_empty() {
                    next_context = Some(PlaybackContext::Queue);
                    next_track_info = state.session.playback_queue.tracks.front().cloned();
                    reconcile_nav_state(state);
                } else {
                    // Queue is empty, continue with Library logic.
                    next_context = Some(PlaybackContext::Library {
                        playlist_name: playlist_name.clone(),
                    });

                    if let Some(playlist) = state
                        .library
                        .playlists
                        .iter()
                        .find(|p| p.name == *playlist_name)
                        && let Some(idx) = playlist
                            .tracks
                            .iter()
                            .position(|t| t.file_path == current.path)
                        && let Some(next_idx) = get_next_index(idx, playlist.tracks.len())
                    {
                        next_track_info = playlist.tracks.get(next_idx).cloned();
                    }
                }
            }
        }
    }

    // 4. EXECUTE
    if let Some(track) = next_track_info {
        // Use the context calculated in Step 3
        let context = next_context.unwrap_or(PlaybackContext::Queue);

        state.session.playback.current = Some(ActiveTrack {
            id: track.id,
            context,
            path: track.file_path.clone(),
        });
        state.session.playback.status = PlaybackStatus::Playing;

        // UI Updates ...
        state.session.notification.current = Some(Notification {
            kind: NotificationKind::Info,
            message: format!("Now Playing: {}", track.title),
            progress: None,
            created_at: Instant::now(),
            dismiss_after: Some(Duration::from_secs(2)),
        });

        state.current_album_art = None;
        if track.has_cover
            && let Some((data, _)) = Library::get_album_art(&track.file_path)
            && let Ok(ascii) = bytes_to_ascii(&data, SizeMode::Height(30))
        {
            state.current_album_art = Some(ascii);
        }

        return vec![AppMsg::Playback(PlaybackMsg::DispatchPlay {
            path: track.file_path.clone(),
        })];
    }

    vec![]
}
