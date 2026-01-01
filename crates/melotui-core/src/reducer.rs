use std::{
    time::{Duration, Instant},
    vec,
};

use crate::{
    AppMsg, AppState, Command, Library, NavMode, UIMode,
    state_registry::{
        DownloadJob, DownloadStatus, Notification, NotificationKind, PlaybackStatus, TrackId,
    },
};

fn reconcile_nav_state(state: &mut AppState) {
    let playlist_count = state.library.playlists.len();

    // Clamp selected playlist
    if playlist_count == 0 {
        state.nav.selected_playlist = 0;
        state.nav.selected_tracks.clear();
        return;
    }

    if state.nav.selected_playlist >= playlist_count {
        state.nav.selected_playlist = playlist_count - 1;
    }

    // Resize per-playlist track cursors
    state.nav.selected_tracks.resize(playlist_count, 0);

    // Clamp each track cursor
    for (i, playlist) in state.library.playlists.iter().enumerate() {
        let track_count = playlist.tracks.len();

        if track_count == 0 {
            state.nav.selected_tracks[i] = 0;
        } else if state.nav.selected_tracks[i] >= track_count {
            state.nav.selected_tracks[i] = track_count - 1;
        }
    }
}

pub fn reduce(state: &mut AppState, msg: AppMsg) -> Vec<AppMsg> {
    match msg {
        AppMsg::RequestQuit => {
            tracing::info!("Application quit requested");
            state.notification.current = Some(Notification {
                kind: NotificationKind::Info,
                message: "Quit".into(),
                progress: None,
                created_at: Instant::now(),
                dismiss_after: None,
            });
        }
        AppMsg::SwitchToPlaylists => {
            tracing::debug!("Switch to Playlist Mode");
            state.nav.mode = NavMode::Playlists;
        }
        AppMsg::SwitchToTracks => {
            tracing::debug!("Switch to Track Mode");
            state.nav.mode = NavMode::Tracks;
        }
        AppMsg::MoveUp => match state.nav.mode {
            NavMode::Playlists => {
                if state.nav.selected_playlist > 0 {
                    state.nav.selected_playlist -= 1;
                }
            }
            NavMode::Tracks => {
                let p = state.nav.selected_playlist;
                let cursor = &mut state.nav.selected_tracks[p];

                if *cursor > 0 {
                    *cursor -= 1;
                }
            }
        },
        AppMsg::MoveDown => match state.nav.mode {
            NavMode::Playlists => {
                if state.nav.selected_playlist + 1 < state.library.playlists.len() {
                    state.nav.selected_playlist += 1;
                }
            }
            NavMode::Tracks => {
                let p = state.nav.selected_playlist;
                let tracks = &state.library.playlists[p].tracks;
                let cursor = &mut state.nav.selected_tracks[p];

                if *cursor + 1 < tracks.len() {
                    *cursor += 1;
                }
            }
        },
        AppMsg::RequestRescanLibrary => {
            match Library::scan(state.library.root.clone()) {
                Ok(new_library) => {
                    state.library = new_library;
                    reconcile_nav_state(state);
                    tracing::info!("Library rescanned");

                    // reconcile playback
                    if let Some(current) = &state.playback.current {
                        let still_exists = state.library.playlists.iter().any(|pl| {
                            pl.name == current.playlist
                                && pl.tracks.iter().any(|p| p == &current.path)
                        });

                        if !still_exists {
                            state.playback.current = None;
                            state.playback.status = PlaybackStatus::Stopped;
                            state.notification.current = Some(Notification {
                                kind: NotificationKind::Info,
                                message: "Playback stopped (file removed)".into(),
                                progress: None,
                                created_at: Instant::now(),
                                dismiss_after: Some(Duration::from_secs(2)),
                            });
                        }
                    }

                    state.notification.current = Some(Notification {
                        kind: NotificationKind::Info,
                        message: "Library rescanned".into(),
                        progress: None,
                        created_at: Instant::now(),
                        dismiss_after: Some(Duration::from_secs(2)),
                    });
                }
                Err(err) => {
                    tracing::error!("Library rescan failed: {}", err);
                    state.notification.current = Some(Notification {
                        kind: NotificationKind::Info,
                        message: format!("Rescan failed: {err}"),
                        progress: None,
                        created_at: Instant::now(),
                        dismiss_after: Some(Duration::from_secs(2)),
                    });
                }
            }
        }
        AppMsg::EnterCommandMode => {
            tracing::debug!("Enter Command Mode");
            state.ui.mode = UIMode::Command;
            state.ui.command_buffer.clear();
        }
        AppMsg::ExitCommandMode => {
            tracing::debug!("Exit Command Mode");
            state.ui.mode = UIMode::Normal;
            state.ui.command_buffer.clear();
        }
        AppMsg::UpdateCommand(c) => {
            state.ui.command_buffer.push(c);
        }
        AppMsg::BackspaceCommand => {
            state.ui.command_buffer.pop();
        }
        AppMsg::SubmitCommand(input) => {
            state.ui.mode = UIMode::Normal;
            state.ui.command_buffer.clear();

            let input = input.trim();
            if input.is_empty() {
                return vec![];
            }

            match Command::parse(input) {
                Command::Quit => {
                    return vec![AppMsg::RequestQuit];
                }

                Command::Rescan => {
                    return vec![AppMsg::RequestRescanLibrary];
                }

                Command::Download { query } => {
                    tracing::debug!("Command::Download <- {}", query);

                    state.notification.current = Some(Notification {
                        kind: NotificationKind::Info,
                        message: format!("Downloading: {}", query),
                        progress: None,
                        created_at: Instant::now(),
                        dismiss_after: Some(Duration::from_secs(2)),
                    });

                    return vec![AppMsg::RequestDownload { query }];
                }

                Command::Unknown(reason) => {
                    state.notification.current = Some(Notification {
                        kind: NotificationKind::Error,
                        message: format!("Unknown command: {}", reason),
                        progress: None,
                        created_at: Instant::now(),
                        dismiss_after: Some(Duration::from_secs(2)),
                    });
                    return vec![];
                }
            }
        }
        AppMsg::RequestDownload { query } => {
            tracing::info!("Queueing download: {}", query);

            let curr_playlist_idx = state.nav.selected_playlist;
            let download_path = match state.library.playlists.get(curr_playlist_idx) {
                Some(pl) => pl.path.to_path_buf(),
                None => {
                    tracing::warn!("No playlist found at index {}.", curr_playlist_idx);
                    return vec![];
                }
            };

            state.download.queue.push_back(DownloadJob {
                query,
                download_path,
            });
            // If nothing is running, start immediately
            if matches!(state.download.status, DownloadStatus::Idle) {
                return vec![AppMsg::BeginNextDownload];
            }
            return vec![];
        }
        AppMsg::BeginNextDownload => {
            let job = match state.download.queue.pop_front() {
                Some(job) => job,
                None => return vec![],
            };

            tracing::info!("Starting download: {}", job.query);
            state.download.status = DownloadStatus::Running { percent: 0 };
            // THIS IS THE IPC TRIGGER
            return vec![AppMsg::DispatchDownload {
                query: job.query,
                download_path: job.download_path,
            }];
        }
        AppMsg::DownloadProgress { percent } => {
            state.notification.current = Some(Notification {
                kind: NotificationKind::Progress,
                message: "".to_owned(),
                progress: Some(percent),
                created_at: Instant::now(),
                dismiss_after: None,
            });
            if let DownloadStatus::Running { .. } = &state.download.status {
                state.download.status = DownloadStatus::Running { percent };
            }
        }
        AppMsg::DownloadFinished => {
            state.notification.current = Some(Notification {
                kind: NotificationKind::Info,
                message: "Download complete".into(),
                progress: Some(100),
                created_at: Instant::now(),
                dismiss_after: Some(Duration::from_secs(2)),
            });

            state.download.status = DownloadStatus::Idle;

            return vec![
                AppMsg::RequestRescanLibrary,
                AppMsg::BeginNextDownload, // ← THIS is the queue
            ];
        }
        AppMsg::DownloadFailed { error } => {
            tracing::error!("Download failed: {}", error);
            state.notification.current = Some(Notification {
                kind: NotificationKind::Error,
                message: error.clone(),
                progress: None,
                created_at: Instant::now(),
                dismiss_after: Some(Duration::from_secs(4)),
            });

            state.download.status = DownloadStatus::Idle;

            return vec![AppMsg::BeginNextDownload];
        }
        AppMsg::RequestCancelDownload => {
            tracing::info!("Cancelling download");

            state.notification.current = Some(Notification {
                kind: NotificationKind::Error,
                message: "Cancelling Download".into(),
                progress: None,
                created_at: Instant::now(),
                dismiss_after: Some(Duration::from_secs(4)),
            });

            if let DownloadStatus::Running { .. } = &state.download.status {
                state.download.status = DownloadStatus::Cancelling;
            }

            return vec![];
        }
        AppMsg::DispatchDownload { .. } => {}
        AppMsg::RequestPlaySelected => {
            if state.nav.mode != NavMode::Tracks {
                return vec![];
            }

            let p = state.nav.selected_playlist;
            if let Some(playlist) = state.library.playlists.get(p) {
                let t = state.nav.selected_tracks[p];
                if let Some(path) = playlist.tracks.get(t) {
                    // A. Update UI State IMMEDIATELY
                    state.playback.current = Some(TrackId {
                        playlist: playlist.name.clone(),
                        path: path.clone(),
                    });
                    state.playback.status = PlaybackStatus::Playing;
                    state.playback.progress_ms = 0;
                    state.playback.duration_ms = None; // Will be filled by AudioStarted later

                    state.notification.current = Some(Notification {
                        kind: NotificationKind::Info,
                        message: format!("Playing {}", path.file_name().unwrap().to_string_lossy()),
                        progress: None,
                        created_at: Instant::now(),
                        dismiss_after: Some(Duration::from_secs(2)),
                    });

                    // B. Trigger Side-Effect
                    return vec![AppMsg::DispatchPlay { path: path.clone() }];
                }
            }
        }

        // 2. User toggles Play/Pause
        AppMsg::RequestTogglePlayback => {
            match state.playback.status {
                // Case A: Playing -> Pause
                PlaybackStatus::Playing => {
                    state.playback.status = PlaybackStatus::Paused;
                    state.notification.current = Some(Notification {
                        kind: NotificationKind::Info,
                        message: "Paused".into(),
                        progress: None,
                        created_at: Instant::now(),
                        dismiss_after: Some(Duration::from_secs(1)),
                    });
                    return vec![AppMsg::DispatchPause];
                }

                // Case B: Paused -> Resume
                PlaybackStatus::Paused => {
                    if state.playback.current.is_some() {
                        state.playback.status = PlaybackStatus::Playing;
                        state.notification.current = Some(Notification {
                            kind: NotificationKind::Info,
                            message: "Resumed".into(),
                            progress: None,
                            created_at: Instant::now(),
                            dismiss_after: Some(Duration::from_secs(1)),
                        });
                        return vec![AppMsg::DispatchResume];
                    }
                }

                // Case C: Stopped -> Replay
                PlaybackStatus::Stopped => {
                    // If we have a selected track, start it over from 0
                    if let Some(track) = &state.playback.current {
                        state.playback.status = PlaybackStatus::Playing;
                        state.playback.progress_ms = 0; // Reset visual progress

                        state.notification.current = Some(Notification {
                            kind: NotificationKind::Info,
                            message: "Replaying".into(),
                            progress: None,
                            created_at: Instant::now(),
                            dismiss_after: Some(Duration::from_secs(1)),
                        });

                        // We use DispatchPlay, NOT DispatchResume
                        return vec![AppMsg::DispatchPlay {
                            path: track.path.clone(),
                        }];
                    }
                }
            }
            return vec![];
        }

        // 3. User stops playback
        AppMsg::RequestStop => {
            state.playback.status = PlaybackStatus::Stopped;
            state.playback.progress_ms = 0;
            state.notification.current = Some(Notification {
                kind: NotificationKind::Info,
                message: "Stopped".into(),
                progress: None,
                created_at: Instant::now(),
                dismiss_after: Some(Duration::from_secs(1)),
            });
            return vec![AppMsg::DispatchStop];
        }

        // 4. User seeks
        AppMsg::RequestSeek { offset_seconds } => {
            let current = state.playback.progress_ms as i64;
            // Default to 0 if we don't know duration yet, or huge number if we do
            let total = state.playback.duration_ms.unwrap_or(u64::MAX) as i64;
            let offset_ms = offset_seconds * 1000;

            // Clamp logic
            let new_pos = (current + offset_ms).max(0).min(total) as u64;

            // Optimistic Update: Jump the progress bar immediately
            state.playback.progress_ms = new_pos;

            return vec![AppMsg::DispatchSeek {
                position_ms: new_pos,
            }];
        }

        // ───── Side-Effects (Ignored by UI) ─────
        // These exist ONLY for the Supervisor to catch and route to IPC.
        AppMsg::DispatchPlay { .. } => {}
        AppMsg::DispatchPause => {}
        AppMsg::DispatchResume => {}
        AppMsg::DispatchStop => {}
        AppMsg::DispatchSeek { .. } => {}

        // ───── External Facts (Truth from Worker) ─────
        // We trust the worker for timing and file duration
        AppMsg::AudioStarted {
            path: _,
            duration_ms,
        } => {
            // Worker finally parsed the file header and knows the real duration
            state.playback.duration_ms = duration_ms;
        }

        AppMsg::AudioProgress { position_ms } => {
            // Sync up. Even if optimistic UI set it, we correct it here.
            // We only update if status is Playing to avoid jitter if user just hit Pause.
            if state.playback.status == PlaybackStatus::Playing {
                state.playback.progress_ms = position_ms;
            }
        }

        AppMsg::AudioFinished => {
            // Natural finish (not user requested)
            state.playback.status = PlaybackStatus::Stopped;
            state.playback.progress_ms = 0;
            // TODO: Trigger "Next Track" logic here if you want auto-play
        }

        AppMsg::AudioPaused | AppMsg::AudioResumed => {
            // No-op: We already updated the UI optimistically in RequestTogglePlayback.
            // We ignore these to prevent the UI from "flickering" back and forth.
        }

        AppMsg::AudioError { error } => {
            state.playback.status = PlaybackStatus::Stopped;
            state.notification.current = Some(Notification {
                kind: NotificationKind::Error,
                message: format!("Audio Error: {}", error),
                progress: None,
                created_at: Instant::now(),
                dismiss_after: Some(Duration::from_secs(4)),
            });
        }
        AppMsg::Noop => {}
    }
    vec![]
}
