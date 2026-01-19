use crate::domain::ids::{PlaylistId, TrackId};
use std::path::PathBuf;

// ─── UI CONCEPTS ─────────────────────────────────────────────────────────────
// These exist only to organize the Sidebar. They are not in the DB.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CategoryId {
    Folders,
    AllSongs,
    Artists,
    Albums,
    Playlists,
}

// ─── THE UNIVERSAL ADDRESS ───────────────────────────────────────────────────
// The Sidebar is a tree of mixed types. This Enum allows us to
// select *anything* using a single variable.

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum SidebarNodeId {
    #[default]
    Root,

    // Virtual Nodes
    Category(CategoryId),
    Folder(PathBuf),

    // Data Nodes
    Playlist(PlaylistId),
    Track {
        id: TrackId,
        parent: Box<SidebarNodeId>,
    },

    // Future expansion
    Artist(String),
}
