use llm_wiki_desktop::NativeApplication;
use tempfile::tempdir;

#[test]
fn work_tracking_schema_is_complete_and_idempotent() {
    let root = tempdir().unwrap();
    let db = root.path().join("state.sqlite3");
    NativeApplication::isolated(&root.path().join("vault"), &db).unwrap();
    NativeApplication::isolated(&root.path().join("vault"), &db).unwrap();
    let connection = rusqlite::Connection::open(db).unwrap();
    for table in [
        "input_images",
        "mcp_connections",
        "work_tracking_evidence_grants",
        "work_tracking_sessions",
        "work_tracking_events",
        "work_tracking_idempotency_records",
        "work_tracking_projection_jobs",
        "work_tracking_projection_results",
        "work_tracking_stream_watermarks",
        "work_tracking_decisions",
        "work_tracking_links",
        "knowledge_drafts",
        "knowledge_publication_decisions",
    ] {
        let count: i64 = connection
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name=?",
                [table],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "missing {table}");
    }
    assert_eq!(
        connection
            .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
            .unwrap(),
        15
    );
}

#[test]
fn event_and_projection_uniqueness_is_enforced() {
    let root = tempdir().unwrap();
    let db = root.path().join("state.sqlite3");
    NativeApplication::isolated(&root.path().join("vault"), &db).unwrap();
    let connection = rusqlite::Connection::open(db).unwrap();
    let event_sql="INSERT INTO work_tracking_events(id,session_id,revision,stream_id,source_sequence,kind,payload_json,payload_hash,occurred_at,ingested_at) VALUES ('event','missing',1,'stream',1,'capture','{}','hash','2026-01-01','2026-01-01')";
    assert!(
        connection.execute(event_sql, []).is_err(),
        "foreign keys must be active"
    );
}
