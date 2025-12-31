use melotui_ipc::{channel::IpcWorker, command::IpcCommand, event::IpcEvent};

pub fn run(worker: impl IpcWorker) {
    tracing::info!("Download worker started");

    loop {
        let Some(cmd) = worker.recv() else {
            tracing::info!("Worker: IPC closed, exiting");
            break;
        };

        tracing::debug!("Worker received command: {:?}", cmd);

        match cmd {
            IpcCommand::Download { query } => {
                worker.send(IpcEvent::DownloadStarted {
                    query: query.clone(),
                });

                for pct in [10, 30, 60, 100] {
                    std::thread::sleep(std::time::Duration::from_millis(300));
                    worker.send(IpcEvent::DownloadProgress { percent: pct });
                }

                worker.send(IpcEvent::DownloadFinished);
            }

            IpcCommand::CancelDownload => {
                worker.send(IpcEvent::DownloadFailed {
                    error: "Cancelled".into(),
                });
            }

            IpcCommand::Shutdown => {
                tracing::info!("Worker: shutdown requested");
                break;
            }

            _ => {}
        }
    }

    tracing::info!("Download worker exited");
}
