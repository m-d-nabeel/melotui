use super::{
    audio::AudioState,
    download::DownloadState,
    navigation::NavigationState,
    notification::NotificationState,
    playback::{PlaybackQueueState, PlaybackState, RepeatMode},
    ui::UIState,
    visualizer::{SpectrumStyle, VisualizerMode},
};

/// The SessionState groups all "Volatile" state.
/// This is the state that changes rapidly based on user input.
#[derive(Debug, Clone)]
pub struct SessionState {
    pub nav: NavigationState,
    pub ui: UIState,
    pub playback: PlaybackState,
    pub playback_queue: PlaybackQueueState,
    pub audio: AudioState,
    pub download: DownloadState,
    pub notification: NotificationState,

    // View/Preference modes (Transient for now)
    pub repeat_mode: RepeatMode,
    pub visualizer_mode: VisualizerMode,
    pub spectrum_style: SpectrumStyle,
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            nav: NavigationState::default(),
            ui: UIState::default(),
            playback: PlaybackState::default(),
            playback_queue: PlaybackQueueState::default(),
            audio: AudioState::default(),
            download: DownloadState::default(),
            notification: NotificationState::default(),
            repeat_mode: RepeatMode::Off,
            visualizer_mode: VisualizerMode::AlbumArt,
            spectrum_style: SpectrumStyle::Default,
        }
    }
}
