use melotui_ipc::{
    channel::IpcWorker, command::AudioCommand, event::IpcEvent, spectrum_bridge::SpectrumProducer,
};
use ringbuf::{HeapRb, traits::Split};
use rodio::{Decoder, Sink, Source};
use std::{fs::File, path::PathBuf, thread, time::Duration};

use fundsp::hacker32::AudioUnit;

use crate::{
    clock::TimeKeeper,
    effects::{AudioParams, create_audio_graph},
    source::FundspSource,
    tap::SpectrumSniffer,
    visualizer::VisualizerManager,
};

pub fn run(mut worker: impl IpcWorker<AudioCommand>, spectrum_producer: SpectrumProducer) {
    let stream_handle =
        rodio::OutputStreamBuilder::open_default_stream().expect("open default audio stream");

    let mut engine = AudioEngine::new(
        rodio::Sink::connect_new(stream_handle.mixer()),
        spectrum_producer,
    );

    loop {
        let commands = fetch_and_coalesce_commands(&mut worker);

        for cmd in commands {
            engine.handle_command(cmd, &mut worker);
        }

        engine.tick(&mut worker);

        // 10ms sleep covers ~100Hz
        thread::sleep(Duration::from_millis(10));
    }
}

fn fetch_and_coalesce_commands(worker: &mut impl IpcWorker<AudioCommand>) -> Vec<AudioCommand> {
    let mut raw_batch = Vec::new();
    while let Some(cmd) = worker.try_recv() {
        raw_batch.push(cmd);
    }

    if raw_batch.is_empty() {
        return Vec::new();
    }

    let mut optimized = Vec::with_capacity(raw_batch.len());

    for cmd in raw_batch {
        match cmd {
            AudioCommand::Play { .. } => {
                optimized.retain(|c: &AudioCommand| c.is_setter());
                optimized.push(cmd);
            }
            AudioCommand::Seek { .. } => {
                optimized.retain(|c| !matches!(c, AudioCommand::Seek { .. }));
                optimized.push(cmd);
            }
            AudioCommand::SetVolume(_) => {
                optimized.retain(|c| !matches!(c, AudioCommand::SetVolume(_)));
                optimized.push(cmd);
            }
            _ => optimized.push(cmd),
        }
    }
    optimized
}

struct AudioEngine {
    sink: Sink,
    producer: SpectrumProducer,
    time_keeper: TimeKeeper,
    vis_manager: VisualizerManager,
    dsp_params: AudioParams,
}

impl AudioEngine {
    fn new(sink: Sink, producer: SpectrumProducer) -> Self {
        Self {
            sink,
            producer,
            time_keeper: TimeKeeper::new(),
            vis_manager: VisualizerManager::new(),
            dsp_params: AudioParams::new(),
        }
    }

    fn handle_command<W: IpcWorker<AudioCommand>>(&mut self, cmd: AudioCommand, worker: &mut W) {
        match cmd {
            AudioCommand::Play { path } => self.do_play(path, worker),
            AudioCommand::Stop => self.do_stop(),
            AudioCommand::Pause => self.do_pause(worker),
            AudioCommand::Resume => self.do_resume(worker),
            AudioCommand::Seek { position_ms } => self.do_seek(position_ms),
            AudioCommand::SetVisualizer { spectrum_style } => match spectrum_style {
                Some(s) => self.vis_manager.set_style(s),
                None => self.vis_manager.set_enabled(false),
            },
            AudioCommand::SetVolume(v) => self.sink.set_volume(v),
            AudioCommand::SetSpeed(v) => {
                self.time_keeper.set_speed(v);
                self.sink.set_speed(v);
            }
            AudioCommand::SetBass(v) => self.dsp_params.bass.set_value(v),
            AudioCommand::SetTreble(v) => self.dsp_params.treble.set_value(v),
            AudioCommand::SetReverb(v) => self.dsp_params.reverb.set_value(v),
            AudioCommand::SetEcho(v) => self.dsp_params.echo.set_value(v),
            AudioCommand::Shutdown => std::process::exit(0),
        }
    }

    fn do_play<W: IpcWorker<AudioCommand>>(&mut self, path: PathBuf, worker: &mut W) {
        if !self.sink.empty() {
            self.sink.stop();
        }

        self.time_keeper.stop();
        self.vis_manager.detach_source();

        match File::open(&path).and_then(|f| Decoder::try_from(f).map_err(std::io::Error::other)) {
            Ok(decoder) => {
                self.time_keeper.start();

                let channels = decoder.channels();
                let unit: Box<dyn AudioUnit> = create_audio_graph(channels, &self.dsp_params);
                let dsp_source = FundspSource::new(decoder, unit);

                let ring = HeapRb::<f32>::new(8192);
                let (producer, consumer) = ring.split();
                self.vis_manager.attach_source(consumer);

                let source_with_sniffer = SpectrumSniffer {
                    input: dsp_source,
                    producer,
                };

                self.sink.append(source_with_sniffer);
                self.sink.play();
                self.sink.set_speed(self.time_keeper.current_speed);

                worker.send(IpcEvent::PlaybackStarted {
                    path: path.to_string_lossy().to_string(),
                });
            }
            Err(_) => {
                worker.send(IpcEvent::Error {
                    message: "Failed to open file".into(),
                });
            }
        }
    }

    fn do_stop(&mut self) {
        self.sink.stop();
        self.time_keeper.stop();
        self.vis_manager.detach_source();
    }

    fn do_pause<W: IpcWorker<AudioCommand>>(&mut self, worker: &mut W) {
        self.sink.pause();
        self.time_keeper.pause();
        worker.send(IpcEvent::PlaybackPaused);
    }

    fn do_resume<W: IpcWorker<AudioCommand>>(&mut self, worker: &mut W) {
        if !self.time_keeper.is_playing && !self.sink.empty() {
            self.sink.play();
            self.time_keeper.start();
            worker.send(IpcEvent::PlaybackResumed);
        }
    }

    fn do_seek(&mut self, ms: u64) {
        let dur = Duration::from_millis(ms);
        if let Err(e) = self.sink.try_seek(dur) {
            tracing::error!("Seek Error: {:?}", e);
        } else {
            self.time_keeper.seek(dur);
        }
    }

    fn tick<W: IpcWorker<AudioCommand>>(&mut self, worker: &mut W) {
        if !self.time_keeper.is_playing {
            return;
        }

        if self.sink.empty() {
            self.do_stop();
            worker.send(IpcEvent::PlaybackFinished);
            return;
        }

        let pos = self.time_keeper.get_position();
        worker.send(IpcEvent::PlaybackProgress {
            position_ms: pos.as_millis() as u64,
        });

        if let Some(spectrum) = self.vis_manager.tick() {
            self.producer.push(&spectrum);
        }
    }
}

trait AudioCommandExt {
    fn is_setter(&self) -> bool;
}

impl AudioCommandExt for AudioCommand {
    fn is_setter(&self) -> bool {
        matches!(
            self,
            AudioCommand::SetVolume(_)
                | AudioCommand::SetSpeed(_)
                | AudioCommand::SetBass(_)
                | AudioCommand::SetTreble(_)
                | AudioCommand::SetReverb(_)
                | AudioCommand::SetEcho(_)
                | AudioCommand::SetVisualizer { .. }
        )
    }
}
