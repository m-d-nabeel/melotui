use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
    vec,
};

use crate::{
    AppMsg, AppState, Command, Library, NavMode, UIMode,
    state::{
        DownloadControl, DownloadJob, DownloadStatus, Notification, NotificationKind,
        PlaybackStatus, TrackId,
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
        AppMsg::Quit => {
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

        AppMsg::PlaySelected => {
            if state.nav.mode != NavMode::Tracks {
                tracing::warn!("PlaySelected called but not in Track Mode.");
                return vec![];
            }

            let p = state.nav.selected_playlist;

            let playlist = match state.library.playlists.get(p) {
                Some(pl) => pl,
                None => {
                    tracing::warn!("No playlist found at index {}.", p);
                    return vec![];
                }
            };

            let t = state.nav.selected_tracks[p];

            let track_path = match playlist.tracks.get(t) {
                Some(path) => path.clone(),
                None => return vec![],
            };

            state.playback.current = Some(TrackId {
                playlist: playlist.name.clone(),
                path: track_path.clone(),
            });

            state.playback.status = PlaybackStatus::Playing;

            state.notification.current = Some(Notification {
                kind: NotificationKind::Info,
                message: format!(
                    "Playing {}",
                    track_path.file_name().unwrap_or_default().to_string_lossy()
                ),
                progress: None,
                created_at: Instant::now(),
                dismiss_after: Some(Duration::from_secs(2)),
            });
        }

        AppMsg::RescanLibrary => {
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
                    return vec![AppMsg::Quit];
                }

                Command::Rescan => {
                    return vec![AppMsg::RescanLibrary];
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

                    return vec![AppMsg::StartDownload { query }];
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

        AppMsg::StartDownload { query } => {
            tracing::info!("Queueing download: {}", query);
            state.download.queue.push_back(DownloadJob { query });
            // If nothing is running, start immediately
            if matches!(state.download.status, DownloadStatus::Idle) {
                return vec![AppMsg::BeginNextDownload];
            }
            return vec![];
        }

        AppMsg::BeginNextDownload => {
            let job = match state.download.queue.pop_front() {
                Some(job) => job,
                None => return vec![], // nothing to do
            };
            let control = DownloadControl {
                cancel: Arc::new(AtomicBool::new(false)),
            };
            tracing::info!("Starting download: {}", job.query);
            state.download.status = DownloadStatus::Running {
                percent: 0,
                control,
            };
            // Trigger side effect ONCE
            return vec![AppMsg::SpawnDownload { query: job.query }];
        }

        AppMsg::SpawnDownload { .. } => return vec![],

        AppMsg::DownloadProgress { message, percent } => {
            state.notification.current = Some(Notification {
                kind: NotificationKind::Progress,
                message,
                progress: Some(percent),
                created_at: Instant::now(),
                dismiss_after: None,
            });
            if let DownloadStatus::Running { control, .. } = &state.download.status {
                state.download.status = DownloadStatus::Running {
                    percent,
                    control: control.clone(),
                };
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
                AppMsg::RescanLibrary,
                AppMsg::BeginNextDownload, // ← THIS is the queue
            ];
        }

        AppMsg::DownloadFailed { error } => {
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

        AppMsg::CancelDownload => {
            state.notification.current = Some(Notification {
                kind: NotificationKind::Error,
                message: "Cancelling Download".into(),
                progress: None,
                created_at: Instant::now(),
                dismiss_after: Some(Duration::from_secs(4)),
            });

            if let DownloadStatus::Running { control, .. } = &state.download.status {
                control.cancel.store(true, Ordering::SeqCst);
                state.download.status = DownloadStatus::Cancelling;
            }

            return vec![];
        }
    }
    vec![]
}
