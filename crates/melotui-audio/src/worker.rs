use melotui_ipc::{
    channel::IpcWorker, command::AudioCommand, event::IpcEvent, spectrum_bridge::SpectrumProducer,
};
use ringbuf::{HeapRb, traits::Split};
use rodio::{Decoder, Source};
use std::{
    fs::File,
    thread,
    time::{Duration, Instant},
};

use crate::{
    analyzer::SpectrumAnalyzer, cava::Cava, dsp_adapter::FundspSource, sniffer::SpectrumSniffer,
};
use fundsp::hacker32::*;

// Helper alias to get the concrete Consumer type from a HeapRb<f32>
type AudioConsumer = <HeapRb<f32> as Split>::Cons;

enum VisualizerBackend {
    Internal(SpectrumAnalyzer<AudioConsumer>),
    External(Cava),
    None,
}

struct AudioParams {
    bass: Shared,
    treble: Shared,
    reverb: Shared,
    echo: Shared,
}

impl AudioParams {
    fn new() -> Self {
        Self {
            bass: shared(1.0),
            treble: shared(1.0),
            reverb: shared(0.0),
            echo: shared(0.0),
        }
    }
}

pub fn run(worker: impl IpcWorker<AudioCommand>, spectrum_producer: SpectrumProducer) {
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

    let mut backend = VisualizerBackend::None;
    let mut visualizer_active = false;
    let mut is_playing = false;

    // --- TIME KEEPING STATE ---
    let mut last_resume_time: Option<Instant> = None;
    let mut accumulated_time = Duration::ZERO;
    let mut current_speed: f32 = 1.0;

    // Persistent DSP Parameters
    let dsp_params = AudioParams::new();

    loop {
        // 1. Handle Commands
        if let Some(cmd) = worker.try_recv() {
            match cmd {
                AudioCommand::Play { path } => {
                    if !sink.empty() {
                        sink.stop();
                    }

                    if let Ok(file) = File::open(&path)
                        && let Ok(decoder) = Decoder::try_from(file)
                    {
                        // Reset Time State
                        is_playing = true;
                        accumulated_time = Duration::ZERO;
                        last_resume_time = Some(Instant::now());
                        // Note: We keep current_speed as user might have set it globally

                        // --- DSP GRAPH CONSTRUCTION ---
                        let channels = decoder.channels();
                        let unit: Box<dyn AudioUnit> = create_audio_graph(channels, &dsp_params);
                        let dsp_source = FundspSource::new(decoder, unit);

                        // --- VISUALIZER TAP ---
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
                                    backend = VisualizerBackend::Internal(SpectrumAnalyzer::new(
                                        consumer,
                                    ));
                                }
                            }
                        } else {
                            backend = VisualizerBackend::Internal(SpectrumAnalyzer::new(consumer));
                        }

                        let source_with_sniffer = SpectrumSniffer {
                            input: dsp_source,
                            producer,
                        };

                        sink.append(source_with_sniffer);
                        sink.play();
                        sink.set_speed(current_speed);

                        worker.send(IpcEvent::PlaybackStarted {
                            path: path.to_string_lossy().to_string(),
                        });
                    } else {
                        worker.send(IpcEvent::Error {
                            message: "Failed to open file".into(),
                        });
                    }
                }
                AudioCommand::Pause => {
                    if is_playing {
                        sink.pause();
                        is_playing = false;

                        // Commit the elapsed time * speed
                        if let Some(start) = last_resume_time {
                            let elapsed = start.elapsed().as_secs_f32();
                            // SPEED CORRECTION:
                            accumulated_time += Duration::from_secs_f32(elapsed * current_speed);
                        }
                        last_resume_time = None;
                        worker.send(IpcEvent::PlaybackPaused);
                    }
                }
                AudioCommand::Resume => {
                    if !is_playing && !sink.empty() {
                        sink.play();
                        is_playing = true;
                        last_resume_time = Some(Instant::now());
                        worker.send(IpcEvent::PlaybackResumed);
                    }
                }
                AudioCommand::Stop => {
                    sink.stop();
                    is_playing = false;
                    accumulated_time = Duration::ZERO;
                    last_resume_time = None;
                    backend = VisualizerBackend::None;
                }
                AudioCommand::Seek { position_ms } => {
                    let seek_dur = Duration::from_millis(position_ms);
                    if let Err(e) = sink.try_seek(seek_dur) {
                        tracing::error!("Seek Error: {:?}", e);
                    } else {
                        // Reset our counter to the new position
                        accumulated_time = seek_dur;
                        if is_playing {
                            last_resume_time = Some(Instant::now());
                        }
                    }
                }
                AudioCommand::SetVisualizer { spectrum_style } => match spectrum_style {
                    Some(s) => {
                        let _ = s;
                        visualizer_active = true;
                    }
                    None => {
                        visualizer_active = false;
                    }
                },

                // --- AUDIO CONTROLS ---
                AudioCommand::SetVolume(v) => sink.set_volume(v),
                AudioCommand::SetSpeed(v) => {
                    // 1. Commit the time spent at the OLD speed
                    if is_playing && let Some(start) = last_resume_time {
                        let elapsed = start.elapsed().as_secs_f32();
                        accumulated_time += Duration::from_secs_f32(elapsed * current_speed);
                        last_resume_time = Some(Instant::now()); // Restart timer for new speed
                    }

                    // 2. Update state
                    current_speed = v;
                    sink.set_speed(v);
                }
                AudioCommand::SetBass(v) => dsp_params.bass.set_value(v),
                AudioCommand::SetTreble(v) => dsp_params.treble.set_value(v),
                AudioCommand::SetReverb(v) => dsp_params.reverb.set_value(v),
                AudioCommand::SetEcho(v) => dsp_params.echo.set_value(v),

                AudioCommand::Shutdown => break,
            }
        }

        // 2. Loop Actions
        if is_playing {
            // A. Progress Calculation with Speed Correction
            let current_pos = if let Some(start) = last_resume_time {
                let elapsed = start.elapsed().as_secs_f32();
                accumulated_time + Duration::from_secs_f32(elapsed * current_speed)
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
                    spectrum_producer.push(&magnitude);
                }
            }
        }

        thread::sleep(Duration::from_millis(10));
    }
}

