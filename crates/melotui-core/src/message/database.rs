use crate::domain::{PlaylistId, TrackId};

#[derive(Debug, Clone)]
pub enum DatabaseMsg {
    DispatchToggleFavorite(TrackId),
    DispatchCreatePlaylist(String),
    DispatchDeletePlaylist(PlaylistId),
    DispatchAddToPlaylist {
        playlist_id: PlaylistId,
        track_id: TrackId,
    },
    DispatchRemoveFromPlaylist {
        playlist_id: PlaylistId,
        track_id: TrackId,
    },
}
