use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::Accessor;
use serde::{Deserialize, Serialize};

// Supported audio extensions to scan
const SUPPORTED_EXTENSIONS: &[&str] = &["mp3", "flac", "m4a", "wav", "ogg", "opus"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Track {
    pub file_path: PathBuf,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration_ms: u64,
    pub has_cover: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Playlist {
    pub name: String,
    pub path: PathBuf,
    pub tracks: Vec<Track>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Library {
    pub root: PathBuf,
    pub playlists: Vec<Playlist>,
}

impl Library {
    /// Scans the directory for audio files and builds the library
    pub fn scan(root: PathBuf) -> Result<Self> {
        if !root.exists() {
            fs::create_dir_all(&root)?;
        }

        let mut playlists = Vec::new();
        let mut untitled_tracks = Vec::new();

        // 1. Scan the Root Directory
        // Use flatten() to silently skip permission errors or read failures
        for entry in fs::read_dir(&root)?.flatten() {
            let path = entry.path();

            // A. Handle Files in Root (Untitled Playlist)
            if path.is_file() {
                if let Some(track) = process_file(&path) {
                    untitled_tracks.push(track);
                }
                continue;
            }

            // B. Handle Subdirectories (Named Playlists)
            if path.is_dir() {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "Unknown".to_string());

                let mut tracks = Vec::new();

                if let Ok(entries) = fs::read_dir(&path) {
                    for sub_entry in entries.flatten() {
                        let sub_path = sub_entry.path();
                        if sub_path.is_file() {
                            if let Some(track) = process_file(&sub_path) {
                                tracks.push(track);
                            }
                        }
                    }
                }

                if !tracks.is_empty() {
                    // Optional: Sort tracks by filename for consistency
                    tracks.sort_by(|a, b| a.file_path.cmp(&b.file_path));
                    playlists.push(Playlist { name, path, tracks });
                }
            }
        }

        // Add "Untitled" playlist if any root tracks were found
        if !untitled_tracks.is_empty() {
            untitled_tracks.sort_by(|a, b| a.file_path.cmp(&b.file_path));
            playlists.insert(
                0,
                Playlist {
                    name: "Untitled".into(),
                    path: root.clone(),
                    tracks: untitled_tracks,
                },
            );
        }

        Ok(Self { root, playlists })
    }

    /// On-demand album art extraction.
    /// Returns the raw bytes (e.g., JPEG/PNG data) and the mime type (e.g., "image/jpeg").
    /// This avoids loading all images into RAM during the initial scan.
    pub fn get_album_art(path: &Path) -> Option<(Vec<u8>, String)> {
        // 1. Probe the file again
        let tagged_file = Probe::open(path).ok()?.read().ok()?;

        // 2. Get the primary tag (ID3, Vorbis, etc.)
        let tag = tagged_file
            .primary_tag()
            .or_else(|| tagged_file.first_tag())?;

        // 3. Find the first picture
        let picture = tag.pictures().first()?;

        Some((picture.data().to_vec(), picture.mime_type()?.to_string()))
    }
}

/// Helper function to extract metadata safely using Lofty
fn process_file(path: &Path) -> Option<Track> {
    // 1. Check Extension
    let ext = path.extension()?.to_string_lossy().to_lowercase();
    if !SUPPORTED_EXTENSIONS.contains(&ext.as_str()) {
        return None;
    }

    // 2. Probe the file
    // We strictly handle errors; if it fails to probe, we skip it (returns None).
    let tagged_file = Probe::open(path).ok()?.read().ok()?;

    // 3. Extract Duration
    let duration_ms = tagged_file.properties().duration().as_millis() as u64;

    // 4. Extract Tags
    let tag = tagged_file
        .primary_tag()
        .or_else(|| tagged_file.first_tag());

    let title = tag
        .and_then(|t| t.title().map(|s| s.to_string()))
        .unwrap_or_else(|| {
            // Fallback: Use filename if no title tag exists
            path.file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "Unknown Track".to_string())
        });

    let artist = tag.and_then(|t| t.artist().map(|s| s.to_string()));
    let album = tag.and_then(|t| t.album().map(|s| s.to_string()));

    // Check if any pictures exist (boolean flag only)
    let has_cover = tag.map(|t| !t.pictures().is_empty()).unwrap_or(false);

    Some(Track {
        file_path: path.to_path_buf(),
        title,
        artist,
        album,
        duration_ms,
        has_cover,
    })
}
