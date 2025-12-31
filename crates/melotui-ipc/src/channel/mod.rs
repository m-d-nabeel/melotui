pub mod inprocess;

use crate::{command::IpcCommand, event::IpcEvent};

pub trait IpcSender: Send {
    fn send(&self, cmd: IpcCommand);
}

pub trait IpcReceiver: Send {
    fn try_recv(&self) -> Option<IpcEvent>;
}
