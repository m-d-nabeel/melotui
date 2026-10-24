pub mod bridge;

use std::{path::PathBuf, sync::mpsc, thread};

use melotui_bus::{
    channel::inprocess,
    command::{AudioCommand, CoverCommand, DatabaseCommand, DownloadCommand, IpcCommand},
    event::IpcEvent,
    spectrum_bridge::SpectrumProducer,
};

/// Configuration for creating a new Engine.
pub struct EngineConfig {
    pub db_path: PathBuf,
    pub spectrum_producer: SpectrumProducer,
    pub fft_size: usize,
    pub max_cover_resolution: u32,
    /// Output sample rate in Hz; `None` uses the device default.
    pub sample_rate: Option<u32>,
    /// Output buffer size in frames; `None` uses cpal's default.
    pub buffer_size: Option<u32>,
    /// The output device's name, or part of it; `None` follows the system
    /// default.
    pub output_device: Option<String>,
}

/// Frontend-agnostic engine that owns all worker threads and the event bus.
///
/// Any frontend (TUI, GUI, daemon) creates an Engine, dispatches commands to it,
/// and polls events from it. The Engine handles all worker lifecycle internally.
pub struct Engine {
    /// Receive events from all workers
    pub event_rx: mpsc::Receiver<IpcEvent>,

    // typed command senders — one per worker
    download_tx: mpsc::Sender<DownloadCommand>,
    audio_tx: mpsc::Sender<AudioCommand>,
    db_tx: mpsc::Sender<DatabaseCommand>,
    cover_tx: mpsc::Sender<CoverCommand>,
}

impl Engine {
    /// Spawn all worker threads and return a connected Engine.
    pub fn new(config: EngineConfig) -> Self {
        let (ui_rx, bus_tx) = inprocess::hub();

        // spawn download worker
        let (dl_tx, dl_worker) = inprocess::join_worker::<DownloadCommand>(bus_tx.clone());
        thread::spawn(move || {
            melotui_download::run(dl_worker);
            tracing::info!("Download worker exited");
        });

        // spawn audio worker
        let (audio_tx, audio_worker) = inprocess::join_worker::<AudioCommand>(bus_tx.clone());
        let spectrum_producer = config.spectrum_producer;
        let audio_config = melotui_audio::AudioRunConfig {
            fft_size: config.fft_size,
            sample_rate: config.sample_rate,
            buffer_size: config.buffer_size,
            output_device: config.output_device,
        };
        thread::spawn(move || {
            melotui_audio::run(audio_worker, spectrum_producer, audio_config);
            tracing::info!("Audio worker exited");
        });

        // spawn database worker
        let (db_tx, db_worker) = inprocess::join_worker::<DatabaseCommand>(bus_tx.clone());
        let db_path = config.db_path;
        thread::spawn(move || {
            melotui_db::run(db_worker, db_path);
            tracing::info!("Database worker exited");
        });

        // spawn cover worker
        let (cover_tx, cover_worker) = inprocess::join_worker::<CoverCommand>(bus_tx.clone());
        let max_cover_resolution = config.max_cover_resolution;
        thread::spawn(move || {
            melotui_cover::run(cover_worker, max_cover_resolution);
            tracing::info!("Cover worker exited");
        });

        Self {
            event_rx: ui_rx,
            download_tx: dl_tx,
            audio_tx,
            db_tx,
            cover_tx,
        }
    }

    /// Route a command to the appropriate worker.
    pub fn dispatch(&self, cmd: IpcCommand) {
        match cmd {
            IpcCommand::Download(c) => {
                let _ = self.download_tx.send(c);
            }
            IpcCommand::Audio(c) => {
                let _ = self.audio_tx.send(c);
            }
            IpcCommand::Database(c) => {
                tracing::info!("Dispatching database command: {:?}", c);
                let _ = self.db_tx.send(c);
            }
            IpcCommand::Cover(c) => {
                let _ = self.cover_tx.send(c);
            }
            IpcCommand::Shutdown => {
                tracing::info!("Engine shutting down all workers...");
                let _ = self.download_tx.send(DownloadCommand::Shutdown);
                let _ = self.audio_tx.send(AudioCommand::Shutdown);
                let _ = self.db_tx.send(DatabaseCommand::Shutdown);
                let _ = self.cover_tx.send(CoverCommand::Shutdown);
            }
        }
    }

