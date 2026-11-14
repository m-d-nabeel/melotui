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
    /// The search whose results are being picked from, if one is open.
    pub search: Option<DownloadSearch>,
    /// How many searches there have been.
    pub searches: u64,
}

/// Where a download on offer was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DownloadSource {
    /// YouTube Music's songs: the recording itself, with its artist and album.
    #[default]
    Songs,
    /// YouTube's videos: everything else, live sets and uploads among them.
    Videos,
}

impl DownloadSource {
    pub fn other(self) -> Self {
        match self {
            Self::Songs => Self::Videos,
            Self::Videos => Self::Songs,
        }
    }
}

/// One thing a search for something to download found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadCandidate {
    pub source: DownloadSource,
    pub url: String,
    pub title: String,
    /// The artist of a song, the channel of a video.
    pub by: Option<String>,
    pub album: Option<String>,
    pub duration_secs: Option<u64>,
}

/// A search for something to download, and the list its results are
/// picked from. They come one at a time, and can be picked as they come.
#[derive(Debug, Clone, Default)]
pub struct DownloadSearch {
    /// Which search this is, so that what a search since closed still has
    /// to say is not taken for this one's.
    pub id: u64,
    pub query: String,
    pub songs: Vec<DownloadCandidate>,
    pub videos: Vec<DownloadCandidate>,
    pub tab: DownloadSource,
    pub row: usize,
    /// More results may still come.
    pub searching: bool,
    /// The tab or row was chosen by hand, and is left where it was put.
    pub moved: bool,
}

impl DownloadSearch {
    pub fn results(&self, source: DownloadSource) -> &[DownloadCandidate] {
        match source {
            DownloadSource::Songs => &self.songs,
            DownloadSource::Videos => &self.videos,
        }
    }

    pub fn shown(&self) -> &[DownloadCandidate] {
        self.results(self.tab)
    }

    pub fn selected(&self) -> Option<&DownloadCandidate> {
        self.shown().get(self.row)
    }
}
