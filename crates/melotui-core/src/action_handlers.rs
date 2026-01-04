use std::{
    fs,
    time::{Duration, Instant},
};

use rand::Rng;

use crate::{
    AppMsg, AppState, Command, Library, UIMode,
    art_converter::{SizeMode, bytes_to_ascii},
    library::Track,
    reconcile_nav_state,
    state_registry::{
        DownloadJob, DownloadStatus, LeftPanelMode, LibraryViewDepth, Notification,
        NotificationKind, PlaybackStatus, RepeatMode, TrackId,
    },
};

pub fn handler_play_current_selection(state: &mut AppState) -> Vec<AppMsg> {
    let track_to_play = match state.nav.active_panel {
        LeftPanelMode::Library => {
            if state.nav.library.depth == LibraryViewDepth::Playlists {
                None
            } else {
                let p = state.nav.library.selected_playlist_idx;
                let t = state
                    .nav
                    .library
                    .selected_tracks_curs
                    .get(p)
                    .copied()
                    .unwrap_or(0);
                state
                    .library
                    .playlists
                    .get(p)
                    .and_then(|pl| pl.tracks.get(t))
                    .cloned()
            }
        }
        LeftPanelMode::Queue => state
            .playback_queue
            .tracks
            .get(state.nav.queue.selected_idx)
            .cloned(),
    };

    if let Some(track) = track_to_play {
        let playlist_name = match state.nav.active_panel {
            LeftPanelMode::Library => state.library.playlists
                [state.nav.library.selected_playlist_idx]
                .name
                .clone(),
            LeftPanelMode::Queue => "Queue".to_string(),
        };

        // A. Update UI State IMMEDIATELY
        state.playback.current = Some(TrackId {
            playlist: playlist_name,
            path: track.file_path.clone(),
        });
        state.playback.status = PlaybackStatus::Playing;
        state.playback.progress_ms = 0;
        state.playback.duration_ms = None;

        state.notification.current = Some(Notification {
            kind: NotificationKind::Info,
            message: format!("Playing {}", track.title),
            progress: None,
            created_at: Instant::now(),
            dismiss_after: Some(Duration::from_secs(2)),
        });

        state.current_album_art = None;
        if track.has_cover
            && let Some((data, _mime)) = Library::get_album_art(&track.file_path)
            && let Ok(ascii) = bytes_to_ascii(&data, SizeMode::Height(30))
        {
            state.current_album_art = Some(ascii);
        }

        return vec![AppMsg::DispatchPlay {
            path: track.file_path.clone(),
        }];
    }
    vec![]
}

pub fn handler_process_command_input(state: &mut AppState, input: String) -> Vec<AppMsg> {
    state.ui.mode = UIMode::Normal;
    state.ui.command_buffer.clear();

    let input = input.trim();
    if input.is_empty() {
        return vec![];
    }

    match crate::Command::parse(input) {
        Command::Quit => {
            vec![AppMsg::RequestQuit]
        }
        Command::Rescan => {
            vec![AppMsg::RequestRescanLibrary]
        }
        Command::Download { query } => {
            state.notification.current = Some(Notification {
                kind: NotificationKind::Info,
                message: format!("Downloading: {}", query),
                progress: None,
                created_at: Instant::now(),
                dismiss_after: Some(Duration::from_secs(2)),
            });
            vec![AppMsg::RequestDownload { query }]
        }
        Command::ChangeVisualizer { arg } => {
            state.spectrum_style = arg;
            vec![]
        }
        Command::Unknown(reason) => {
            state.notification.current = Some(Notification {
                kind: NotificationKind::Error,
                message: format!("Unknown command: {}", reason),
                progress: None,
                created_at: Instant::now(),
                dismiss_after: Some(Duration::from_secs(2)),
            });
            vec![]
        }
    }
}

