use lofty::{file::TaggedFileExt, probe::Probe};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::domain::{PlaylistId, PlaylistMeta, TrackId, TrackMeta};
use crate::navigation::ids::SidebarNodeId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibrarySnapshot {
    pub root: PathBuf,
    pub playlists: HashMap<PlaylistId, PlaylistMeta>,
    pub tracks: HashMap<TrackId, TrackMeta>,
    pub playlist_order: Vec<PlaylistId>,
}

impl LibrarySnapshot {
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

    pub fn resolve_tracks_for_node(&self, node: &SidebarNodeId) -> Vec<&TrackMeta> {
        match node {
            SidebarNodeId::Playlist(pid) => {
                if let Some(p) = self.playlists.get(pid) {
                    p.tracks
                        .iter()
                        .filter_map(|tid| self.tracks.get(tid))
                        .collect()
                } else {
                    vec![]
                }
            }
            _ => vec![],
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
