use anyhow::Result;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::Accessor;
use melotui_ipc::event::IpcEvent;
use melotui_ipc::{channel::IpcWorker, command::DatabaseCommand};
use rusqlite::params;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::info;

use crate::init_db;

const SUPPORTED_EXTENSIONS: &[&str] = &["mp3", "flac", "m4a", "wav", "ogg", "opus"];

pub fn run(worker: impl IpcWorker<DatabaseCommand>, db_path: PathBuf) {
    let conn = init_db(&db_path).expect("Failed to open DB");
    info!("Database worker initialized");

    loop {
        match worker.try_recv() {
            Some(DatabaseCommand::ScanLibrary { root }) => {
                tracing::info!("Starting Library Scan at {:?}...", root);
                if let Err(e) = handle_scan_library(&conn, &root) {
                    tracing::error!("Failed to scan library: {:?}", e);
                    worker.send(IpcEvent::Error {
                        message: e.to_string(),
                    });
                } else {
                    tracing::info!("Library Scan Complete.");
                    worker.send(IpcEvent::LibraryRescanned);
                }
            }
            // ... (Other commands todo) ...
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

    // CRITICAL FIX: Do NOT delete all tracks. This destroys stable IDs.
    // Instead, we will upsert found tracks, and optionally delete missing ones later.
    // For now, we just scan and add/update.

    let mut count = 0;
    let scan_start = std::time::Instant::now();

    // Use a transaction for speed
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

    // Lofty parsing (same as before)
    let tagged_file = Probe::open(path).ok()?.read().ok()?;
    let properties = tagged_file.properties();

    let duration_ms = properties.duration().as_millis() as u64;
    let bitrate = properties.audio_bitrate();
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
            duration_ms, bitrate, sample_rate, format, has_cover,
            file_size, modified_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(file_path) DO UPDATE SET
            title=excluded.title,
            artist=excluded.artist,
            album=excluded.album,
            modified_at=excluded.modified_at,
            has_cover=excluded.has_cover",
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
