use std::sync::mpsc;

use crate::{channel::IpcWorker, event::IpcEvent};

#[derive(Debug)]
pub struct WorkerEndpoint<C> {
    pub(crate) cmd_rx: mpsc::Receiver<C>,
    pub(crate) evt_tx: mpsc::Sender<IpcEvent>,
}

pub fn hub() -> (mpsc::Receiver<IpcEvent>, mpsc::Sender<IpcEvent>) {
    let (tx, rx) = mpsc::channel();
    (rx, tx)
}

pub fn join_worker<C>(event_bus: mpsc::Sender<IpcEvent>) -> (mpsc::Sender<C>, WorkerEndpoint<C>) {
    let (cmd_tx, cmd_rx) = mpsc::channel();
    (
        cmd_tx,
        WorkerEndpoint {
            cmd_rx,
            evt_tx: event_bus,
        },
    )
}

impl<C: Send> IpcWorker<C> for WorkerEndpoint<C> {
    fn recv(&self) -> Option<C> {
        self.cmd_rx.recv().ok()
    }

    fn try_recv(&self) -> Option<C> {
        self.cmd_rx.try_recv().ok()
    }

    fn send(&self, evt: IpcEvent) {
        if let Err(err) = self.evt_tx.send(evt) {
            tracing::error!("IPC worker failed to send event: {}", err);
        }
    }
}
