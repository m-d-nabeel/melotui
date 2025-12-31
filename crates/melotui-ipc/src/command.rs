#[derive(Debug, Clone)]
pub enum IpcCommand {
    // ───── Downloads ─────
    Download { query: String },
    CancelDownload,

    // ───── Playback (next step) ─────
    Play { path: String },
    Pause,
    Resume,
    Stop,
    Seek { position_ms: u64 },

    // ───── Lifecycle ─────
    Shutdown,
}
