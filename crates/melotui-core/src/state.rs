use std::path::PathBuf;

use crate::library::Library;

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

#[derive(Debug, Clone)]
pub struct AppState {
    pub app_name: String,
    pub status: String,

    // navigation
    pub nav: UINavState,

    pub library: Library,

    pub playback: PlaybackState,
}
