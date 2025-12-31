pub mod inprocess;

use crate::{command::IpcCommand, event::IpcEvent};

pub trait IpcSender: Send {
    fn send(&self, cmd: IpcCommand);
}

pub trait IpcReceiver: Send {
    fn try_recv(&mut self) -> Option<IpcEvent>;
}

pub trait IpcWorker: Send {
    fn recv(&self) -> Option<IpcCommand>;
    // Non-blocking receive
    fn try_recv(&self) -> Option<IpcCommand>;
    fn send(&self, evt: IpcEvent);
}
