#[derive(Debug, Clone)]
pub enum IpcEvent {
    // ───── Downloads ─────
    DownloadStarted {
        query: String,
    },
    DownloadProgress {
        percent: u8,
    },
    DownloadFinished,
    DownloadFailed {
        error: String,
    },

    // ───── Playback ─────
    PlaybackStarted {
        path: String,
        total_duration_ms: u64,
    },
    PlaybackPaused,
    PlaybackResumed,
    PlaybackStopped,
    PlaybackProgress {
        position_ms: u64,
    },
    PlaybackFinished,

    // ───── Errors ─────
    Error {
        message: String,
    },
}
