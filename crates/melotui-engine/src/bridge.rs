//! Message translation between AppMsg (core domain) and IpcCommand/IpcEvent (worker protocol).
//!
//! These functions are pure translations — no state mutations, no side effects.
//! The `appmsg_to_ipc` function takes a `&Path` for the library root instead of
//! `&AppState`, keeping this layer decoupled from UI state.

use std::path::Path;

use melotui_bus::{
    command::{AudioCommand, CoverCommand, DatabaseCommand, DownloadCommand, IpcCommand},
    event::IpcEvent,
};
use melotui_core::{
    AppMsg,
    message::{
        audio::AudioMsg, cover::CoverMsg, database::DatabaseMsg, download::DownloadMsg,
        library::LibraryMsg, playback::PlaybackMsg,
    },
};

/// Translate an AppMsg into an IpcCommand to dispatch to a worker.
///
/// Returns `None` if the message doesn't map to any worker command.
/// `library_root` is needed for `RequestScanLibrary` — the caller passes
/// `state.library.root` without leaking `&AppState` into this layer.
pub fn appmsg_to_ipc(msg: &AppMsg, library_root: &Path) -> Option<IpcCommand> {
    match msg {
        // --- Cover ---
        AppMsg::Cover(CoverMsg::RequestLoad { id, path, context }) => {
            Some(IpcCommand::Cover(CoverCommand::Load {
                id: *id,
                path: path.clone(),
                context: *context,
            }))
        }

        // --- Download ---
        AppMsg::Download(download_msg) => match download_msg {
            DownloadMsg::DispatchDownload {
                query,
                download_path,
            } => Some(IpcCommand::Download(DownloadCommand::Request {
                query: query.clone(),
                download_path: download_path.clone(),
            })),
            DownloadMsg::RequestCancelDownload => {
                Some(IpcCommand::Download(DownloadCommand::Cancel))
            }
            _ => None,
        },

        // --- Audio ---
        AppMsg::Playback(playback_msg) => match playback_msg {
            PlaybackMsg::DispatchPlay { path } => {
                Some(IpcCommand::Audio(AudioCommand::Play { path: path.clone() }))
            }
            PlaybackMsg::DispatchPause => Some(IpcCommand::Audio(AudioCommand::Pause)),
            PlaybackMsg::DispatchResume => Some(IpcCommand::Audio(AudioCommand::Resume)),
            PlaybackMsg::DispatchStop => Some(IpcCommand::Audio(AudioCommand::Stop)),
            PlaybackMsg::DispatchSeek { position_ms, epoch } => {
                Some(IpcCommand::Audio(AudioCommand::Seek {
                    position_ms: *position_ms,
                    epoch: *epoch,
                }))
            }
            PlaybackMsg::DispatchSetVisualizer { spectrum_style } => {
                Some(IpcCommand::Audio(AudioCommand::SetVisualizer {
                    spectrum_style: spectrum_style.clone(),
                }))
            }
            _ => None,
        },

        // --- Audio Controls ---
        AppMsg::Audio(audio_msg) => match audio_msg {
            AudioMsg::DispatchSetVolume(v) => Some(IpcCommand::Audio(AudioCommand::SetVolume(*v))),
            AudioMsg::DispatchSetSpeed(v) => Some(IpcCommand::Audio(AudioCommand::SetSpeed(*v))),
            AudioMsg::DispatchSetBass(v) => Some(IpcCommand::Audio(AudioCommand::SetBass(*v))),
            AudioMsg::DispatchSetTreble(v) => Some(IpcCommand::Audio(AudioCommand::SetTreble(*v))),
            AudioMsg::DispatchSetReverb(v) => Some(IpcCommand::Audio(AudioCommand::SetReverb(*v))),
            AudioMsg::DispatchSetEcho(v) => Some(IpcCommand::Audio(AudioCommand::SetEcho(*v))),
            _ => None,
        },

        // --- Database Controls ---
        AppMsg::Library(LibraryMsg::RequestScanLibrary) => {
            Some(IpcCommand::Database(DatabaseCommand::ScanLibrary {
                root: library_root.to_path_buf(),
            }))
        }
        AppMsg::Database(DatabaseMsg::DispatchToggleFavorite(track_id)) => {
            Some(IpcCommand::Database(DatabaseCommand::ToggleFavorite {
                track_id: track_id.0 as i64,
            }))
        }
        AppMsg::Database(DatabaseMsg::DispatchCreatePlaylist(name)) => {
            Some(IpcCommand::Database(DatabaseCommand::CreatePlaylist {
                name: name.clone(),
            }))
        }
        AppMsg::Database(DatabaseMsg::DispatchDeletePlaylist(pid)) => {
            Some(IpcCommand::Database(DatabaseCommand::DeletePlaylist {
                playlist_id: pid.0 as i64,
            }))
        }
        AppMsg::Database(DatabaseMsg::DispatchAddToPlaylist {
            playlist_id,
            track_id,
        }) => Some(IpcCommand::Database(DatabaseCommand::AddTrackToPlaylist {
            playlist_id: playlist_id.0 as i64,
            track_id: track_id.0 as i64,
        })),
        AppMsg::Database(DatabaseMsg::DispatchRemoveFromPlaylist {
            playlist_id,
            track_id,
        }) => Some(IpcCommand::Database(
            DatabaseCommand::RemoveTrackFromPlaylist {
                playlist_id: playlist_id.0 as i64,
                track_id: track_id.0 as i64,
            },
        )),

        _ => None,
    }
}

