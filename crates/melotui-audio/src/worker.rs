use melotui_bus::{
    channel::IpcWorker, command::AudioCommand, event::IpcEvent, spectrum_bridge::SpectrumProducer,
};
use ringbuf::{HeapRb, traits::Split};
use rodio::{Decoder, Sink, Source};
use std::{
    fs::File,
    path::PathBuf,
    sync::{Arc, atomic::AtomicBool},
    thread,
    time::Duration,
};

use fundsp::hacker32::AudioUnit;

use crate::{
    clock::TimeKeeper,
    effects::{AudioParams, create_audio_graph},
    reaper::Reaper,
    source::FundspSource,
    tap::SpectrumSniffer,
    visualizer::VisualizerManager,
};

/// Output stream settings, plumbed through from `[audio]` in the user's config.
pub struct AudioRunConfig {
    pub fft_size: usize,
    /// Output rate in Hz; `None` takes whatever the device reports as default.
    pub sample_rate: Option<u32>,
    /// Output buffer in frames; `None` leaves cpal on its default
    /// (25 ms period inside a 100 ms buffer).
    pub buffer_size: Option<u32>,
}

/// Opens the output stream, honouring the configured rate and buffer size.
///
/// Both settings matter for dropout resistance. The ALSA "default" PCM usually
/// reports 44100 even when the sound server runs at 48000, which costs us a
/// resample in each direction; and cpal's default buffer only leaves ~75 ms of
/// slack for the callback thread to miss its deadline in. On failure rodio
/// retries every config the device advertises before giving up.
fn open_output_stream(config: &AudioRunConfig) -> Result<rodio::OutputStream, rodio::StreamError> {
    let mut builder = rodio::OutputStreamBuilder::from_default_device()?;

    if let Some(rate) = config.sample_rate {
        builder = builder.with_sample_rate(rate);
    }
    if let Some(frames) = config.buffer_size {
        builder = builder.with_buffer_size(rodio::cpal::BufferSize::Fixed(frames));
    }

    builder.open_stream_or_fallback()
}

