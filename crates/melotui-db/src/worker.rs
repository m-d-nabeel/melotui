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
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::info;

use crate::migrations::run_migrations;

/// How long to wait for another connection to let go of the database.
const BUSY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

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
    // Settings of the connection, not of the file, so they are made every
    // time it is opened: set only when the database was created, deleting a
    // track took its playlist entries and history with it only because the
    // bundled SQLite happens to be built with foreign keys on.
    conn.pragma_update(None, "foreign_keys", true)?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    // A second melotui, or a backup reading the file, is waited for rather
    // than failed on at once with "database is locked".
    conn.busy_timeout(BUSY_TIMEOUT)?;
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
            let mut changed = 0;
            let mut unreadable = 0;
            // Roots whose tracks are not to be pruned this time.
            let mut kept: Vec<PathBuf> = Vec::new();
            for root in &roots {
                tracing::info!("Scanning {:?}...", root);
                match handle_scan_library(conn, worker, root) {
                    Ok(outcome) => {
                        changed += outcome.processed;
                        unreadable += outcome.unreadable;
                        // A folder with nothing at all in it, that the library
                        // has music from, is a drive that is not mounted: its
                        // mount point is there and empty. Pruning then would
                        // take every one of its tracks, and with them their
                        // favorites, play counts and places in playlists.
                        let known = tracks_under(conn, root);
                        let empty = fs::read_dir(root).is_ok_and(|mut dir| dir.next().is_none());
                        if empty && known > 0 {
                            tracing::warn!("{root:?} is empty; keeping its {known} tracks");
                            worker.send(IpcEvent::Error {
                                message: format!(
                                    "No music found in {} — is it mounted? Keeping its {known} tracks",
                                    root.display()
                                ),
                            });
                            kept.push(root.clone());
                        } else if outcome.unreadable > 0 {
                            // What is in a folder that could not be read
                            // cannot be told from what is gone.
                            kept.push(root.clone());
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to scan {:?}: {:?}", root, e);
                        worker.send(IpcEvent::Error {
                            message: format!("Failed to scan {}: {}", root.display(), e),
                        });
                        kept.push(root.clone());
                    }
                }
            }
            if unreadable > 0 {
                worker.send(IpcEvent::Error {
                    message: format!(
                        "{unreadable} folder(s) could not be read and were passed over; the log names them"
                    ),
                });
            }
            match prune_library(conn, &kept) {
                Ok(pruned) => {
                    if pruned > 0 {
                        tracing::info!("Pruned {} dead tracks", pruned);
                    }
                    changed += pruned;
                }
                Err(e) => tracing::error!("Failed to prune: {e:?}"),
            }
            tracing::info!("Library Scan Complete: {changed} changed");
            worker.send(IpcEvent::LibraryRescanned {
                changed: changed > 0,
            });
        }
        DatabaseCommand::PruneLibrary => {
            info!("Manual prune requested");
            match prune_library(conn, &[]) {
                Ok(count) => {
                    info!("Pruned {} dead tracks", count);
                    worker.send(IpcEvent::LibraryRescanned { changed: count > 0 });
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

/// What scanning one music folder came to.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct ScanOutcome {
    /// Tracks read and written: new files, and files that had changed.
    processed: usize,
    /// Folders inside the root that could not be read, and were passed over.
    unreadable: usize,
}

/// One scan of one root, as it walks the folders.
struct Scan<'a, W> {
    conn: &'a rusqlite::Connection,
    worker: &'a W,
    /// Each known file's size and modification time, to skip the unchanged.
    cache: &'a HashMap<String, (i64, i64)>,
    outcome: ScanOutcome,
    skipped: usize,
    /// Folders walked already, by their real path: a symlink back up the
    /// tree would otherwise be walked for ever.
    visited: HashSet<PathBuf>,
}

#[tracing::instrument(skip(conn, worker))]
fn handle_scan_library(
    conn: &rusqlite::Connection,
    worker: &impl IpcWorker<DatabaseCommand>,
    root: &Path,
) -> Result<ScanOutcome> {
    info!("Scanning library at: {:?}", root);
    let scan_start = std::time::Instant::now();

    // The root itself has to be readable: one that is not is a drive that is
    // not there, and the caller keeps its tracks.
    fs::read_dir(root)?;

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

    let tx = conn.unchecked_transaction()?;
    let mut scan = Scan {
        conn,
        worker,
        cache: &cache,
        outcome: ScanOutcome::default(),
        skipped: 0,
        visited: HashSet::new(),
    };
    scan.directory(root);
    let (outcome, skipped) = (scan.outcome, scan.skipped);
    tx.commit()?;

    tracing::info!(
        "Scanned directory and processed {} tracks (skipped {}, {} unreadable folders) in {:?}",
        outcome.processed,
        skipped,
        outcome.unreadable,
        scan_start.elapsed()
    );

    Ok(outcome)
}

/// Removes the tracks whose files are gone, except those under `kept`: the
/// roots that could not be scanned, or were found empty — a drive that is not
/// mounted leaves its folder behind, and looks exactly like every song in it
/// having been deleted. A track goes only when its file is known to be
/// missing; one that cannot be looked at stays.
fn prune_library(conn: &rusqlite::Connection, kept: &[PathBuf]) -> Result<usize> {
    let mut stmt = conn.prepare("SELECT id, file_path FROM tracks")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
    })?;

    let mut dead_ids: Vec<i64> = Vec::new();
    for row in rows {
        let (id, path) = row?;
        let path = Path::new(&path);
        if kept.iter().any(|root| path.starts_with(root)) {
            continue;
        }
        if matches!(path.try_exists(), Ok(false)) {
            dead_ids.push(id);
        }
    }

    if dead_ids.is_empty() {
        return Ok(0);
    }

    let tx = conn.unchecked_transaction()?;
    for id in &dead_ids {
        tx.execute("DELETE FROM tracks WHERE id = ?", params![id])?;
    }
    tx.commit()?;

    Ok(dead_ids.len())
}

