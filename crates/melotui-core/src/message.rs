#[derive(Debug)]
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
    SubmitCommand,
}
