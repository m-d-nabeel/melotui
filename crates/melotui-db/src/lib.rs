use melotui_core::{Library, Playlist, library::Track};
use rusqlite::Connection;
use std::path::{Path, PathBuf};

pub mod worker;
pub use crate::worker::run;

pub fn init_db(path: &Path) -> anyhow::Result<Connection> {
    let conn = Connection::open(path)?;
    let schema_sql = include_str!("queries/schema.sql");
    conn.execute_batch(schema_sql)?;
    Ok(conn)
}

/// NEW: Replaces the file system scan with a DB Query
#[tracing::instrument]
pub fn load_library_from_db(db_path: &Path, music_root: PathBuf) -> anyhow::Result<Library> {
    let conn = Connection::open(db_path)?;

    // 1. Fetch all tracks from SQLite
    let query_start = std::time::Instant::now();
    let mut stmt = conn.prepare(
        "SELECT 
            id, file_path, title, artist, album, genre, year, 
            track_number, bitrate, sample_rate, format, 
            duration_ms, has_cover 
         FROM tracks 
         ORDER BY file_path ASC",
    )?;

    let track_iter = stmt.query_map([], |row| {
        Ok(Track {
            id: row.get::<_, i64>(0)? as usize, // Cast DB i64 to Core usize
            file_path: PathBuf::from(row.get::<_, String>(1)?),
            title: row.get(2)?,
            artist: row.get(3)?,
            album: row.get(4)?,
            genre: row.get(5)?,
            year: row.get(6)?,
            track_number: row.get(7)?,
            bitrate: row.get(8)?,
            sample_rate: row.get(9)?,
            format: row.get(10)?,
            duration_ms: row.get::<_, i64>(11)? as u64,
            has_cover: row.get(12)?,
        })
    })?;

    // 2. Collect into a Vector
    let mut all_tracks = Vec::new();
    for track in track_iter {
        all_tracks.push(track?);
    }
    let query_elapsed = query_start.elapsed();
    tracing::info!(
        "Loaded {} tracks from database in {:?}",
        all_tracks.len(),
        query_elapsed
    );

    // 3. Construct the Library
    // Group tracks by their parent directory under music_root
    use std::collections::HashMap;
    let mut folder_tracks: HashMap<String, Vec<Track>> = HashMap::new();

    for track in all_tracks {
        let path = PathBuf::from(&track.file_path);
        let parent = path.parent().unwrap_or(path.as_path());
        let folder_name = if parent == music_root {
            "Root".to_string()
        } else {
            parent
                .strip_prefix(&music_root)
                .unwrap_or(parent)
                .to_string_lossy()
                .to_string()
        };
        folder_tracks.entry(folder_name).or_default().push(track);
    }

    let mut playlists: Vec<Playlist> = folder_tracks
        .into_iter()
        .map(|(name, tracks)| Playlist { name, tracks })
        .collect();

    // Sort playlists by name
    playlists.sort_by(|a, b| a.name.cmp(&b.name));

    // If no playlists, create empty "All Tracks"
    if playlists.is_empty() {
        playlists.push(Playlist {
            name: "All Tracks".to_string(),
            tracks: Vec::new(),
        });
    }

    Ok(Library {
        root: music_root,
        playlists,
    })
}