/// Helper to build the Fundsp Graph
fn create_audio_graph(channels: u16, params: &AudioParams) -> Box<dyn AudioUnit> {
    let headroom = constant(0.707);

    let bass_node = ((pass() * headroom) | constant(80.0) | constant(0.707) | var(&params.bass))
        >> lowshelf()
        >> (pass() | constant(60.0) | constant(0.8) | constant(3.5))
        >> lowshelf();

    // Treble (High Shelf): Freq=3000Hz, Q=0.7
    let treble_node =
        (pass() | constant(3000.0) | constant(0.7) | var(&params.treble)) >> highshelf();

    // Echo: Feedback Delay (Dry & (Wet >> Delay))
    let echo_node = pass() & ((pass() * var(&params.echo)) >> delay(0.5));

    let mono_chain = bass_node >> treble_node >> echo_node;

    // Stereo Reverb & Mixer Construction

    let wet_vol = var(&params.reverb);
    let wet_stereo = wet_vol.clone() | wet_vol; // [WetL, WetR]

    let dry_gain = constant(1.0) - var(&params.reverb);
    let dry_stereo = dry_gain.clone() | dry_gain; // [DryL, DryR]

    let stereo_pass = pass() | pass();
    let dry_path = stereo_pass.clone() * dry_stereo;

    // Fixed reverb settings, user controls mix only
    let reverb_unit = reverb_stereo(10.0, 3.0, 0.5);
    let wet_path = reverb_unit * wet_stereo;

    let mixer_core = dry_path + wet_path;
    let feeder = stereo_pass.clone() ^ stereo_pass; // Split input to both paths
    let stereo_mixer = feeder >> mixer_core;

    // Limiter
    let limiter_stereo = limiter(0.001, 0.1) | limiter(0.001, 0.1);

    if channels == 1 {
        // Mono -> EQ -> Split -> Mixer -> Limit
        let chain = mono_chain >> split() >> stereo_mixer >> limiter_stereo;
        Box::new(chain)
    } else {
        // Stereo -> Dual Mono EQ -> Mixer -> Limit
        let stereo_eq = mono_chain.clone() | mono_chain;
        let chain = stereo_eq >> stereo_mixer >> limiter_stereo;
        Box::new(chain)
    }
}
