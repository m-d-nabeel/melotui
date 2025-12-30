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

    // command execution
    ExecuteCommand(String),

    // downloads
    StartDownload { query: String },
    DownloadProgress { message: String },
    DownloadFinished,
    DownloadFailed { error: String },
}
