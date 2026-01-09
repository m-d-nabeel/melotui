use std::{
    time::{Duration, Instant},
    vec,
};

use crate::{
    AppMsg, AppState, Library, UIMode,
    action_handlers::{
        handler_enqueue_download, handler_play_current_selection, handler_playback_ended,
        handler_process_command_input,
    },
    art_converter::{SizeMode, bytes_to_ascii},
    reconcile_nav_state,
    state::{
        DownloadStatus, LeftPanelMode, LibraryViewDepth, Notification, NotificationKind,
        PlaybackContext, PlaybackStatus, RepeatMode, VisualizerMode,
    },
};

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

        AppMsg::ToggleLibraryQueue => {
            tracing::info!("Toggle Library Queue");
            state.nav.active_panel = match state.nav.active_panel {
                LeftPanelMode::Library => {
                    state.nav.active_panel = LeftPanelMode::Queue;
                    // FIX: When switching TO Queue, default to Index 1 if playing
                    let playing_from_queue = state
                        .playback
                        .current
                        .as_ref()
                        .is_some_and(|c| c.context == PlaybackContext::Queue);

                    if playing_from_queue && state.playback_queue.tracks.len() > 1 {
                        state.nav.queue.selected_idx = 1;
                    }
                    LeftPanelMode::Queue
                }
                LeftPanelMode::Queue => LeftPanelMode::Library,
            };
        }

        AppMsg::SwitchToPlaylists => {
            tracing::debug!("Switch to Playlist Mode");
            state.last_nav_time = state.time_elapsed;
            state.nav.active_panel = LeftPanelMode::Library;
            state.nav.library.depth = LibraryViewDepth::Playlists;
        }

        AppMsg::SwitchToTracks => {
            tracing::debug!("Switch to Tracks");
            state.last_nav_time = state.time_elapsed;
            state.nav.active_panel = LeftPanelMode::Library;
            state.nav.library.depth = LibraryViewDepth::Tracks;
        }

        AppMsg::SwitchToTrackInfo => {
            tracing::debug!("Switch to Track Info");
            state.last_nav_time = state.time_elapsed;
            state.nav.active_panel = LeftPanelMode::Library;
            state.nav.library.depth = LibraryViewDepth::TrackInfo;

            state.track_info_art = None;
            let p_idx = state.nav.library.selected_playlist_idx;
            if let Some(playlist) = state.library.playlists.get(p_idx) {
                let t_idx = state.nav.library.selected_tracks_curs[p_idx];
                if let Some(track) = playlist.tracks.get(t_idx)
                    && track.has_cover
                    && let Some((data, _)) = Library::get_album_art(&track.file_path)
                    && let Ok(ascii) = bytes_to_ascii(
                        &data,
                        SizeMode::Cover {
                            width: 32,
                            height: 15,
                        },
                    )
                {
                    state.track_info_art = Some(ascii);
                }
            }
        }

        AppMsg::MoveUp => {
            state.last_nav_time = state.time_elapsed;
            match state.nav.active_panel {
                LeftPanelMode::Library => match state.nav.library.depth {
                    LibraryViewDepth::Playlists => {
                        if state.nav.library.selected_playlist_idx > 0 {
                            state.nav.library.selected_playlist_idx -= 1;
                        }
                    }
                    LibraryViewDepth::Tracks => {
                        let p = state.nav.library.selected_playlist_idx;
                        if p < state.nav.library.selected_tracks_curs.len()
                            && state.nav.library.selected_tracks_curs[p] > 0
                        {
                            state.nav.library.selected_tracks_curs[p] -= 1;
                        }
                    }
                    LibraryViewDepth::TrackInfo => {}
                },
                LeftPanelMode::Queue => {
                    let playing_from_queue = state
                        .playback
                        .current
                        .as_ref()
                        .is_some_and(|c| c.context == PlaybackContext::Queue);

                    let min_index = if playing_from_queue && state.playback_queue.tracks.len() > 1 {
                        1
                    } else {
                        0
                    };

                    if state.nav.queue.selected_idx > min_index {
                        state.nav.queue.selected_idx -= 1;
                    }
                }
            }
        }

        AppMsg::MoveDown => {
            state.last_nav_time = state.time_elapsed;
            match state.nav.active_panel {
                LeftPanelMode::Library => match state.nav.library.depth {
                    LibraryViewDepth::Playlists => {
                        if state.nav.library.selected_playlist_idx + 1
                            < state.library.playlists.len()
                        {
                            state.nav.library.selected_playlist_idx += 1;
                        }
                    }
                    LibraryViewDepth::Tracks => {
                        let p = state.nav.library.selected_playlist_idx;
                        if let Some(playlist) = state.library.playlists.get(p)
                            && state.nav.library.selected_tracks_curs[p] + 1 < playlist.tracks.len()
                        {
                            state.nav.library.selected_tracks_curs[p] += 1;
                        }
                    }
                    LibraryViewDepth::TrackInfo => {}
                },
                LeftPanelMode::Queue => {
                    let max_len = state.playback_queue.tracks.len();
                    if max_len > 0 && state.nav.queue.selected_idx + 1 < max_len {
                        state.nav.queue.selected_idx += 1;
                    }
                }
            }
        }

        AppMsg::AddToQueue(track) => {
            state.playback_queue.tracks.push_back(track);
            state.notification.current = Some(Notification {
                kind: NotificationKind::Info,
                message: "Added to Queue".into(),
                progress: None,
                created_at: Instant::now(),
                dismiss_after: Some(Duration::from_secs(1)),
            });
        }

        AppMsg::RemoveFromQueue => {
            if state.nav.active_panel == LeftPanelMode::Queue {
                let idx = state.nav.queue.selected_idx;

                let playing_from_queue = state
                    .playback
                    .current
                    .as_ref()
                    .is_some_and(|c| c.context == PlaybackContext::Queue);

                if playing_from_queue && idx == 0 {
                    return vec![];
                }

                if idx < state.playback_queue.tracks.len() {
                    state.playback_queue.tracks.remove(idx);
                    // reconcile_nav_state(state);

                    // Adjust selection after removal
                    if idx >= state.playback_queue.tracks.len() && idx > 0 {
                        state.nav.queue.selected_idx = idx - 1;
                    }

                    // FIX: Ensure selection doesn't fall back to 0 if playing
                    if playing_from_queue
                        && state.nav.queue.selected_idx == 0
                        && state.playback_queue.tracks.len() > 1
                    {
                        state.nav.queue.selected_idx = 1;
                    }
                    state.notification.current = Some(Notification {
                        kind: NotificationKind::Info,
                        message: "Removed from Queue".into(),
                        progress: None,
                        created_at: Instant::now(),
                        dismiss_after: Some(Duration::from_secs(1)),
                    });
                }
            }
        }

        AppMsg::ClearQueue => {
            let playing_from_queue = state
                .playback
                .current
                .as_ref()
                .is_some_and(|c| c.context == PlaybackContext::Queue);

            if playing_from_queue {
                state.playback_queue.tracks.truncate(1);
            } else {
                state.playback_queue.tracks.clear();
            }

            reconcile_nav_state(state);
            state.notification.current = Some(Notification {
                kind: NotificationKind::Info,
                message: "Queue Cleared".into(),
                progress: None,
                created_at: Instant::now(),
                dismiss_after: Some(Duration::from_secs(1)),
            });
        }

        AppMsg::ToggleShuffle => {
            state.playback.shuffle = !state.playback.shuffle;
            let status = if state.playback.shuffle { "On" } else { "Off" };
            state.notification.current = Some(Notification {
                kind: NotificationKind::Info,
                message: format!("Shuffle: {}", status),
                progress: None,
                created_at: Instant::now(),
                dismiss_after: Some(Duration::from_secs(1)),
            });
        }

        AppMsg::ToggleRepeatMode => {
            state.repeat_mode = state.repeat_mode.next();

            let mode_str = match state.repeat_mode {
                RepeatMode::Off => "Linear (Default)",
                RepeatMode::PlayOnce => "Play Once",
                RepeatMode::Track => "Repeat Track",
                RepeatMode::Playlist => "Repeat List",
            };

            state.notification.current = Some(Notification {
                kind: NotificationKind::Info,
                message: format!("Mode: {}", mode_str),
                progress: None,
                created_at: Instant::now(),
                dismiss_after: Some(Duration::from_secs(1)),
            });

            return vec![];
        }

        AppMsg::RequestPlaySelected => {
            tracing::info!("Play Requested");
            return handler_play_current_selection(state);
        }

        AppMsg::RequestRescanLibrary => match Library::scan(state.library.root.clone()) {
            Ok(new_library) => {
                state.library = new_library;
                reconcile_nav_state(state);
                tracing::info!("Library rescanned");

                state.notification.current = Some(Notification {
                    kind: NotificationKind::Info,
                    message: "Library rescanned".into(),
                    progress: None,
                    created_at: Instant::now(),
                    dismiss_after: Some(Duration::from_secs(2)),
                });
            }
            Err(err) => {
                tracing::error!("Error rescanning library: {:?}", err);
                state.notification.current = Some(Notification {
                    kind: NotificationKind::Error,
                    message: format!("Rescan failed: {err}"),
                    progress: None,
                    created_at: Instant::now(),
                    dismiss_after: Some(Duration::from_secs(2)),
                });
            }
        },

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
            tracing::info!("SubmitCommand called with :{:?}", input);
            return handler_process_command_input(state, input);
        }

        AppMsg::RequestDownload { query } => {
            tracing::info!("RequestDownload called with: {:?}", query);
            return handler_enqueue_download(state, query);
        }

        AppMsg::BeginNextDownload => {
            let job = match state.download.queue.pop_front() {
                Some(job) => job,
                None => return vec![],
            };
            tracing::info!("Next Download Started with: {:?}", job.query);
            state.download.status = DownloadStatus::Running { percent: 0 };
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
            return vec![AppMsg::RequestRescanLibrary, AppMsg::BeginNextDownload];
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

        AppMsg::RequestCancelDownload => {
            if let DownloadStatus::Running { .. } = &state.download.status {
                state.download.status = DownloadStatus::Cancelling;
            }
            return vec![AppMsg::RequestCancelDownload];
        }

        AppMsg::DispatchDownload { .. } => {}

        AppMsg::AdjustVolume(delta) => {
            state.audio.volume = (state.audio.volume + delta).clamp(0.0, 1.5);
            return vec![AppMsg::DispatchSetVolume(state.audio.volume)];
        }
        AppMsg::AdjustSpeed(delta) => {
            state.audio.speed = (state.audio.speed + delta).clamp(0.5, 2.0);
            return vec![AppMsg::DispatchSetSpeed(state.audio.speed)];
        }
        AppMsg::AdjustBass(delta) => {
            state.audio.bass = (state.audio.bass + delta).clamp(0.1, 5.0);
            return vec![AppMsg::DispatchSetBass(state.audio.bass)];
        }
        AppMsg::AdjustTreble(delta) => {
            state.audio.treble = (state.audio.treble + delta).clamp(0.0, 5.0);
            return vec![AppMsg::DispatchSetTreble(state.audio.treble)];
        }
        AppMsg::AdjustReverb(delta) => {
            state.audio.reverb = (state.audio.reverb + delta).clamp(0.0, 1.0);
            return vec![AppMsg::DispatchSetReverb(state.audio.reverb)];
        }
        AppMsg::AdjustEcho(delta) => {
            state.audio.echo = (state.audio.echo + delta).clamp(0.0, 0.9);
            return vec![AppMsg::DispatchSetEcho(state.audio.echo)];
        }

        AppMsg::DispatchSetVolume(_) => {}
        AppMsg::DispatchSetSpeed(_) => {}
        AppMsg::DispatchSetBass(_) => {}
        AppMsg::DispatchSetTreble(_) => {}
        AppMsg::DispatchSetReverb(_) => {}
        AppMsg::DispatchSetEcho(_) => {}

        AppMsg::RequestTogglePlayback => match state.playback.status {
            PlaybackStatus::Playing => {
                state.playback.status = PlaybackStatus::Paused;
                return vec![AppMsg::DispatchPause];
            }
            PlaybackStatus::Paused => {
                if state.playback.current.is_some() {
                    state.playback.status = PlaybackStatus::Playing;
                    return vec![AppMsg::DispatchResume];
                }
            }
            PlaybackStatus::Stopped => {
                if let Some(track_id) = &state.playback.current {
                    state.playback.status = PlaybackStatus::Playing;
                    return vec![AppMsg::DispatchPlay {
                        path: track_id.path.clone(),
                    }];
                }
            }
        },

        AppMsg::RequestStop => {
            state.playback.status = PlaybackStatus::Stopped;
            state.playback.status = PlaybackStatus::Stopped;
            state.playback.progress_ms = 0;
            return vec![AppMsg::DispatchStop];
        }

        AppMsg::RequestSeek { .. } => {
            // Placeholder for seek logic
            tracing::error!("Seeking not implemented yet");
        }
        AppMsg::DispatchPlay { .. } => {}
        AppMsg::DispatchPause => {}
        AppMsg::DispatchResume => {}
        AppMsg::DispatchStop => {}
        AppMsg::DispatchSeek { .. } => {}
        AppMsg::AudioStarted { .. } => {}

        AppMsg::AudioProgress { position_ms } => {
            if state.playback.status == PlaybackStatus::Playing {
                state.playback.progress_ms = position_ms;
            }
        }

        AppMsg::AudioFinished => {
            tracing::info!("Audio Finished...");
            return handler_playback_ended(state);
        }

        AppMsg::AudioPaused | AppMsg::AudioResumed => {}
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
        AppMsg::ToggleArt => {
            state.visualizer_mode = match state.visualizer_mode {
                VisualizerMode::Spectrum => VisualizerMode::AlbumArt,
                VisualizerMode::AlbumArt => VisualizerMode::Spectrum,
            };
            let is_spectrum = state.visualizer_mode == VisualizerMode::Spectrum;
            return vec![AppMsg::DispatchSetVisualizer {
                active: is_spectrum,
            }];
        }
        AppMsg::ToggleHelp => {
            state.ui.show_help = !state.ui.show_help;
        }
        AppMsg::DispatchSetVisualizer { .. } => {}
        AppMsg::Noop => {}
        AppMsg::PlayNext(track) => {
            state.playback_queue.tracks.push_front(track.clone());

            state.notification.current = Some(Notification {
                kind: NotificationKind::Info,
                message: format!("Set to play next: {}", track.title),
                progress: None,
                created_at: Instant::now(),
                dismiss_after: Some(Duration::from_secs(2)),
            });
        }
    }
    vec![]
}
