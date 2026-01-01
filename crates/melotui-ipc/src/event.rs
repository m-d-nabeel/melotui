#[derive(Debug, Clone)]
pub enum IpcEvent {
    // ───── Downloads ─────
    DownloadStarted { query: String },
    DownloadProgress { percent: u8 },
    DownloadFinished,
    DownloadFailed { error: String },

    // ───── Playback ─────
    PlaybackStarted { path: String },
    PlaybackPaused,
    PlaybackResumed,
    PlaybackStopped,
    PlaybackProgress { position_ms: u64 },
    SpectrumData { magnitude: Vec<f32> },
    PlaybackFinished,

    // ───── Errors ─────
    Error { message: String },
}
