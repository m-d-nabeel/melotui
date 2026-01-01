use std::sync::mpsc;

use crate::{channel::IpcWorker, command::IpcCommand, event::IpcEvent};

/// IPC endpoint owned by a worker.
/// It receives commands specifically for itself (Ingress),
/// but sends events to the shared Bus (Egress).
#[derive(Debug)]
pub struct WorkerEndpoint {
    pub(crate) cmd_rx: mpsc::Receiver<IpcCommand>,
    pub(crate) evt_tx: mpsc::Sender<IpcEvent>,
}

/// Creates the central Event Hub (The "Bus").
///
/// Returns:
/// 1. `mpsc::Receiver<IpcEvent>`: The SINGLE listener for the UI/Runtime.
/// 2. `mpsc::Sender<IpcEvent>`: The transmitter to be cloned and given to workers.
pub fn hub() -> (mpsc::Receiver<IpcEvent>, mpsc::Sender<IpcEvent>) {
    // Standard creation: (Sender, Receiver)
    let (tx, rx) = mpsc::channel();

    // We swap them to match the return signature:
    // 1. rx (For YOU, the Runtime, to keep)
    // 2. tx (For YOU to clone and give to workers)
    (rx, tx)
}

/// Creates a new worker endpoint that attaches to an existing Event Hub.
///
/// Arguments:
/// * `event_bus`: A clone of the central event sender.
///
/// Returns:
/// 1. `mpsc::Sender<IpcCommand>`: The dedicated command channel for THIS worker.
/// 2. `WorkerEndpoint`: The endpoint the worker consumes.
pub fn join_worker(
    event_bus: mpsc::Sender<IpcEvent>,
) -> (mpsc::Sender<IpcCommand>, WorkerEndpoint) {
    let (cmd_tx, cmd_rx) = mpsc::channel();
    (
        cmd_tx,
        WorkerEndpoint {
            cmd_rx,
            evt_tx: event_bus,
        },
    )
}

impl IpcWorker for WorkerEndpoint {
    fn recv(&self) -> Option<IpcCommand> {
        self.cmd_rx.recv().ok()
    }

    fn try_recv(&self) -> Option<IpcCommand> {
        self.cmd_rx.try_recv().ok()
    }

    fn send(&self, evt: IpcEvent) {
        if let Err(err) = self.evt_tx.send(evt) {
            tracing::error!("IPC worker failed to send event: {}", err);
        }
    }
}
