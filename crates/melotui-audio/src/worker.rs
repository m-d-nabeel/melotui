use melotui_ipc::{
    channel::IpcWorker, command::IpcCommand, event::IpcEvent, spectrum_bridge::SpectrumProducer,
};
use ringbuf::{HeapRb, traits::Split};
use rodio::Decoder;
use std::{
    fs::File,
    thread,
    time::{Duration, Instant},
};

use crate::{analyzer::SpectrumAnalyzer, cava::Cava, sniffer::SpectrumSniffer};

// Helper alias to get the concrete Consumer type from a HeapRb<f32>
type AudioConsumer = <HeapRb<f32> as Split>::Cons;

enum VisualizerBackend {
    Internal(SpectrumAnalyzer<AudioConsumer>),
    External(Cava),
    None,
}

pub fn run(worker: impl IpcWorker, spectrum_producer: SpectrumProducer) {
    tracing::info!("Audio worker started");

    let stream_handle =
        rodio::OutputStreamBuilder::open_default_stream().expect("open default audio stream");
    let sink = rodio::Sink::connect_new(stream_handle.mixer());

    // 1. Determine Backend Strategy
    let use_cava = Cava::is_available();
    if use_cava {
        tracing::info!("CAVA detected. Using external visualizer backend.");
    } else {
        tracing::info!("CAVA not found. Using internal FFT backend.");
    }

    // Initialize backend container
    let mut backend = VisualizerBackend::None;

    // State
    let mut visualizer_active = false;
    let mut is_playing = false;
    let mut last_resume_time: Option<Instant> = None;
    let mut accumulated_time = Duration::ZERO;

    loop {
        // 1. Handle Commands
        if let Some(cmd) = worker.try_recv() {
            match cmd {
                IpcCommand::Play { path } => {
                    if !sink.empty() {
                        sink.stop();
                    }

                    if let Ok(file) = File::open(&path)
                        && let Ok(decoder) = Decoder::try_from(file)
                    {
                        // Always create a ringbuffer so we can attach the Sniffer.
                        // If we use CAVA, the data in this ringbuffer just won't be read
                        // by the Internal analyzer (or we fallback if CAVA fails).
                        let ring = HeapRb::<f32>::new(8192);
                        let (producer, consumer) = ring.split();

                        // Initialize Backend
                        if use_cava {
                            match Cava::new() {
                                Ok(c) => {
                                    backend = VisualizerBackend::External(c);
                                }
                                Err(e) => {
                                    tracing::error!("Failed to start CAVA: {}", e);
                                    // Fallback to internal
                                    backend = VisualizerBackend::Internal(SpectrumAnalyzer::new(
                                        consumer,
                                    ));
                                }
                            }
                        } else {
                            backend = VisualizerBackend::Internal(SpectrumAnalyzer::new(consumer));
                        }

                        // Attach Sniffer to the audio pipeline
                        let source_with_sniffer = SpectrumSniffer {
                            input: decoder,
                            producer,
                        };

                        sink.append(source_with_sniffer);
                        sink.play();
                        is_playing = true;
                        accumulated_time = Duration::ZERO;
                        last_resume_time = Some(Instant::now());

                        worker.send(IpcEvent::PlaybackStarted {
                            path: path.to_string_lossy().to_string(),
                        });
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
                    // Reset backend to free resources/processes
                    backend = VisualizerBackend::None;
                }
                IpcCommand::Seek { position_ms } => {
                    if let Err(e) = sink.try_seek(Duration::from_millis(position_ms)) {
                        tracing::error!("Seek Error: {:?}", e);
                    }
                }
                IpcCommand::SetVisualizer { active } => {
                    visualizer_active = active;
                    if !active {
                        // Optional: Drop backend to save CPU/kill CAVA process when not viewing
                        // backend = VisualizerBackend::None;
                    }
                }
                IpcCommand::Shutdown => break,
                _ => {}
            }
        }

        // 2. Loop Actions (Progress & Spectrum)
        if is_playing {
            // A. Progress
            let current_pos = if let Some(start) = last_resume_time {
                accumulated_time + start.elapsed()
            } else {
                accumulated_time
            };

            if sink.empty() {
                is_playing = false;
                accumulated_time = Duration::ZERO;
                last_resume_time = None;
                backend = VisualizerBackend::None;
                worker.send(IpcEvent::PlaybackFinished);
            } else {
                worker.send(IpcEvent::PlaybackProgress {
                    position_ms: current_pos.as_millis() as u64,
                });
            }

            // B. Spectrum
            if visualizer_active {
                let spectrum_opt = match &mut backend {
                    VisualizerBackend::Internal(analyzer) => analyzer.next_spectrum(),
                    VisualizerBackend::External(cava) => cava.next_spectrum(),
                    VisualizerBackend::None => None,
                };

                if let Some(magnitude) = spectrum_opt {
                    // OLD: worker.send(IpcEvent::SpectrumData { magnitude });

                    // NEW: Atomic Write (Zero-Copy logic)
                    spectrum_producer.push(&magnitude);
                }
            }
        }

        thread::sleep(Duration::from_millis(20));
    }
}
