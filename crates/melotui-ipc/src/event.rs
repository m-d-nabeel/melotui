use melotui_core::library::LibrarySnapshot;

#[derive(Debug, Clone)]
pub enum IpcEvent {
    // Downloads
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

    // Playback
    PlaybackStarted {
        path: String,
    },
    PlaybackPaused,
    PlaybackResumed,
    PlaybackStopped,
    PlaybackProgress {
        position_ms: u64,
    },
    PlaybackFinished,

    // Errors
    Error {
        message: String,
    },

    // Database
    LibraryLoaded {
        snapshot: Box<LibrarySnapshot>,
    },
    LibraryRescanned,
    FavoriteToggled {
        track_id: melotui_core::domain::TrackId,
        is_favorite: bool,
    },

    // Cover
    CoverLoaded {
        id: melotui_core::domain::TrackId,
        cover: Option<std::sync::Arc<image::DynamicImage>>,
        context: melotui_core::message::cover::CoverContext,
    },
}
