use rusqlite::{params, Connection, Transaction, TransactionBehavior};
use serde_json::Value;

pub const CURRENT_SCHEMA_VERSION: i64 = 5;

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
    Migration {
        version: 3,
        name: "normalize AI job defaults",
        run: normalize_ai_jobs,
    },
    Migration {
        version: 4,
        name: "add dual-chat work tracking",
        run: add_work_tracking,
    },
    Migration {
        version: 5,
        name: "add bound work reviews",
        run: add_bound_reviews,
    },
];

fn add_bound_reviews(tx: &Transaction<'_>) -> Result<(), String> {
    tx.execute_batch("CREATE TABLE work_tracking_reviews (
      id TEXT PRIMARY KEY, connection_id TEXT NOT NULL REFERENCES mcp_connections(id),
      operation_id TEXT NOT NULL, action TEXT NOT NULL, payload_hash TEXT NOT NULL,
      payload_json TEXT NOT NULL, state TEXT NOT NULL DEFAULT 'pending',
      expires_at TEXT NOT NULL, created_at TEXT NOT NULL,
      UNIQUE(connection_id,operation_id,action)
    ); CREATE TABLE work_tracking_workspace (id INTEGER PRIMARY KEY CHECK(id=1), revision INTEGER NOT NULL DEFAULT 0, selection_json TEXT);
    INSERT INTO work_tracking_workspace(id) VALUES(1);
    CREATE TABLE work_tracking_publication_jobs(review_id TEXT PRIMARY KEY REFERENCES work_tracking_reviews(id),state TEXT NOT NULL DEFAULT 'pending',attempts INTEGER NOT NULL DEFAULT 0,next_attempt_at TEXT,safe_error_code TEXT);
    CREATE TABLE work_tracking_entity_versions(entity_type TEXT NOT NULL,entity_id TEXT NOT NULL,revision INTEGER NOT NULL DEFAULT 0,PRIMARY KEY(entity_type,entity_id));
    ALTER TABLE mcp_elicitation_challenges ADD COLUMN target_snapshot_json TEXT;
    CREATE TABLE work_tracking_topic_memberships(topic_id TEXT NOT NULL,entity_type TEXT NOT NULL,entity_id TEXT NOT NULL,PRIMARY KEY(topic_id,entity_type,entity_id));
    CREATE TABLE knowledge_publication_undos(review_id TEXT PRIMARY KEY,draft_id TEXT NOT NULL,draft_revision INTEGER NOT NULL,recovery_path TEXT NOT NULL,created_at TEXT NOT NULL);
    CREATE INDEX work_tracking_pending_projection_order ON work_tracking_projection_jobs(created_at) WHERE state IN ('pending','claimed') AND attempts<5;
    CREATE TRIGGER topic_membership_removed AFTER DELETE ON work_tracking_topic_memberships BEGIN DELETE FROM work_tracking_evidence_grants WHERE scope_kind='topic' AND scope_target=OLD.topic_id AND path=OLD.entity_id; UPDATE work_tracking_workspace SET revision=revision+1 WHERE id=1; END;
    CREATE TRIGGER topic_membership_added AFTER INSERT ON work_tracking_topic_memberships BEGIN UPDATE work_tracking_workspace SET revision=revision+1 WHERE id=1; END;").map_err(|error| error.to_string())?;
    for table in [
        "captures",
        "problems",
        "features",
        "solution_progress_entries",
        "completions",
        "work_tracking_sessions",
    ] {
        let exists: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?)",
                [table],
                |row| row.get(0),
            )
            .map_err(|error| error.to_string())?;
        if !exists {
            continue;
        }
        tx.execute_batch(&format!("INSERT INTO work_tracking_entity_versions(entity_type,entity_id) SELECT '{table}',id FROM {table};")).map_err(|error|error.to_string())?;
        for action in ["INSERT", "UPDATE", "DELETE"] {
            let row = if action == "DELETE" { "OLD" } else { "NEW" };
            tx.execute_batch(&format!("CREATE TRIGGER tracked_revision_{table}_{action} AFTER {action} ON {table} BEGIN UPDATE work_tracking_workspace SET revision=revision+1 WHERE id=1; INSERT INTO work_tracking_entity_versions(entity_type,entity_id,revision) VALUES('{table}',{row}.id,1) ON CONFLICT(entity_type,entity_id) DO UPDATE SET revision=revision+1; END;")).map_err(|error|error.to_string())?;
        }
    }
    for (table,related,actions) in [
        ("captures","s.capture_id={row}.id",vec!["UPDATE"]),
        ("solution_checklist_items","s.id IN (SELECT session_id FROM work_tracking_links WHERE entity_type='features' AND entity_id={row}.feature_id)",vec!["INSERT","UPDATE","DELETE"]),
        ("solution_progress_comments","s.id IN (SELECT l.session_id FROM work_tracking_links l JOIN solution_progress_entries p ON p.feature_id=l.entity_id WHERE l.entity_type='features' AND p.id={row}.entry_id)",vec!["INSERT","UPDATE","DELETE"]),
        ("solution_progress_entries","s.id IN (SELECT session_id FROM work_tracking_links WHERE entity_type='features' AND entity_id={row}.feature_id)",vec!["UPDATE","DELETE"]),
        ("completion_reviews","s.id IN (SELECT session_id FROM work_tracking_links WHERE entity_type='features' AND entity_id={row}.feature_id)",vec!["INSERT","UPDATE","DELETE"]),
        ("completions","s.id IN (SELECT session_id FROM work_tracking_links WHERE entity_type='features' AND entity_id={row}.feature_id)",vec!["INSERT","UPDATE","DELETE"]),
    ] {
        let exists:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?)",[table],|row|row.get(0)).map_err(|error|error.to_string())?;if !exists{continue;}
        for action in actions {
            let row=if action=="DELETE"{"OLD"}else{"NEW"};let related=related.replace("{row}",row);
            tx.execute_batch(&format!("CREATE TRIGGER tracked_detail_{table}_{action} AFTER {action} ON {table} BEGIN
            INSERT INTO work_tracking_events(id,session_id,revision,previous_event_id,stream_id,source_sequence,kind,payload_json,payload_hash,occurred_at,ingested_at)
            SELECT lower(hex(randomblob(16))),s.id,s.head_revision+1,s.head_event_id,'workbench',COALESCE((SELECT MAX(source_sequence)+1 FROM work_tracking_events WHERE session_id=s.id AND stream_id='workbench'),1),'workflow_link',json_object('entityType','{table}','entityId',{row}.id,'operation','{action}'),lower(hex(randomblob(32))),strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now') FROM work_tracking_sessions s WHERE {related};
            UPDATE work_tracking_sessions SET head_revision=head_revision+1,head_event_id=(SELECT id FROM work_tracking_events WHERE session_id=work_tracking_sessions.id ORDER BY revision DESC LIMIT 1),updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id IN (SELECT s.id FROM work_tracking_sessions s WHERE {related});
            END;")).map_err(|error|error.to_string())?;
        }
    }
    Ok(())
}

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