pub fn run(
    mut worker: impl IpcWorker<AudioCommand>,
    spectrum_producer: SpectrumProducer,
    config: AudioRunConfig,
) {
    let mut stream_handle = match open_output_stream(&config) {
        Ok(handle) => handle,
        Err(e) => {
            // Previously this was `.expect(...)`, which panicked this thread and
            // left the app running with no audio and no explanation. Tell the UI
            // and stop; `Engine::dispatch` ignores sends to a dead worker.
            tracing::error!("Failed to open audio output stream: {e}");
            worker.send(IpcEvent::Error {
                message: format!("No audio output available: {e}"),
            });
            return;
        }
    };
    stream_handle.log_on_drop(false);

    tracing::info!(
        "Audio output opened: {} Hz, {} ch, buffer {:?}",
        stream_handle.config().sample_rate(),
        stream_handle.config().channel_count(),
        stream_handle.config().buffer_size(),
    );

    let mut engine = AudioEngine::new(
        Sink::connect_new(stream_handle.mixer()),
        spectrum_producer,
        config.fft_size,
    );

    loop {
        let commands = fetch_and_coalesce_commands(&mut worker);

        for cmd in commands {
            if !engine.handle_command(cmd, &mut worker) {
                return;
            }
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
    seek_epoch: u64,
    bypass: Arc<AtomicBool>,
    reaper: Reaper,
}

impl AudioEngine {
    pub fn new(sink: Sink, spectrum_producer: SpectrumProducer, fft_size: usize) -> Self {
        Self {
            sink,
            producer: spectrum_producer,
            time_keeper: TimeKeeper::new(),
            vis_manager: VisualizerManager::new(fft_size),
            dsp_params: AudioParams::new(),
            seek_epoch: 0,
            bypass: Arc::new(AtomicBool::new(true)),
            reaper: Reaper::spawn(),
        }
    }

    fn handle_command<W: IpcWorker<AudioCommand>>(
        &mut self,
        cmd: AudioCommand,
        worker: &mut W,
    ) -> bool {
        let is_setter = cmd.is_setter();
        match cmd {
            AudioCommand::Play { path } => self.do_play(path, worker),
            AudioCommand::Stop => self.do_stop(),
            AudioCommand::Pause => self.do_pause(worker),
            AudioCommand::Resume => self.do_resume(worker),
            AudioCommand::Seek { position_ms, epoch } => self.do_seek(position_ms, epoch),
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
            AudioCommand::SetPan(v) => self.dsp_params.pan.set_value(v),
            AudioCommand::SetLowpass(v) => self.dsp_params.lowpass.set_value(v),
            AudioCommand::SetHighpass(v) => self.dsp_params.highpass.set_value(v),
            AudioCommand::SetEqBand { band, gain } => {
                if band < 10 {
                    self.dsp_params.eq_bands[band].set_value(gain);
                }
            }
            AudioCommand::SetAutoPanRate(v) => self.dsp_params.auto_pan_rate.set_value(v),
            AudioCommand::SetAutoPanDepth(v) => self.dsp_params.auto_pan_depth.set_value(v),
            AudioCommand::SetWobble(v) => self.dsp_params.wobble.set_value(v),
            AudioCommand::SetKaraoke(v) => self.dsp_params.karaoke.set_value(v),
            AudioCommand::SetWidth(v) => self.dsp_params.width.set_value(v),
            AudioCommand::SetBitcrush(v) => self.dsp_params.bitcrush.set_value(v),
            AudioCommand::SetExciter(v) => self.dsp_params.exciter.set_value(v),
            AudioCommand::SetPingPong(v) => self.dsp_params.ping_pong.set_value(v),
            AudioCommand::SetPhaser(v) => self.dsp_params.phaser.set_value(v),
            AudioCommand::Shutdown => {
                if !self.sink.empty() {
                    self.sink.stop();
                }
                self.time_keeper.stop();
                self.vis_manager.detach_source();
                return false;
            }
        }

        if is_setter {
            self.bypass.store(
                self.dsp_params.is_all_default(),
                std::sync::atomic::Ordering::Relaxed,
            );
        }

        true
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
                let sample_rate = decoder.sample_rate();
                let unit: Box<dyn AudioUnit> = create_audio_graph(channels, &self.dsp_params);
                let dsp_source = FundspSource::new(decoder, unit, self.bypass.clone());

                let ring = HeapRb::<f32>::new(8192);
                let (producer, consumer) = ring.split();
                self.vis_manager.attach_source(consumer, sample_rate);

                let source_with_sniffer = SpectrumSniffer::new(dsp_source, producer);

                // Box here, on the worker thread, so that when rodio drops this
                // source on the audio callback thread the handoff to the reaper
                // is a bare pointer move instead of ~1 MB of `free()` calls.
                self.sink
                    .append(self.reaper.guard(Box::new(source_with_sniffer)));
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

    fn do_seek(&mut self, ms: u64, epoch: u64) {
        let dur = Duration::from_millis(ms);
        // rodio's Speed wrapper multiplies the seek duration by its speed factor,
        // assuming the duration is in 'output time'. Since we track and provide
        // absolute 'file time', we divide by the speed to counteract the multiplication.
        let adjusted_dur =
            Duration::from_secs_f64(dur.as_secs_f64() / (self.time_keeper.current_speed as f64));

        if let Err(e) = self.sink.try_seek(adjusted_dur) {
            tracing::error!("Seek Error: {:?}", e);
        } else {
            self.time_keeper.seek(dur);
            self.seek_epoch = epoch;
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
            epoch: self.seek_epoch,
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
                | AudioCommand::SetPan(_)
                | AudioCommand::SetLowpass(_)
                | AudioCommand::SetHighpass(_)
                | AudioCommand::SetEqBand { .. }
                | AudioCommand::SetAutoPanRate(_)
                | AudioCommand::SetAutoPanDepth(_)
                | AudioCommand::SetWobble(_)
                | AudioCommand::SetKaraoke(_)
                | AudioCommand::SetWidth(_)
                | AudioCommand::SetBitcrush(_)
                | AudioCommand::SetExciter(_)
                | AudioCommand::SetPingPong(_)
                | AudioCommand::SetPhaser(_)
                | AudioCommand::SetVisualizer { .. }
        )
    }
}