/// How many tracks the library holds under `root`.
fn tracks_under(conn: &rusqlite::Connection, root: &Path) -> usize {
    let Ok(mut stmt) = conn.prepare("SELECT file_path FROM tracks") else {
        return 0;
    };
    stmt.query_map([], |row| row.get::<_, String>(0))
        .map(|rows| {
            rows.flatten()
                .filter(|path| Path::new(path).starts_with(root))
                .count()
        })
        .unwrap_or(0)
}

impl<W: IpcWorker<DatabaseCommand>> Scan<'_, W> {
    /// Scans `dir` and the folders in it. A folder that cannot be read — no
    /// permission, a `lost+found`, a link to nowhere — is passed over and
    /// counted, and the rest of the scan goes on.
    fn directory(&mut self, dir: &Path) {
        if let Ok(real) = fs::canonicalize(dir)
            && !self.visited.insert(real)
        {
            tracing::debug!("Already scanned {dir:?}, reached again through a link");
            return;
        }
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(e) => {
                tracing::warn!("Cannot read {dir:?}, passing over it: {e}");
                self.outcome.unreadable += 1;
                return;
            }
        };
        for entry in entries {
            let path = match entry {
                Ok(entry) => entry.path(),
                Err(e) => {
                    tracing::warn!("Cannot read an entry of {dir:?}: {e}");
                    continue;
                }
            };
            if path.is_dir() {
                self.directory(&path);
            } else if path.is_file() {
                self.file(&path);
            }
        }
    }

    fn file(&mut self, path: &Path) {
        if !is_audio(path) {
            return;
        }
        let path_str = path.to_string_lossy().into_owned();
        if let Ok(meta) = fs::metadata(path)
            && let Some(&cached) = self.cache.get(&path_str)
            && cached == (meta.len() as i64, modified_secs(&meta))
        {
            self.skipped += 1;
            self.progress();
            return;
        }

        // A tag library that panics over some odd file must not take the
        // database worker, and with it the library, down.
        let track_data = melotui_core::utils::panic::guarded(|| extract_track_data(path))
            .unwrap_or_else(|_| {
                tracing::error!("The tag library failed on {path:?}");
                None
            });
        if let Some(track_data) = track_data {
            if let Err(e) = insert_or_replace_track(self.conn, &track_data) {
                tracing::error!("Failed to insert track {:?}: {:?}", path, e);
                return; // resilient scan, do not fail transaction
            }
            self.outcome.processed += 1;
            self.progress();
        }
    }

    fn progress(&self) {
        if (self.outcome.processed + self.skipped).is_multiple_of(50) {
            self.worker.send(IpcEvent::ScanProgress {
                scanned: self.outcome.processed as u32,
                skipped: self.skipped as u32,
                total: self.cache.len() as u32,
            });
        }
    }
}

/// Whether `path` has the extension of a format melotui plays.
fn is_audio(path: &Path) -> bool {
    path.extension().is_some_and(|ext| {
        let ext = ext.to_string_lossy().to_lowercase();
        SUPPORTED_EXTENSIONS.contains(&ext.as_str())
    })
}

