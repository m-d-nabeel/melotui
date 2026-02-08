use crate::domain::ids::{PlaylistId, TrackId};
use std::path::{Path, PathBuf};

// UI concepts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CategoryId {
    Folders,
    Artists,
    Albums,
    Genres,
    Years,
    Playlists,
    Favorites,
}

// Universal node selector for the sidebar tree

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum SidebarNodeId {
    #[default]
    Root,

    // Virtual nodes
    Category(CategoryId),
    Folder(PathBuf),

    // Data nodes
    Playlist(PlaylistId),
    Track {
        id: TrackId,
        parent: Box<SidebarNodeId>,
    },

    // Metadata groupings
    Artist(String),
    Album(String),
    Genre(String),
    Year(u32),
}

impl SidebarNodeId {
    /// Resolves the parent of this node for tree traversal.
    pub fn parent(&self, library_root: &Path) -> Option<Self> {
        match self {
            SidebarNodeId::Root => None,

            // Categories are root children
            SidebarNodeId::Category(_) => Some(SidebarNodeId::Root),

            // Playlists -> playlist category
            SidebarNodeId::Playlist(_) => Some(SidebarNodeId::Category(CategoryId::Playlists)),

            // Tracks -> explicit parent (playlist or folder)
            SidebarNodeId::Track { parent, .. } => Some(*parent.clone()),

            // Folders -> parent folder or folder category
            SidebarNodeId::Folder(path) => {
                if path == library_root {
                    Some(SidebarNodeId::Category(CategoryId::Folders))
                } else {
                    match path.parent() {
                        Some(p) => {
                            if p == library_root {
                                // Direct child of root -> folders category
                                Some(SidebarNodeId::Category(CategoryId::Folders))
                            } else if p.starts_with(library_root) {
                                // Subfolder -> parent folder
                                Some(SidebarNodeId::Folder(p.to_path_buf()))
                            } else {
                                None
                            }
                        }
                        None => None,
                    }
                }
            }

            // Metadata Groupings -> Respective Categories
            SidebarNodeId::Artist(_) => Some(SidebarNodeId::Category(CategoryId::Artists)),
            SidebarNodeId::Album(_) => Some(SidebarNodeId::Category(CategoryId::Albums)),
            SidebarNodeId::Genre(_) => Some(SidebarNodeId::Category(CategoryId::Genres)),
            SidebarNodeId::Year(_) => Some(SidebarNodeId::Category(CategoryId::Years)),
        }
    }
}
