use std::{thread, time::Duration};

use melotui_ipc::channel::IpcWorker;

pub fn run(worker: impl IpcWorker) {
    tracing::info!("Download worker started");

    loop {
        if let Some(cmd) = worker.try_recv() {
            tracing::debug!("Worker received command: {:?}", cmd);

            match cmd {
                melotui_ipc::command::IpcCommand::Play { path } => {
                    tracing::info!(
                        "melotui_ipc::command::IpcCommand::Play {:?}",
                        path.display()
                    );
                }
                melotui_ipc::command::IpcCommand::Pause => todo!(),
                melotui_ipc::command::IpcCommand::Resume => todo!(),
                melotui_ipc::command::IpcCommand::Stop => todo!(),
                melotui_ipc::command::IpcCommand::Seek { .. } => todo!(),
                melotui_ipc::command::IpcCommand::Shutdown => todo!(),
                _ => {}
            }
        }
        thread::sleep(Duration::from_millis(50));
    }
}
