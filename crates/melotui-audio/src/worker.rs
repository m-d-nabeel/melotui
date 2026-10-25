use melotui_bus::{
    channel::IpcWorker, command::AudioCommand, event::IpcEvent, spectrum_bridge::SpectrumProducer,
};
use ringbuf::{HeapRb, traits::Split};
use rodio::{Decoder, Player, Source};
use std::{
    fs::File,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use fundsp::hacker32::{AudioUnit, Shared, shared};

use crate::{
    clock::TimeKeeper,
    effects::{AudioParams, create_audio_graph},
    output::{self, Output},
    reaper::Reaper,
    source::FundspSource,
    tap::SpectrumSniffer,
    visualizer::VisualizerManager,
    volume::VolumeRamp,
};

/// Output stream settings, plumbed through from `[audio]` in the user's config.
pub struct AudioRunConfig {
    pub fft_size: usize,
    /// Output rate in Hz; `None` takes whatever the device reports as default.
    pub sample_rate: Option<u32>,
    /// Output buffer in frames; `None` leaves cpal on its default
    /// (25 ms period inside a 100 ms buffer).
    pub buffer_size: Option<u32>,
    /// The output device's name, or part of it; `None` follows the system
    /// default.
    pub output_device: Option<String>,
}

/// How long to wait before trying an output device again after it could not
/// be opened.
const RETRY_EVERY: Duration = Duration::from_secs(2);
/// How often a chosen device that could not be opened is tried again, while
/// the default plays instead.
const RETRY_CHOSEN_EVERY: Duration = Duration::from_secs(5);

/// The output device as the worker sees it: open, or waiting to be tried
/// again.
struct OutputSlot {
    output: Option<Output>,
    /// Set by the stream's error callback when its device goes away.
    lost: Arc<AtomicBool>,
    /// The device asked for; `None` follows the system default.
    wanted: Option<String>,
    /// Whether the default is playing in place of the device asked for.
    standing_in: bool,
    sample_rate: Option<u32>,
    buffer_size: Option<u32>,
    /// When to try again after a failure, and whether the failure was said.
    retry_at: Option<Instant>,
    failure_reported: bool,
}

impl OutputSlot {
    fn open(&self, device: Option<&str>) -> Result<Output, String> {
        self.lost.store(false, Ordering::SeqCst);
        // Rate and buffer size are honoured for dropout resistance: the ALSA
        // "default" PCM usually reports 44100 even when the sound server runs
        // at 48000, and cpal's default buffer leaves little slack. rodio
        // retries every config the device advertises before giving up.
        output::open(
            device,
            self.sample_rate,
            self.buffer_size,
            Arc::clone(&self.lost),
        )
    }

    /// Whether the output has to be opened (again) now.
    fn needs_opening(&self) -> bool {
        if self.lost.load(Ordering::SeqCst) {
            return true;
        }
        let due = self.retry_at.is_none_or(|at| Instant::now() >= at);
        (self.output.is_none() || self.standing_in) && due
    }

    /// Opens the device asked for, or the default in its place if it cannot
    /// be, reporting a failure the first time and quietly retrying after
    /// that. `Some` with a newly opened output.
    fn try_open(&mut self, worker: &mut impl IpcWorker<AudioCommand>) -> Option<&Output> {
        let wanted = self.wanted.clone();
        let opened = match self.open(wanted.as_deref()) {
            Ok(output) => {
                self.standing_in = false;
                Ok(output)
            }
            // A device asked for that cannot be opened — unplugged, or held
            // by the sound server — gives way to the default until it can.
            Err(e) if wanted.is_some() => {
                let name = wanted.as_deref().unwrap_or_default();
                if self.standing_in && self.output.is_some() && !self.lost.load(Ordering::SeqCst) {
                    // Something is playing already: keep it, say so once,
                    // and try again later.
                    if !self.failure_reported {
                        let current = self.output.as_ref().map_or("", |o| o.name.as_str());
                        worker.send(IpcEvent::Error {
                            message: format!(
                                "Cannot play through {name} ({e}); staying on {current}"
                            ),
                        });
                        self.failure_reported = true;
                    }
                    self.retry_at = Some(Instant::now() + RETRY_CHOSEN_EVERY);
                    return None;
                }
                match self.open(None) {
                    Ok(output) => {
                        worker.send(IpcEvent::Error {
                            message: format!(
                                "Cannot play through {name} ({e}); using {} until it can",
                                output.name
                            ),
                        });
                        self.standing_in = true;
                        self.retry_at = Some(Instant::now() + RETRY_CHOSEN_EVERY);
                        Ok(output)
                    }
                    Err(default_error) => Err(format!("{e}; the default: {default_error}")),
                }
            }
            Err(e) => Err(e),
        };
        match opened {
            Ok(output) => {
                if !self.standing_in {
                    self.retry_at = None;
                }
                self.failure_reported = false;
                worker.send(IpcEvent::AudioOutputOpened {
                    name: output.name.clone(),
                });
                self.output = Some(output);
                self.output.as_ref()
            }
            Err(e) => {
                tracing::warn!("Cannot open audio output: {e}");
                if !self.failure_reported {
                    worker.send(IpcEvent::Error {
                        message: format!("No audio output ({e}); trying again"),
                    });
                    self.failure_reported = true;
                }
                self.output = None;
                self.retry_at = Some(Instant::now() + RETRY_EVERY);
                None
            }
        }
    }
}

pub fn run(
    mut worker: impl IpcWorker<AudioCommand>,
    spectrum_producer: SpectrumProducer,
    config: AudioRunConfig,
) {
    let mut slot = OutputSlot {
        output: None,
        lost: Arc::new(AtomicBool::new(false)),
        wanted: config.output_device.clone(),
        standing_in: false,
        sample_rate: config.sample_rate,
        buffer_size: config.buffer_size,
        retry_at: None,
        failure_reported: false,
    };

    // A player until there is a device to play through: commands arriving
    // meanwhile are kept, and take effect once one is open.
    let (detached, _queue) = Player::new();
    let mut engine = AudioEngine::new(detached, spectrum_producer, config.fft_size);

    loop {
        if slot.lost.load(Ordering::SeqCst) {
            // The device is gone and nothing drives the player any more.
            engine.live = false;
        }
        if slot.needs_opening()
            && let Some(output) = slot.try_open(&mut worker)
        {
            // Whatever was playing carries on through the new device, from
            // where it was.
            engine.reconnect(Player::connect_new(output.sink.mixer()), &mut worker);
        }

        let commands = fetch_and_coalesce_commands(&mut worker);
        for cmd in commands {
            match cmd {
                AudioCommand::SetOutputDevice(device) => {
                    // Opened on the next pass, the current one playing until
                    // then; what is playing carries on through the new one.
                    slot.wanted = device;
                    slot.standing_in = slot.output.is_some();
                    slot.retry_at = None;
                    slot.failure_reported = false;
                }
                AudioCommand::ListOutputDevices => worker.send(IpcEvent::OutputDevices {
                    names: output::device_names(),
                }),
                cmd => {
                    if !engine.handle_command(cmd, &mut worker) {
                        return;
                    }
                }
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
    sink: Player,
    producer: SpectrumProducer,
    time_keeper: TimeKeeper,
    vis_manager: VisualizerManager,
    dsp_params: AudioParams,
    /// The volume setting, applied by a `VolumeRamp` on the playing source
    /// rather than by the sink, whose own control steps rather than glides.
    volume: Shared,
    seek_epoch: u64,
    bypass: Arc<AtomicBool>,
    reaper: Reaper,
    /// The file playing or paused, to carry on with on another device.
    current: Option<PathBuf>,
    /// Whether the player is connected to a device that is running. Without
    /// one nothing answers a seek, which rodio waits for.
    live: bool,
}

impl AudioEngine {
    pub fn new(sink: Player, spectrum_producer: SpectrumProducer, fft_size: usize) -> Self {
        Self {
            sink,
            producer: spectrum_producer,
            time_keeper: TimeKeeper::new(),
            vis_manager: VisualizerManager::new(fft_size),
            dsp_params: AudioParams::new(),
            volume: shared(1.0),
            seek_epoch: 0,
            bypass: Arc::new(AtomicBool::new(true)),
            reaper: Reaper::spawn(),
            current: None,
            live: false,
        }
    }

    /// Plays through `player` from now on — a new output device — carrying
    /// on with whatever was playing or paused, from where it was.
    fn reconnect<W: IpcWorker<AudioCommand>>(&mut self, player: Player, worker: &mut W) {
        let resume = self.current.clone().map(|path| {
            (
                path,
                self.time_keeper.get_position(),
                self.time_keeper.is_playing,
            )
        });
        self.sink.stop();
        self.sink = player;
        self.live = true;
        if let Some((path, position, playing)) = resume {
            self.time_keeper.stop();
            self.vis_manager.detach_source();
            if let Err(e) = self.start(&path) {
                worker.send(IpcEvent::Error { message: e });
                return;
            }
            let epoch = self.seek_epoch;
            self.do_seek(position.as_millis() as u64, epoch);
            if !playing {
                self.sink.pause();
                self.time_keeper.pause();
            }
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
            AudioCommand::SetVolume(v) => self.volume.set_value(v),
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
            // The device is the run loop's business, which takes these first.
            AudioCommand::SetOutputDevice(_) | AudioCommand::ListOutputDevices => {}
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

        match self.start(&path) {
            Ok(()) => worker.send(IpcEvent::PlaybackStarted {
                path: path.to_string_lossy().to_string(),
            }),
            Err(message) => worker.send(IpcEvent::Error { message }),
        }
    }

    /// Decodes `path` and starts it playing through the effects.
    fn start(&mut self, path: &std::path::Path) -> Result<(), String> {
        let name = path
            .file_name()
            .unwrap_or(path.as_os_str())
            .to_string_lossy();
        let decoder = File::open(path)
            .map_err(|e| format!("Cannot play {name}: {e}"))
            .and_then(|f| Decoder::try_from(f).map_err(|e| format!("Cannot play {name}: {e}")))?;
        self.current = Some(path.to_path_buf());
        self.time_keeper.start();

        let channels = decoder.channels().get();
        let sample_rate = decoder.sample_rate().get();
        let unit: Box<dyn AudioUnit> = create_audio_graph(channels, &self.dsp_params);
        let dsp_source = FundspSource::new(decoder, unit, self.bypass.clone());

        // A second of audio: room for the biggest lump the device pulls at
        // once plus the backlog the analyser keeps behind it.
        let ring = HeapRb::<f32>::new((sample_rate as usize).max(8192));
        let (producer, consumer) = ring.split();
        self.vis_manager.attach_source(consumer, sample_rate);

        let source_with_sniffer = SpectrumSniffer::new(dsp_source, producer);
        let source_with_volume = VolumeRamp::new(source_with_sniffer, self.volume.clone());

        // Box here, on the worker thread, so that when rodio drops this
        // source on the audio callback thread the handoff to the reaper
        // is a bare pointer move instead of ~1 MB of `free()` calls.
        self.sink
            .append(self.reaper.guard(Box::new(source_with_volume)));
        self.sink.play();
        self.sink.set_speed(self.time_keeper.current_speed);
        Ok(())
    }

    fn do_stop(&mut self) {
        self.current = None;
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
        // With no device running, nothing would answer the seek; the clock
        // moves, and playback picks up from there on the next device.
        if !self.live {
            self.time_keeper.seek(dur);
            self.seek_epoch = epoch;
            return;
        }
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
            self.vis_manager.restart();
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

#[cfg(test)]
mod tests {
    use super::*;
    use melotui_bus::{channel::inprocess, command::AudioCommand, spectrum_bridge};
    use std::sync::mpsc;

    /// Ten seconds of quiet stereo at 44.1 kHz, as a WAV file.
    fn wav(dir: &std::path::Path) -> PathBuf {
        let path = dir.join("quiet.wav");
        let frames: u32 = 44_100 * 10;
        let data_len = frames * 4;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes()); // PCM
        bytes.extend_from_slice(&2u16.to_le_bytes()); // channels
        bytes.extend_from_slice(&44_100u32.to_le_bytes());
        bytes.extend_from_slice(&(44_100u32 * 4).to_le_bytes());
        bytes.extend_from_slice(&4u16.to_le_bytes());
        bytes.extend_from_slice(&16u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data_len.to_le_bytes());
        bytes.resize(bytes.len() + data_len as usize, 0);
        std::fs::write(&path, bytes).unwrap();
        path
    }

    fn engine() -> AudioEngine {
        let (producer, _consumer) = spectrum_bridge::create_bridge();
        AudioEngine::new(Player::new().0, producer, 2048)
    }

    /// A player with a thread pulling its samples, as a device's callback
    /// would, so that seeks are answered. The thread stops once the test's
    /// handle is dropped, and rests between blocks so tests running beside
    /// it are not starved.
    fn live_player() -> (Player, Arc<()>) {
        let (mixer, mut source) = rodio::mixer::mixer(rodio::nz!(2), rodio::nz!(44_100));
        let alive = Arc::new(());
        let running = Arc::downgrade(&alive);
        std::thread::spawn(move || {
            while running.upgrade().is_some() {
                for _ in 0..1024 {
                    source.next();
                }
                std::thread::sleep(Duration::from_micros(200));
            }
        });
        (Player::connect_new(&mixer), alive)
    }

    #[test]
    fn a_new_device_carries_on_with_the_track_from_where_it_was() {
        let dir = tempfile::tempdir().unwrap();
        let path = wav(dir.path());
        let (bus_tx, bus_rx) = mpsc::channel();
        let (_tx, mut worker) = inprocess::join_worker::<AudioCommand>(bus_tx);
        let mut engine = engine();
        let (player, _device_1) = live_player();
        engine.reconnect(player, &mut worker);

        engine.do_play(path.clone(), &mut worker);
        engine.do_seek(4_000, 1);
        let (player, _device_2) = live_player();
        engine.reconnect(player, &mut worker);

        assert_eq!(engine.current.as_deref(), Some(path.as_path()));
        assert!(engine.time_keeper.is_playing);
        let at = engine.time_keeper.get_position().as_millis();
        assert!((4_000..4_500).contains(&at), "resumed at {at} ms");
        assert!(!engine.sink.empty(), "nothing queued on the new device");
        // The track carrying on is not a new track starting.
        let started = bus_rx
            .try_iter()
            .filter(|e| matches!(e, IpcEvent::PlaybackStarted { .. }))
            .count();
        assert_eq!(started, 1);
    }

    #[test]
    fn a_paused_track_stays_paused_on_the_new_device() {
        let dir = tempfile::tempdir().unwrap();
        let path = wav(dir.path());
        let (bus_tx, _bus_rx) = mpsc::channel();
        let (_tx, mut worker) = inprocess::join_worker::<AudioCommand>(bus_tx);
        let mut engine = engine();
        let (player, _device_3) = live_player();
        engine.reconnect(player, &mut worker);

        engine.do_play(path, &mut worker);
        engine.do_pause(&mut worker);
        let (player, _device_4) = live_player();
        engine.reconnect(player, &mut worker);

        assert!(!engine.time_keeper.is_playing);
        assert!(engine.sink.is_paused());
    }

    #[test]
    fn with_nothing_playing_a_new_device_is_just_taken_up() {
        let (bus_tx, _bus_rx) = mpsc::channel();
        let (_tx, mut worker) = inprocess::join_worker::<AudioCommand>(bus_tx);
        let mut engine = engine();
        let (player, _device_5) = live_player();
        engine.reconnect(player, &mut worker);
        assert!(engine.current.is_none());
        assert!(engine.sink.empty());
    }

    #[test]
    fn a_seek_with_no_device_running_does_not_wait_for_one() {
        // rodio waits for the device to answer a seek; with none open, or one
        // just lost, that wait was for ever. The position is kept instead, for
        // the next device to pick up.
        let dir = tempfile::tempdir().unwrap();
        let path = wav(dir.path());
        let (bus_tx, _bus_rx) = mpsc::channel();
        let (_tx, mut worker) = inprocess::join_worker::<AudioCommand>(bus_tx);
        let mut engine = engine();
        engine.do_play(path.clone(), &mut worker);
        engine.do_seek(3_000, 2);
        assert!((3_000..3_200).contains(&engine.time_keeper.get_position().as_millis()));

        let (player, _device_6) = live_player();

        engine.reconnect(player, &mut worker);
        let at = engine.time_keeper.get_position().as_millis();
        assert!((3_000..3_500).contains(&at), "picked up at {at} ms");
    }

    #[test]
    fn a_file_that_cannot_be_played_is_named_in_the_error() {
        let (bus_tx, bus_rx) = mpsc::channel();
        let (_tx, mut worker) = inprocess::join_worker::<AudioCommand>(bus_tx);
        let mut engine = engine();
        engine.do_play(PathBuf::from("/nonexistent/song.flac"), &mut worker);
        match bus_rx.try_recv() {
            Ok(IpcEvent::Error { message }) => {
                assert!(message.starts_with("Cannot play song.flac:"), "{message}")
            }
            other => panic!("{other:?}"),
        }
        assert!(engine.current.is_none());
    }
}
