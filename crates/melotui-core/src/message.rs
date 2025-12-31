use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum AppMsg {
    // ───── Lifecycle ─────
    RequestQuit,
    Noop,

    // ───── Navigation ─────
    SwitchToPlaylists,
    SwitchToTracks,
    MoveUp,
    MoveDown,

    // ───── Library ─────
    RequestRescanLibrary,

    // ───── Playback (future IPC) ─────
    RequestPlaySelected, // intent
    DispatchPlay {
        path: PathBuf,
    }, // side-effect boundary (future)

    // ───── Command bar ─────
    EnterCommandMode,
    ExitCommandMode,
    UpdateCommand(char),
    BackspaceCommand,
    SubmitCommand(String),

    // ───── Downloads ─────
    /// User intent (pure)
    /// Example: user types `dl queen`
    RequestDownload {
        query: String,
    },

    /// Internal scheduler signal (pure)
    /// Fired when the queue should advance
    BeginNextDownload,

    /// Side-effect boundary (execution trigger)
    /// IPC listens ONLY for this
    DispatchDownload {
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
    RequestCancelDownload,
}
