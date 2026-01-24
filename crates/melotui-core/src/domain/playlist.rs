use serde::{Deserialize, Serialize};

use crate::domain::ids::{PlaylistId, TrackId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistMeta {
    pub id: PlaylistId,
    pub name: String,
    pub tracks: Vec<TrackId>,
}
