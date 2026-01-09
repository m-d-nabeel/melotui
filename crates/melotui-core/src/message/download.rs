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
    DownloadProgress {
        percent: u8,
    },
    DownloadFinished,
    DownloadFailed {
        error: String,
    },
    RequestCancelDownload,
}
