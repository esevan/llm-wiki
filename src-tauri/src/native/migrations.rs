use rusqlite::{params, Connection, Transaction, TransactionBehavior};
use serde_json::Value;

pub const CURRENT_SCHEMA_VERSION: i64 = 2;

type MigrationFunction = for<'connection> fn(&Transaction<'connection>) -> Result<(), String>;
type LegacyLocalizationRow = (String, String, String, String, String, String, String);

struct Migration {
    version: i64,
    name: &'static str,
    run: MigrationFunction,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "create native schema",
        run: create_native_schema,
    },
    Migration {
        version: 2,
        name: "normalize legacy Python schema",
        run: normalize_legacy_schema,
    },
];

pub fn apply(connection: &mut Connection) -> Result<(), String> {
    apply_plan(connection, MIGRATIONS, CURRENT_SCHEMA_VERSION)
}

fn apply_plan(
    connection: &mut Connection,
    migrations: &[Migration],
    target_version: i64,
) -> Result<(), String> {
    validate_plan(migrations, target_version)?;
    let mut current_version = schema_version(connection)?;
    if current_version > target_version {
        return Err(format!(
            "Database schema version {current_version} is newer than supported version {target_version}"
        ));
    }

    for migration in migrations {
        if migration.version <= current_version {
            continue;
        }
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| migration_error(migration, error))?;
        let observed_version = schema_version(&transaction)?;
        if observed_version != current_version {
            return Err(format!(
                "Database schema version changed during migration: expected {current_version}, found {observed_version}"
            ));
        }
        (migration.run)(&transaction).map_err(|error| migration_error(migration, error))?;
        transaction
            .pragma_update(None, "user_version", migration.version)
            .map_err(|error| migration_error(migration, error))?;
        transaction
            .commit()
            .map_err(|error| migration_error(migration, error))?;
        current_version = migration.version;
    }

    Ok(())
}

fn validate_plan(migrations: &[Migration], target_version: i64) -> Result<(), String> {
    if target_version < 0 || migrations.len() as i64 != target_version {
        return Err("Database migration plan does not match its target version".into());
    }
    for (index, migration) in migrations.iter().enumerate() {
        if migration.version != index as i64 + 1 {
            return Err("Database migration versions must be contiguous and ordered".into());
        }
    }
    Ok(())
}

fn schema_version(connection: &Connection) -> Result<i64, String> {
    connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|error| error.to_string())
}

fn migration_error(migration: &Migration, error: impl std::fmt::Display) -> String {
    format!(
        "Database migration {} ({}) failed: {error}",
        migration.version, migration.name
    )
}

fn create_native_schema(transaction: &Transaction<'_>) -> Result<(), String> {
    transaction
        .execute_batch(include_str!("schema.sql"))
        .map_err(|error| error.to_string())
}

fn normalize_legacy_schema(transaction: &Transaction<'_>) -> Result<(), String> {
    migrate_native_localization(transaction)?;
    for (table, column, declaration) in [
        ("problems", "detail", "TEXT NOT NULL DEFAULT ''"),
        (
            "features",
            "validation_criteria",
            "TEXT NOT NULL DEFAULT ''",
        ),
        (
            "completion_playbooks",
            "lineage_snapshot_id",
            "TEXT NOT NULL DEFAULT ''",
        ),
        (
            "completion_playbooks",
            "lineage_version",
            "INTEGER NOT NULL DEFAULT 0",
        ),
        (
            "completion_playbooks",
            "lineage_schema_version",
            "INTEGER NOT NULL DEFAULT 0",
        ),
        (
            "completion_playbooks",
            "report_input_hash",
            "TEXT NOT NULL DEFAULT ''",
        ),
        (
            "completion_playbooks",
            "report_generation_status",
            "TEXT NOT NULL DEFAULT 'deterministic_fallback'",
        ),
        (
            "importance_assessments",
            "created_at",
            "TEXT NOT NULL DEFAULT ''",
        ),
        (
            "provider_settings",
            "advanced_model",
            "TEXT NOT NULL DEFAULT ''",
        ),
        (
            "provider_settings",
            "advanced_tasks",
            "TEXT NOT NULL DEFAULT '{}'",
        ),
        (
            "provider_settings",
            "report_language",
            "TEXT NOT NULL DEFAULT 'ko'",
        ),
        (
            "provider_settings",
            "async_worker_count",
            "INTEGER NOT NULL DEFAULT 2",
        ),
    ] {
        add_missing_column(transaction, table, column, declaration)?;
    }
    Ok(())
}

