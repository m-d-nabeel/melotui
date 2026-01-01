use std::collections::VecDeque;

use crate::{
    Library, NavMode, UIMode, UINavState,
    state_registry::{
        DownloadState, DownloadStatus, NotificationState, PlaybackState, PlaybackStatus, UIState,
        VisualizerMode,
    },
};

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
    pub visualizer_mode: VisualizerMode,
    pub current_album_art: Option<ratatui::text::Text<'static>>,
    pub spectrum_raw: Vec<f32>,    // Incoming from worker
    pub spectrum_smooth: Vec<f32>, // Rendered (smoothed)
    pub spectrum_peaks: Vec<f32>,  // Falling peaks}
}

impl AppState {
    pub fn new(library: Library) -> Self {
        let playlist_count = library.playlists.len();

        let nav = UINavState {
            mode: NavMode::Playlists,
            selected_playlist: 0,
            selected_tracks: vec![0; playlist_count],
        };

        let playback = PlaybackState {
            current: None,
            status: PlaybackStatus::Stopped,
            progress_ms: 0,
            duration_ms: None,
        };
        let ui = UIState {
            mode: UIMode::Normal,
            command_buffer: String::new(),
        };
        let download = DownloadState {
            status: DownloadStatus::Idle,
            queue: VecDeque::new(),
        };
        let notification = NotificationState { current: None };

        Self {
            app_name: "Melotui".into(),
            nav,
            library,
            playback,
            ui,
            download,
            notification,
            visualizer_mode: VisualizerMode::AlbumArt,
            current_album_art: None,
            spectrum_raw: vec![0.0; 256],
            spectrum_smooth: vec![0.0; 256],
            spectrum_peaks: vec![0.0; 256],
        }
    }
}
