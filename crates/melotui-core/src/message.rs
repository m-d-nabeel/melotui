use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum AppMsg {
    Quit,
    Noop,

    // ───── Navigation ─────
    SwitchToPlaylists,
    SwitchToTracks,
    MoveUp,
    MoveDown,

    // ───── Library ─────
    RescanLibrary,

    // ───── Playback (future IPC) ─────
    PlaySelected,

    // ───── Command bar ─────
    EnterCommandMode,
    ExitCommandMode,
    UpdateCommand(char),
    BackspaceCommand,
    SubmitCommand(String),

    // ───── Downloads (UI + state machine) ─────
    /// User intent or derived intent (pure)
    /// Example: user types `dl queen`
    StartDownload {
        query: String,
    },

    /// Internal scheduler signal (pure, local)
    /// Fired when the queue should advance
    BeginNextDownload,

    /// Side-effect boundary (execution trigger)
    /// THIS is what IPC listens for
    SpawnDownload {
        query: String,
        download_path: PathBuf,
    },

    /// External facts (from IPC)
    DownloadProgress {
        percent: u8,
    },
    DownloadFinished,
    DownloadFailed {
        error: String,
    },

    /// UI intent
    CancelDownload,
}
