pub mod inprocess;

use std::time::Duration;

use crate::{command::IpcCommand, event::IpcEvent};

pub trait IpcSender: Send {
    fn send(&self, cmd: IpcCommand);
}

pub trait IpcReceiver: Send {
    fn try_recv(&mut self) -> Option<IpcEvent>;
}

pub trait IpcWorker<C>: Send {
    fn recv(&self) -> Option<C>;
    fn try_recv(&self) -> Option<C>;
    fn recv_timeout(&self, timeout: Duration) -> Option<C>;
    fn send(&self, evt: IpcEvent);
}