fn normalize_ai_jobs(transaction: &Transaction<'_>) -> Result<(), String> {
    if !table_exists(transaction, "ai_jobs_v2")? {
        return Ok(());
    }
    transaction
        .execute_batch(
            "CREATE TABLE ai_jobs_v3 (
               id TEXT PRIMARY KEY, task_kind TEXT NOT NULL, entity_type TEXT NOT NULL DEFAULT '',
               entity_id TEXT NOT NULL DEFAULT '', status TEXT NOT NULL,
               input_json TEXT NOT NULL DEFAULT '{}', result_json TEXT NOT NULL DEFAULT '{}',
               source_hash TEXT NOT NULL DEFAULT '', model TEXT NOT NULL DEFAULT '',
               execution_mode TEXT NOT NULL DEFAULT 'native', idempotency_key TEXT NOT NULL DEFAULT '',
               result_interface TEXT NOT NULL DEFAULT 'inline_preview',
               notification_policy TEXT NOT NULL DEFAULT 'none',
               progress_completed INTEGER NOT NULL DEFAULT 0, progress_total INTEGER NOT NULL DEFAULT 1,
               attempt INTEGER NOT NULL DEFAULT 0, worker_id TEXT NOT NULL DEFAULT '',
               lease_token TEXT NOT NULL DEFAULT '', lease_expires_at TEXT, heartbeat_at TEXT,
               available_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
               error_code TEXT NOT NULL DEFAULT '', error_message TEXT NOT NULL DEFAULT '',
               created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, started_at TEXT, finished_at TEXT
             );
             INSERT INTO ai_jobs_v3(
               id,task_kind,entity_type,entity_id,status,input_json,result_json,source_hash,model,
               execution_mode,idempotency_key,result_interface,notification_policy,
               progress_completed,progress_total,attempt,worker_id,lease_token,lease_expires_at,
               heartbeat_at,available_at,error_code,error_message,created_at,started_at,finished_at
             )
             SELECT
               id,task_kind,entity_type,entity_id,status,input_json,result_json,source_hash,model,
               execution_mode,idempotency_key,result_interface,notification_policy,
               progress_completed,progress_total,attempt,worker_id,lease_token,lease_expires_at,
               heartbeat_at,available_at,error_code,error_message,created_at,started_at,finished_at
             FROM ai_jobs_v2;
             DROP TABLE ai_jobs_v2;
             ALTER TABLE ai_jobs_v3 RENAME TO ai_jobs_v2;",
        )
        .map_err(|error| error.to_string())
}

