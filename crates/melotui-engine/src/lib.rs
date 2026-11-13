pub mod bridge;

use std::{
    path::PathBuf,
    sync::mpsc,
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

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
    /// Called when a worker has sent an event, so that whoever reads
    /// `event_rx` need not look for them on a timer.
    pub wake: Option<inprocess::Waker>,
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

    /// The worker threads, by name, for waiting on at shutdown.
    workers: Vec<(&'static str, JoinHandle<()>)>,
}

/// Starts a worker's thread under `name`.
fn spawn(
    name: &'static str,
    run: impl FnOnce() + Send + 'static,
) -> (&'static str, JoinHandle<()>) {
    let handle = thread::Builder::new()
        .name(name.into())
        .spawn(move || {
            run();
            tracing::info!("{name} worker exited");
        })
        .expect("spawn a worker thread");
    (name, handle)
}

impl Engine {
    /// Spawn all worker threads and return a connected Engine.
    pub fn new(config: EngineConfig) -> Self {
        let (ui_rx, bus_tx) = inprocess::hub();
        let wake = config.wake;
        // Each worker's end of the bus, waking the reader if it asked to be.
        fn end<C>(
            bus: &mpsc::Sender<IpcEvent>,
            wake: &Option<inprocess::Waker>,
        ) -> (mpsc::Sender<C>, inprocess::WorkerEndpoint<C>) {
            let (tx, worker) = inprocess::join_worker::<C>(bus.clone());
            match wake {
                Some(wake) => (tx, worker.waking(wake.clone())),
                None => (tx, worker),
            }
        }

        // spawn download worker
        let (dl_tx, dl_worker) = end::<DownloadCommand>(&bus_tx, &wake);
        let download = spawn("download", move || melotui_download::run(dl_worker));

        // spawn audio worker
        let (audio_tx, audio_worker) = end::<AudioCommand>(&bus_tx, &wake);
        let spectrum_producer = config.spectrum_producer;
        let audio_config = melotui_audio::AudioRunConfig {
            fft_size: config.fft_size,
            sample_rate: config.sample_rate,
            buffer_size: config.buffer_size,
            output_device: config.output_device,
        };
        let audio = spawn("audio", move || {
            melotui_audio::run(audio_worker, spectrum_producer, audio_config)
        });

        // spawn database worker
        let (db_tx, db_worker) = end::<DatabaseCommand>(&bus_tx, &wake);
        let db_path = config.db_path;
        let database = spawn("database", move || melotui_db::run(db_worker, db_path));

        // spawn cover worker
        let (cover_tx, cover_worker) = end::<CoverCommand>(&bus_tx, &wake);
        let max_cover_resolution = config.max_cover_resolution;
        let cover = spawn("cover", move || {
            melotui_cover::run(cover_worker, max_cover_resolution)
        });

        Self {
            event_rx: ui_rx,
            download_tx: dl_tx,
            audio_tx,
            db_tx,
            cover_tx,
            workers: vec![database, download, audio, cover],
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

    /// Tells every worker to stop. They finish what they were sent first;
    /// [`Engine::join`] waits for them to.
    pub fn shutdown(&self) {
        self.dispatch(IpcCommand::Shutdown);
    }

    /// Waits up to `timeout` for the workers to finish after
    /// [`Engine::shutdown`], and names those that have not.
    ///
    /// The process ending under them cut the database worker off before the
    /// last writes sent to it — a favorite, a play count — and before SQLite
    /// was closed, and left a download's yt-dlp running with nothing to
    /// report to. A worker that is still busy when the time is up — a long
    /// scan, a cover being decoded — is left behind rather than waited on
    /// for ever: quitting has to end.
    pub fn join(&mut self, timeout: Duration) -> Vec<&'static str> {
        let deadline = Instant::now() + timeout;
        let mut running = std::mem::take(&mut self.workers);
        loop {
            let (finished, still): (Vec<_>, Vec<_>) =
                running.into_iter().partition(|(_, h)| h.is_finished());
            for (name, handle) in finished {
                if handle.join().is_err() {
                    tracing::error!("The {name} worker had panicked");
                }
            }
            running = still;
            if running.is_empty() || Instant::now() >= deadline {
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }
        let names: Vec<&'static str> = running.iter().map(|(name, _)| *name).collect();
        if !names.is_empty() {
            tracing::warn!("Quitting without waiting further for: {}", names.join(", "));
        }
        names
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

#[cfg(test)]
mod tests {
    use super::*;
    use melotui_bus::spectrum_bridge;

    fn engine(db_path: PathBuf) -> Engine {
        let (spectrum_producer, _consumer) = spectrum_bridge::create_bridge();
        Engine::new(EngineConfig {
            db_path,
            spectrum_producer,
            fft_size: 2048,
            max_cover_resolution: 64,
            sample_rate: None,
            buffer_size: None,
            output_device: None,
            wake: None,
        })
    }

    #[test]
    fn quitting_waits_for_what_the_workers_were_sent() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("library.db");
        let mut engine = engine(db.clone());

        // Sent just before quitting, as a favorite toggled and then `q`.
        engine.dispatch(IpcCommand::Database(DatabaseCommand::CreatePlaylist {
            name: "Last thing".into(),
        }));
        engine.shutdown();
        let left = engine.join(Duration::from_secs(10));
        assert!(left.is_empty(), "still running: {left:?}");

        let created = engine
            .event_rx
            .try_iter()
            .any(|e| matches!(e, IpcEvent::PlaylistCreated { name, .. } if name == "Last thing"));
        assert!(created, "the last command was dropped");
        // SQLite closed: its write-ahead log is folded back in and removed.
        assert!(db.exists());
        assert!(
            !dir.path().join("library.db-wal").exists(),
            "the database was not closed"
        );
    }

    #[test]
    fn a_worker_that_is_still_busy_is_named_not_waited_on_for_ever() {
        let dir = tempfile::tempdir().unwrap();
        let mut engine = engine(dir.path().join("library.db"));
        // No shutdown sent: every worker is still waiting for commands.
        let started = Instant::now();
        let left = engine.join(Duration::from_millis(50));
        assert_eq!(left.len(), 4, "{left:?}");
        assert!(started.elapsed() < Duration::from_secs(2));
        engine.shutdown();
    }
}
