#[derive(Debug, Clone)]
pub enum AppMsg {
    Quit,

    // navigation
    SwitchToPlaylists,
    SwitchToTracks,
    MoveUp,
    MoveDown,

    // library
    RescanLibrary,

    // future
    PlaySelected,

    // command bar
    EnterCommandMode,
    ExitCommandMode,
    UpdateCommand(char),
    BackspaceCommand,
    SubmitCommand(String),

    // downloads
    StartDownload { query: String },
    BeginNextDownload,
    SpawnDownload { query: String },
    DownloadProgress { message: String, percent: u8 },
    DownloadFinished,
    DownloadFailed { error: String },
    CancelDownload,
}
