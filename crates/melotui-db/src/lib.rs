use melotui_core::{
    domain::{PlaylistId, PlaylistMeta, TrackId, TrackMeta},
    library::LibrarySnapshot,
};
use rusqlite::Connection;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

pub mod worker;
pub use crate::worker::run;

pub fn init_db(path: &Path) -> anyhow::Result<Connection> {
    let conn = Connection::open(path)?;
    let schema_sql = include_str!("queries/schema.sql");
    conn.execute_batch(schema_sql)?;
    Ok(conn)
}

#[tracing::instrument]
pub fn load_library_from_db(
    db_path: &Path,
    music_root: PathBuf,
) -> anyhow::Result<LibrarySnapshot> {
    let conn = Connection::open(db_path)?;

    // Fetch all tracks from SQLite
    let query_start = std::time::Instant::now();
    let mut stmt = conn.prepare(
        "SELECT 
            id, file_path, title, artist, album, genre, year, 
            track_number, bitrate, sample_rate, format, 
            duration_ms, has_cover 
         FROM tracks 
         ORDER BY file_path ASC",
    )?;

    // Map DB rows to domain types
    let track_iter = stmt.query_map([], |row| {
        let id_raw: i64 = row.get(0)?;
        let id = TrackId(id_raw as u64);

        Ok((
            id,
            TrackMeta {
                id,
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
            },
        ))
    })?;

    // Build track map
    let mut tracks_map: HashMap<TrackId, TrackMeta> = HashMap::new();
    let mut all_track_ids = Vec::new();

    for t in track_iter {
        let (id, track) = t?;
        tracks_map.insert(id, track);
        all_track_ids.push(id);
    }

    let query_elapsed = query_start.elapsed();
    tracing::info!(
        "Loaded {} tracks from database in {:?}",
        tracks_map.len(),
        query_elapsed
    );

    // Generate folder-based playlists
    // In real implementation, also fetch playlists from DB
    let mut playlist_map: HashMap<PlaylistId, PlaylistMeta> = HashMap::new();
    let mut playlist_order: Vec<PlaylistId> = Vec::new();
    let mut folder_groups: HashMap<String, Vec<TrackId>> = HashMap::new();

    // Group by folder
    for (id, track) in &tracks_map {
        let parent = track.file_path.parent().unwrap_or(&track.file_path);
        let folder_name = if parent == music_root {
            "Root".to_string()
        } else {
            parent
                .strip_prefix(&music_root)
                .unwrap_or(parent)
                .to_string_lossy()
                .to_string()
        };
        folder_groups.entry(folder_name).or_default().push(*id);
    }

    // Create "All Tracks" (ID 0)
    let all_tracks_id = PlaylistId(0);
    playlist_map.insert(
        all_tracks_id,
        PlaylistMeta {
            id: all_tracks_id,
            name: "All Tracks".to_string(),
            tracks: all_track_ids,
        },
    );
    playlist_order.push(all_tracks_id);

    // Create playlists from folders (ID 1..N)
    // Sort folder names for deterministic IDs (transient IDs are risky)
    let mut sorted_folders: Vec<_> = folder_groups.into_iter().collect();
    sorted_folders.sort_by(|a, b| a.0.cmp(&b.0));

    for (i, (name, p_tracks)) in sorted_folders.into_iter().enumerate() {
        // ID 0 taken, start at 1
        let pid = PlaylistId((i + 1) as u64);

        playlist_map.insert(
            pid,
            PlaylistMeta {
                id: pid,
                name,
                tracks: p_tracks, // Just IDs, cheap to clone
            },
        );
        playlist_order.push(pid);
    }

    // Return snapshot
    Ok(LibrarySnapshot::new(
        music_root,
        // We pass the map values as a Vec because `LibrarySnapshot::new`
        // currently takes a Vec and rebuilds the map (based on your previous code).
        // It's efficient enough for startup.
        playlist_map.into_values().collect(),
        tracks_map,
    ))
}
