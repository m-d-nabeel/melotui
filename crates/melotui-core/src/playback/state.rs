use std::collections::VecDeque;

use crate::{domain::TrackId, navigation::state::QueueNavState};

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
    pub current: Option<TrackId>,
    pub playing_from_queue: bool,
    pub source_name: String,         // "Playlist: Jazz", "All Tracks"
    pub context_queue: Vec<TrackId>, // persistent playlist
    pub context_idx: usize,
    pub status: PlaybackStatus,
    pub progress_ms: u64,
    pub duration_ms: Option<u64>,
    pub shuffle: bool,
}

#[derive(Debug, Clone, Default)]
pub struct PlaybackQueueState {
    pub items: VecDeque<TrackId>,
}

impl PlaybackQueueState {
    pub fn selected_track_id(&self, nav: &QueueNavState) -> Option<TrackId> {
        self.items.get(nav.selected_idx).copied()
    }

    pub fn contains(&self, id: TrackId) -> bool {
        self.items.iter().any(|t| *t == id)
    }
}
