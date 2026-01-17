use crate::{library::Track, state::QueueNavState};
use std::collections::VecDeque;

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
    pub current: Option<Track>,
    pub playing_from_queue: bool,
    pub source_name: String,       // "Playlist: Jazz", "All Tracks"
    pub context_queue: Vec<Track>, // persistent playlist
    pub context_idx: usize,
    pub status: PlaybackStatus,
    pub progress_ms: u64,
    pub duration_ms: Option<u64>,
    pub shuffle: bool,
}

#[derive(Debug, Clone, Default)]
pub struct PlaybackQueueState {
    pub items: VecDeque<Track>,
}

impl PlaybackQueueState {
    pub fn get_selected_track(&self, nav: &QueueNavState) -> Option<&Track> {
        self.items.get(nav.selected_idx)
    }

    pub fn get_track_by_id(&self, id: usize) -> Option<&Track> {
        self.items.iter().find(|t| t.id == id)
    }
}