pub fn handler_enqueue_download(state: &mut AppState, query: String) -> Vec<AppMsg> {
    let root = &state.library.root;
    let download_dir = root.join("Downloads");

    if !download_dir.exists()
        && let Err(e) = fs::create_dir_all(&download_dir)
    {
        state.notification.current = Some(Notification {
            kind: NotificationKind::Error,
            message: format!("Failed to create Downloads folder: {e}"),
            progress: None,
            created_at: Instant::now(),
            dismiss_after: Some(Duration::from_secs(4)),
        });
        return vec![];
    }

    state.download.queue.push_back(DownloadJob {
        query: query.clone(),
        download_path: download_dir,
    });

    state.notification.current = Some(Notification {
        kind: NotificationKind::Info,
        message: format!("Queued: {}", query),
        progress: None,
        created_at: Instant::now(),
        dismiss_after: Some(Duration::from_secs(2)),
    });

    if matches!(state.download.status, DownloadStatus::Idle) {
        return vec![AppMsg::BeginNextDownload];
    }
    vec![]
}

pub fn handler_playback_ended(state: &mut AppState) -> Vec<AppMsg> {
    state.playback.status = PlaybackStatus::Stopped;
    state.playback.progress_ms = 0;

    // 1. PRIORITY: Repeat One Track
    // If Repeat is Track, we ignore Shuffle and Queue logic, just loop.
    if state.repeat_mode == RepeatMode::Track
        && let Some(current) = &state.playback.current
    {
        state.playback.status = PlaybackStatus::Playing;
        return vec![AppMsg::DispatchPlay {
            path: current.path.clone(),
        }];
    }

    // 2. PRIORITY: Play Single (Stop)
    // If Single mode is on, we stop here.
    if state.repeat_mode == RepeatMode::PlayOnce {
        state.notification.current = Some(Notification {
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
    let mut next_context_name = String::new();

    if let Some(current) = &state.playback.current {
        // Helper closure to pick a track index based on mode
        let get_next_index = |current_idx: usize, total_len: usize| -> Option<usize> {
            // A. SHUFFLE MODE
            if state.playback.shuffle {
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
            if state.repeat_mode == RepeatMode::Playlist {
                return Some(0);
            }

            // RepeatMode::Off -> Stop at end
            None
        };

        // --- CONTEXT LOGIC ---

        if current.playlist == "Queue" {
            // Case A: We are ALREADY in the Queue.
            // Consumer behavior: remove the current track and play the next one
            if let Some(idx) = state
                .playback_queue
                .tracks
                .iter()
                .position(|t| t.file_path == current.path)
            {
                state.playback_queue.tracks.remove(idx);
                reconcile_nav_state(state);
            }

            next_context_name = "Queue".to_string();
            if !state.playback_queue.tracks.is_empty() {
                next_track_info = state.playback_queue.tracks.front().cloned();
            }
        } else {
            // Case B: We are in the Library (or a named Playlist).

            // **CRITICAL FIX**: Check if the Queue has pending tracks ("Play Next").
            // If the queue is not empty, we switch context to the Queue and play index 0.
            if !state.playback_queue.tracks.is_empty() {
                next_context_name = "Queue".to_string();
                // Play the first item in the queue (FIFO) and remove it (consumer behavior)
                next_track_info = state.playback_queue.tracks.pop_front();
                reconcile_nav_state(state);
            } else {
                // Queue is empty, continue with Library logic.
                next_context_name = current.playlist.clone();
                if let Some(playlist) = state
                    .library
                    .playlists
                    .iter()
                    .find(|p| p.name == current.playlist)
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

    // 4. EXECUTE
    if let Some(track) = next_track_info {
        state.playback.current = Some(TrackId {
            playlist: next_context_name,
            path: track.file_path.clone(),
        });
        state.playback.status = PlaybackStatus::Playing;

        // UI Updates (Notification + Art)
        state.notification.current = Some(Notification {
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

        return vec![AppMsg::DispatchPlay {
            path: track.file_path.clone(),
        }];
    }

    vec![]
}
