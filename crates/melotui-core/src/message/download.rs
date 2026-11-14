use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum DownloadMsg {
    RequestDownload {
        query: String,
    },
    BeginNextDownload,
    DispatchDownload {
        query: String,
        download_path: PathBuf,
        /// The browser whose cookies yt-dlp is to use, if one is configured.
        cookies_from_browser: Option<String>,
    },
    RequestFetchLyrics {
        track_name: String,
        artist_name: String,
        /// Narrow the match to the song's album and length when known.
        album_name: Option<String>,
        duration_secs: Option<u64>,
        save_path: PathBuf,
        provider: Option<String>,
        provider_id: Option<String>,
    },
    DownloadProgress {
        percent: u8,
    },
    DownloadFinished,
    DownloadFailed {
        error: String,
    },
    LyricsFetched {
        save_path: PathBuf,
    },
    LyricsFetchFailed {
        save_path: PathBuf,
        error: String,
    },
    RequestCancelDownload,
    RequestSearchLyrics {
        query: String,
    },
    /// Asks the download worker what there is to download for `query`.
    DispatchSearchDownloads {
        id: u64,
        query: String,
    },
    /// Tells the download worker the search is no longer wanted.
    CancelSearchDownloads,
    DownloadSearchResult {
        id: u64,
        candidate: crate::state::DownloadCandidate,
    },
    /// The search `id` has nothing more to say; `error` is what went wrong
    /// if it found nothing because something did.
    DownloadSearchFinished {
        id: u64,
        error: Option<String>,
    },
    LyricsSearchResults {
        lrclib_results: Vec<crate::state::lyrics::LyricsCandidate>,
        ovh_results: Vec<crate::state::lyrics::LyricsCandidate>,
    },
}
