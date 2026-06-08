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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::DatabaseCommand;

    #[test]
    fn test_bus_channel_communication() {
        // Create the central event bus (hub)
        let (bus_rx, bus_tx) = hub();

        // Join a mock database worker to the bus
        let (cmd_tx, worker) = join_worker::<DatabaseCommand>(bus_tx);

        // Send a command to the worker
        cmd_tx
            .send(DatabaseCommand::ToggleFavorite { track_id: 1 })
            .unwrap();

        // Worker should receive it
        let cmd = worker.try_recv().expect("Worker should receive command");
        match cmd {
            DatabaseCommand::ToggleFavorite { track_id } => assert_eq!(track_id, 1),
            _ => panic!("Expected ToggleFavorite"),
        }

        // Worker sends an event back
        worker.send(IpcEvent::LibraryRescanned);

        // Hub should receive the event
        let evt = bus_rx.try_recv().expect("Hub should receive event");
        match evt {
            IpcEvent::LibraryRescanned => {}
            _ => panic!("Expected LibraryRescanned"),
        }
    }
}
