use std::sync::mpsc;

use crate::{
    channel::{IpcReceiver, IpcSender},
    command::IpcCommand,
    event::IpcEvent,
};

/// IPC endpoint owned by the UI / runtime
///
/// Responsibilities:
/// - Send IPC commands to workers
/// - Receive IPC events from workers
#[derive(Debug)]
pub struct UiEndpoint {
    cmd_tx: mpsc::Sender<IpcCommand>,
    evt_rx: mpsc::Receiver<IpcEvent>,
    disconnected: bool,
}

/// IPC endpoint owned by a worker
///
/// Responsibilities:
/// - Receive IPC commands from UI
/// - Send IPC events back to UI
#[allow(unused)]
#[derive(Debug)]
pub struct WorkerEndpoint {
    pub(crate) cmd_rx: mpsc::Receiver<IpcCommand>,
    pub(crate) evt_tx: mpsc::Sender<IpcEvent>,
}

impl IpcSender for UiEndpoint {
    fn send(&self, cmd: IpcCommand) {
        tracing::trace!("IPC → send command: {:?}", cmd);
        if let Err(err) = self.cmd_tx.send(cmd) {
            tracing::error!("IPC send command failed: {}", err);
        }
    }
}

impl IpcReceiver for UiEndpoint {
    fn try_recv(&mut self) -> Option<IpcEvent> {
        match self.evt_rx.try_recv() {
            Ok(evt) => Some(evt),

            Err(mpsc::TryRecvError::Empty) => None,

            Err(mpsc::TryRecvError::Disconnected) => {
                if !self.disconnected {
                    tracing::warn!("IPC event channel disconnected");
                    self.disconnected = true;
                }
                None
            }
        }
    }
}
/// Create an in-process IPC channel
///
/// Returns:
/// - `UiEndpoint`: owned by UI / runtime
/// - `WorkerEndpoint`: moved into a worker thread/process
pub fn channel() -> (UiEndpoint, WorkerEndpoint) {
    let (cmd_tx, cmd_rx) = mpsc::channel::<IpcCommand>();
    let (evt_tx, evt_rx) = mpsc::channel::<IpcEvent>();

    tracing::info!("IPC in-process channel initialized");

    (
        UiEndpoint {
            cmd_tx,
            evt_rx,
            disconnected: false,
        },
        WorkerEndpoint { cmd_rx, evt_tx },
    )
}

impl crate::channel::IpcWorker for WorkerEndpoint {
    fn recv(&self) -> Option<IpcCommand> {
        self.cmd_rx.recv().ok()
    }

    fn send(&self, evt: IpcEvent) {
        if let Err(err) = self.evt_tx.send(evt) {
            tracing::error!("IPC worker send failed: {}", err);
        }
    }
}
