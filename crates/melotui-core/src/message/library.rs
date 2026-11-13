use crate::command::SearchField;
use crate::domain::{PlaylistId, TrackId};

#[derive(Debug, Clone)]
pub enum LibraryMsg {
    /// The library has been read from the database. With it, the first time,
    /// comes the session saved at the last quit, if there is one and it is
    /// to be restored.
    Loaded {
        snapshot: Box<crate::library::Library>,
        session: Option<Box<crate::state::PersistedSession>>,
    },
    /// A scan still under way has more to show: the library as it stands.
    Grew(Box<crate::library::Library>),
    /// A scan finished; `changed` if it added, updated or removed tracks.
    Rescanned {
        changed: bool,
    },
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
}
