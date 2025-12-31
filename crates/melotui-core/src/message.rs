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
    /// User intent or derived intent
    StartDownload {
        query: String,
    },

    /// Internal scheduler signal (pure, local)
    BeginNextDownload,

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
