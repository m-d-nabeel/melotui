use melotui_ipc::{channel::IpcWorker, command::IpcCommand, event::IpcEvent};
use rodio::{Decoder, Source};
use std::{
    fs::File,
    io::BufReader,
    thread,
    time::{Duration, Instant},
};

pub fn run(worker: impl IpcWorker) {
    tracing::info!("Audio worker started");

    let stream_handle =
        rodio::OutputStreamBuilder::open_default_stream().expect("open default audio stream");
    let sink = rodio::Sink::connect_new(stream_handle.mixer());

    // --- TRACKING STATE ---
    let mut is_playing = false;
    let mut total_duration: Option<Duration>;
    let mut last_resume_time: Option<Instant> = None; // last started/resumed
    let mut accumulated_time = Duration::ZERO; // time already played before the current resume

    loop {
        // 1. Handle Commands
        if let Some(cmd) = worker.try_recv() {
            match cmd {
                IpcCommand::Play { path } => {
                    if !sink.empty() {
                        sink.stop();
                    }

                    if let Ok(file) = File::open(&path) {
                        let reader = BufReader::new(file);
                        if let Ok(source) = Decoder::new(reader) {
                            // Get duration before appending to sink as sink consumes source
                            total_duration = source.total_duration();

                            let td_ms = match total_duration {
                                Some(duration) => duration.as_millis() as u64,
                                None => {
                                    tracing::warn!(
                                        "Unable to determine total duration for the file."
                                    );
                                    0
                                }
                            };

                            sink.append(source);
                            sink.play();

                            is_playing = true;
                            accumulated_time = Duration::ZERO;
                            last_resume_time = Some(Instant::now());

                            worker.send(IpcEvent::PlaybackStarted {
                                path: path.to_string_lossy().to_string(),
                                total_duration_ms: td_ms,
                            });
                        }
                    }
                }
                IpcCommand::Pause => {
                    if is_playing {
                        sink.pause();
                        is_playing = false;

                        if let Some(start) = last_resume_time {
                            accumulated_time += start.elapsed();
                        }
                        last_resume_time = None;
                        worker.send(IpcEvent::PlaybackPaused);
                    }
                }
                IpcCommand::Resume => {
                    if !is_playing && !sink.empty() {
                        sink.play();
                        is_playing = true;
                        last_resume_time = Some(Instant::now());
                        worker.send(IpcEvent::PlaybackResumed);
                    }
                }
                IpcCommand::Stop => {
                    sink.stop();
                    is_playing = false;
                    accumulated_time = Duration::ZERO;
                    last_resume_time = None;
                }
                IpcCommand::Seek { position_ms } => {
                    if let Err(e) = sink.try_seek(Duration::from_millis(position_ms)) {
                        tracing::error!("Seek Error: {:?}", e);
                    }
                }
                IpcCommand::Shutdown => break,
                _ => {}
            }
        }

        // 2. Calculate and Send Progress
        if is_playing {
            // Current position = (Time since last resume) + (Time played before pause)
            let current_pos = if let Some(start) = last_resume_time {
                accumulated_time + start.elapsed()
            } else {
                accumulated_time
            };

            // Check if song finished naturally
            if sink.empty() {
                is_playing = false;
                accumulated_time = Duration::ZERO;
                last_resume_time = None;
                worker.send(IpcEvent::PlaybackFinished);
            } else {
                worker.send(IpcEvent::PlaybackProgress {
                    position_ms: current_pos.as_millis() as u64,
                });
            }
        }

        thread::sleep(Duration::from_millis(100));
    }
}
