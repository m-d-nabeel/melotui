#[derive(Debug, Clone)]
pub enum NavigationMsg {
    SwitchToPlaylists,
    SwitchToTracks,
    SwitchToTrackInfo,
    ToggleLibraryQueue,
    MoveUp,
    MoveDown,
    ToggleArt,
    ToggleTreeExpand,
}
