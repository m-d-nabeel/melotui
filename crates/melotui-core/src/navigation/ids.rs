use crate::domain::ids::{PlaylistId, TrackId};
use std::path::{Path, PathBuf};

// ─── UI CONCEPTS ─────────────────────────────────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CategoryId {
    Folders,
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

impl SidebarNodeId {
    /// Resolves the logical parent of this node for tree traversal.
    pub fn parent(&self, library_root: &Path) -> Option<Self> {
        match self {
            SidebarNodeId::Root => None,

            // Categories are children of Root
            SidebarNodeId::Category(_) => Some(SidebarNodeId::Root),

            // Playlists -> Playlist Category
            SidebarNodeId::Playlist(_) => Some(SidebarNodeId::Category(CategoryId::Playlists)),

            // Tracks -> Explicit Parent (Playlist or Folder)
            SidebarNodeId::Track { parent, .. } => Some(*parent.clone()),

            // Folders -> Parent Folder or Folder Category
            SidebarNodeId::Folder(path) => {
                if path == library_root {
                    Some(SidebarNodeId::Category(CategoryId::Folders))
                } else {
                    match path.parent() {
                        Some(p) => {
                            if p == library_root {
                                // Direct child of root -> Folders Category
                                Some(SidebarNodeId::Category(CategoryId::Folders))
                            } else if p.starts_with(library_root) {
                                // Subfolder -> Parent Folder
                                Some(SidebarNodeId::Folder(p.to_path_buf()))
                            } else {
                                None
                            }
                        }
                        None => None,
                    }
                }
            }

            // Default fallbacks
            _ => Some(SidebarNodeId::Root),
        }
    }
}
