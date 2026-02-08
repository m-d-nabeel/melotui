//! DB repository

use anyhow::Result;
use melotui_core::domain::{PlaylistId, TrackId};
use rusqlite::{Connection, params};
use std::path::Path;

use crate::migrations::run_migrations;

/// track query filter
#[derive(Default, Debug, Clone)]
pub struct TrackFilter {
    pub query: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub year_min: Option<u32>,
    pub year_max: Option<u32>,
    pub is_favorite: Option<bool>,
}

/// track row summary
#[derive(Debug, Clone)]
pub struct TrackRow {
    pub id: i64,
    pub file_path: String,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration_ms: i64,
}

/// database repo
pub struct LibraryRepository {
    conn: Connection,
}

impl LibraryRepository {
    /// open db and migrate
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        run_migrations(&conn)?;
        Ok(Self { conn })
    }

    /// get all tracks balance
    pub fn get_all_tracks(&self) -> Result<Vec<TrackRow>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT id, file_path, title, artist, album, duration_ms FROM tracks ORDER BY file_path",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(TrackRow {
                id: row.get(0)?,
                file_path: row.get(1)?,
                title: row.get(2)?,
                artist: row.get(3)?,
                album: row.get(4)?,
                duration_ms: row.get(5)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// query tracks (FTS5)
    pub fn query_tracks(&self, filter: &TrackFilter) -> Result<Vec<TrackRow>> {
        let mut sql = String::from(
            "SELECT id, file_path, title, artist, album, duration_ms FROM tracks WHERE 1=1",
        );
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(ref q) = filter.query {
            sql.push_str(" AND id IN (SELECT rowid FROM tracks_fts WHERE tracks_fts MATCH ?)");
            params_vec.push(Box::new(q.clone()));
        }
        if let Some(ref artist) = filter.artist {
            sql.push_str(" AND artist = ?");
            params_vec.push(Box::new(artist.clone()));
        }
        if let Some(ref album) = filter.album {
            sql.push_str(" AND album = ?");
            params_vec.push(Box::new(album.clone()));
        }
        if let Some(ref genre) = filter.genre {
            sql.push_str(" AND genre = ?");
            params_vec.push(Box::new(genre.clone()));
        }
        if let Some(year_min) = filter.year_min {
            sql.push_str(" AND year >= ?");
            params_vec.push(Box::new(year_min as i32));
        }
        if let Some(year_max) = filter.year_max {
            sql.push_str(" AND year <= ?");
            params_vec.push(Box::new(year_max as i32));
        }
        if let Some(fav) = filter.is_favorite {
            sql.push_str(" AND is_favorite = ?");
            params_vec.push(Box::new(fav as i32));
        }

        sql.push_str(" ORDER BY file_path");

        let mut stmt = self.conn.prepare(&sql)?;
        let params_refs: Vec<&dyn rusqlite::ToSql> =
            params_vec.iter().map(|b| b.as_ref()).collect();

        let rows = stmt.query_map(params_refs.as_slice(), |row| {
            Ok(TrackRow {
                id: row.get(0)?,
                file_path: row.get(1)?,
                title: row.get(2)?,
                artist: row.get(3)?,
                album: row.get(4)?,
                duration_ms: row.get(5)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// create playlist
    pub fn create_playlist(&self, name: &str) -> Result<PlaylistId> {
        self.conn
            .execute("INSERT INTO playlists (name) VALUES (?)", params![name])?;
        let id = self.conn.last_insert_rowid();
        Ok(PlaylistId(id as u64))
    }

    /// delete playlist
    pub fn delete_playlist(&self, id: PlaylistId) -> Result<()> {
        self.conn
            .execute("DELETE FROM playlists WHERE id = ?", params![id.0 as i64])?;
        Ok(())
    }

    /// add track to playlist
    pub fn add_to_playlist(&self, playlist_id: PlaylistId, track_id: TrackId) -> Result<()> {
        let position: i64 = self.conn.query_row(
            "SELECT COALESCE(MAX(position), -1) + 1 FROM playlist_tracks WHERE playlist_id = ?",
            params![playlist_id.0 as i64],
            |row| row.get(0),
        )?;

        self.conn.execute(
            "INSERT OR IGNORE INTO playlist_tracks (playlist_id, track_id, position) VALUES (?, ?, ?)",
            params![playlist_id.0 as i64, track_id.0 as i64, position],
        )?;
        Ok(())
    }

    /// inc play count
    pub fn increment_play_count(&self, id: TrackId) -> Result<()> {
        self.conn.execute(
            "UPDATE tracks SET play_count = play_count + 1, updated_at = strftime('%s', 'now') WHERE id = ?",
            params![id.0 as i64],
        )?;
        Ok(())
    }

    /// toggle favorite
    pub fn toggle_favorite(&self, id: TrackId) -> Result<bool> {
        self.conn.execute(
            "UPDATE tracks SET is_favorite = 1 - is_favorite, updated_at = strftime('%s', 'now') WHERE id = ?",
            params![id.0 as i64],
        )?;
        let new_state: i32 = self.conn.query_row(
            "SELECT is_favorite FROM tracks WHERE id = ?",
            params![id.0 as i64],
            |row| row.get(0),
        )?;
        Ok(new_state == 1)
    }

    /// get favorites
    pub fn get_favorites(&self) -> Result<Vec<TrackRow>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT id, file_path, title, artist, album, duration_ms FROM tracks WHERE is_favorite = 1 ORDER BY title",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(TrackRow {
                id: row.get(0)?,
                file_path: row.get(1)?,
                title: row.get(2)?,
                artist: row.get(3)?,
                album: row.get(4)?,
                duration_ms: row.get(5)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }
}
