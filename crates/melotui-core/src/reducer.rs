use crate::{
    AppMsg, AppState, Command, Library, NavMode, UIMode,
    state::{DownloadStatus, PlaybackStatus, TrackId},
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

pub fn reduce(state: &mut AppState, msg: AppMsg) {
    match msg {
        AppMsg::Quit => {}

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
                return;
            }

            let p = state.nav.selected_playlist;

            let playlist = match state.library.playlists.get(p) {
                Some(pl) => pl,
                None => {
                    tracing::warn!("No playlist found at index {}.", p);
                    return;
                }
            };

            let t = state.nav.selected_tracks[p];

            let track_path = match playlist.tracks.get(t) {
                Some(path) => path.clone(),
                None => return,
            };

            state.playback.current = Some(TrackId {
                playlist: playlist.name.clone(),
                path: track_path.clone(),
            });

            state.playback.status = PlaybackStatus::Playing;

            state.status = format!(
                "Playing {}",
                track_path.file_name().unwrap_or_default().to_string_lossy()
            );
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
                            state.status = "Playback stopped (file removed)".into();
                        }
                    }

                    state.status = "Library rescanned".into();
                }
                Err(err) => {
                    state.status = format!("Rescan failed: {err}");
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

            if input.trim().is_empty() {
                return;
            }

            reduce(state, AppMsg::ExecuteCommand(input));
        }

        AppMsg::ExecuteCommand(input) => {
            match Command::parse(&input) {
                Command::Quit => {
                    // handled in main loop
                    state.status = "Quit".into();
                }

                Command::Rescan => {
                    reduce(state, AppMsg::RescanLibrary);
                }

                Command::Download { query } => {
                    state.status = format!("Downloading: {}", query);
                    reduce(state, AppMsg::StartDownload { query });
                }

                Command::Unknown(reason) => {
                    state.status = format!("Unknown command: {}", reason);
                }
            }
        }

        AppMsg::StartDownload { query } => {
            state.download.status = DownloadStatus::Running {
                info: format!("Starting download: {}", query),
            };
            tracing::info!("StartDownload called with {}", query);
        }

        AppMsg::DownloadProgress { message } => {
            state.download.status = DownloadStatus::Running { info: message };
        }

        AppMsg::DownloadFinished => {
            state.download.status = DownloadStatus::Idle;
            reduce(state, AppMsg::RescanLibrary);
            tracing::info!("DownloadFinished called");
        }

        AppMsg::DownloadFailed { error } => {
            state.download.status = DownloadStatus::Failed { error };
        }
    }
}
