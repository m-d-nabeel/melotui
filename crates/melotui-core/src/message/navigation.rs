#[derive(Debug, Clone)]
pub enum NavigationMsg {
    SwitchToPlaylists,
    SwitchToTracks,
    ToggleTrackInfo,
    ToggleLibraryQueue,
    MoveUp,
    MoveDown,
    /// To the first row of the tree or the queue, and to the last.
    MoveTop,
    MoveBottom,
    ToggleArt,
    ToggleTreeExpand,
    CycleVisualizerStyle,
}
