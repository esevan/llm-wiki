use rusqlite::Connection;
use serde_json::Value;
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
        .execute_batch(include_str!("schema.sql"))
        .map_err(|error| error.to_string())?;
    migrate_native_localization(&mut connection)?;
    add_missing_column(
        &connection,
        "importance_assessments",
        "created_at",
        "TEXT NOT NULL DEFAULT ''",
    )?;
    Ok(())
}

fn add_missing_column(
    connection: &Connection,
    table: &str,
    column: &str,
    declaration: &str,
) -> Result<(), String> {
    let columns = connection
        .prepare(&format!("PRAGMA table_info({table})"))
        .and_then(|mut statement| {
            statement
                .query_map([], |row| row.get::<_, String>(1))?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|error| error.to_string())?;
    if !columns.iter().any(|existing| existing == column) {
        connection
            .execute_batch(&format!(
                "ALTER TABLE {table} ADD COLUMN {column} {declaration}"
            ))
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn migrate_native_localization(connection: &mut Connection) -> Result<(), String> {
    let columns = connection
        .prepare("PRAGMA table_info(localized_content)")
        .and_then(|mut statement| {
            statement
                .query_map([], |row| row.get::<_, String>(1))?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|error| error.to_string())?;
    if !columns.iter().any(|column| column == "fields_json") {
        return Ok(());
    }
    let transaction = connection
        .transaction()
        .map_err(|error| error.to_string())?;
    transaction
        .execute_batch(
            "ALTER TABLE localized_content RENAME TO localized_content_native_v0;
             CREATE TABLE localized_content (
               entity_type TEXT NOT NULL, entity_id TEXT NOT NULL, field_name TEXT NOT NULL,
               locale TEXT NOT NULL, value TEXT NOT NULL, origin TEXT NOT NULL DEFAULT 'ai',
               source_hash TEXT NOT NULL DEFAULT '', created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
               updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
               PRIMARY KEY(entity_type,entity_id,field_name,locale));",
        )
        .map_err(|error| error.to_string())?;
    let records = transaction
        .prepare("SELECT entity_type,entity_id,locale,fields_json,source_hash,created_at,updated_at FROM localized_content_native_v0")
        .and_then(|mut statement| {
            statement.query_map([], |row| Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,String>(2)?,row.get::<_,String>(3)?,row.get::<_,String>(4)?,row.get::<_,String>(5)?,row.get::<_,String>(6)?)))?.collect::<Result<Vec<_>,_>>()
        }).map_err(|error| error.to_string())?;
    for (entity_type, entity_id, locale, raw, source_hash, created_at, updated_at) in records {
        let fields = serde_json::from_str::<Value>(&raw)
            .ok()
            .and_then(|value| value.as_object().cloned())
            .unwrap_or_default();
        for (field, value) in fields {
            if let Some(value) = value.as_str() {
                transaction.execute(
                    "INSERT INTO localized_content(entity_type,entity_id,field_name,locale,value,origin,source_hash,created_at,updated_at) VALUES (?,?,?,?,?,'ai',?,?,?)",
                    rusqlite::params![entity_type,entity_id,field,locale,value,source_hash,created_at,updated_at],
                ).map_err(|error| error.to_string())?;
            }
        }
    }
    transaction.execute_batch(
        "DROP TABLE localized_content_native_v0;
         CREATE INDEX IF NOT EXISTS idx_localized_content_entity ON localized_content(entity_type,entity_id,locale);",
    ).map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())
}
