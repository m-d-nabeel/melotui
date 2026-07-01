#[derive(Debug, Clone)]
pub enum NavigationMsg {
    SwitchToPlaylists,
    SwitchToTracks,
    ToggleTrackInfo,
    ToggleLibraryQueue,
    MoveUp,
    MoveDown,
    ToggleArt,
    ToggleTreeExpand,
    CycleVisualizerStyle,
}
