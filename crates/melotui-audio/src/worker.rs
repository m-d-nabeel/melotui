use melotui_ipc::{
    channel::IpcWorker, command::AudioCommand, event::IpcEvent, spectrum_bridge::SpectrumProducer,
};
use ringbuf::{HeapRb, traits::Split};
use rodio::{Decoder, Source};
use std::{fs::File, thread, time::Duration};

use fundsp::hacker32::AudioUnit;

use crate::{
    clock::TimeKeeper,
    effects::{AudioParams, create_audio_graph},
    source::FundspSource,
    tap::SpectrumSniffer,
    visualizer::VisualizerManager,
};

pub fn run(worker: impl IpcWorker<AudioCommand>, spectrum_producer: SpectrumProducer) {
    tracing::info!("Audio worker started");

    let stream_handle =
        rodio::OutputStreamBuilder::open_default_stream().expect("open default audio stream");
    let sink = rodio::Sink::connect_new(stream_handle.mixer());

    // --- MANAGERS ---
    let mut time_keeper = TimeKeeper::new();
    let mut vis_manager = VisualizerManager::new();
    let dsp_params = AudioParams::new();

    loop {
        // 1. Handle Commands
        if let Some(cmd) = worker.try_recv() {
            match cmd {
                AudioCommand::Play { path } => {
                    if !sink.empty() {
                        sink.stop();
                    }

                    // Managers handle the reset logic
                    time_keeper.stop();
                    vis_manager.detach_source();

                    if let Ok(file) = File::open(&path)
                        && let Ok(decoder) = Decoder::try_from(file)
                    {
                        time_keeper.start();

                        // DSP Setup
                        let channels = decoder.channels();
                        let unit: Box<dyn AudioUnit> = create_audio_graph(channels, &dsp_params);
                        let dsp_source = FundspSource::new(decoder, unit);

                        // Visualizer Setup
                        let ring = HeapRb::<f32>::new(8192);
                        let (producer, consumer) = ring.split();
                        vis_manager.attach_source(consumer);

                        // Playback
                        let source_with_sniffer = SpectrumSniffer {
                            input: dsp_source,
                            producer,
                        };
                        sink.append(source_with_sniffer);
                        sink.play();
                        sink.set_speed(time_keeper.current_speed);

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
                    sink.pause();
                    time_keeper.pause();
                    worker.send(IpcEvent::PlaybackPaused);
                }
                AudioCommand::Resume => {
                    if !time_keeper.is_playing && !sink.empty() {
                        sink.play();
                        time_keeper.start();
                        worker.send(IpcEvent::PlaybackResumed);
                    }
                }
                AudioCommand::Stop => {
                    sink.stop();
                    time_keeper.stop();
                    vis_manager.detach_source();
                }
                AudioCommand::Seek { position_ms } => {
                    let seek_dur = Duration::from_millis(position_ms);
                    if let Err(e) = sink.try_seek(seek_dur) {
                        tracing::error!("Seek Error: {:?}", e);
                    } else {
                        time_keeper.seek(seek_dur);
                    }
                }
                AudioCommand::SetVisualizer { spectrum_style } => match spectrum_style {
                    Some(s) => vis_manager.set_style(s),
                    None => vis_manager.set_enabled(false),
                },
                AudioCommand::SetVolume(v) => sink.set_volume(v),
                AudioCommand::SetSpeed(v) => {
                    time_keeper.set_speed(v);
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
        if time_keeper.is_playing {
            let current_pos = time_keeper.get_position();

            if sink.empty() {
                time_keeper.stop();
                vis_manager.detach_source();
                worker.send(IpcEvent::PlaybackFinished);
            } else {
                worker.send(IpcEvent::PlaybackProgress {
                    position_ms: current_pos.as_millis() as u64,
                });
            }

            // Visualizer Tick
            if let Some(spectrum) = vis_manager.tick() {
                spectrum_producer.push(&spectrum);
            }
        }

        thread::sleep(Duration::from_millis(10));
    }
}
