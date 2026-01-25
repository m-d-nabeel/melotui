//! Database migration runner using PRAGMA user_version.

use anyhow::Result;
use rusqlite::Connection;
use tracing::info;

/// Current schema version. Increment when making schema changes.
const CURRENT_VERSION: i32 = 2;

/// Run migrations to bring the database up to the current version.
pub fn run_migrations(conn: &Connection) -> Result<()> {
    let user_version: i32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    info!(
        "Database version: {}, target: {}",
        user_version, CURRENT_VERSION
    );

    if user_version < 1 {
        migrate_v1(conn)?;
    }

    if user_version < 2 {
        migrate_v2(conn)?;
    }

    conn.pragma_update(None, "user_version", CURRENT_VERSION)?;
    info!("Database migrated to version {}", CURRENT_VERSION);
    Ok(())
}

/// V1: Initial schema with FTS5
fn migrate_v1(conn: &Connection) -> Result<()> {
    info!("Applying migration V1: Initial schema");
    let schema_sql = include_str!("queries/schema.sql");
    conn.execute_batch(schema_sql)?;
    Ok(())
}

/// V2: Ensure audio property columns exist (fix for early adopters)
fn migrate_v2(conn: &Connection) -> Result<()> {
    info!("Applying migration V2: Fix missing audio columns");

    // Helper to check and add column
    let check_and_add = |col_name: &str, col_def: &str| -> Result<()> {
        let count: i32 = conn.query_row(
            "SELECT count(*) FROM pragma_table_info('tracks') WHERE name = ?",
            [col_name],
            |row| row.get(0),
        )?;

        if count == 0 {
            info!("Adding missing column: {}", col_name);
            conn.execute(
                &format!("ALTER TABLE tracks ADD COLUMN {} {}", col_name, col_def),
                [],
            )?;
        }
        Ok(())
    };

    check_and_add(
        "bitrate_kbps",
        "INTEGER CHECK (bitrate_kbps IS NULL OR bitrate_kbps > 0)",
    )?;
    check_and_add(
        "sample_rate_hz",
        "INTEGER CHECK (sample_rate_hz IS NULL OR sample_rate_hz > 0)",
    )?;

    Ok(())
}
