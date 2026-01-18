use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::domain::{PlaylistId, PlaylistMeta, TrackId, TrackMeta};

use lofty::{file::TaggedFileExt, probe::Probe};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Library {
    pub root: PathBuf,
    pub playlists: Vec<PlaylistMeta>,
    tracks: HashMap<TrackId, TrackMeta>,
}

impl Library {
    pub fn new(
        root: PathBuf,
        playlists: Vec<PlaylistMeta>,
        tracks: HashMap<TrackId, TrackMeta>,
    ) -> Self {
        Self {
            root,
            playlists,
            tracks,
        }
    }

    pub fn get_track(&self, id: TrackId) -> Option<&TrackMeta> {
        self.tracks.get(&id)
    }

    pub fn get_tracks(&self, ids: &[TrackId]) -> Vec<&TrackMeta> {
        ids.iter().filter_map(|id| self.tracks.get(id)).collect()
    }

    pub fn all_tracks(&self) -> impl Iterator<Item = &TrackMeta> {
        self.tracks.values()
    }

    pub fn playlist(&self, idx: usize) -> Option<&PlaylistMeta> {
        self.playlists.get(idx)
    }

    pub fn playlist_by_id(&self, id: PlaylistId) -> Option<&PlaylistMeta> {
        self.playlists.iter().find(|p| p.id == id)
    }

    pub fn playlist_tracks(&self, idx: usize) -> Option<Vec<&TrackMeta>> {
        let playlist = self.playlists.get(idx)?;
        Some(self.get_tracks(&playlist.tracks))
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
                    || t.album.as_deref().unwrap_or("").to_lowercase().contains(&q)
            })
            .collect()
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
