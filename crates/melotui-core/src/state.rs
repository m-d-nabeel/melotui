use std::{
    path::PathBuf,
    sync::{Arc, atomic::AtomicBool},
    time::Instant,
};

use crate::library::Library;

// ----------- UI Mode & State -------------
#[derive(Debug, Clone, PartialEq)]
pub enum NavMode {
    Playlists,
    Tracks,
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
}
// ----------- UI Mode & State -------------
// --------------- Playback ----------------
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackId {
    pub playlist: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone)]
pub enum PlaybackStatus {
    Playing,
    Paused,
    Stopped,
}

#[derive(Debug, Clone)]
pub struct PlaybackState {
    pub current: Option<TrackId>,
    pub status: PlaybackStatus,
}
// --------------- Playback ----------------
// --------------- Download ----------------
#[derive(Debug, Clone)]
pub struct DownloadControl {
    pub cancel: Arc<AtomicBool>,
}

#[derive(Debug, Clone)]
pub enum DownloadStatus {
    Idle,
    Running {
        percent: u8,
        control: DownloadControl,
    },
    Cancelling,
    Failed {
        error: String,
    },
}

#[derive(Debug, Clone)]
pub struct DownloadState {
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

#[derive(Debug, Clone)]
pub struct AppState {
    pub app_name: String,
    // navigation
    pub nav: UINavState,
    // UI-only
    pub ui: UIState,
    pub library: Library,
    pub playback: PlaybackState,
    pub download: DownloadState,
    pub notification: NotificationState,
}
