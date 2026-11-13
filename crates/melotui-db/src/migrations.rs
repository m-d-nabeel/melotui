//! DB migration runner

use anyhow::Result;
use rusqlite::Connection;
use tracing::info;

/// current schema version
const CURRENT_VERSION: i32 = 3;

/// run migrations
pub fn run_migrations(conn: &Connection) -> Result<()> {
    let user_version: i32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    info!(
        "Database version: {}, target: {}",
        user_version, CURRENT_VERSION
    );

    if user_version < 1 {
        migrate_v1(conn)?;
        conn.pragma_update(None, "user_version", 1)?;
        info!("Database migrated to version 1");
    }

    if user_version < 2 {
        migrate_v2(conn)?;
        conn.pragma_update(None, "user_version", 2)?;
        info!("Database migrated to version 2");
    }

    if user_version < 3 {
        migrate_v3(conn)?;
        conn.pragma_update(None, "user_version", 3)?;
        info!("Database migrated to version 3");
    }

    Ok(())
}

/// v3: a fingerprint of each file's contents, by which a file that has been
/// moved or renamed is known for the track it was. Empty for the tracks
/// already there; the next scan fills it in.
fn migrate_v3(conn: &Connection) -> Result<()> {
    info!("Applying migration V3: Fingerprints");
    conn.execute_batch(
        "
        ALTER TABLE tracks ADD COLUMN fingerprint TEXT;
        CREATE INDEX IF NOT EXISTS idx_tracks_fingerprint ON tracks(fingerprint);
        ",
    )?;
    Ok(())
}

/// v1: schema
fn migrate_v1(conn: &Connection) -> Result<()> {
    info!("Applying migration V1: Consolidated Schema");
    let schema_sql = include_str!("queries/schema.sql");
    conn.execute_batch(schema_sql)?;
    Ok(())
}

/// v2: play_history
fn migrate_v2(conn: &Connection) -> Result<()> {
    info!("Applying migration V2: Play History");
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS play_history (
            id         INTEGER PRIMARY KEY,
            track_id   INTEGER NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
            played_at  INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
            duration_s INTEGER
        );
        CREATE INDEX IF NOT EXISTS idx_play_history_track ON play_history(track_id);
        CREATE INDEX IF NOT EXISTS idx_play_history_time ON play_history(played_at DESC);
        ",
    )?;
    Ok(())
}
