use std::collections::VecDeque;

use crate::{domain::TrackId, navigation::ids::SidebarNodeId};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct QueueId(pub u64);

#[derive(Debug, Clone)]
pub struct QueueEntry {
    pub id: QueueId,
    pub track_id: TrackId,
}

#[derive(Debug, Clone, Default)]
pub struct PlaybackState {
    pub current: Option<TrackId>,
    pub active_node: Option<SidebarNodeId>,
    pub playing_from_queue: bool,
    pub active_queue_id: Option<QueueId>,
    pub source_name: String,         // e.g. "Playlist: Jazz"
    pub context_queue: Vec<TrackId>, // persistent playlist
    pub context_idx: usize,
    pub status: PlaybackStatus,
    pub progress_ms: u64,
    pub duration_ms: Option<u64>,
    pub shuffle: bool,
}

#[derive(Debug, Clone, Default)]
pub struct PlaybackQueueState {
    pub items: VecDeque<QueueEntry>,
    pub next_id_counter: u64,
}

impl PlaybackQueueState {
    pub fn add_track(&mut self, track_id: TrackId) {
        let id = QueueId(self.next_id_counter);
        self.next_id_counter += 1;
        self.items.push_back(QueueEntry { id, track_id });
    }

    pub fn add_track_next(&mut self, track_id: TrackId) {
        let id = QueueId(self.next_id_counter);
        self.next_id_counter += 1;
        self.items.push_front(QueueEntry { id, track_id });
    }
}
