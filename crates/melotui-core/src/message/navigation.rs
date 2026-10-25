#[derive(Debug, Clone)]
pub enum NavigationMsg {
    SwitchToPlaylists,
    SwitchToTracks,
    ToggleTrackInfo,
    ToggleLibraryQueue,
    ToggleMixerFocus,
    LeaveMixer,
    MoveUp,
    MoveDown,
    /// To the first row of the tree or the queue, and to the last.
    MoveTop,
    MoveBottom,
    /// A screenful down the tree or the queue, or up.
    MovePage {
        down: bool,
    },
    ToggleArt,
    ToggleTreeExpand,
    CycleVisualizerStyle,
}
