use std::{collections::VecDeque, process::Command};

use crate::{
    Library, NavMode, UIMode, UINavState,
    state_registry::{
        DownloadState, DownloadStatus, NotificationState, PlaybackState, PlaybackStatus,
        SpectrumStyle, UIState, VisualizerMode,
    },
    visualizer_interface::{NoOpSpectrum, SpectrumSource},
};

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

    pub spectrum_source: Box<dyn SpectrumSource>,
    pub spectrum_peaks: Vec<f32>, // Falling peaks}
    pub spectrum_style: SpectrumStyle,
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
        let spectrum_style = match Command::new("cava").arg("--version").output() {
            Ok(_) => {
                tracing::info!("Cava installation detected. Using it for spectrum");
                SpectrumStyle::Cava
            }
            Err(_) => {
                tracing::info!("Cava installation not detected. Switching to manual mode.");
                tracing::info!("Install Cava for better spectrum visualization");
                SpectrumStyle::Default
            }
        };

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
            spectrum_raw: vec![0.0; 512],
            spectrum_smooth: vec![0.0; 512],
            spectrum_peaks: vec![0.0; 512],
            spectrum_style,
            spectrum_source: Box::new(NoOpSpectrum),
        }
    }
}
