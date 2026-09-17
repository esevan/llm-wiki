use llm_wiki_desktop::NativeApplication;
use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};
use tempfile::tempdir;

const MIGRATED_SCHEMA_VERSION: i64 = 12;

fn make_v8_session_fixture(db: &std::path::Path, vault: &std::path::Path) {
    let app = NativeApplication::isolated(vault, db).unwrap();
    let service = app.work_tracking_service();
    let connection = service
        .create_connection(
            "v9 fixture",
            &[
                "session:read".into(),
                "session:write".into(),
                "workbench:current:read".into(),
            ],
            &[],
            "confirm_each",
        )
        .unwrap();
    let connection_id = connection["id"].as_str().unwrap();
    let open = serde_json::json!({
        "operationId":"v9-open",
        "lineageKey":"v9-fixture",
        "mode":"create",
        "capture":{"title":"V9 fixture","summary":"preserve this capture"}
    });
    let preview = service.open(connection_id, &open).unwrap();
    let session = service
        .finish_open(
            connection_id,
            &open,
            preview["reviewState"].as_str().unwrap(),
            "accept",
        )
        .unwrap();
    let event = service
        .append(
            connection_id,
            &serde_json::json!({
                "operationId":"v9-event",
                "sessionId":session["sessionId"],
                "expectedHeadRevision":1,
                "event":{"kind":"work_log_checkpoint","summary":"immutable event"}
            }),
        )
        .unwrap();
    let db_conn = Connection::open(db).unwrap();
    // Remove post-v8 additions before replaying migrations from the historical fixture.
    db_conn.execute_batch("DROP TABLE input_images; DROP TABLE task_auto_publications;
        DROP TABLE task_refinements; DROP TABLE task_subtasks; PRAGMA user_version=8;").unwrap();
    let parent = session["sessionId"].as_str().unwrap();
    let child = "v9-child";
    db_conn
        .execute(
            "INSERT INTO captures(id,text,source_mode,last_user_activity_at,created_at) VALUES('v9-child-capture','child capture','capture','2026-01-01','2026-01-01')",
            [],
        )
        .unwrap();
    db_conn
        .execute(
            "INSERT INTO work_tracking_sessions(id,connection_id,source_interface,conversation_ref_hash,capture_id,head_event_id,head_revision,state,publication_state,parent_session_id,created_at,updated_at) SELECT ?,connection_id,'fixture',?,'v9-child-capture','v9-child-event',1,'active','not_requested',id,created_at,updated_at FROM work_tracking_sessions WHERE id=?",
            params![child, "v9-child-lineage", parent],
        )
        .unwrap();
    let event_id = event["eventId"].as_str().unwrap();
    db_conn
        .execute(
            "INSERT INTO work_tracking_events(id,session_id,revision,previous_event_id,stream_id,source_sequence,kind,payload_json,payload_hash,occurred_at,ingested_at) SELECT 'v9-child-event',?,1,NULL,stream_id,1,'task_binding','{\"taskId\":\"task-v9\"}',payload_hash,occurred_at,ingested_at FROM work_tracking_events WHERE id=?",
            params![child, event_id],
        )
        .unwrap();
    db_conn
        .execute(
            "INSERT INTO work_tracking_links(id,session_id,source_event_id,entity_type,entity_id,relationship,created_at) VALUES('v9-link',?,'v9-child-event','tasks','task-v9','binds', '2026-01-01')",
            [child],
        )
        .unwrap();
    db_conn
        .execute(
            "INSERT INTO work_tracking_decisions(id,session_id,event_id,decision,accepted_payload_hash,decision_channel,created_at) VALUES('v9-decision',?,'v9-child-event','accepted','hash','fixture','2026-01-01')",
            [child],
        )
        .unwrap();
    db_conn
        .execute(
            "INSERT INTO work_tracking_projection_jobs(event_id,projection_name,state,created_at,updated_at) VALUES('v9-child-event','workflow','pending','2026-01-01','2026-01-01')",
            [],
        )
        .unwrap();
    let legacy_body = "# Historical Knowledge\n\nPreserve the original reviewed body.\n";
    let legacy_hash = format!("{:x}", Sha256::digest(legacy_body.as_bytes()));
    db_conn.execute("INSERT INTO knowledge_drafts(id,revision,session_id,completion_event_id,title,summary,body_markdown,content_hash,state,published_path,created_at) VALUES('legacy-knowledge',1,?,'v9-child-event','Historical Knowledge','Original summary',?,?,'published','knowledge/historical.md','2026-01-01')", params![parent,legacy_body,legacy_hash]).unwrap();
    db_conn.execute("INSERT INTO knowledge_publication_decisions(id,draft_id,draft_revision,content_hash,decision_channel,published_path,created_at) VALUES('legacy-publication','legacy-knowledge',1,?,'historical-reviewed','knowledge/historical.md','2026-01-01')",[&legacy_hash]).unwrap();
    let objects: Vec<(String, String, String)> = db_conn
        .prepare("SELECT type,name,sql FROM sqlite_master WHERE sql IS NOT NULL AND ((type='index' AND tbl_name='work_tracking_sessions') OR (type='trigger' AND sql LIKE '%work_tracking_sessions%')) ORDER BY type,name")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap().collect::<Result<_, _>>().unwrap();
    db_conn
        .execute_batch("PRAGMA foreign_keys=OFF; BEGIN IMMEDIATE;")
        .unwrap();
    for (kind, name, _) in &objects {
        if kind == "trigger" {
            db_conn
                .execute_batch(&format!("DROP TRIGGER \"{}\"", name.replace('"', "\"\"")))
                .unwrap();
        }
    }
    db_conn.execute_batch(
        "CREATE TABLE work_tracking_sessions_v8 (
           id TEXT PRIMARY KEY, connection_id TEXT, source_interface TEXT NOT NULL,
           conversation_ref_hash TEXT NOT NULL, capture_id TEXT NOT NULL UNIQUE,
           head_event_id TEXT NOT NULL, head_revision INTEGER NOT NULL,
           state TEXT NOT NULL DEFAULT 'active', publication_state TEXT NOT NULL DEFAULT 'not_requested',
           publication_offer_revision INTEGER, parent_session_id TEXT,
           created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
           UNIQUE(connection_id, conversation_ref_hash), FOREIGN KEY(connection_id) REFERENCES mcp_connections(id),
           FOREIGN KEY(capture_id) REFERENCES captures(id), FOREIGN KEY(parent_session_id) REFERENCES work_tracking_sessions(id)
         );
         INSERT INTO work_tracking_sessions_v8 SELECT * FROM work_tracking_sessions;
         DROP TABLE work_tracking_sessions;
         ALTER TABLE work_tracking_sessions_v8 RENAME TO work_tracking_sessions;"
    ).unwrap();
    for (_, _, sql) in &objects {
        db_conn.execute_batch(sql).unwrap();
    }
    db_conn.execute_batch(
        "CREATE VIEW v9_fixture_session_view AS SELECT id,parent_session_id,capture_id FROM work_tracking_sessions;
         CREATE VIEW v9_fixture_indirect_view AS SELECT id FROM v9_fixture_session_view;
         CREATE INDEX v9_fixture_session_index ON work_tracking_sessions(source_interface,updated_at);
         CREATE TRIGGER v9_fixture_session_trigger AFTER UPDATE OF state ON work_tracking_sessions BEGIN UPDATE mcp_connections SET updated_at=NEW.updated_at WHERE id=NEW.connection_id; END;
         PRAGMA user_version=8; COMMIT; PRAGMA foreign_keys=ON;"
    ).unwrap();
    let fk_errors: i64 = db_conn
        .query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |row| {
            row.get(0)
        })
        .unwrap();
    if fk_errors != 0 {
        let mut check = db_conn
            .prepare("SELECT * FROM pragma_foreign_key_check")
            .unwrap();
        let rows: Vec<(String, i64, String, i64)> = check
            .query_map([], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })
            .unwrap()
            .map(|row| row.unwrap())
            .collect();
        panic!("invalid v8 fixture foreign keys: {rows:?}");
    }
    assert_eq!(fk_errors, 0, "invalid v8 fixture foreign keys");
    db_conn.close().unwrap();
}

