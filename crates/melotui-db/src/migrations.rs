//! DB migration runner

use anyhow::Result;
use rusqlite::Connection;
use tracing::info;

/// current schema version
const CURRENT_VERSION: i32 = 1;

/// run migrations
pub fn run_migrations(conn: &Connection) -> Result<()> {
    let user_version: i32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    info!(
        "Database version: {}, target: {}",
        user_version, CURRENT_VERSION
    );

    if user_version < 1 {
        migrate_v1(conn)?;
    }

    // set version 1
    conn.pragma_update(None, "user_version", 1)?;
    info!("Database migrated to version 1");
    Ok(())
}

/// v1: schema
fn migrate_v1(conn: &Connection) -> Result<()> {
    info!("Applying migration V1: Consolidated Schema");
    let schema_sql = include_str!("queries/schema.sql");
    conn.execute_batch(schema_sql)?;
    Ok(())
}
