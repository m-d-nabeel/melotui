use std::{collections::VecDeque, process::Command};

use crate::{
    Library,
    state::{
        AudioState, DownloadState, DownloadStatus, LeftPanelMode, LibraryNavState,
        LibraryViewDepth, NavigationState, NotificationState, PlaybackQueueState, PlaybackState,
        PlaybackStatus, QueueNavState, RepeatMode, SpectrumStyle, UIMode, UIState, VisualizerMode,
    },
    utils::{NoOpSpectrum, SpectrumSource},
};

pub struct AppState {
    pub app_name: String,
    pub time_elapsed: f32,
    pub last_nav_time: f32,

    pub nav: NavigationState,
    pub ui: UIState,
    pub library: Library,
    pub playback: PlaybackState,
    pub playback_queue: PlaybackQueueState,
    pub repeat_mode: RepeatMode,
    pub audio: AudioState,
    pub download: DownloadState,
    pub notification: NotificationState,
    pub visualizer_mode: VisualizerMode,
    pub current_album_art: Option<ratatui::text::Text<'static>>,
    pub track_info_art: Option<ratatui::text::Text<'static>>,
    pub spectrum_raw: Vec<f32>,
    pub spectrum_smooth: Vec<f32>,

    pub spectrum_source: Box<dyn SpectrumSource>,
    pub spectrum_peaks: Vec<f32>,
    pub spectrum_style: SpectrumStyle,
}

impl AppState {
    pub fn new(library: Library) -> Self {
        let playlist_count = library.playlists.len();

        let nav = NavigationState {
            active_panel: LeftPanelMode::Library,
            library: LibraryNavState {
                depth: LibraryViewDepth::Playlists,
                selected_playlist_idx: 0,
                selected_tracks_curs: vec![0; playlist_count],
            },
            queue: QueueNavState { selected_idx: 0 },
        };
        let playback = PlaybackState {
            current: None,
            status: PlaybackStatus::Stopped,
            progress_ms: 0,
            duration_ms: None,
            shuffle: false,
        };
        let ui = UIState {
            mode: UIMode::Normal,
            command_buffer: String::new(),
            show_help: false,
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
        let playback_queue = PlaybackQueueState {
            tracks: VecDeque::new(),
        };

        Self {
            app_name: "Melotui".into(),
            time_elapsed: 0.0,
            last_nav_time: 0.0,
            nav,
            library,
            playback,
            playback_queue,
            repeat_mode: RepeatMode::Off,
            audio: AudioState::default(),
            ui,
            download,
            notification,
            visualizer_mode: VisualizerMode::AlbumArt,
            current_album_art: None,
            track_info_art: None,
            spectrum_raw: vec![0.0; 512],
            spectrum_smooth: vec![0.0; 512],
            spectrum_peaks: vec![0.0; 512],
            spectrum_style,
            spectrum_source: Box::new(NoOpSpectrum),
        }
    }
}
