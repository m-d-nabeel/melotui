use std::{collections::VecDeque, path::PathBuf, str::FromStr, time::Instant};

// ----------- UI Mode & State -------------
#[derive(Debug, Clone, PartialEq)]
pub enum NavMode {
    Playlists,
    Tracks,
    TrackInfo,
}

#[derive(Debug, Clone)]
pub struct UINavState {
    pub mode: NavMode,
    pub selected_playlist: usize,
    pub selected_tracks: Vec<usize>,
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
// ----------- UI Mode & State -------------
// --------------- Playback ----------------
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackId {
    pub playlist: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)] // Added PartialEq for easier comparisons
pub enum PlaybackStatus {
    Playing,
    Paused,
    Stopped,
}

#[derive(Debug, Clone)]
pub struct PlaybackState {
    pub current: Option<TrackId>,
    pub status: PlaybackStatus,
    // Added these for the progress bar:
    pub progress_ms: u64,
    pub duration_ms: Option<u64>,
}
// --------------- Playback ----------------
// ---------------- Audio ------------------
#[derive(Debug, Clone)]
pub struct AudioState {
    pub volume: f32, // 0.0 - 1.0
    pub speed: f32,  // 0.5 - 2.0
    pub bass: f32,   // 0.0 - 3.0
    pub treble: f32, // 0.0 - 3.0
    pub reverb: f32, // 0.0 - 1.0
    pub echo: f32,   // 0.0 - 1.0
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
    pub progress: Option<u8>, // 0–100
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
    Spectrum, // future visualizer
    AlbumArt, // The new ANSI art mode
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
