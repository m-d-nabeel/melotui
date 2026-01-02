use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum AppMsg {
    // ───── Lifecycle ─────
    RequestQuit,
    Noop,

    // ───── Navigation ─────
    SwitchToPlaylists,
    SwitchToTracks,
    SwitchToTrackInfo,
    MoveUp,
    MoveDown,
    ToggleArt,

    // ───── Library ─────
    RequestRescanLibrary,

    // ───── Playback (future IPC) ─────
    /// User intent (pure)
    /// Triggered when user presses Enter/Space on a track
    RequestPlaySelected,

    /// User intent (pure)
    /// Triggered when user presses Spacebar (global)
    RequestTogglePlayback,

    /// User intent (pure)
    RequestStop,

    /// User intent (pure)
    /// e.g., "Seek forward 5s"
    RequestSeek {
        offset_seconds: i64,
    },

    /// Side-effect boundary (execution trigger)
    /// IPC listens for this to start a new stream
    DispatchPlay {
        path: PathBuf,
    },

    /// Side-effect boundary
    DispatchPause,

    /// Side-effect boundary
    DispatchResume,

    /// Side-effect boundary
    DispatchStop,

    /// Side-effect boundary
    DispatchSeek {
        position_ms: u64,
    },

    /// Side-effect boundary
    DispatchSetVisualizer {
        active: bool,
    },

    /// External facts (from IPC)
    /// Used to update the progress bar
    AudioProgress {
        position_ms: u64,
    },

    /// External facts
    /// Used to update UI state (icon changes) or auto-advance playlist
    AudioStarted {
        path: String,
    },
    AudioPaused,
    AudioResumed,
    AudioFinished,
    AudioError {
        error: String,
    },

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
