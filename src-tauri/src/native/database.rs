use rusqlite::Connection;
use std::path::Path;
use std::time::Duration;

pub fn open(path: &Path) -> Result<Connection, String> {
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    connection
        .busy_timeout(Duration::from_secs(5))
        .map_err(|error| error.to_string())?;
    connection
        .execute_batch("PRAGMA foreign_keys=ON;")
        .map_err(|error| error.to_string())?;
    Ok(connection)
}

pub fn initialize(path: &Path) -> Result<(), String> {
    let mut connection = open(path)?;
    connection
        .execute_batch("PRAGMA journal_mode=WAL;")
        .map_err(|error| error.to_string())?;
    super::migrations::apply(&mut connection)
}
