use crate::{
    navigation::state::NavigationState,
    playback::state::{PlaybackQueueState, PlaybackState, RepeatMode},
};

use super::{
    audio::AudioState,
    download::DownloadState,
    lyrics::LyricsState,
    notification::NotificationState,
    ui::UIState,
    visualizer::{SpectrumStyle, VisualizerMode},
};

#[derive(Debug, Clone)]
pub struct SessionState {
    pub nav: NavigationState,
    pub ui: UIState,

    pub playback: PlaybackState,
    pub playback_queue: PlaybackQueueState,

    pub audio: AudioState,
    pub download: DownloadState,
    pub lyrics: LyricsState,
    pub notification: NotificationState,

    pub repeat_mode: RepeatMode,
    pub visualizer_mode: VisualizerMode,
    pub spectrum_style: SpectrumStyle,

    pub seek_epoch: u64,

    pub last_play_time: Option<std::time::Instant>,
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
            lyrics: LyricsState::default(),
            notification: NotificationState::default(),

            repeat_mode: RepeatMode::Off,
            visualizer_mode: VisualizerMode::AlbumArt,
            spectrum_style: SpectrumStyle::default(),
            seek_epoch: 0,
            last_play_time: None,
        }
    }
}
