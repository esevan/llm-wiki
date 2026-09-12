use llm_wiki_desktop::NativeApplication;
use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};
use tempfile::tempdir;

const NATIVE_CONNECTION: &str = "native-in-app-chat";

fn sha256(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

fn open_legacy_session(app: &NativeApplication, operation_id: &str) -> String {
    let service = app.work_tracking_service();
    let request = serde_json::json!({
        "operationId": operation_id,
        "lineageKey": format!("legacy-{operation_id}"),
        "mode": "create",
        "capture": {"title": "Historical session", "summary": "Preserve reviewed history"}
    });
    let preview = service.open(NATIVE_CONNECTION, &request).unwrap();
    service
        .finish_open(
            NATIVE_CONNECTION,
            &request,
            preview["reviewState"].as_str().unwrap(),
            "accept",
        )
        .unwrap()["sessionId"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn insert_legacy_publication_job(
    connection: &Connection,
    id: &str,
    session_id: &str,
    body: &str,
    state: &str,
) -> String {
    let draft_id = format!("legacy-draft-{id}");
    let hash = sha256(body);
    let path = format!("knowledge/{id}.md");
    connection
        .execute(
            "INSERT INTO knowledge_drafts(id,revision,session_id,completion_event_id,title,summary,body_markdown,content_hash,state,published_path,created_at)
             VALUES(?,1,?,'historical-completion','Historical Knowledge','Exact historical body',?,?,?,?,'2026-01-01')",
            params![draft_id, session_id, body, hash, "draft", path],
        )
        .unwrap();
    let payload = serde_json::json!({
        "draftId": draft_id,
        "expectedDraftRevision": 1,
        "expectedContentHash": hash,
    });
    connection
        .execute(
            "INSERT INTO work_tracking_reviews(id,connection_id,operation_id,action,payload_hash,payload_json,state,expires_at,created_at)
             VALUES(?,?,?,'publish',?,?,?,'2999-01-01T00:00:00Z','2026-01-01')",
            params![
                id,
                NATIVE_CONNECTION,
                format!("historical-{id}"),
                sha256(&payload.to_string()),
                payload.to_string(),
                state,
            ],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO work_tracking_publication_jobs(review_id,state,attempts) VALUES(?,'pending',0)",
            [id],
        )
        .unwrap();
    path
}

#[test]
fn only_accepted_legacy_publication_replays_its_exact_historical_bytes() {
    let root = tempdir().unwrap();
    let vault = root.path().join("vault");
    let database = root.path().join("history.sqlite");
    let app = NativeApplication::isolated(&vault, &database).unwrap();
    let session = open_legacy_session(&app, "open-legacy-recovery");
    let connection = Connection::open(&database).unwrap();
    let accepted_body = "# Historical Knowledge\n\nKeep these exact reviewed bytes.\n";
    let accepted_path = insert_legacy_publication_job(
        &connection,
        "accepted-historical-publication",
        &session,
        accepted_body,
        "accept",
    );
    let skipped_path = insert_legacy_publication_job(
        &connection,
        "pending-historical-publication",
        &session,
        "# Not approved\n",
        "pending",
    );
    let guarded_body = "# Exact historical source\n";
    let guarded_path = insert_legacy_publication_job(
        &connection,
        "accepted-historical-conflict",
        &session,
        guarded_body,
        "accept",
    );
    drop(connection);
    let guarded_file = vault.join(&guarded_path);
    std::fs::create_dir_all(guarded_file.parent().unwrap()).unwrap();
    std::fs::write(&guarded_file, "# Externally replaced bytes\n").unwrap();

    let processed = app.work_tracking_service().drain(20).unwrap();
    assert!(processed <= 20);
    assert_eq!(
        std::fs::read_to_string(vault.join(&accepted_path)).unwrap(),
        accepted_body
    );
    assert!(
        !vault.join(&skipped_path).exists(),
        "an unaccepted legacy review must never publish"
    );
    assert_eq!(
        std::fs::read_to_string(&guarded_file).unwrap(),
        "# Externally replaced bytes\n",
        "accepted legacy recovery must not overwrite changed external bytes"
    );

    let connection = Connection::open(&database).unwrap();
    let (state, path, body): (String, String, String) = connection
        .query_row(
            "SELECT state,published_path,body_markdown FROM knowledge_drafts WHERE id='legacy-draft-accepted-historical-publication' AND revision=1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(state, "published");
    assert_eq!(path, accepted_path);
    assert_eq!(body, accepted_body);
    assert_eq!(
        connection
            .query_row(
                "SELECT state FROM work_tracking_publication_jobs WHERE review_id='accepted-historical-publication'",
                [],
                |row| row.get::<_, String>(0),
            )
            .unwrap(),
        "complete"
    );
    assert_eq!(
        connection
            .query_row("SELECT count(*) FROM task_knowledge_drafts", [], |row| row
                .get::<_, i64>(
                0
            ))
            .unwrap(),
        0,
        "legacy recovery must not create a new canonical draft"
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT state FROM work_tracking_publication_jobs WHERE review_id='pending-historical-publication'",
                [],
                |row| row.get::<_, String>(0),
            )
            .unwrap(),
        "pending"
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT state FROM knowledge_drafts WHERE id='legacy-draft-accepted-historical-conflict' AND revision=1",
                [],
                |row| row.get::<_, String>(0),
            )
            .unwrap(),
        "draft"
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT safe_error_code FROM work_tracking_publication_jobs WHERE review_id='accepted-historical-conflict'",
                [],
                |row| row.get::<_, Option<String>>(0),
            )
            .unwrap()
            .as_deref(),
        Some("publish_conflict")
    );
}
