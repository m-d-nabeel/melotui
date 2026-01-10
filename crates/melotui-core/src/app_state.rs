use crate::{
    Library,
    state::{
        AudioState, DownloadState, NavigationState, NotificationState, PlaybackQueueState,
        PlaybackState, RepeatMode, SpectrumStyle, UIState, VisualizerMode,
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

        Self {
            app_name: "Melotui".into(),
            time_elapsed: 0.0,
            last_nav_time: 0.0,
            nav: NavigationState::new(playlist_count),
            library,
            playback: PlaybackState::default(),
            playback_queue: PlaybackQueueState::default(),
            repeat_mode: RepeatMode::Off,
            audio: AudioState::default(),
            ui: UIState::default(),
            download: DownloadState::default(),
            notification: NotificationState::default(),
            visualizer_mode: VisualizerMode::AlbumArt,
            current_album_art: None,
            track_info_art: None,
            spectrum_raw: vec![0.0; 512],
            spectrum_smooth: vec![0.0; 512],
            spectrum_peaks: vec![0.0; 512],
            spectrum_style: SpectrumStyle::default(),
            spectrum_source: Box::new(NoOpSpectrum),
        }
    }
}
