use std::{path::PathBuf, sync::mpsc, thread};

use melotui_core::{AppMsg, AppState, message::library::LibraryMsg};
use melotui_ipc::{
    channel::inprocess,
    command::{AudioCommand, DatabaseCommand, DownloadCommand, IpcCommand},
    event::IpcEvent,
    spectrum_bridge::SpectrumProducer,
};

pub struct IpcRuntime {
    // Egress: Events coming FROM workers
    pub event_rx: mpsc::Receiver<IpcEvent>,

    // Ingress: Typed routers
    download_tx: mpsc::Sender<DownloadCommand>,
    audio_tx: mpsc::Sender<AudioCommand>,
    db_tx: mpsc::Sender<DatabaseCommand>,
}

impl IpcRuntime {
    pub fn new(spectrum_producer: SpectrumProducer, db_path: PathBuf) -> Self {
        let (ui_rx, bus_tx) = inprocess::hub();

        // Spawn Download Worker
        let (dl_tx, dl_worker) = inprocess::join_worker::<DownloadCommand>(bus_tx.clone());
        thread::spawn(move || {
            melotui_download::run(dl_worker);
            tracing::info!("IPC Runtime initialized with Download worker");
        });

        // Spawn Audio Worker
        let (audio_tx, audio_worker) = inprocess::join_worker::<AudioCommand>(bus_tx.clone());
        thread::spawn(move || {
            melotui_audio::run(audio_worker, spectrum_producer);
            tracing::info!("IPC Runtime initialized with Audio worker");
        });

        // Spawn Audio Worker
        let (db_tx, db_worker) = inprocess::join_worker::<DatabaseCommand>(bus_tx.clone());
        thread::spawn(move || {
            melotui_db::run(db_worker, db_path);
            tracing::info!("IPC Runtime initialized with Database worker");
        });

        Self {
            event_rx: ui_rx,
            download_tx: dl_tx,
            audio_tx,
            db_tx,
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
            IpcCommand::Shutdown => {
                tracing::info!("IPC Runtime shutting down all workers...");
                let _ = self.download_tx.send(DownloadCommand::Shutdown);
                let _ = self.audio_tx.send(AudioCommand::Shutdown);
                let _ = self.db_tx.send(DatabaseCommand::Shutdown);
            }
        }
    }
}

pub fn appmsg_to_ipc(msg: &AppMsg, state: &AppState) -> Option<IpcCommand> {
    use melotui_core::message::{audio::AudioMsg, download::DownloadMsg, playback::PlaybackMsg};

    match msg {
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
            PlaybackMsg::DispatchSeek { position_ms } => {
                Some(IpcCommand::Audio(AudioCommand::Seek {
                    position_ms: *position_ms,
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
        AppMsg::Library(LibraryMsg::RequestRescanLibrary) => Some(IpcCommand::Database(
            melotui_ipc::command::DatabaseCommand::ScanLibrary {
                root: state.library.root.clone(),
            },
        )),

        _ => None,
    }
}

pub fn ipc_to_appmsg(evt: IpcEvent) -> AppMsg {
    use melotui_core::message::{download::DownloadMsg, playback::PlaybackMsg};

    match evt {
        IpcEvent::DownloadProgress { percent } => {
            AppMsg::Download(DownloadMsg::DownloadProgress { percent })
        }
        IpcEvent::DownloadFinished => AppMsg::Download(DownloadMsg::DownloadFinished),
        IpcEvent::DownloadFailed { error } => {
            AppMsg::Download(DownloadMsg::DownloadFailed { error })
        }
        IpcEvent::PlaybackStarted { path } => AppMsg::Playback(PlaybackMsg::AudioStarted { path }),
        IpcEvent::PlaybackProgress { position_ms } => {
            AppMsg::Playback(PlaybackMsg::AudioProgress { position_ms })
        }
        IpcEvent::PlaybackFinished => AppMsg::Playback(PlaybackMsg::AudioFinished),
        IpcEvent::PlaybackPaused => AppMsg::Playback(PlaybackMsg::AudioPaused),
        IpcEvent::PlaybackResumed => AppMsg::Playback(PlaybackMsg::AudioResumed),
        _ => AppMsg::Noop,
    }
}
