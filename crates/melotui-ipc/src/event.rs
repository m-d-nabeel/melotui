use melotui_core::{
    domain::{PlaylistId, TrackId},
    library::LibrarySnapshot,
    message::cover::CoverContext,
};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum IpcEvent {
    // downloads
    DownloadStarted {
        query: String,
    },
    DownloadProgress {
        percent: u8,
    },
    DownloadFinished,
    DownloadFailed {
        error: String,
    },

    // playback
    PlaybackStarted {
        path: String,
    },
    PlaybackPaused,
    PlaybackResumed,
    PlaybackStopped,
    PlaybackProgress {
        position_ms: u64,
        epoch: u64,
    },
    PlaybackFinished,

    // errors
    Error {
        message: String,
    },

    // database
    LibraryLoaded {
        snapshot: Box<LibrarySnapshot>,
    },
    LibraryRescanned,
    FavoriteToggled {
        track_id: TrackId,
        is_favorite: bool,
    },

    // playlists
    PlaylistCreated {
        id: PlaylistId,
        name: String,
    },
    PlaylistDeleted {
        id: PlaylistId,
    },
    PlaylistTrackAdded {
        playlist_id: PlaylistId,
        track_id: TrackId,
    },
    PlaylistTrackRemoved {
        playlist_id: PlaylistId,
        track_id: TrackId,
    },

    // cover
    CoverLoaded {
        id: TrackId,
        cover: Option<Arc<image::DynamicImage>>,
        context: CoverContext,
    },
}