#[test]
fn v9_preserves_sessions_children_events_and_custom_objects_and_allows_null_capture() {
    let root = tempdir().unwrap();
    let db = root.path().join("state.db");
    make_v8_session_fixture(&db, &root.path().join("vault"));
    let before_connection = Connection::open(&db).unwrap();
    let before = preserved_rows(&before_connection);
    let parent: String = before_connection
        .query_row(
            "SELECT parent_session_id FROM work_tracking_sessions WHERE id='v9-child'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    drop(before_connection);
    NativeApplication::isolated(&root.path().join("vault"), &db).unwrap();
    let connection = Connection::open(&db).unwrap();
    assert_eq!(preserved_rows(&connection), before);
    assert_eq!(
        connection
            .query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap(),
        0
    );
    assert_eq!(
        connection
            .pragma_query_value(None, "foreign_keys", |row| row.get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        connection
            .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
            .unwrap(),
        MIGRATED_SCHEMA_VERSION
    );
    assert_eq!(
        connection
            .query_row("SELECT count(*) FROM work_tracking_sessions", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        2
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT parent_session_id FROM work_tracking_sessions WHERE id='v9-child'",
                [],
                |row| row.get::<_, String>(0)
            )
            .unwrap(),
        parent
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT payload_json FROM work_tracking_events WHERE id='v9-child-event'",
                [],
                |row| row.get::<_, String>(0)
            )
            .unwrap(),
        r#"{"taskId":"task-v9"}"#
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT count(*) FROM work_tracking_links WHERE id='v9-link'",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        1
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT count(*) FROM work_tracking_decisions WHERE id='v9-decision'",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        1
    );
    assert_eq!(connection.query_row("SELECT count(*) FROM work_tracking_projection_jobs WHERE event_id='v9-child-event'", [], |row| row.get::<_, i64>(0)).unwrap(), 1);
    assert!(connection
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='index' AND name='v9_fixture_session_index'",
            [],
            |_| Ok(())
        )
        .is_ok());
    assert!(connection.query_row("SELECT 1 FROM sqlite_master WHERE type='trigger' AND name='v9_fixture_session_trigger'", [], |_| Ok(())).is_ok());
    assert!(connection.execute("INSERT INTO work_tracking_sessions(id,connection_id,source_interface,conversation_ref_hash,capture_id,head_event_id,head_revision,created_at,updated_at) SELECT 'duplicate-lineage',connection_id,source_interface,conversation_ref_hash,NULL,head_event_id,head_revision,created_at,updated_at FROM work_tracking_sessions WHERE id='v9-child'", []).is_err());
    let view_rows: i64 = connection
        .query_row("SELECT count(*) FROM v9_fixture_session_view", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(view_rows, before[0].len() as i64);
    connection.execute("UPDATE work_tracking_sessions SET state=state,updated_at='fixture-trigger-restored' WHERE id='v9-child'", []).unwrap();
    assert_eq!(connection.query_row("SELECT updated_at FROM mcp_connections WHERE id=(SELECT connection_id FROM work_tracking_sessions WHERE id='v9-child')", [], |row| row.get::<_, String>(0)).unwrap(), "fixture-trigger-restored");
    connection.execute("INSERT INTO work_tracking_sessions(id,connection_id,source_interface,conversation_ref_hash,capture_id,head_event_id,head_revision,created_at,updated_at) VALUES('v9-null',NULL,'fixture','v9-null-lineage',NULL,'v9-child-event',1,'2026-01-01','2026-01-01')", []).unwrap();
    connection.execute("INSERT INTO work_tracking_sessions(id,connection_id,source_interface,conversation_ref_hash,capture_id,head_event_id,head_revision,created_at,updated_at) VALUES('v9-null-2',NULL,'fixture','v9-null-lineage-2',NULL,'v9-child-event',1,'2026-01-01','2026-01-01')", []).unwrap();
    assert!(connection.execute("INSERT INTO work_tracking_sessions(id,connection_id,source_interface,conversation_ref_hash,capture_id,head_event_id,head_revision,created_at,updated_at) SELECT 'duplicate-capture',NULL,'fixture','dup',capture_id,head_event_id,head_revision,created_at,updated_at FROM work_tracking_sessions WHERE id='v9-child'", []).is_err());
    assert!(connection
        .query_row(
            "SELECT capture_id FROM work_tracking_sessions WHERE id='v9-null'",
            [],
            |row| row.get::<_, Option<String>>(0)
        )
        .unwrap()
        .is_none());
}

#[test]
fn v9_failure_rolls_back_version_and_retry_succeeds_with_backup_manifest() {
    let root = tempdir().unwrap();
    let db = root.path().join("state.db");
    make_v8_session_fixture(&db, &root.path().join("vault"));
    let original = preserved_rows(&Connection::open(&db).unwrap());
    Connection::open(&db)
        .unwrap()
        .execute_batch("CREATE TABLE work_tracking_sessions_v9(id TEXT); PRAGMA user_version=8;")
        .unwrap();
    let error = match NativeApplication::isolated(&root.path().join("vault"), &db) {
        Ok(_) => panic!("collision must fail migration"),
        Err(error) => error,
    };
    assert!(error.contains("migration_failed"));
    let connection = Connection::open(&db).unwrap();
    assert_eq!(preserved_rows(&connection), original);
    assert_eq!(
        connection
            .query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap(),
        0
    );
    assert_eq!(
        connection
            .pragma_query_value(None, "foreign_keys", |row| row.get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        connection
            .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
            .unwrap(),
        8
    );
    assert_eq!(
        connection
            .query_row("SELECT count(*) FROM work_tracking_sessions", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        2
    );
    verify_v8_backup(root.path());
    drop(connection);
    Connection::open(&db)
        .unwrap()
        .execute("DROP TABLE work_tracking_sessions_v9", [])
        .unwrap();
    let recovery = NativeApplication::isolated(&root.path().join("vault"), &db).unwrap();
    assert!(recovery.migration_recovery_status().unwrap()["recovery"].is_object());
    assert_eq!(
        recovery.execute(llm_wiki_desktop::NativeOperation {
            name: "task.create".into(),
            input: serde_json::json!({"operationId":"blocked-before-retry","inputText":"blocked","title":"blocked"}),
        }).status,
        503
    );
    assert_eq!(recovery.retry_migration().unwrap()["recovered"], true);
    assert_eq!(preserved_rows(&Connection::open(&db).unwrap()), original);
    assert_eq!(
        Connection::open(&db)
            .unwrap()
            .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
            .unwrap(),
        MIGRATED_SCHEMA_VERSION
    );
}

#[test]
fn newer_schema_is_rejected_without_mutation() {
    let root = tempdir().unwrap();
    let db = root.path().join("state.db");
    NativeApplication::isolated(&root.path().join("vault"), &db).unwrap();
    Connection::open(&db)
        .unwrap()
        .execute_batch("PRAGMA user_version=99;")
        .unwrap();
    let error = match NativeApplication::isolated(&root.path().join("vault"), &db) {
        Ok(_) => panic!("newer schema must be rejected"),
        Err(error) => error,
    };
    assert!(error.contains("newer than supported"));
    assert_eq!(
        Connection::open(&db)
            .unwrap()
            .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
            .unwrap(),
        99
    );
}

fn preserved_rows(connection: &Connection) -> Vec<Vec<Vec<rusqlite::types::Value>>> {
    [
        "work_tracking_sessions",
        "work_tracking_events",
        "work_tracking_decisions",
        "work_tracking_links",
        "work_tracking_idempotency_records",
        "work_tracking_projection_jobs",
        "work_tracking_projection_results",
        "work_tracking_stream_watermarks",
        "work_tracking_reviews",
        "knowledge_drafts",
        "knowledge_publication_decisions",
    ]
    .into_iter()
    .map(|table| {
        let mut statement = connection
            .prepare(&format!("SELECT * FROM {table} ORDER BY 1,2"))
            .unwrap();
        let count = statement.column_count();
        statement
            .query_map([], |row| {
                (0..count)
                    .map(|column| row.get(column))
                    .collect::<Result<Vec<rusqlite::types::Value>, _>>()
            })
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    })
    .collect()
}
fn verify_v8_backup(root: &std::path::Path) {
    let manifest = std::fs::read_dir(root)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .contains("manifest")
        })
        .expect("v8 backup manifest");
    let value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&manifest).unwrap()).unwrap();
    assert_eq!(value["sourceSchemaVersion"], 8);
    assert_eq!(value["targetSchemaVersion"], MIGRATED_SCHEMA_VERSION);
    let backup = manifest
        .parent()
        .unwrap()
        .join(value["backupFile"].as_str().unwrap());
    let bytes = std::fs::read(&backup).unwrap();
    assert_eq!(value["size"].as_u64().unwrap(), bytes.len() as u64);
    assert_eq!(
        value["sha256"].as_str().unwrap(),
        format!("{:x}", Sha256::digest(&bytes))
    );
    let connection = Connection::open(backup).unwrap();
    assert_eq!(
        connection
            .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
            .unwrap(),
        8
    );
    assert_eq!(
        connection
            .query_row("PRAGMA integrity_check", [], |row| row.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
}