/// Translate an IpcEvent from a worker into an AppMsg for the reducer.
pub fn ipc_to_appmsg(evt: IpcEvent) -> AppMsg {
    match evt {
        IpcEvent::CoverLoaded { id, cover, context } => {
            AppMsg::Cover(CoverMsg::Loaded { id, cover, context })
        }
        IpcEvent::DownloadProgress { percent } => {
            AppMsg::Download(DownloadMsg::DownloadProgress { percent })
        }
        IpcEvent::DownloadItemFinished => AppMsg::Library(LibraryMsg::RequestScanLibrary),
        IpcEvent::DownloadFinished => AppMsg::Download(DownloadMsg::DownloadFinished),
        IpcEvent::DownloadFailed { error } => {
            AppMsg::Download(DownloadMsg::DownloadFailed { error })
        }
        IpcEvent::PlaybackStarted { path } => AppMsg::Playback(PlaybackMsg::AudioStarted { path }),
        IpcEvent::PlaybackProgress { position_ms, epoch } => {
            AppMsg::Playback(PlaybackMsg::AudioProgress { position_ms, epoch })
        }
        IpcEvent::PlaybackFinished => AppMsg::Playback(PlaybackMsg::AudioFinished),
        IpcEvent::PlaybackPaused => AppMsg::Playback(PlaybackMsg::AudioPaused),
        IpcEvent::PlaybackResumed => AppMsg::Playback(PlaybackMsg::AudioResumed),
        IpcEvent::FavoriteToggled {
            track_id,
            is_favorite,
        } => {
            tracing::info!("Favorite toggled for track {:?}: {}", track_id, is_favorite);
            AppMsg::Noop
        }
        // playlist events - state updated optimistically
        IpcEvent::PlaylistCreated { id, name } => {
            tracing::info!("Playlist created: {:?} '{}'", id, name);
            AppMsg::Library(LibraryMsg::PlaylistCreated { id, name })
        }
        IpcEvent::PlaylistDeleted { id } => {
            tracing::info!("Playlist deleted: {:?}", id);
            AppMsg::Noop
        }
        IpcEvent::PlaylistTrackAdded {
            playlist_id,
            track_id,
        } => {
            tracing::info!("Track {:?} added to playlist {:?}", track_id, playlist_id);
            AppMsg::Noop
        }
        IpcEvent::PlaylistTrackRemoved {
            playlist_id,
            track_id,
        } => {
            tracing::info!(
                "Track {:?} removed from playlist {:?}",
                track_id,
                playlist_id
            );
            AppMsg::Noop
        }
        _ => AppMsg::Noop,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use melotui_core::domain::TrackId;
    use std::path::PathBuf;

    #[test]
    fn test_ipc_to_appmsg_translation() {
        // Test playback progress translation
        let evt = IpcEvent::PlaybackProgress {
            position_ms: 1000,
            epoch: 1,
        };
        let msg = ipc_to_appmsg(evt);
        match msg {
            AppMsg::Playback(PlaybackMsg::AudioProgress { position_ms, epoch }) => {
                assert_eq!(position_ms, 1000);
                assert_eq!(epoch, 1);
            }
            _ => panic!("Expected AudioProgress msg"),
        }

        // Test download progress
        let evt = IpcEvent::DownloadProgress { percent: 45 };
        let msg = ipc_to_appmsg(evt);
        match msg {
            AppMsg::Download(DownloadMsg::DownloadProgress { percent }) => {
                assert_eq!(percent, 45);
            }
            _ => panic!("Expected DownloadProgress msg"),
        }
    }

    #[test]
    fn test_appmsg_to_ipc_translation() {
        let root = PathBuf::from("/music");

        // Test UI Playback Stop command
        let msg = AppMsg::Playback(PlaybackMsg::DispatchStop);
        let cmd = appmsg_to_ipc(&msg, &root).unwrap();
        match cmd {
            IpcCommand::Audio(AudioCommand::Stop) => {}
            _ => panic!("Expected Audio Stop command"),
        }

        // Test Database toggle favorite command
        let msg = AppMsg::Database(
            melotui_core::message::database::DatabaseMsg::DispatchToggleFavorite(TrackId(42)),
        );
        let cmd = appmsg_to_ipc(&msg, &root).unwrap();
        match cmd {
            IpcCommand::Database(DatabaseCommand::ToggleFavorite { track_id }) => {
                assert_eq!(track_id, 42);
            }
            _ => panic!("Expected Database ToggleFavorite command"),
        }
    }

    #[test]
    fn test_scan_library_uses_provided_root() {
        let root = PathBuf::from("/my/music/library");
        let msg = AppMsg::Library(LibraryMsg::RequestScanLibrary);
        let cmd = appmsg_to_ipc(&msg, &root).unwrap();
        match cmd {
            IpcCommand::Database(DatabaseCommand::ScanLibrary { root: scan_root }) => {
                assert_eq!(scan_root, PathBuf::from("/my/music/library"));
            }
            _ => panic!("Expected Database ScanLibrary command"),
        }
    }
}
