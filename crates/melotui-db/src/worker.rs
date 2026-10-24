use anyhow::Result;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::Accessor;
use melotui_bus::event::IpcEvent;
use melotui_bus::{channel::IpcWorker, command::DatabaseCommand};
use melotui_core::{
    domain::{PlaylistId, PlaylistMeta, TrackId, TrackMeta},
    library::Library,
};
use rusqlite::params;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::info;

use crate::migrations::run_migrations;

const SUPPORTED_EXTENSIONS: &[&str] = &["mp3", "flac", "m4a", "wav", "ogg", "opus"];

pub fn run(worker: impl IpcWorker<DatabaseCommand>, db_path: PathBuf) {
    // A database that cannot be opened — corrupt, locked, unreadable — is
    // reported rather than panicked over: a panic here left the app showing
    // "Loading library..." for good, with the reason only in the log.
    let conn = match open(&db_path) {
        Ok(conn) => conn,
        Err(e) => {
            tracing::error!("Cannot open the library database {:?}: {:?}", db_path, e);
            worker.send(IpcEvent::Error {
                message: format!(
                    "Cannot open the library database {}: {e}",
                    db_path.display()
                ),
            });
            return;
        }
    };
    info!("Database worker initialized");

    while let Some(cmd) = worker.recv() {
        match cmd {
            DatabaseCommand::Shutdown => {
                info!("Database worker shutting down");
                break;
            }
            cmd => handle_command(&conn, &worker, cmd),
        }
    }
}

fn open(db_path: &Path) -> Result<rusqlite::Connection> {
    let conn = rusqlite::Connection::open(db_path)?;
    run_migrations(&conn)?;
    Ok(conn)
}

