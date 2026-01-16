use rusqlite::Connection;
use std::path::Path;

pub mod worker;
pub use crate::worker::run;

pub fn init_db(path: &Path) -> anyhow::Result<Connection> {
    let conn = Connection::open(path)?;
    let schema_sql = include_str!("queries/schema.sql");
    conn.execute_batch(schema_sql)?;

    Ok(conn)
}
