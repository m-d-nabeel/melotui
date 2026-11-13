use crate::domain::{PlaylistId, TrackId};

#[derive(Debug, Clone)]
pub enum DatabaseMsg {
    /// Make the track a favorite, or not: what it is to be, not "the other
    /// of what it is", which a write that failed once would leave the wrong
    /// way round against the screen from then on.
    DispatchSetFavorite {
        id: TrackId,
        favorite: bool,
    },
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
    DispatchIncrementPlayCount(TrackId),
    DispatchSearchTracks(String),
}