fn table_exists(connection: &Connection, table: &str) -> Result<bool, String> {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?)",
            [table],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())
}

fn columns(connection: &Connection, table: &str) -> Result<Vec<String>, String> {
    connection
        .prepare(&format!("PRAGMA table_info({table})"))
        .and_then(|mut statement| {
            statement
                .query_map([], |row| row.get::<_, String>(1))?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|error| error.to_string())
}

fn add_missing_column(
    connection: &Connection,
    table: &str,
    column: &str,
    declaration: &str,
) -> Result<(), String> {
    if !table_exists(connection, table)?
        || columns(connection, table)?
            .iter()
            .any(|item| item == column)
    {
        return Ok(());
    }
    connection
        .execute_batch(&format!(
            "ALTER TABLE {table} ADD COLUMN {column} {declaration}"
        ))
        .map_err(|error| error.to_string())
}

fn migrate_native_localization(transaction: &Transaction<'_>) -> Result<(), String> {
    if !table_exists(transaction, "localized_content")?
        || !columns(transaction, "localized_content")?
            .iter()
            .any(|column| column == "fields_json")
    {
        return Ok(());
    }
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
    let records = legacy_localization_rows(transaction)?;
    for (entity_type, entity_id, locale, raw, source_hash, created_at, updated_at) in records {
        let fields = serde_json::from_str::<Value>(&raw)
            .ok()
            .and_then(|value| value.as_object().cloned())
            .unwrap_or_default();
        for (field, value) in fields {
            if let Some(value) = value.as_str() {
                transaction.execute(
                    "INSERT INTO localized_content(entity_type,entity_id,field_name,locale,value,origin,source_hash,created_at,updated_at) VALUES (?,?,?,?,?,'ai',?,?,?)",
                    params![entity_type,entity_id,field,locale,value,source_hash,created_at,updated_at],
                ).map_err(|error| error.to_string())?;
            }
        }
    }
    transaction.execute_batch(
        "DROP TABLE localized_content_native_v0;
         CREATE INDEX IF NOT EXISTS idx_localized_content_entity ON localized_content(entity_type,entity_id,locale);",
    ).map_err(|error| error.to_string())
}

fn legacy_localization_rows(connection: &Connection) -> Result<Vec<LegacyLocalizationRow>, String> {
    let mut statement = connection
        .prepare(
            "SELECT entity_type,entity_id,locale,fields_json,source_hash,created_at,updated_at
             FROM localized_content_native_v0",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
            ))
        })
        .map_err(|error| error.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_database_reaches_current_version() {
        let mut connection = Connection::open_in_memory().unwrap();

        apply(&mut connection).unwrap();

        assert_eq!(schema_version(&connection).unwrap(), CURRENT_SCHEMA_VERSION);
        assert!(table_exists(&connection, "captures").unwrap());
    }

    #[test]
    fn legacy_database_migrates_data_and_known_columns() {
        let mut connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                "CREATE TABLE problems(id TEXT PRIMARY KEY);
                 CREATE TABLE features(id TEXT PRIMARY KEY);
                 CREATE TABLE completion_playbooks(problem_id TEXT PRIMARY KEY,path TEXT NOT NULL,source_hash TEXT NOT NULL);
                 CREATE TABLE importance_assessments(id TEXT PRIMARY KEY);
                 CREATE TABLE provider_settings(id INTEGER PRIMARY KEY,base_url TEXT NOT NULL,model TEXT NOT NULL);
                 CREATE TABLE localized_content(
                   entity_type TEXT NOT NULL,entity_id TEXT NOT NULL,locale TEXT NOT NULL,
                   fields_json TEXT NOT NULL,source_hash TEXT NOT NULL DEFAULT '',
                   created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                   updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                   PRIMARY KEY(entity_type,entity_id,locale));
                 INSERT INTO localized_content(entity_type,entity_id,locale,fields_json)
                   VALUES ('captures','legacy','ko','{\"text\":\"기존 데이터\"}');",
            )
            .unwrap();

        apply(&mut connection).unwrap();

        assert_eq!(schema_version(&connection).unwrap(), CURRENT_SCHEMA_VERSION);
        for (table, column) in [
            ("problems", "detail"),
            ("features", "validation_criteria"),
            ("completion_playbooks", "lineage_snapshot_id"),
            ("importance_assessments", "created_at"),
            ("provider_settings", "advanced_model"),
            ("provider_settings", "async_worker_count"),
        ] {
            assert!(columns(&connection, table)
                .unwrap()
                .contains(&column.into()));
        }
        assert_eq!(
            connection
                .query_row(
                    "SELECT value FROM localized_content WHERE entity_type='captures' AND entity_id='legacy' AND field_name='text' AND locale='ko'",
                    [],
                    |row| row.get::<_, String>(0),
                )
                .unwrap(),
            "기존 데이터"
        );
    }

    #[test]
    fn applying_migrations_again_preserves_existing_data() {
        let mut connection = Connection::open_in_memory().unwrap();
        apply(&mut connection).unwrap();
        connection
            .execute("INSERT INTO captures(id,text) VALUES ('one','Keep me')", [])
            .unwrap();

        apply(&mut connection).unwrap();

        assert_eq!(
            connection
                .query_row("SELECT text FROM captures WHERE id='one'", [], |row| {
                    row.get::<_, String>(0)
                })
                .unwrap(),
            "Keep me"
        );
    }

    #[test]
    fn database_at_previous_version_advances_without_data_loss() {
        let mut connection = Connection::open_in_memory().unwrap();
        apply_plan(&mut connection, &MIGRATIONS[..1], 1).unwrap();
        connection
            .execute(
                "INSERT INTO captures(id,text) VALUES ('v1','Version one')",
                [],
            )
            .unwrap();

        apply(&mut connection).unwrap();

        assert_eq!(schema_version(&connection).unwrap(), CURRENT_SCHEMA_VERSION);
        assert_eq!(
            connection
                .query_row("SELECT text FROM captures WHERE id='v1'", [], |row| {
                    row.get::<_, String>(0)
                })
                .unwrap(),
            "Version one"
        );
    }

    #[test]
    fn newer_database_is_rejected_without_downgrading() {
        let mut connection = Connection::open_in_memory().unwrap();
        connection.pragma_update(None, "user_version", 99).unwrap();

        let error = apply(&mut connection).unwrap_err();

        assert!(error.contains("newer than supported"));
        assert_eq!(schema_version(&connection).unwrap(), 99);
    }

    #[test]
    fn failed_migration_rolls_back_schema_and_version() {
        fn fail_after_write(transaction: &Transaction<'_>) -> Result<(), String> {
            transaction
                .execute_batch("CREATE TABLE incomplete(id INTEGER);")
                .map_err(|error| error.to_string())?;
            Err("intentional failure".into())
        }
        let plan = [Migration {
            version: 1,
            name: "failing test migration",
            run: fail_after_write,
        }];
        let mut connection = Connection::open_in_memory().unwrap();

        let error = apply_plan(&mut connection, &plan, 1).unwrap_err();

        assert!(error.contains("migration 1"));
        assert!(!table_exists(&connection, "incomplete").unwrap());
        assert_eq!(schema_version(&connection).unwrap(), 0);
    }
}
