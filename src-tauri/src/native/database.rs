use rusqlite::Connection;
use std::path::Path;

pub fn open(path: &Path) -> Result<Connection, String> {
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    connection
        .execute_batch(include_str!("schema.sql"))
        .map_err(|error| error.to_string())?;
    Ok(connection)
}
