use crate::command::SearchField;
use crate::domain::{PlaylistId, TrackId};

#[derive(Debug, Clone)]
pub enum LibraryMsg {
    AddToQueue(TrackId),
    PlayNext(TrackId),
    ToggleRepeatMode,
    ToggleShuffle,
    ToggleSort,
    RequestToggleFavorite(TrackId),
    RemoveFromQueue,
    ClearQueue,
    Search {
        field: SearchField,
        query: String,
    },
    SearchNext,
    SearchPrev,
    ClearSearch,
    RequestScanLibrary,
    /// A scan nobody asked for — at start, or after the music folders
    /// changed — that says nothing unless it finds something.
    RequestBackgroundScan,
    RequestCreatePlaylist(String),
    RequestDeletePlaylist(String),
    RequestAddToPlaylist {
        playlist_id: PlaylistId,
        track_id: TrackId,
    },
    RequestRemoveFromPlaylist {
        playlist_id: PlaylistId,
        track_id: TrackId,
    },
    PlaylistCreated {
        id: PlaylistId,
        name: String,
    },
    UpdateScanProgress {
        scanned: u32,
        skipped: u32,
        total: u32,
    },
    SetSearchResults {
        query: String,
        results: Vec<TrackId>,
    },
}