    // === Typed convenience methods ===

    /// Start playback of a file.
    pub fn play(&self, path: PathBuf) {
        self.dispatch(IpcCommand::Audio(AudioCommand::Play { path }));
    }

    /// Pause playback.
    pub fn pause(&self) {
        self.dispatch(IpcCommand::Audio(AudioCommand::Pause));
    }

    /// Resume playback.
    pub fn resume(&self) {
        self.dispatch(IpcCommand::Audio(AudioCommand::Resume));
    }

    /// Stop playback.
    pub fn stop(&self) {
        self.dispatch(IpcCommand::Audio(AudioCommand::Stop));
    }

    /// Seek to a position.
    pub fn seek(&self, position_ms: u64, epoch: u64) {
        self.dispatch(IpcCommand::Audio(AudioCommand::Seek { position_ms, epoch }));
    }

    /// Set volume (0.0 - 1.0+).
    pub fn set_volume(&self, volume: f32) {
        self.dispatch(IpcCommand::Audio(AudioCommand::SetVolume(volume)));
    }

    /// Load the library from the database.
    pub fn load_library(&self, roots: Vec<PathBuf>) {
        self.dispatch(IpcCommand::Database(DatabaseCommand::LoadLibrary { roots }));
    }

    /// Scan the filesystem and update the database.
    pub fn scan_library(&self, roots: Vec<PathBuf>) {
        self.dispatch(IpcCommand::Database(DatabaseCommand::ScanLibrary { roots }));
    }

    /// Shut down all workers gracefully.
    pub fn shutdown(&self) {
        self.dispatch(IpcCommand::Shutdown);
    }

    /// Sync all audio DSP controls to match the given AudioState.
    pub fn sync_dsp(&self, audio: &melotui_core::state::AudioState) {
        self.set_volume(audio.volume);
        self.dispatch(IpcCommand::Audio(AudioCommand::SetSpeed(audio.speed)));
        self.dispatch(IpcCommand::Audio(AudioCommand::SetBass(audio.bass)));
        self.dispatch(IpcCommand::Audio(AudioCommand::SetTreble(audio.treble)));
        self.dispatch(IpcCommand::Audio(AudioCommand::SetReverb(audio.reverb)));
        self.dispatch(IpcCommand::Audio(AudioCommand::SetEcho(audio.echo)));
        self.dispatch(IpcCommand::Audio(AudioCommand::SetPan(audio.pan)));
        self.dispatch(IpcCommand::Audio(AudioCommand::SetLowpass(audio.lowpass)));
        self.dispatch(IpcCommand::Audio(AudioCommand::SetHighpass(audio.highpass)));
        self.dispatch(IpcCommand::Audio(AudioCommand::SetAutoPanRate(
            audio.auto_pan_rate,
        )));
        self.dispatch(IpcCommand::Audio(AudioCommand::SetAutoPanDepth(
            audio.auto_pan_depth,
        )));
        for (i, &gain) in audio.eq_bands.iter().enumerate() {
            self.dispatch(IpcCommand::Audio(AudioCommand::SetEqBand { band: i, gain }));
        }
        self.dispatch(IpcCommand::Audio(AudioCommand::SetExciter(audio.exciter)));
        self.dispatch(IpcCommand::Audio(AudioCommand::SetBitcrush(audio.bitcrush)));
        self.dispatch(IpcCommand::Audio(AudioCommand::SetWidth(audio.width)));
        self.dispatch(IpcCommand::Audio(AudioCommand::SetKaraoke(audio.karaoke)));
        self.dispatch(IpcCommand::Audio(AudioCommand::SetWobble(audio.wobble)));
        self.dispatch(IpcCommand::Audio(AudioCommand::SetPingPong(
            audio.ping_pong,
        )));
        self.dispatch(IpcCommand::Audio(AudioCommand::SetPhaser(audio.phaser)));
    }
}
