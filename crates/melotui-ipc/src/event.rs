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
    PlaybackStopped,
    PlaybackProgress { position_ms: u64 },

    // ───── Errors ─────
    Error { message: String },
}
