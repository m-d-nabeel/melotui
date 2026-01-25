//! Database migration runner using PRAGMA user_version.

use anyhow::Result;
use rusqlite::Connection;
use tracing::info;

/// Current schema version. Increment when making schema changes.
/// Current schema version. Increment when making schema changes.
const CURRENT_VERSION: i32 = 1;

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

    // Set version to 1 as the new "Perfect" baseline
    conn.pragma_update(None, "user_version", 1)?;
    info!("Database migrated to version 1");
    Ok(())
}

/// V1: Consolidated Schema (Includes bitrate/sample_rate fixes)
fn migrate_v1(conn: &Connection) -> Result<()> {
    info!("Applying migration V1: Consolidated Schema");
    let schema_sql = include_str!("queries/schema.sql");
    conn.execute_batch(schema_sql)?;
    Ok(())
}
