use crate::state::LibraryNavState;
use lofty::file::TaggedFileExt;
use lofty::probe::Probe;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub type TrackId = usize;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Track {
    pub id: TrackId,
    pub file_path: PathBuf,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub year: Option<u32>,
    pub track_number: Option<u32>,
    pub bitrate: Option<u32>,     // kbps
    pub sample_rate: Option<u32>, // Hz
    pub format: String,           // e.g. "MP3"
    pub duration_ms: u64,
    pub has_cover: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Playlist {
    pub name: String,
    pub tracks: Vec<Track>,
}

/// The Read-Only Snapshot of the Library.
/// This is constructed by `melotui-db` and passed to `AppState`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Library {
    pub root: PathBuf,
    pub playlists: Vec<Playlist>,
}

impl Library {
    /// Creates a new empty library wrapper
    pub fn new(root: PathBuf, playlists: Vec<Playlist>) -> Self {
        Self { root, playlists }
    }

    /// Helper to find a track by ID within the in-memory snapshot
    pub fn get_track(&self, id: TrackId) -> Option<&Track> {
        for playlist in &self.playlists {
            for track in &playlist.tracks {
                if track.id == id {
                    return Some(track);
                }
            }
        }
        None
    }

    /// In-memory search of the loaded snapshot
    pub fn search(&self, query: &str) -> Vec<&Track> {
        let q = query.to_lowercase();
        self.playlists
            .iter()
            .flat_map(|p| &p.tracks)
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

    pub fn get_selected_playlist(&self, nav: &LibraryNavState) -> Option<&Playlist> {
        self.playlists.get(nav.selected_playlist_idx)
    }

    pub fn get_selected_track(&self, nav: &LibraryNavState) -> Option<&Track> {
        let playlist = self.get_selected_playlist(nav)?;

        let track_idx = *nav
            .selected_tracks_curs
            .get(nav.selected_playlist_idx)
            .unwrap_or(&0);

        playlist.tracks.get(track_idx)
    }

    pub fn get_track_at(&self, playlist_idx: usize, track_idx: usize) -> Option<&Track> {
        self.playlists.get(playlist_idx)?.tracks.get(track_idx)
    }

    /// Utility: Extracts album art from file on disk.
    /// Kept here for UI convenience, but logically belongs in an Asset loader.
    pub fn get_album_art(path: &Path) -> Option<(Vec<u8>, String)> {
        let tagged_file = Probe::open(path).ok()?.read().ok()?;
        let tag = tagged_file
            .primary_tag()
            .or_else(|| tagged_file.first_tag())?;
        let picture = tag.pictures().first()?;
        Some((picture.data().to_vec(), picture.mime_type()?.to_string()))
    }
}
