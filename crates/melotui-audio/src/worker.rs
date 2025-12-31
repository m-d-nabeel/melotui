use melotui_ipc::{channel::IpcWorker, command::IpcCommand, event::IpcEvent};
use rodio::Decoder;
use std::{fs::File, io::BufReader, thread, time::Duration};

pub fn run(worker: impl IpcWorker) {
    tracing::info!("Audio worker started");

    let stream_handle =
        rodio::OutputStreamBuilder::open_default_stream().expect("open default audio stream");
    let sink = rodio::Sink::connect_new(stream_handle.mixer());

    loop {
        // 2. Poll for commands
        if let Some(cmd) = worker.try_recv() {
            tracing::debug!("Audio Worker received: {:?}", cmd);

            match cmd {
                IpcCommand::Play { path } => {
                    if !sink.empty() {
                        sink.stop();
                    }

                    match File::open(&path) {
                        Ok(file) => {
                            let reader = BufReader::new(file);
                            match Decoder::new(reader) {
                                Ok(source) => {
                                    sink.append(source);
                                    sink.play();

                                    worker.send(IpcEvent::PlaybackStarted {
                                        path: path.to_string_lossy().to_string(),
                                    });
                                }
                                Err(e) => tracing::error!("Error decoding audio: {}", e),
                            }
                        }
                        Err(e) => tracing::error!("Failed to open file {:?}: {}", path, e),
                    }
                }
                IpcCommand::Pause => {
                    tracing::info!("Pause received");
                    sink.pause();
                }
                IpcCommand::Resume => {
                    tracing::info!("Resume received");
                    sink.play();
                }
                IpcCommand::Stop => {
                    tracing::info!("Stop received");
                    sink.stop();
                }
                IpcCommand::Seek { position_ms } => {
                    tracing::info!("Seek Call received for position: {}", position_ms);
                    if let Err(e) = sink.try_seek(Duration::from_millis(position_ms)) {
                        tracing::error!("SeekError: {:?}", e);
                    }
                }
                IpcCommand::Shutdown => {
                    tracing::info!("Shutdown received");
                    sink.stop();
                    break;
                }
                _ => {}
            }
        }

        // 3. Optional: Send progress updates back to UI
        // (You would need to track position manually or use a customized source)

        thread::sleep(Duration::from_millis(50));
    }
}
