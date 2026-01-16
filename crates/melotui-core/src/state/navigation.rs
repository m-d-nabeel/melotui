#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum LibraryViewDepth {
    #[default]
    Playlists, // Start at the playlist list
    Tracks,
    TrackInfo,
}

#[derive(Debug, Clone, Default)]
pub struct LibraryNavState {
    pub depth: LibraryViewDepth,
    pub selected_playlist_idx: usize,
    /// Stores the selected track index for *every* playlist.
    pub selected_tracks_curs: Vec<usize>,
}

#[derive(Debug, Clone, Default)]
pub struct QueueNavState {
    pub selected_idx: usize,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum LeftPanelMode {
    #[default]
    Library,
    Queue,
}

#[derive(Debug, Clone, Default)]
pub struct NavigationState {
    pub active_panel: LeftPanelMode,
    pub library: LibraryNavState,
    pub queue: QueueNavState,
}

impl NavigationState {
    pub fn new(playlist_count: usize) -> Self {
        NavigationState {
            active_panel: LeftPanelMode::Library,
            library: LibraryNavState {
                depth: LibraryViewDepth::Playlists,
                selected_playlist_idx: 0,
                selected_tracks_curs: vec![0; playlist_count],
            },
            queue: QueueNavState { selected_idx: 0 },
        }
    }
}
