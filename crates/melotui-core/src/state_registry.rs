use std::{collections::VecDeque, path::PathBuf, str::FromStr, time::Instant};

use crate::library::Track;

// ----------- UI Mode & State -------------

/// Defines how "deep" we are in the Library view
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LibraryViewDepth {
    Playlists, // Column 0: List of Playlists
    Tracks,    // Column 1: List of Tracks
    TrackInfo, // Column 2: Track Details
}

/// State specific to the Library Panel
#[derive(Debug, Clone)]
pub struct LibraryNavState {
    pub depth: LibraryViewDepth,
    pub selected_playlist_idx: usize,
    /// Stores the selected track index for *every* playlist.
    pub selected_tracks_curs: Vec<usize>,
}

/// State specific to the Queue Panel
#[derive(Debug, Clone)]
pub struct QueueNavState {
    pub selected_idx: usize,
}

/// High-level switch for the Sidebar
#[derive(Debug, Clone, PartialEq)]
pub enum LeftPanelMode {
    Library,
    Queue,
}

/// The Master Navigation State
#[derive(Debug, Clone)]
pub struct NavigationState {
    /// Which panel is currently visible/active?
    pub active_panel: LeftPanelMode,

    /// State for the Library (persists even when viewing Queue)
    pub library: LibraryNavState,

    /// State for the Queue (persists even when viewing Library)
    pub queue: QueueNavState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UIMode {
    Normal,
    Command,
}

#[derive(Debug, Clone)]
pub struct UIState {
    pub mode: UIMode,
    pub command_buffer: String,
    pub show_help: bool,
}

// --------------- Playback ----------------
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackId {
    pub playlist: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlaybackStatus {
    Playing,
    Paused,
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepeatMode {
    Off,      // Linear: Play Next -> Next -> Stop at End (Default)
    PlayOnce, // Play One -> Stop
    Track,    // Loop One -> Loop
    Playlist, // Play Next -> Loop to Start at End
}

impl RepeatMode {
    pub fn next(&self) -> Self {
        match self {
            RepeatMode::Off => RepeatMode::PlayOnce,
            RepeatMode::PlayOnce => RepeatMode::Track,
            RepeatMode::Track => RepeatMode::Playlist,
            RepeatMode::Playlist => RepeatMode::Off,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlaybackState {
    pub current: Option<TrackId>,
    pub status: PlaybackStatus,
    pub progress_ms: u64,
    pub duration_ms: Option<u64>,
    pub shuffle: bool,
}

#[derive(Debug, Clone)]
pub struct PlaybackQueueState {
    pub tracks: VecDeque<Track>,
}
// --------------- Playback ----------------

// ---------------- Audio ------------------
#[derive(Debug, Clone)]
pub struct AudioState {
    pub volume: f32, // 0.0 - 1.5
    pub speed: f32,  // 0.5 - 2.0
    pub bass: f32,   // 0.0 - 5.0
    pub treble: f32, // 0.0 - 5.0
    pub reverb: f32, // 0.0 - 1.0
    pub echo: f32,   // 0.0 - 0.9
}

impl Default for AudioState {
    fn default() -> Self {
        Self {
            volume: 1.0,
            speed: 1.0,
            bass: 1.0,
            treble: 1.0,
            reverb: 0.0,
            echo: 0.0,
        }
    }
}
// ---------------- Audio ------------------

// --------------- Download ----------------
#[derive(Debug, Clone)]
pub struct DownloadJob {
    pub query: String,
    pub download_path: PathBuf,
}

#[derive(Debug, Clone)]
pub enum DownloadStatus {
    Idle,
    Running { percent: u8 },
    Cancelling,
    Failed { error: String },
}

#[derive(Debug, Clone)]
pub struct DownloadState {
    pub queue: VecDeque<DownloadJob>,
    pub status: DownloadStatus,
}
// --------------- Download ----------------

// ------------- Notification --------------
#[derive(Debug, Clone)]
pub enum NotificationKind {
    Info,
    Progress,
    Error,
}

#[derive(Debug, Clone)]
pub struct Notification {
    pub kind: NotificationKind,
    pub message: String,
    pub progress: Option<u8>,
    pub created_at: Instant,
    pub dismiss_after: Option<std::time::Duration>,
}

#[derive(Debug, Clone)]
pub struct NotificationState {
    pub current: Option<Notification>,
}
// ------------- Notification --------------

// -------------- Visualizer  --------------
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VisualizerMode {
    Spectrum,
    AlbumArt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpectrumStyle {
    Cava,
    Default,
    Bar,
    Particle,
}

impl SpectrumStyle {
    pub fn as_str(&self) -> &'static str {
        match self {
            SpectrumStyle::Cava => "Cava",
            SpectrumStyle::Default => "Default",
            SpectrumStyle::Bar => "Bar",
            SpectrumStyle::Particle => "Paraticle",
        }
    }
}

impl FromStr for SpectrumStyle {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "cava" => Ok(SpectrumStyle::Cava),
            "default" | "def" => Ok(SpectrumStyle::Default),
            "bar" => Ok(SpectrumStyle::Bar),
            "particle" | "part" | "p" => Ok(SpectrumStyle::Particle),
            _ => Err(()),
        }
    }
}
// -------------- Visualizer  --------------