fn handle_command(
    conn: &rusqlite::Connection,
    worker: &impl IpcWorker<DatabaseCommand>,
    cmd: DatabaseCommand,
) {
    match cmd {
        DatabaseCommand::LoadLibrary { roots } => {
            info!("Loading library from database for roots: {:?}", roots);
            match load_library_snapshot(conn, roots) {
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
        DatabaseCommand::ScanLibrary { roots } => {
            tracing::info!("Starting Library Scan for {} directories...", roots.len());
            let mut scan_failed = false;
            for root in &roots {
                tracing::info!("Scanning {:?}...", root);
                if let Err(e) = handle_scan_library(conn, worker, root) {
                    tracing::error!("Failed to scan {:?}: {:?}", root, e);
                    worker.send(IpcEvent::Error {
                        message: format!("Failed to scan {:?}: {}", root, e),
                    });
                    scan_failed = true;
                }
            }
            if !scan_failed {
                // auto-prune after scan
                let pruned = prune_library(conn).unwrap_or(0);
                if pruned > 0 {
                    tracing::info!("Pruned {} dead tracks", pruned);
                }
                tracing::info!("Library Scan Complete.");
                worker.send(IpcEvent::LibraryRescanned);
            }
        }
        DatabaseCommand::PruneLibrary => {
            info!("Manual prune requested");
            match prune_library(conn) {
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
        DatabaseCommand::ToggleFavorite { track_id } => {
            match toggle_favorite(conn, TrackId(track_id as u64)) {
                Ok(is_favorite) => {
                    worker.send(IpcEvent::FavoriteToggled {
                        track_id: TrackId(track_id as u64),
                        is_favorite,
                    });
                }
                Err(e) => {
                    tracing::error!("Failed to toggle favorite: {:?}", e);
                    worker.send(IpcEvent::Error {
                        message: format!("Failed to toggle favorite: {}", e),
                    });
                }
            }
        }
        DatabaseCommand::CreatePlaylist { name } => match create_playlist(conn, &name) {
            Ok(id) => {
                info!("Created playlist '{}' with id {:?}", name, id);
                worker.send(IpcEvent::PlaylistCreated { id, name });
            }
            Err(e) => {
                tracing::error!("Failed to create playlist: {:?}", e);
                worker.send(IpcEvent::Error {
                    message: format!("Failed to create playlist: {}", e),
                });
            }
        },
        DatabaseCommand::DeletePlaylist { playlist_id } => {
            let pid = PlaylistId(playlist_id as u64);
            match delete_playlist(conn, pid) {
                Ok(()) => {
                    info!("Deleted playlist {:?}", pid);
                    worker.send(IpcEvent::PlaylistDeleted { id: pid });
                }
                Err(e) => {
                    tracing::error!("Failed to delete playlist: {:?}", e);
                    worker.send(IpcEvent::Error {
                        message: format!("Failed to delete playlist: {}", e),
                    });
                }
            }
        }
        DatabaseCommand::AddTrackToPlaylist {
            playlist_id,
            track_id,
        } => {
            let pid = PlaylistId(playlist_id as u64);
            let tid = TrackId(track_id as u64);
            match add_to_playlist(conn, pid, tid) {
                Ok(()) => {
                    info!("Added track {:?} to playlist {:?}", tid, pid);
                    worker.send(IpcEvent::PlaylistTrackAdded {
                        playlist_id: pid,
                        track_id: tid,
                    });
                }
                Err(e) => {
                    tracing::error!("Failed to add to playlist: {:?}", e);
                    worker.send(IpcEvent::Error {
                        message: format!("Failed to add to playlist: {}", e),
                    });
                }
            }
        }
        DatabaseCommand::RemoveTrackFromPlaylist {
            playlist_id,
            track_id,
        } => {
            let pid = PlaylistId(playlist_id as u64);
            let tid = TrackId(track_id as u64);
            match remove_from_playlist(conn, pid, tid) {
                Ok(()) => {
                    info!("Removed track {:?} from playlist {:?}", tid, pid);
                    worker.send(IpcEvent::PlaylistTrackRemoved {
                        playlist_id: pid,
                        track_id: tid,
                    });
                }
                Err(e) => {
                    tracing::error!("Failed to remove from playlist: {:?}", e);
                    worker.send(IpcEvent::Error {
                        message: format!("Failed to remove from playlist: {}", e),
                    });
                }
            }
        }
        DatabaseCommand::IncrementPlayCount { track_id } => {
            let tid = TrackId(track_id as u64);
            if let Err(e) = increment_play_count(conn, tid) {
                tracing::error!("Failed to increment play count: {:?}", e);
            }
        }
        DatabaseCommand::SearchTracks { query } => {
            match conn.prepare("SELECT rowid FROM tracks_fts WHERE tracks_fts MATCH ?") {
                Ok(mut stmt) => {
                    match stmt.query_map(params![query], |row| {
                        Ok(TrackId(row.get::<_, i64>(0)? as u64))
                    }) {
                        Ok(rows) => {
                            let results: Vec<TrackId> = rows.flatten().collect();
                            worker.send(IpcEvent::SearchResults { query, results });
                        }
                        Err(e) => {
                            tracing::error!("Failed to query FTS: {:?}", e);
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to prepare FTS query: {:?}", e);
                }
            }
        }
        DatabaseCommand::Shutdown => unreachable!("handled above"),
    }
}

#[tracing::instrument(skip(conn, worker))]
fn handle_scan_library(
    conn: &rusqlite::Connection,
    worker: &impl IpcWorker<DatabaseCommand>,
    root: &Path,
) -> Result<()> {
    info!("Scanning library at: {:?}", root);

    // do not delete all tracks

    let mut count = 0;
    let mut skipped = 0;
    let scan_start = std::time::Instant::now();

    // load cache
    let mut cache: HashMap<String, (i64, i64)> = HashMap::new();
    if let Ok(mut stmt) =
        conn.prepare("SELECT file_path, file_size_bytes, file_modified_at FROM tracks")
        && let Ok(rows) = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })
    {
        for row in rows.flatten() {
            cache.insert(row.0, (row.1, row.2));
        }
    }

    // use transaction
    conn.execute("BEGIN TRANSACTION", [])?;

    let res = scan_directory(
        conn,
        worker,
        root,
        &mut count,
        &mut skipped,
        &cache,
        cache.len(),
    );

    if res.is_ok() {
        conn.execute("COMMIT", [])?;
    } else {
        conn.execute("ROLLBACK", [])?;
        return res;
    }

    let scan_elapsed = scan_start.elapsed();
    tracing::info!(
        "Scanned directory and processed {} tracks (skipped {}) in {:?}",
        count,
        skipped,
        scan_elapsed
    );

    Ok(())
}

/// remove non-existent tracks
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

    // delete in batches
    conn.execute("BEGIN TRANSACTION", [])?;
    for id in &dead_ids {
        conn.execute("DELETE FROM tracks WHERE id = ?", params![id])?;
    }
    conn.execute("COMMIT", [])?;

    Ok(dead_ids.len())
}

fn scan_directory(
    conn: &rusqlite::Connection,
    worker: &impl IpcWorker<DatabaseCommand>,
    dir: &Path,
    count: &mut usize,
    skipped: &mut usize,
    cache: &HashMap<String, (i64, i64)>,
    total_cached: usize,
) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            scan_directory(conn, worker, &path, count, skipped, cache, total_cached)?;
        } else if path.is_file() {
            let path_str = path.to_string_lossy().into_owned();
            if let Ok(meta) = fs::metadata(&path) {
                let size = meta.len() as i64;
                let modified = meta
                    .modified()
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64;

                if let Some(&(cached_size, cached_modified)) = cache.get(&path_str)
                    && size == cached_size
                    && modified == cached_modified
                {
                    *skipped += 1;
                    if (*count + *skipped).is_multiple_of(50) {
                        worker.send(IpcEvent::ScanProgress {
                            scanned: *count as u32,
                            skipped: *skipped as u32,
                            total: total_cached as u32,
                        });
                    }
                    continue;
                }
            }

            if let Some(track_data) = extract_track_data(&path) {
                if let Err(e) = insert_or_replace_track(conn, &track_data) {
                    tracing::error!("Failed to insert track {:?}: {:?}", path, e);
                    continue; // resilient scan, do not fail transaction
                }
                *count += 1;
                if (*count + *skipped).is_multiple_of(50) {
                    worker.send(IpcEvent::ScanProgress {
                        scanned: *count as u32,
                        skipped: *skipped as u32,
                        total: total_cached as u32,
                    });
                }
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

    // lofty parsing
    let tagged_file = Probe::open(path).ok()?.read().ok()?;
    let properties = tagged_file.properties();

    let duration_ms = properties.duration().as_millis() as u64;

    let bitrate = properties.audio_bitrate();

    // logging for troubleshoot
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

// upsert to keep IDs stable
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

/// load snapshot
fn load_library_snapshot(conn: &rusqlite::Connection, roots: Vec<PathBuf>) -> Result<Library> {
    let mut stmt = conn.prepare(
        "SELECT id, file_path, title, artist, album, genre, year, is_favorite,
                track_number, bitrate_kbps, sample_rate_hz, format,
                duration_ms, play_count, has_cover, file_size_bytes
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
                is_favorite: row.get::<_, i64>(7)? != 0,
                track_number: row.get(8)?,
                bitrate: row.get(9)?,
                sample_rate: row.get(10)?,
                format: row.get(11)?,
                duration_ms: row.get::<_, i64>(12)? as u64,
                play_count: row.get::<_, i64>(13)? as u32,
                // Filled in below from `play_history`, which is the only
                // place this fact lives on disk.
                last_played_at: None,
                has_cover: row.get(14)?,
                file_size_bytes: row.get(15)?,
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

    // Each track's most recent play, from history. This used to instead be a
    // separately-loaded `recently_played: Vec<TrackId>` ordered list; now that
    // `Library::recently_played()` derives its answer from this field on each
    // track, loading it here is the only DB-specific step left — everything
    // downstream (ordering, the 100-item cap) lives in one place in
    // `melotui-core` instead of being duplicated between this query and that
    // method.
    if let Ok(mut stmt) =
        conn.prepare("SELECT track_id, MAX(played_at) FROM play_history GROUP BY track_id")
        && let Ok(rows) = stmt.query_map([], |row| {
            Ok((
                TrackId(row.get::<_, i64>(0)? as u64),
                row.get::<_, i64>(1)? as u64,
            ))
        })
    {
        for row in rows.flatten() {
            let (id, last_played_at) = row;
            if let Some(track) = tracks_map.get_mut(&id) {
                track.last_played_at = Some(last_played_at);
            }
        }
    }

    // load playlists
    let mut playlists_vec = Vec::new();
    let mut playlist_stmt = conn.prepare("SELECT id, name FROM playlists ORDER BY name")?;
    let playlist_iter = playlist_stmt.query_map([], |row| {
        let id: i64 = row.get(0)?;
        let name: String = row.get(1)?;
        Ok((PlaylistId(id as u64), name))
    })?;

    for p in playlist_iter {
        let (pid, name) = p?;
        // load playlist tracks
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

    Ok(Library::new(roots, playlists_vec, tracks_map))
}

/// toggle favorite
fn toggle_favorite(conn: &rusqlite::Connection, id: TrackId) -> Result<bool> {
    conn.execute(
        "UPDATE tracks SET is_favorite = 1 - is_favorite, updated_at = strftime('%s', 'now') WHERE id = ?",
        params![id.0 as i64],
    )?;
    let new_state: i32 = conn.query_row(
        "SELECT is_favorite FROM tracks WHERE id = ?",
        params![id.0 as i64],
        |row| row.get(0),
    )?;
    Ok(new_state == 1)
}

fn create_playlist(conn: &rusqlite::Connection, name: &str) -> Result<PlaylistId> {
    conn.execute("INSERT INTO playlists (name) VALUES (?)", params![name])?;
    let id = conn.last_insert_rowid();
    Ok(PlaylistId(id as u64))
}

fn delete_playlist(conn: &rusqlite::Connection, id: PlaylistId) -> Result<()> {
    conn.execute("DELETE FROM playlists WHERE id = ?", params![id.0 as i64])?;
    Ok(())
}

fn add_to_playlist(
    conn: &rusqlite::Connection,
    playlist_id: PlaylistId,
    track_id: TrackId,
) -> Result<()> {
    let position: i64 = conn.query_row(
        "SELECT COALESCE(MAX(position), -1) + 1 FROM playlist_tracks WHERE playlist_id = ?",
        params![playlist_id.0 as i64],
        |row| row.get(0),
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO playlist_tracks (playlist_id, track_id, position) VALUES (?, ?, ?)",
        params![playlist_id.0 as i64, track_id.0 as i64, position],
    )?;
    Ok(())
}

fn remove_from_playlist(
    conn: &rusqlite::Connection,
    playlist_id: PlaylistId,
    track_id: TrackId,
) -> Result<()> {
    conn.execute(
        "DELETE FROM playlist_tracks WHERE playlist_id = ? AND track_id = ?",
        params![playlist_id.0 as i64, track_id.0 as i64],
    )?;
    Ok(())
}

fn increment_play_count(conn: &rusqlite::Connection, track_id: TrackId) -> Result<()> {
    conn.execute(
        "UPDATE tracks SET play_count = play_count + 1 WHERE id = ?",
        params![track_id.0 as i64],
    )?;
    conn.execute(
        "INSERT INTO play_history (track_id, duration_s) VALUES (?, ?)",
        params![track_id.0 as i64, 0], // Note: duration_s is 0 for now, could be passed later
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use melotui_bus::channel::inprocess;
    use std::sync::mpsc;

    #[test]
    fn a_database_that_cannot_be_opened_is_reported_not_panicked_over() {
        // A directory where the database file should be cannot be opened.
        let dir = tempfile::tempdir().unwrap();
        let (bus_tx, bus_rx) = mpsc::channel();
        let (_commands, worker) = inprocess::join_worker::<DatabaseCommand>(bus_tx);
        run(worker, dir.path().to_path_buf());
        match bus_rx.try_recv() {
            Ok(IpcEvent::Error { message }) => {
                assert!(
                    message.starts_with("Cannot open the library database"),
                    "{message}"
                )
            }
            other => panic!("expected an error event, got {other:?}"),
        }
    }
}
