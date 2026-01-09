use std::{collections::VecDeque, path::PathBuf};

#[derive(Debug, Clone)]
pub struct DownloadJob {
    pub query: String,
    pub download_path: PathBuf,
}

#[derive(Debug, Clone, Default)]
pub enum DownloadStatus {
    #[default]
    Idle,
    Running {
        percent: u8,
    },
    Cancelling,
    Failed {
        error: String,
    },
}

#[derive(Debug, Clone, Default)]
pub struct DownloadState {
    pub queue: VecDeque<DownloadJob>,
    pub status: DownloadStatus,
}
