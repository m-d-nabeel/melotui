pub mod inprocess;

use std::time::Duration;

use crate::event::IpcEvent;

pub trait IpcWorker<C>: Send {
    fn recv(&self) -> Option<C>;
    fn try_recv(&self) -> Option<C>;
    fn recv_timeout(&self, timeout: Duration) -> Option<C>;
    fn send(&self, evt: IpcEvent);
}
