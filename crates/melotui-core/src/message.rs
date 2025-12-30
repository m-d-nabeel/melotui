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
}