fn add_work_tracking(transaction: &Transaction<'_>) -> Result<(), String> {
    for (column, declaration) in [
        ("model", "TEXT NOT NULL DEFAULT ''"),
        ("source_revision", "TEXT NOT NULL DEFAULT ''"),
        ("context_scope", "TEXT NOT NULL DEFAULT ''"),
        ("retrieval_snapshot_hash", "TEXT NOT NULL DEFAULT ''"),
    ] {
        add_missing_column(transaction, "ai_runs", column, declaration)?;
    }
    if table_exists(transaction, "mcp_connections")? {
        add_missing_column(
            transaction,
            "mcp_connections",
            "allowed_topics_json",
            "TEXT NOT NULL DEFAULT '[]'",
        )?;
    }
    let schema = include_str!("work_tracking_schema.sql");
    let (tables, triggers) = schema
        .split_once("-- Existing Workbench mutations")
        .ok_or("Work-tracking schema marker is missing")?;
    transaction
        .execute_batch(tables)
        .map_err(|error| error.to_string())?;
    if table_exists(transaction, "problems")?
        && table_exists(transaction, "features")?
        && table_exists(transaction, "solution_progress_entries")?
    {
        transaction
            .execute_batch(&format!("-- Existing Workbench mutations{triggers}"))
            .map_err(|error| error.to_string())?;
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
    fn legacy_ai_job_schema_gains_safe_defaults_without_losing_history() {
        let mut connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                "CREATE TABLE ai_jobs_v2 (
                   id TEXT PRIMARY KEY, task_kind TEXT NOT NULL, entity_type TEXT NOT NULL DEFAULT '',
                   entity_id TEXT NOT NULL DEFAULT '', status TEXT NOT NULL,
                   input_json TEXT NOT NULL DEFAULT '{}', result_json TEXT NOT NULL DEFAULT '{}',
                   source_hash TEXT NOT NULL DEFAULT '', model TEXT NOT NULL DEFAULT '',
                   execution_mode TEXT NOT NULL, idempotency_key TEXT NOT NULL DEFAULT '',
                   result_interface TEXT NOT NULL DEFAULT 'none',
                   notification_policy TEXT NOT NULL DEFAULT 'none',
                   progress_completed INTEGER NOT NULL DEFAULT 0, progress_total INTEGER NOT NULL DEFAULT 0,
                   attempt INTEGER NOT NULL DEFAULT 0, worker_id TEXT NOT NULL DEFAULT '',
                   lease_token TEXT NOT NULL DEFAULT '', lease_expires_at TEXT, heartbeat_at TEXT,
                   available_at TEXT NOT NULL, error_code TEXT NOT NULL DEFAULT '',
                   error_message TEXT NOT NULL DEFAULT '', created_at TEXT NOT NULL,
                   started_at TEXT, finished_at TEXT
                 );
                 INSERT INTO ai_jobs_v2(
                   id,task_kind,status,execution_mode,available_at,created_at
                 ) VALUES ('legacy','workflow_draft','completed','asynchronous','before','before');
                 PRAGMA user_version=2;",
            )
            .unwrap();

        apply(&mut connection).unwrap();

        assert_eq!(schema_version(&connection).unwrap(), CURRENT_SCHEMA_VERSION);
        assert_eq!(
            connection
                .query_row(
                    "SELECT execution_mode,available_at,created_at FROM ai_jobs_v2 WHERE id='legacy'",
                    [],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?)),
                )
                .unwrap(),
            ("asynchronous".into(), "before".into(), "before".into())
        );
        connection
            .execute(
                "INSERT INTO ai_jobs_v2(id,task_kind,status) VALUES ('new','workflow_draft','queued')",
                [],
            )
            .unwrap();
        assert_eq!(
            connection
                .query_row(
                    "SELECT execution_mode,result_interface,progress_total FROM ai_jobs_v2 WHERE id='new'",
                    [],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, i64>(2)?)),
                )
                .unwrap(),
            ("native".into(), "inline_preview".into(), 1)
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
    fn work_tracking_database_advances_to_bound_reviews() {
        let mut connection = Connection::open_in_memory().unwrap();
        apply_plan(&mut connection, &MIGRATIONS[..4], 4).unwrap();
        assert_eq!(schema_version(&connection).unwrap(), 4);

        apply(&mut connection).unwrap();

        assert_eq!(schema_version(&connection).unwrap(), CURRENT_SCHEMA_VERSION);
        assert!(table_exists(&connection, "work_tracking_reviews").unwrap());
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