/// A file's modification time in seconds since 1970, 0 if it has none.
fn modified_secs(meta: &fs::Metadata) -> i64 {
    meta.modified()
        .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
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

    Some(
        TrackData {
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
        }
        .storable(),
    )
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

impl TrackData {
    /// The track with whatever the library's table would refuse left out.
    ///
    /// The table checks its numbers — a year from 1900 to 2100, a track
    /// number and rates above nothing — and a file tagged with a year of 0,
    /// or as track 0 of its album, failed the check and was left out of the
    /// library altogether, with only the log to say so. A number that makes
    /// no sense is not known, which the table has room for; the song is
    /// still a song.
    fn storable(mut self) -> Self {
        self.year = self.year.filter(|year| (1900..=2100).contains(year));
        self.track_number = self.track_number.filter(|&n| n > 0);
        self.bitrate = self.bitrate.filter(|&rate| rate > 0);
        self.sample_rate = self.sample_rate.filter(|&rate| rate > 0);
        self
    }
}

// Upsert, to keep IDs stable. Everything read from the file is written
// again: a column left out stays as it was when the file was first seen, and
// the size left out had a changed file read again on every scan for good.
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
            genre=excluded.genre,
            year=excluded.year,
            track_number=excluded.track_number,
            duration_ms=excluded.duration_ms,
            bitrate_kbps=excluded.bitrate_kbps,
            sample_rate_hz=excluded.sample_rate_hz,
            format=excluded.format,
            has_cover=excluded.has_cover,
            file_size_bytes=excluded.file_size_bytes,
            file_modified_at=excluded.file_modified_at,
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

    /// A second of silent mono 8 kHz audio, as a WAV file.
    fn wav(path: &Path) {
        wav_of(path, 1);
    }

    /// `seconds` of silent mono 8 kHz audio, as a WAV file.
    fn wav_of(path: &Path, seconds: u32) {
        let data_len: u32 = 8_000 * 2 * seconds;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&8_000u32.to_le_bytes());
        bytes.extend_from_slice(&16_000u32.to_le_bytes());
        bytes.extend_from_slice(&2u16.to_le_bytes());
        bytes.extend_from_slice(&16u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data_len.to_le_bytes());
        bytes.resize(bytes.len() + data_len as usize, 0);
        std::fs::write(path, bytes).unwrap();
    }

    /// Runs the worker on `commands`, returning what it sent.
    fn run_commands(db: &Path, commands: Vec<DatabaseCommand>) -> Vec<IpcEvent> {
        let (bus_tx, bus_rx) = mpsc::channel();
        let (tx, worker) = inprocess::join_worker::<DatabaseCommand>(bus_tx);
        for command in commands {
            tx.send(command).unwrap();
        }
        tx.send(DatabaseCommand::Shutdown).unwrap();
        run(worker, db.to_path_buf());
        bus_rx.try_iter().collect()
    }

    fn rescanned(events: &[IpcEvent]) -> Option<bool> {
        events.iter().find_map(|e| match e {
            IpcEvent::LibraryRescanned { changed } => Some(*changed),
            _ => None,
        })
    }

    #[test]
    fn a_scan_says_whether_it_changed_the_library() {
        let dir = tempfile::tempdir().unwrap();
        let music = dir.path().join("music");
        std::fs::create_dir(&music).unwrap();
        let db = dir.path().join("library.db");
        let scan = || DatabaseCommand::ScanLibrary {
            roots: vec![music.clone()],
        };

        // Nothing there yet: nothing changed.
        assert_eq!(rescanned(&run_commands(&db, vec![scan()])), Some(false));

        // A new file is a change; scanning again with nothing new is not.
        wav(&music.join("song.wav"));
        assert_eq!(rescanned(&run_commands(&db, vec![scan()])), Some(true));
        assert_eq!(rescanned(&run_commands(&db, vec![scan()])), Some(false));

        // Removing it is a change again. (Something else stays in the
        // folder: one left with nothing in it is taken for a drive that is
        // not mounted.)
        std::fs::write(music.join("notes.txt"), "not music").unwrap();
        std::fs::remove_file(music.join("song.wav")).unwrap();
        assert_eq!(rescanned(&run_commands(&db, vec![scan()])), Some(true));
    }

    #[test]
    fn a_folder_that_fails_to_scan_still_ends_the_scan_but_prunes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let music = dir.path().join("music");
        std::fs::create_dir(&music).unwrap();
        wav(&music.join("song.wav"));
        let db = dir.path().join("library.db");
        run_commands(
            &db,
            vec![DatabaseCommand::ScanLibrary {
                roots: vec![music.clone()],
            }],
        );

        // The folder goes missing — an unplugged drive.
        let events = run_commands(
            &db,
            vec![DatabaseCommand::ScanLibrary {
                roots: vec![dir.path().join("unplugged"), music.clone()],
            }],
        );
        assert!(events.iter().any(|e| matches!(e, IpcEvent::Error { .. })));
        assert_eq!(
            rescanned(&events),
            Some(false),
            "pruned after a failed scan"
        );
    }

    /// The titles in the library, in order.
    fn titles(db: &Path) -> Vec<String> {
        let conn = rusqlite::Connection::open(db).unwrap();
        let mut stmt = conn
            .prepare("SELECT title FROM tracks ORDER BY title")
            .unwrap();
        stmt.query_map([], |row| row.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect()
    }

    #[test]
    #[cfg(unix)]
    fn a_folder_that_cannot_be_read_is_passed_over_and_the_rest_scanned() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let music = dir.path().join("music");
        let locked = music.join("lost+found");
        std::fs::create_dir_all(&locked).unwrap();
        std::fs::create_dir(music.join("album")).unwrap();
        wav(&music.join("first.wav"));
        wav(&music.join("album").join("second.wav"));
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
        if std::fs::read_dir(&locked).is_ok() {
            // Run as root, nothing is unreadable, and there is nothing to test.
            return;
        }
        let db = dir.path().join("library.db");

        let events = run_commands(
            &db,
            vec![DatabaseCommand::ScanLibrary {
                roots: vec![music.clone()],
            }],
        );
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();

        assert_eq!(titles(&db), ["first", "second"]);
        assert_eq!(rescanned(&events), Some(true));
        assert!(
            events.iter().any(
                |e| matches!(e, IpcEvent::Error { message } if message.contains("could not be read"))
            ),
            "the folder passed over was not mentioned"
        );
    }

    #[test]
    #[cfg(unix)]
    fn a_link_back_up_the_tree_is_not_followed_round_and_round() {
        let dir = tempfile::tempdir().unwrap();
        let music = dir.path().join("music");
        std::fs::create_dir_all(music.join("album")).unwrap();
        wav(&music.join("album").join("song.wav"));
        std::os::unix::fs::symlink(&music, music.join("album").join("back")).unwrap();
        let db = dir.path().join("library.db");

        let events = run_commands(
            &db,
            vec![DatabaseCommand::ScanLibrary {
                roots: vec![music.clone()],
            }],
        );

        assert_eq!(titles(&db), ["song"], "each file once");
        assert!(!events.iter().any(|e| matches!(e, IpcEvent::Error { .. })));
    }

    #[test]
    fn a_music_folder_found_empty_keeps_its_tracks() {
        // A drive that is not mounted leaves its mount point behind, empty:
        // the scan goes through, and every file in the library is missing.
        let dir = tempfile::tempdir().unwrap();
        let music = dir.path().join("music");
        std::fs::create_dir(&music).unwrap();
        wav(&music.join("song.wav"));
        let db = dir.path().join("library.db");
        let scan = || DatabaseCommand::ScanLibrary {
            roots: vec![music.clone()],
        };
        run_commands(&db, vec![scan()]);

        std::fs::rename(music.join("song.wav"), dir.path().join("away.wav")).unwrap();
        let events = run_commands(&db, vec![scan()]);

        assert_eq!(titles(&db), ["song"], "pruned with the drive away");
        assert_eq!(rescanned(&events), Some(false));
        assert!(
            events.iter().any(
                |e| matches!(e, IpcEvent::Error { message } if message.contains("is it mounted"))
            ),
            "nothing said about the empty folder"
        );

        // Back again, nothing has changed; and a song really deleted, with
        // others still there, is pruned as before.
        std::fs::rename(dir.path().join("away.wav"), music.join("song.wav")).unwrap();
        wav(&music.join("other.wav"));
        run_commands(&db, vec![scan()]);
        std::fs::remove_file(music.join("song.wav")).unwrap();
        run_commands(&db, vec![scan()]);
        assert_eq!(titles(&db), ["other"]);
    }

    #[test]
    fn a_folder_that_fails_to_scan_does_not_stop_another_being_pruned() {
        let dir = tempfile::tempdir().unwrap();
        let (here, away) = (dir.path().join("here"), dir.path().join("away"));
        for folder in [&here, &away] {
            std::fs::create_dir(folder).unwrap();
            wav(&folder.join("kept.wav"));
        }
        wav(&here.join("deleted.wav"));
        let db = dir.path().join("library.db");
        let scan = || DatabaseCommand::ScanLibrary {
            roots: vec![here.clone(), away.clone()],
        };
        run_commands(&db, vec![scan()]);

        std::fs::remove_file(here.join("deleted.wav")).unwrap();
        std::fs::rename(&away, dir.path().join("unplugged")).unwrap();
        run_commands(&db, vec![scan()]);

        assert_eq!(titles(&db), ["kept", "kept"]);
    }

    #[test]
    fn a_file_that_changed_is_read_again_once_and_all_of_it() {
        let dir = tempfile::tempdir().unwrap();
        let music = dir.path().join("music");
        std::fs::create_dir(&music).unwrap();
        wav(&music.join("song.wav"));
        let db = dir.path().join("library.db");
        let scan = || DatabaseCommand::ScanLibrary {
            roots: vec![music.clone()],
        };
        run_commands(&db, vec![scan()]);

        // Edited: longer, and so larger.
        wav_of(&music.join("song.wav"), 3);
        assert_eq!(rescanned(&run_commands(&db, vec![scan()])), Some(true));

        let conn = rusqlite::Connection::open(&db).unwrap();
        let (duration, size): (i64, i64) = conn
            .query_row(
                "SELECT duration_ms, file_size_bytes FROM tracks",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(duration, 3_000, "the length is the old file's");
        assert_eq!(
            size as u64,
            std::fs::metadata(music.join("song.wav")).unwrap().len()
        );
        drop(conn);

        // And then it is as it is in the library: nothing more to read.
        assert_eq!(
            rescanned(&run_commands(&db, vec![scan()])),
            Some(false),
            "read again though nothing changed"
        );
    }

    #[test]
    fn a_track_with_tags_that_make_no_sense_is_still_in_the_library() {
        let dir = tempfile::tempdir().unwrap();
        let conn = open(&dir.path().join("library.db")).unwrap();
        let odd = TrackData {
            file_path: "/music/odd.mp3".into(),
            title: "Odd".into(),
            artist: None,
            album: None,
            genre: None,
            year: Some(0),
            track_number: Some(0),
            duration_ms: 1_000,
            bitrate: Some(0),
            sample_rate: Some(0),
            format: "MP3".into(),
            has_cover: false,
            file_size: 10,
            modified_at: 0,
        }
        .storable();
        insert_or_replace_track(&conn, &odd).expect("refused by the table");
        let (year, number): (Option<u32>, Option<u32>) = conn
            .query_row("SELECT year, track_number FROM tracks", [], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .unwrap();
        assert_eq!((year, number), (None, None));

        // What does make sense is kept.
        let fine = TrackData {
            file_path: "/music/fine.mp3".into(),
            year: Some(1999),
            track_number: Some(7),
            ..odd
        }
        .storable();
        assert_eq!((fine.year, fine.track_number), (Some(1999), Some(7)));
    }

    #[test]
    fn every_connection_has_foreign_keys_on_and_waits_for_a_busy_database() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("library.db");
        // Opened twice: the second time nothing is created, which is when
        // the settings used to go unmade.
        drop(open(&db).unwrap());
        let conn = open(&db).unwrap();
        let pragma = |name: &str| -> i64 {
            conn.pragma_query_value(None, name, |row| row.get(0))
                .unwrap()
        };
        assert_eq!(pragma("foreign_keys"), 1);
        assert_eq!(pragma("busy_timeout"), 5_000);
        assert_eq!(pragma("synchronous"), 1, "NORMAL");

        // And so a track that goes takes its playlist entries with it.
        conn.execute_batch(
            "INSERT INTO tracks (id, file_path, title, duration_ms, format, file_size_bytes, file_modified_at)
                 VALUES (1, '/a.mp3', 'A', 1, 'MP3', 1, 0);
             INSERT INTO playlists (id, name) VALUES (1, 'P');
             INSERT INTO playlist_tracks (playlist_id, track_id, position) VALUES (1, 1, 0);
             DELETE FROM tracks WHERE id = 1;",
        )
        .unwrap();
        let left: i64 = conn
            .query_row("SELECT COUNT(*) FROM playlist_tracks", [], |row| row.get(0))
            .unwrap();
        assert_eq!(left, 0);
    }

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
