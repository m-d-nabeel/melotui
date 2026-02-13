use std::{path::PathBuf, sync::mpsc, thread};

use melotui_core::{
    AppMsg, AppState,
    message::{
        cover::CoverMsg, database::DatabaseMsg, download::DownloadMsg, library::LibraryMsg,
        playback::PlaybackMsg,
    },
};
use melotui_ipc::{
    channel::inprocess,
    command::{AudioCommand, CoverCommand, DatabaseCommand, DownloadCommand, IpcCommand},
    event::IpcEvent,
    spectrum_bridge::SpectrumProducer,
};

pub struct IpcRuntime {
    // events from workers
    pub event_rx: mpsc::Receiver<IpcEvent>,

    // typed routers
    download_tx: mpsc::Sender<DownloadCommand>,
    audio_tx: mpsc::Sender<AudioCommand>,
    db_tx: mpsc::Sender<DatabaseCommand>,
    cover_tx: mpsc::Sender<CoverCommand>,
}

impl IpcRuntime {
    pub fn new(spectrum_producer: SpectrumProducer, db_path: PathBuf) -> Self {
        let (ui_rx, bus_tx) = inprocess::hub();

        // spawn download worker
        let (dl_tx, dl_worker) = inprocess::join_worker::<DownloadCommand>(bus_tx.clone());
        thread::spawn(move || {
            melotui_download::run(dl_worker);
            tracing::info!("IPC Runtime initialized with Download worker");
        });

        // spawn audio worker
        let (audio_tx, audio_worker) = inprocess::join_worker::<AudioCommand>(bus_tx.clone());
        thread::spawn(move || {
            melotui_audio::run(audio_worker, spectrum_producer);
            tracing::info!("IPC Runtime initialized with Audio worker");
        });

        // spawn database worker
        let (db_tx, db_worker) = inprocess::join_worker::<DatabaseCommand>(bus_tx.clone());
        thread::spawn(move || {
            melotui_db::run(db_worker, db_path);
            tracing::info!("IPC Runtime initialized with Database worker");
        });

        // spawn cover worker
        let (cover_tx, cover_worker) = inprocess::join_worker::<CoverCommand>(bus_tx.clone());
        thread::spawn(move || {
            melotui_cover::run(cover_worker);
            tracing::info!("IPC Runtime initialized with Cover worker");
        });

        Self {
            event_rx: ui_rx,
            download_tx: dl_tx,
            audio_tx,
            db_tx,
            cover_tx,
        }
    }

    pub fn dispatch(&self, cmd: IpcCommand) {
        match cmd {
            IpcCommand::Download(c) => {
                let _ = self.download_tx.send(c);
            }
            IpcCommand::Audio(c) => {
                let _ = self.audio_tx.send(c);
            }
            IpcCommand::Database(c) => {
                tracing::info!("Dispatching database command: {:?}", c);
                let _ = self.db_tx.send(c);
            }
            IpcCommand::Cover(c) => {
                let _ = self.cover_tx.send(c);
            }
            IpcCommand::Shutdown => {
                tracing::info!("IPC Runtime shutting down all workers...");
                let _ = self.download_tx.send(DownloadCommand::Shutdown);
                let _ = self.audio_tx.send(AudioCommand::Shutdown);
                let _ = self.db_tx.send(DatabaseCommand::Shutdown);
                let _ = self.cover_tx.send(CoverCommand::Shutdown);
            }
        }
    }
}

pub fn appmsg_to_ipc(msg: &AppMsg, state: &AppState) -> Option<IpcCommand> {
    use melotui_core::message::{audio::AudioMsg, download::DownloadMsg, playback::PlaybackMsg};

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
                root: state.library.root.clone(),
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

pub fn ipc_to_appmsg(evt: IpcEvent) -> AppMsg {
    match evt {
        IpcEvent::CoverLoaded { id, cover, context } => {
            AppMsg::Cover(CoverMsg::Loaded { id, cover, context })
        }
        IpcEvent::DownloadProgress { percent } => {
            AppMsg::Download(DownloadMsg::DownloadProgress { percent })
        }
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
