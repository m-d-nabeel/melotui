use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::Accessor; // Needed for year/genre/track
use serde::{Deserialize, Serialize};

// Supported audio extensions to scan
const SUPPORTED_EXTENSIONS: &[&str] = &["mp3", "flac", "m4a", "wav", "ogg", "opus"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Track {
    pub file_path: PathBuf,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    // --- Extra Metadata ---
    pub genre: Option<String>,
    pub year: Option<u32>,
    pub track_number: Option<u32>,
    pub bitrate: Option<u32>,     // kbps
    pub sample_rate: Option<u32>, // Hz
    pub format: String,           // e.g. "MP3"
    // -----------------------
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
    pub fn scan(root: PathBuf) -> Result<Self> {
        if !root.exists() {
            fs::create_dir_all(&root)?;
        }

        let mut playlists = Vec::new();
        let mut untitled_tracks = Vec::new();

        for entry in fs::read_dir(&root)?.flatten() {
            let path = entry.path();

            if path.is_file() {
                if let Some(track) = process_file(&path) {
                    untitled_tracks.push(track);
                }
                continue;
            }

            if path.is_dir() {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "Unknown".to_string());

                let mut tracks = Vec::new();

                if let Ok(entries) = fs::read_dir(&path) {
                    for sub_entry in entries.flatten() {
                        let sub_path = sub_entry.path();
                        if sub_path.is_file()
                            && let Some(track) = process_file(&sub_path)
                        {
                            tracks.push(track);
                        }
                    }
                }

                if !tracks.is_empty() {
                    tracks.sort_by(|a, b| a.file_path.cmp(&b.file_path));
                    playlists.push(Playlist { name, path, tracks });
                }
            }
        }

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

    pub fn get_album_art(path: &Path) -> Option<(Vec<u8>, String)> {
        let tagged_file = Probe::open(path).ok()?.read().ok()?;
        let tag = tagged_file
            .primary_tag()
            .or_else(|| tagged_file.first_tag())?;
        let picture = tag.pictures().first()?;
        Some((picture.data().to_vec(), picture.mime_type()?.to_string()))
    }
}

fn process_file(path: &Path) -> Option<Track> {
    let ext = path.extension()?.to_string_lossy().to_lowercase();
    if !SUPPORTED_EXTENSIONS.contains(&ext.as_str()) {
        return None;
    }

    // 1. Probe & Read Properties
    let tagged_file = Probe::open(path).ok()?.read().ok()?;
    let properties = tagged_file.properties();

    let duration_ms = properties.duration().as_millis() as u64;
    let bitrate = properties.audio_bitrate();
    let sample_rate = properties.sample_rate();
    let format = ext.to_uppercase();

    // 2. Read Tags
    let tag = tagged_file
        .primary_tag()
        .or_else(|| tagged_file.first_tag());

    let title = tag
        .and_then(|t| t.title().map(|s| s.to_string()))
        .unwrap_or_else(|| {
            path.file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "Unknown Track".to_string())
        });

    let artist = tag.and_then(|t| t.artist().map(|s| s.to_string()));
    let album = tag.and_then(|t| t.album().map(|s| s.to_string()));
    let genre = tag.and_then(|t| t.genre().map(|s| s.to_string()));
    let year = tag.and_then(|t| t.year());
    let track_number = tag.and_then(|t| t.track());

    let has_cover = tag.map(|t| !t.pictures().is_empty()).unwrap_or(false);

    Some(Track {
        file_path: path.to_path_buf(),
        title,
        artist,
        album,
        genre,
        year,
        track_number,
        bitrate,
        sample_rate,
        format,
        duration_ms,
        has_cover,
    })
}
