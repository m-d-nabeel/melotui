use anyhow::Result;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::Accessor;
use melotui_core::{
    domain::{PlaylistId, PlaylistMeta, TrackId, TrackMeta},
    library::LibrarySnapshot,
};
use melotui_ipc::event::IpcEvent;
use melotui_ipc::{channel::IpcWorker, command::DatabaseCommand};
use rusqlite::params;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::info;

use crate::migrations::run_migrations;

const SUPPORTED_EXTENSIONS: &[&str] = &["mp3", "flac", "m4a", "wav", "ogg", "opus"];

pub fn run(worker: impl IpcWorker<DatabaseCommand>, db_path: PathBuf) {
    let conn = rusqlite::Connection::open(&db_path).expect("Failed to open DB");
    run_migrations(&conn).expect("Failed to run migrations");
    info!("Database worker initialized");

    loop {
        match worker.try_recv() {
            Some(DatabaseCommand::LoadLibrary { root }) => {
                info!("Loading library from database for root: {:?}", root);
                match load_library_snapshot(&conn, root) {
                    Ok(snapshot) => {
                        info!("Library loaded: {} tracks", snapshot.tracks.len());
                        worker.send(IpcEvent::LibraryLoaded {
                            snapshot: Box::new(snapshot),
                        });
                    }
                    Err(e) => {
                        tracing::error!("Failed to load library: {:?}", e);
                        worker.send(IpcEvent::Error {
                            message: e.to_string(),
                        });
                    }
                }
            }
            Some(DatabaseCommand::ScanLibrary { root }) => {
                tracing::info!("Starting Library Scan at {:?}...", root);
                if let Err(e) = handle_scan_library(&conn, &root) {
                    tracing::error!("Failed to scan library: {:?}", e);
                    worker.send(IpcEvent::Error {
                        message: e.to_string(),
                    });
                } else {
                    // Auto-prune after scan
                    let pruned = prune_library(&conn).unwrap_or(0);
                    if pruned > 0 {
                        tracing::info!("Pruned {} dead tracks", pruned);
                    }
                    tracing::info!("Library Scan Complete.");
                    worker.send(IpcEvent::LibraryRescanned);
                }
            }
            Some(DatabaseCommand::PruneLibrary) => {
                info!("Manual prune requested");
                match prune_library(&conn) {
                    Ok(count) => {
                        info!("Pruned {} dead tracks", count);
                        worker.send(IpcEvent::LibraryRescanned);
                    }
                    Err(e) => {
                        tracing::error!("Failed to prune: {:?}", e);
                        worker.send(IpcEvent::Error {
                            message: e.to_string(),
                        });
                    }
                }
            }
            Some(DatabaseCommand::Shutdown) => {
                info!("Database worker shutting down");
                break;
            }
            Some(_) => { /* TODO: Implement other commands */ }
            None => {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
    }
}

#[tracing::instrument(skip(conn))]
fn handle_scan_library(conn: &rusqlite::Connection, root: &Path) -> Result<()> {
    info!("Scanning library at: {:?}", root);

    // Critical: Do NOT delete all tracks (destroys stable IDs)
    // Upsert found tracks, optionally delete missing ones later
    // For now, we just scan and add/update.

    let mut count = 0;
    let scan_start = std::time::Instant::now();

    // Use transaction for speed
    conn.execute("BEGIN TRANSACTION", [])?;

    let res = scan_directory(conn, root, &mut count);

    if res.is_ok() {
        conn.execute("COMMIT", [])?;
    } else {
        conn.execute("ROLLBACK", [])?;
        return res;
    }

    let scan_elapsed = scan_start.elapsed();
    tracing::info!(
        "Scanned directory and processed {} tracks in {:?}",
        count,
        scan_elapsed
    );

    Ok(())
}

/// Remove tracks from DB where the file no longer exists on disk.
fn prune_library(conn: &rusqlite::Connection) -> Result<usize> {
    let mut stmt = conn.prepare("SELECT id, file_path FROM tracks")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
    })?;

    let mut dead_ids: Vec<i64> = Vec::new();
    for row in rows {
        let (id, path) = row?;
        if !Path::new(&path).exists() {
            dead_ids.push(id);
        }
    }

    if dead_ids.is_empty() {
        return Ok(0);
    }

    // Delete in batches using transaction
    conn.execute("BEGIN TRANSACTION", [])?;
    for id in &dead_ids {
        conn.execute("DELETE FROM tracks WHERE id = ?", params![id])?;
    }
    conn.execute("COMMIT", [])?;

    Ok(dead_ids.len())
}

fn scan_directory(conn: &rusqlite::Connection, dir: &Path, count: &mut usize) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            scan_directory(conn, &path, count)?;
        } else if path.is_file()
            && let Some(track_data) = extract_track_data(&path)
        {
            insert_or_replace_track(conn, &track_data)?;
            *count += 1;
            if (*count).is_multiple_of(100) {
                tracing::info!("Processed {} tracks...", *count);
            }
        }
    }
    Ok(())
}

