use melotui_core::{
    domain::{PlaylistId, TrackId},
    library::Library,
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
    DownloadItemFinished,
    DownloadFinished,
    DownloadFailed {
        error: String,
    },
    LyricsFetched {
        save_path: std::path::PathBuf,
    },
    LyricsFetchFailed {
        error: String,
    },
    LyricsSearchResults {
        lrclib_results: Vec<melotui_core::state::lyrics::LyricsCandidate>,
        ovh_results: Vec<melotui_core::state::lyrics::LyricsCandidate>,
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

    // audio output
    /// An output device was opened, at start or after the last one went.
    AudioOutputOpened {
        name: String,
    },
    /// The output devices there are.
    OutputDevices {
        names: Vec<String>,
    },

    // database
    LibraryLoaded {
        snapshot: Box<Library>,
    },
    LibraryRescanned,
    FavoriteToggled {
        track_id: TrackId,
        is_favorite: bool,
    },
    ScanProgress {
        scanned: u32,
        skipped: u32,
        total: u32,
    },
    SearchResults {
        query: String,
        results: Vec<TrackId>,
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
        cache_path: Option<std::path::PathBuf>,
    },
}
