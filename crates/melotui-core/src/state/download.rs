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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadSource {
    /// YouTube Music's songs: the recording itself, with its artist and album.
    Songs,
    /// Everything else on YouTube: live sets, covers and uploads.
    YouTube,
}

/// One thing a search for something to download found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadCandidate {
    pub source: DownloadSource,
    pub url: String,
    pub title: String,
    /// The artist of a song; whose channel anything else is on.
    pub by: Option<String>,
    pub album: Option<String>,
    pub duration_secs: Option<u64>,
}

/// What the list of results takes the place of, to be put back when it
/// closes: the lyrics panel as it was — shown, closed by hand, opened for
/// a song's lyrics — and the layout, where it was one without that panel.
#[derive(Debug, Clone, Copy, Default)]
pub struct PutAside {
    pub show_lyrics: bool,
    pub panel_closed: bool,
    pub panel_opened_for_lyrics: bool,
    pub show_dsp_studio: bool,
    pub focus_mode: crate::state::ui::FocusMode,
}

/// A search for something to download, and the list its results are
/// picked from: the songs, then the rest of YouTube. They come one at a
/// time, and can be picked as they come.
#[derive(Debug, Clone, Default)]
pub struct DownloadSearch {
    /// Which search this is, so that what a search since closed still has
    /// to say is not taken for this one's.
    pub id: u64,
    pub query: String,
    pub songs: Vec<DownloadCandidate>,
    pub youtube: Vec<DownloadCandidate>,
    /// The result selected, counting through the songs and then the rest.
    pub row: usize,
    /// More results may still come.
    pub searching: bool,
    /// A row was chosen by hand, and the selection stays on that result.
    pub moved: bool,
    /// The screen as it was before the list took the lyrics panel's place.
    pub before: PutAside,
}

impl DownloadSearch {
    pub fn results(&self, source: DownloadSource) -> &[DownloadCandidate] {
        match source {
            DownloadSource::Songs => &self.songs,
            DownloadSource::YouTube => &self.youtube,
        }
    }

    pub fn len(&self) -> usize {
        self.songs.len() + self.youtube.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn selected(&self) -> Option<&DownloadCandidate> {
        self.songs.iter().chain(&self.youtube).nth(self.row)
    }
}
