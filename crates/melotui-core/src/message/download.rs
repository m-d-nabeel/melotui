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
    },
    RequestFetchLyrics {
        track_name: String,
        artist_name: String,
        save_path: PathBuf,
        provider: Option<String>,
        provider_id: Option<String>,
    },
    DispatchFetchLyrics {
        track_name: String,
        artist_name: String,
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
        error: String,
    },
    RequestCancelDownload,
    RequestSearchLyrics {
        query: String,
    },
    DispatchSearchLyrics {
        query: String,
    },
    LyricsSearchResults {
        lrclib_results: Vec<crate::state::lyrics::LyricsCandidate>,
        ovh_results: Vec<crate::state::lyrics::LyricsCandidate>,
    },
}
