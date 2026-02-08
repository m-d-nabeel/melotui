use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use lofty::{file::TaggedFileExt, probe::Probe};
use serde::{Deserialize, Serialize};

use crate::{
    domain::{PlaylistId, PlaylistMeta, TrackId, TrackMeta},
    navigation::ids::SidebarNodeId,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Library {
    pub root: PathBuf,
    pub playlists: HashMap<PlaylistId, PlaylistMeta>,
    pub tracks: HashMap<TrackId, TrackMeta>,
    pub playlist_order: Vec<PlaylistId>,
}

impl Library {
    pub fn new(
        root: PathBuf,
        playlists_vec: Vec<PlaylistMeta>,
        tracks: HashMap<TrackId, TrackMeta>,
    ) -> Self {
        let mut playlists = HashMap::new();
        let mut playlist_order = Vec::new();

        for p in playlists_vec {
            playlist_order.push(p.id);
            playlists.insert(p.id, p);
        }

        Self {
            root,
            playlists,
            playlist_order,
            tracks,
        }
    }

    pub fn get_track(&self, id: TrackId) -> Option<&TrackMeta> {
        self.tracks.get(&id)
    }

    pub fn get_playlist(&self, id: PlaylistId) -> Option<&PlaylistMeta> {
        self.playlists.get(&id)
    }

    pub fn get_tracks(&self, ids: &[TrackId]) -> Vec<&TrackMeta> {
        ids.iter().filter_map(|id| self.tracks.get(id)).collect()
    }

    pub fn search(&self, query: &str) -> Vec<&TrackMeta> {
        let q = query.to_lowercase();
        self.tracks
            .values()
            .filter(|t| {
                t.title.to_lowercase().contains(&q)
                    || t.artist
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&q)
            })
            .collect()
    }

    pub fn resolve_node_tracks(&self, node: &SidebarNodeId) -> Vec<&TrackMeta> {
        match node {
            SidebarNodeId::Playlist(pid) => {
                if let Some(p) = self.playlists.get(pid) {
                    self.get_tracks(&p.tracks)
                } else {
                    vec![]
                }
            }
            SidebarNodeId::Category(_) => vec![],
            SidebarNodeId::Folder(_) => vec![],
            SidebarNodeId::Track { .. } => vec![],
            SidebarNodeId::Artist(_) => vec![],
            SidebarNodeId::Album(_) => vec![],
            SidebarNodeId::Genre(_) => vec![],
            SidebarNodeId::Year(_) => vec![],
            SidebarNodeId::Root => vec![],
        }
    }

    pub fn get_album_art(path: &Path) -> Option<(Vec<u8>, String)> {
        let tagged_file = Probe::open(path).ok()?.read().ok()?;
        let tag = tagged_file
            .primary_tag()
            .or_else(|| tagged_file.first_tag())?;
        let picture = tag.pictures().first()?;
        Some((picture.data().to_vec(), picture.mime_type()?.to_string()))
    }
}
