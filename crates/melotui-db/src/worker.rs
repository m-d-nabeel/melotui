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
    let conn = rusqlite::Connection::open(&db_path).expect("Failed to open DB");
    run_migrations(&conn).expect("Failed to run migrations");
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
                if let Err(e) = handle_scan_library(conn, root) {
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
        DatabaseCommand::Shutdown => unreachable!("handled above"),
        _ => { /* unhandled commands */ }
    }
}

#[tracing::instrument(skip(conn))]
fn handle_scan_library(conn: &rusqlite::Connection, root: &Path) -> Result<()> {
    info!("Scanning library at: {:?}", root);

    // do not delete all tracks

    let mut count = 0;
    let scan_start = std::time::Instant::now();

    // use transaction
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
                is_favorite: row.get::<_, i64>(7)? != 0,
                track_number: row.get(8)?,
                bitrate: row.get(9)?,
                sample_rate: row.get(10)?,
                format: row.get(11)?,
                duration_ms: row.get::<_, i64>(12)? as u64,
                has_cover: row.get(13)?,
                file_size_bytes: row.get(14)?,
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