fn extract_track_data(path: &Path) -> Option<TrackData> {
    let ext = path.extension()?.to_string_lossy().to_lowercase();
    if !SUPPORTED_EXTENSIONS.contains(&ext.as_str()) {
        return None;
    }

    // Lofty parsing
    let tagged_file = Probe::open(path).ok()?.read().ok()?;
    let properties = tagged_file.properties();

    let duration_ms = properties.duration().as_millis() as u64;

    let bitrate = properties.audio_bitrate();

    // Debug logging for troubleshooting
    if bitrate.is_none() {
        tracing::warn!("Bitrate extraction failed for {:?}", path);
    }

    let sample_rate = properties.sample_rate();
    let format = ext.to_uppercase();

    let tag = tagged_file
        .primary_tag()
        .or_else(|| tagged_file.first_tag());

    let title = tag
        .and_then(|t| t.title().map(|s| s.to_string()))
        .unwrap_or_else(|| {
            path.file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default()
        });

    let artist = tag.and_then(|t| t.artist().map(|s| s.to_string()));
    let album = tag.and_then(|t| t.album().map(|s| s.to_string()));
    let genre = tag.and_then(|t| t.genre().map(|s| s.to_string()));
    let year = tag.and_then(|t| t.year());
    let track_number = tag.and_then(|t| t.track());
    let has_cover = tag.map(|t| !t.pictures().is_empty()).unwrap_or(false);

    let file_path = path.to_string_lossy().to_string();
    let file_size = fs::metadata(path).ok()?.len() as i64;
    let modified_at = fs::metadata(path)
        .ok()?
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs() as i64;

    Some(TrackData {
        file_path,
        title,
        artist,
        album,
        genre,
        year,
        track_number,
        duration_ms,
        bitrate,
        sample_rate,
        format,
        has_cover,
        file_size,
        modified_at,
    })
}

struct TrackData {
    file_path: String,
    title: String,
    artist: Option<String>,
    album: Option<String>,
    genre: Option<String>,
    year: Option<u32>,
    track_number: Option<u32>,
    duration_ms: u64,
    bitrate: Option<u32>,
    sample_rate: Option<u32>,
    format: String,
    has_cover: bool,
    file_size: i64,
    modified_at: i64,
}

// CHANGED: Use ON CONFLICT to update instead of insert.
// This keeps the primary key (ID) stable if the file path exists.
fn insert_or_replace_track(conn: &rusqlite::Connection, data: &TrackData) -> Result<()> {
    conn.execute(
        "INSERT INTO tracks (
            file_path, title, artist, album, genre, year, track_number,
            duration_ms, bitrate_kbps, sample_rate_hz, format, has_cover,
            file_size_bytes, file_modified_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(file_path) DO UPDATE SET
            title=excluded.title,
            artist=excluded.artist,
            album=excluded.album,
            file_modified_at=excluded.file_modified_at,
            has_cover=excluded.has_cover,
            bitrate_kbps=excluded.bitrate_kbps,
            sample_rate_hz=excluded.sample_rate_hz,
            updated_at=strftime('%s', 'now')",
        params![
            data.file_path,
            data.title,
            data.artist,
            data.album,
            data.genre,
            data.year,
            data.track_number,
            data.duration_ms as i64,
            data.bitrate,
            data.sample_rate,
            data.format,
            data.has_cover,
            data.file_size,
            data.modified_at
        ],
    )?;
    Ok(())
}

/// Load all tracks from DB and build a LibrarySnapshot.
fn load_library_snapshot(conn: &rusqlite::Connection, root: PathBuf) -> Result<LibrarySnapshot> {
    let mut stmt = conn.prepare(
        "SELECT id, file_path, title, artist, album, genre, year,
                track_number, bitrate_kbps, sample_rate_hz, format,
                duration_ms, has_cover, file_size_bytes
         FROM tracks ORDER BY file_path ASC",
    )?;

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
                file_size_bytes: row.get(13)?,
            },
        ))
    })?;

    let mut tracks_map: HashMap<TrackId, TrackMeta> = HashMap::new();
    let mut all_track_ids = Vec::new();

    for t in track_iter {
        let (id, track) = t?;
        tracks_map.insert(id, track);
        all_track_ids.push(id);
    }

    // Load playlists from DB
    let mut playlists_vec = Vec::new();
    let mut playlist_stmt = conn.prepare("SELECT id, name FROM playlists ORDER BY name")?;
    let playlist_iter = playlist_stmt.query_map([], |row| {
        let id: i64 = row.get(0)?;
        let name: String = row.get(1)?;
        Ok((PlaylistId(id as u64), name))
    })?;

    for p in playlist_iter {
        let (pid, name) = p?;
        // Load tracks for this playlist
        let mut items_stmt = conn.prepare(
            "SELECT track_id FROM playlist_tracks WHERE playlist_id = ? ORDER BY position",
        )?;
        let track_ids: Vec<TrackId> = items_stmt
            .query_map(params![pid.0 as i64], |row| {
                Ok(TrackId(row.get::<_, i64>(0)? as u64))
            })?
            .filter_map(|r| r.ok())
            .collect();

        playlists_vec.push(PlaylistMeta {
            id: pid,
            name,
            tracks: track_ids,
        });
    }

    Ok(LibrarySnapshot::new(root, playlists_vec, tracks_map))
}
