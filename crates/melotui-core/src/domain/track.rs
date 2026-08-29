use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::domain::ids::TrackId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackMeta {
    pub id: TrackId,
    pub file_path: PathBuf,

    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub year: Option<u32>,
    pub is_favorite: bool,
    pub track_number: Option<u32>,

    pub duration_ms: u64,
    pub play_count: u32,
    /// Unix seconds of the most recent play, if any. The one field
    /// [`crate::library::snapshot::Library::record_play`] is responsible for —
    /// see that method's docs for why this and `play_count` are not
    /// separately-cached lists.
    pub last_played_at: Option<u64>,
    pub bitrate: Option<u32>,
    pub sample_rate: Option<u32>,
    pub format: String,
    pub has_cover: bool,
    pub file_size_bytes: i64,
}
