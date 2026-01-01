use std::{sync::mpsc, thread};

use melotui_core::AppMsg;
use melotui_ipc::{
    channel::inprocess, command::IpcCommand, event::IpcEvent, spectrum_bridge::SpectrumProducer,
};

/// The "Supervisor" that manages all workers and routing.
/// The UI holds this struct.
pub struct IpcRuntime {
    // 1. Egress: Events coming FROM workers (Single Event Stream)
    // We keep the Receiver here so the UI can poll it.
    pub event_rx: mpsc::Receiver<IpcEvent>,

    // 2. Ingress: Routers to specific workers
    // Private! The UI doesn't get to touch these.
    download_tx: mpsc::Sender<IpcCommand>,
    audio_tx: mpsc::Sender<IpcCommand>,
}

impl IpcRuntime {
    /// Boot up the runtime and all workers
    pub fn new(spectrum_producer: SpectrumProducer) -> Self {
        // A. Create the "Bus" (The single event stream)
        // If you used mpsc::channel(), you would get (Sender, Receiver)
        // for that reason we use inprocess::hub to get the consistency
        let (ui_rx, bus_tx) = inprocess::hub();

        // B. Spawn Download Worker (The Child)
        // bus_tx is now correctly a Sender, so it can be cloned.
        let (dl_tx, dl_worker) = inprocess::join_worker(bus_tx.clone());
        thread::spawn(move || {
            melotui_download::run(dl_worker);
        });

        // C. Spawn Audio Worker (The Child)
        // Pass the producer to the audio worker
        let (audio_tx, audio_worker) = inprocess::join_worker(bus_tx.clone());
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

    /// The SINGLE method the UI calls to send commands.
    /// The UI does not know *who* handles the command.
    pub fn dispatch(&self, cmd: IpcCommand) {
        match cmd {
            // Route to Download Worker
            IpcCommand::Download { .. } | IpcCommand::CancelDownload => {
                let _ = self.download_tx.send(cmd);
            }

            // Route to Audio Worker
            IpcCommand::Play { .. }
            | IpcCommand::Pause
            | IpcCommand::Resume
            | IpcCommand::Stop
            | IpcCommand::Seek { .. }
            | IpcCommand::SetVisualizer { .. } => {
                let _ = self.audio_tx.send(cmd);
            }

            // Lifecycle: Supervisor manages shutdown order
            IpcCommand::Shutdown => {
                tracing::info!("IPC Runtime shutting down all workers...");
                let _ = self.download_tx.send(IpcCommand::Shutdown);
                let _ = self.audio_tx.send(IpcCommand::Shutdown);
            }
        }
    }
}

// Helper converters
pub fn appmsg_to_ipc(msg: &AppMsg) -> Option<IpcCommand> {
    match msg {
        AppMsg::DispatchDownload {
            query,
            download_path,
        } => Some(IpcCommand::Download {
            query: query.clone(),
            download_path: download_path.clone(),
        }),
        AppMsg::RequestCancelDownload => Some(IpcCommand::CancelDownload),
        AppMsg::RequestQuit => Some(IpcCommand::Shutdown),
        AppMsg::DispatchPlay { path } => Some(IpcCommand::Play { path: path.clone() }),
        AppMsg::DispatchPause => Some(IpcCommand::Pause),
        AppMsg::DispatchResume => Some(IpcCommand::Resume),
        AppMsg::DispatchStop => Some(IpcCommand::Stop),
        AppMsg::DispatchSeek { position_ms } => Some(IpcCommand::Seek {
            position_ms: *position_ms,
        }),
        AppMsg::DispatchSetVisualizer { active } => {
            Some(IpcCommand::SetVisualizer { active: *active })
        }
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
