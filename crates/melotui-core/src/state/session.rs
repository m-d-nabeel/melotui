use super::{
    audio::AudioState,
    download::DownloadState,
    navigation::NavigationState,
    notification::NotificationState,
    playback::{PlaybackQueueState, PlaybackState, RepeatMode},
    ui::UIState,
    visualizer::{SpectrumStyle, VisualizerMode},
};

/// The Transient State: Lives only while the app is running.
/// Reset on restart.
#[derive(Debug, Clone, Default)]
pub struct SessionState {
    pub nav: NavigationState,
    pub ui: UIState,
    pub playback: PlaybackState,
    pub playback_queue: PlaybackQueueState,
    pub audio: AudioState,
    pub download: DownloadState,
    pub notification: NotificationState,

    // Preferences that might become persistent later
    pub repeat_mode: RepeatMode,
    pub visualizer_mode: VisualizerMode,
    pub spectrum_style: SpectrumStyle,
}
