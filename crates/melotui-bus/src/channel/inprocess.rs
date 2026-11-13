use std::sync::{Arc, mpsc};
use std::time::Duration;

use crate::{channel::IpcWorker, event::IpcEvent};

/// Called when a worker has sent an event, to wake whoever reads them.
pub type Waker = Arc<dyn Fn() + Send + Sync>;

pub struct WorkerEndpoint<C> {
    pub(crate) cmd_rx: mpsc::Receiver<C>,
    pub(crate) evt_tx: mpsc::Sender<IpcEvent>,
    pub(crate) wake: Option<Waker>,
}

impl<C> std::fmt::Debug for WorkerEndpoint<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkerEndpoint")
            .field("wakes", &self.wake.is_some())
            .finish_non_exhaustive()
    }
}

impl<C> WorkerEndpoint<C> {
    /// The endpoint, calling `wake` after each event it sends that is worth
    /// waking for.
    ///
    /// Events wait in a channel until the app's loop next looks, and with
    /// nothing to tell it there was one it looked on a timer: a track that
    /// ended waited up to a tenth of a second for the next to be asked for.
    pub fn waking(mut self, wake: Waker) -> Self {
        self.wake = Some(wake);
        self
    }
}

/// Whether the reader should be woken for `evt` rather than find it when it
/// next looks. Not for progress — the playback position a hundred times a
/// second, a scan's count as fast as files can be looked at — which is
/// wanted no sooner than the next frame, and would have the reader drawing
/// for every one.
fn worth_waking_for(evt: &IpcEvent) -> bool {
    !matches!(
        evt,
        IpcEvent::PlaybackProgress { .. }
            | IpcEvent::ScanProgress { .. }
            | IpcEvent::DownloadProgress { .. }
    )
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
            wake: None,
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

    fn recv_timeout(&self, timeout: Duration) -> Option<C> {
        self.cmd_rx.recv_timeout(timeout).ok()
    }

    fn send(&self, evt: IpcEvent) {
        let wakes = worth_waking_for(&evt);
        if let Err(err) = self.evt_tx.send(evt) {
            tracing::error!("IPC worker failed to send event: {}", err);
            return;
        }
        if wakes && let Some(wake) = &self.wake {
            wake();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::DatabaseCommand;

    #[test]
    fn the_reader_is_woken_for_an_event_but_not_for_the_position() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let (bus_rx, bus_tx) = hub();
        let woken = Arc::new(AtomicUsize::new(0));
        let count = woken.clone();
        let (_cmd_tx, worker) = join_worker::<DatabaseCommand>(bus_tx);
        let worker = worker.waking(Arc::new(move || {
            count.fetch_add(1, Ordering::SeqCst);
        }));

        worker.send(IpcEvent::PlaybackFinished);
        assert_eq!(woken.load(Ordering::SeqCst), 1);
        // There to be read by the time the reader is woken.
        assert!(matches!(bus_rx.try_recv(), Ok(IpcEvent::PlaybackFinished)));

        for position_ms in 0..100 {
            worker.send(IpcEvent::PlaybackProgress {
                position_ms,
                epoch: 0,
            });
        }
        assert_eq!(woken.load(Ordering::SeqCst), 1, "woken for the position");
        assert_eq!(bus_rx.try_iter().count(), 100, "but it is still sent");
    }

    #[test]
    fn test_bus_channel_communication() {
        // Create the central event bus (hub)
        let (bus_rx, bus_tx) = hub();

        // Join a mock database worker to the bus
        let (cmd_tx, worker) = join_worker::<DatabaseCommand>(bus_tx);

        // Send a command to the worker
        cmd_tx
            .send(DatabaseCommand::SetFavorite {
                track_id: 1,
                favorite: true,
            })
            .unwrap();

        // Worker should receive it
        let cmd = worker.try_recv().expect("Worker should receive command");
        match cmd {
            DatabaseCommand::SetFavorite { track_id, .. } => assert_eq!(track_id, 1),
            _ => panic!("Expected SetFavorite"),
        }

        // Worker sends an event back
        worker.send(IpcEvent::LibraryRescanned { changed: true });

        // Hub should receive the event
        let evt = bus_rx.try_recv().expect("Hub should receive event");
        match evt {
            IpcEvent::LibraryRescanned { .. } => {}
            _ => panic!("Expected LibraryRescanned"),
        }
    }
}
