use crate::{
    library::{Track, TrackId},
    state::QueueNavState,
};
use std::{collections::VecDeque, path::PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlaybackContext {
    Library { playlist_name: String },
    Queue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveTrack {
    pub id: TrackId,
    pub path: PathBuf,
    pub context: PlaybackContext,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum PlaybackStatus {
    Playing,
    Paused,
    #[default]
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum RepeatMode {
    #[default]
    Off,
    PlayOnce,
    Track,
    Playlist,
}

impl RepeatMode {
    pub fn next(&self) -> Self {
        match self {
            RepeatMode::Off => RepeatMode::PlayOnce,
            RepeatMode::PlayOnce => RepeatMode::Track,
            RepeatMode::Track => RepeatMode::Playlist,
            RepeatMode::Playlist => RepeatMode::Off,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct PlaybackState {
    pub current: Option<ActiveTrack>,
    pub status: PlaybackStatus,
    pub progress_ms: u64,
    pub duration_ms: Option<u64>,
    pub shuffle: bool,
}

#[derive(Debug, Clone, Default)]
pub struct PlaybackQueueState {
    pub tracks: VecDeque<Track>,
}

impl PlaybackQueueState {
    pub fn get_selected_track(&self, nav: &QueueNavState) -> Option<&Track> {
        self.tracks.get(nav.selected_idx)
    }

    pub fn get_track_by_id(&self, id: usize) -> Option<&Track> {
        self.tracks.iter().find(|t| t.id == id)
    }
}
