use std::{sync::mpsc, thread};

use melotui_core::AppMsg;
use melotui_ipc::{
    channel::inprocess,
    command::{AudioCommand, DownloadCommand, IpcCommand},
    event::IpcEvent,
    spectrum_bridge::SpectrumProducer,
};

pub struct IpcRuntime {
    // Egress: Events coming FROM workers
    pub event_rx: mpsc::Receiver<IpcEvent>,

    // Ingress: Typed routers
    download_tx: mpsc::Sender<DownloadCommand>,
    audio_tx: mpsc::Sender<AudioCommand>,
}

impl IpcRuntime {
    pub fn new(spectrum_producer: SpectrumProducer) -> Self {
        let (ui_rx, bus_tx) = inprocess::hub();

        // Spawn Download Worker
        let (dl_tx, dl_worker) = inprocess::join_worker::<DownloadCommand>(bus_tx.clone());
        thread::spawn(move || {
            melotui_download::run(dl_worker);
        });

        // Spawn Audio Worker
        let (audio_tx, audio_worker) = inprocess::join_worker::<AudioCommand>(bus_tx.clone());
        thread::spawn(move || {
            melotui_audio::run(audio_worker, spectrum_producer);
        });

        tracing::info!("IPC Runtime initialized with Download and Audio workers");

        Self {
            event_rx: ui_rx,
            download_tx: dl_tx,
            audio_tx,
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
            IpcCommand::Shutdown => {
                tracing::info!("IPC Runtime shutting down all workers...");
                let _ = self.download_tx.send(DownloadCommand::Shutdown);
                let _ = self.audio_tx.send(AudioCommand::Shutdown);
            }
            _ => {}
        }
    }
}

pub fn appmsg_to_ipc(msg: &AppMsg) -> Option<IpcCommand> {
    match msg {
        // --- Download ---
        AppMsg::DispatchDownload {
            query,
            download_path,
        } => Some(IpcCommand::Download(DownloadCommand::Request {
            query: query.clone(),
            download_path: download_path.clone(),
        })),
        AppMsg::RequestCancelDownload => Some(IpcCommand::Download(DownloadCommand::Cancel)),

        // --- Audio Lifecycle ---
        AppMsg::DispatchPlay { path } => {
            Some(IpcCommand::Audio(AudioCommand::Play { path: path.clone() }))
        }
        AppMsg::DispatchPause => Some(IpcCommand::Audio(AudioCommand::Pause)),
        AppMsg::DispatchResume => Some(IpcCommand::Audio(AudioCommand::Resume)),
        AppMsg::DispatchStop => Some(IpcCommand::Audio(AudioCommand::Stop)),
        AppMsg::DispatchSeek { position_ms } => Some(IpcCommand::Audio(AudioCommand::Seek {
            position_ms: *position_ms,
        })),
        AppMsg::DispatchSetVisualizer { active } => {
            Some(IpcCommand::Audio(AudioCommand::SetVisualizer {
                active: *active,
            }))
        }

        // --- Audio Effects ---
        AppMsg::DispatchSetVolume(v) => Some(IpcCommand::Audio(AudioCommand::SetVolume(*v))),
        AppMsg::DispatchSetSpeed(v) => Some(IpcCommand::Audio(AudioCommand::SetSpeed(*v))),
        AppMsg::DispatchSetBass(v) => Some(IpcCommand::Audio(AudioCommand::SetBass(*v))),
        AppMsg::DispatchSetTreble(v) => Some(IpcCommand::Audio(AudioCommand::SetTreble(*v))),
        AppMsg::DispatchSetReverb(v) => Some(IpcCommand::Audio(AudioCommand::SetReverb(*v))),
        AppMsg::DispatchSetEcho(v) => Some(IpcCommand::Audio(AudioCommand::SetEcho(*v))),

        // --- Global ---
        AppMsg::RequestQuit => Some(IpcCommand::Shutdown),

        _ => None,
    }
}

pub fn ipc_to_appmsg(evt: IpcEvent) -> AppMsg {
    match evt {
        IpcEvent::DownloadProgress { percent } => AppMsg::DownloadProgress { percent },
        IpcEvent::DownloadFinished => AppMsg::DownloadFinished,
        IpcEvent::DownloadFailed { error } => AppMsg::DownloadFailed { error },
        IpcEvent::PlaybackStarted { path } => AppMsg::AudioStarted { path },
        IpcEvent::PlaybackProgress { position_ms } => AppMsg::AudioProgress { position_ms },
        IpcEvent::PlaybackFinished => AppMsg::AudioFinished,
        IpcEvent::PlaybackPaused => AppMsg::AudioPaused,
        IpcEvent::PlaybackResumed => AppMsg::AudioResumed,
        _ => AppMsg::Noop,
    }
}
