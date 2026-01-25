-- Melotui Database Schema v1.0
-- Standards: snake_case, CHECK constraints, FTS5, WAL mode

PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;

-- Tracks: Core metadata
CREATE TABLE IF NOT EXISTS tracks (
    id              INTEGER PRIMARY KEY,
    file_path       TEXT    NOT NULL UNIQUE,
    title           TEXT    NOT NULL,
    artist          TEXT,
    album           TEXT,
    album_artist    TEXT,
    genre           TEXT,
    year            INTEGER CHECK (year IS NULL OR (year >= 1900 AND year <= 2100)),
    track_number    INTEGER CHECK (track_number IS NULL OR track_number > 0),
    disc_number     INTEGER DEFAULT 1,
    duration_ms     INTEGER NOT NULL CHECK (duration_ms >= 0),
    bitrate_kbps    INTEGER CHECK (bitrate_kbps IS NULL OR bitrate_kbps > 0),
    sample_rate_hz  INTEGER CHECK (sample_rate_hz IS NULL OR sample_rate_hz > 0),
    format          TEXT    NOT NULL,
    has_cover       INTEGER NOT NULL DEFAULT 0 CHECK (has_cover IN (0, 1)),
    file_size_bytes INTEGER NOT NULL CHECK (file_size_bytes > 0),
    file_modified_at INTEGER NOT NULL,
    play_count      INTEGER NOT NULL DEFAULT 0 CHECK (play_count >= 0),
    is_favorite     INTEGER NOT NULL DEFAULT 0 CHECK (is_favorite IN (0, 1)),
    created_at      INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at      INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- Playlists: User-created collections
CREATE TABLE IF NOT EXISTS playlists (
    id          INTEGER PRIMARY KEY,
    name        TEXT    NOT NULL UNIQUE,
    description TEXT,
    created_at  INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at  INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- Playlist Tracks: Many-to-many with ordering
CREATE TABLE IF NOT EXISTS playlist_tracks (
    playlist_id INTEGER NOT NULL REFERENCES playlists(id) ON DELETE CASCADE,
    track_id    INTEGER NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    position    INTEGER NOT NULL CHECK (position >= 0),
    added_at    INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    PRIMARY KEY (playlist_id, track_id)
);

-- Indices for common queries
CREATE INDEX IF NOT EXISTS idx_tracks_artist ON tracks(artist);
CREATE INDEX IF NOT EXISTS idx_tracks_album ON tracks(album);
CREATE INDEX IF NOT EXISTS idx_tracks_genre ON tracks(genre);
CREATE INDEX IF NOT EXISTS idx_tracks_year ON tracks(year);
CREATE INDEX IF NOT EXISTS idx_tracks_is_favorite ON tracks(is_favorite);
CREATE INDEX IF NOT EXISTS idx_playlist_tracks_position ON playlist_tracks(playlist_id, position);

-- FTS5 for full-text search
CREATE VIRTUAL TABLE IF NOT EXISTS tracks_fts USING fts5(
    title, artist, album, genre,
    content='tracks',
    content_rowid='id'
);

-- Triggers to keep FTS in sync
CREATE TRIGGER IF NOT EXISTS tracks_ai AFTER INSERT ON tracks BEGIN
    INSERT INTO tracks_fts(rowid, title, artist, album, genre)
    VALUES (new.id, new.title, new.artist, new.album, new.genre);
END;

CREATE TRIGGER IF NOT EXISTS tracks_ad AFTER DELETE ON tracks BEGIN
    INSERT INTO tracks_fts(tracks_fts, rowid, title, artist, album, genre)
    VALUES ('delete', old.id, old.title, old.artist, old.album, old.genre);
END;

CREATE TRIGGER IF NOT EXISTS tracks_au AFTER UPDATE ON tracks BEGIN
    INSERT INTO tracks_fts(tracks_fts, rowid, title, artist, album, genre)
    VALUES ('delete', old.id, old.title, old.artist, old.album, old.genre);
    INSERT INTO tracks_fts(rowid, title, artist, album, genre)
    VALUES (new.id, new.title, new.artist, new.album, new.genre);
END;
