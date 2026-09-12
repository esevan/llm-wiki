use llm_wiki_desktop::{NativeApplication, NativeOperation};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use tempfile::tempdir;
use tokio::task::JoinHandle;

fn send(stdin: &mut impl Write, value: Value) {
    writeln!(stdin, "{value}").unwrap();
    stdin.flush().unwrap();
}
fn receive(reader: &mut impl BufRead) -> Value {
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    assert!(!line.is_empty(), "MCP server closed unexpectedly");
    serde_json::from_str(&line).unwrap()
}

fn ipc_endpoint(root: &std::path::Path) -> String {
    #[cfg(unix)]
    return root.join("ipc/mcp.sock").to_string_lossy().into_owned();
    #[cfg(windows)]
    return format!(r"\\.\pipe\llm-wiki-test-{}", uuid::Uuid::new_v4());
}

async fn start_gui_ipc(app: &NativeApplication, endpoint: &str) -> JoinHandle<()> {
    let service = app.work_tracking_service();
    let listener_endpoint = endpoint.to_owned();
    let listener = tokio::spawn(async move {
        llm_wiki_desktop::mcp_ipc::run_gui_listener_at(service, listener_endpoint)
            .await
            .unwrap();
    });
    #[cfg(unix)]
    for _ in 0..100 {
        if std::path::Path::new(endpoint).exists() {
            return listener;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    #[cfg(windows)]
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    listener
}

#[test]
fn stdio_bridge_fails_closed_without_the_gui_and_never_opens_persistence() {
    let root = tempdir().unwrap();
    let endpoint = ipc_endpoint(root.path());
    let forbidden_db = root.path().join("must-not-be-created.sqlite");
    let output = Command::new(env!("CARGO_BIN_EXE_llm-wiki-desktop"))
        .args(["--mcp", "--connection", "unreachable"])
        .env("LLM_WIKI_MCP_ENDPOINT", endpoint)
        .env("LLM_WIKI_DB", &forbidden_db)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(!forbidden_db.exists());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Open LLM Wiki before using MCP"));
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn ipc_rejects_native_unknown_and_revoked_principals() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let root = tempdir().unwrap();
    let app =
        NativeApplication::isolated(&root.path().join("vault"), &root.path().join("db.sqlite"))
            .unwrap();
    let service = app.work_tracking_service();
    let grant = service
        .create_connection("revoked", &["session:read".into()], &[], "confirm_each")
        .unwrap();
    let revoked = grant["id"].as_str().unwrap();
    service.revoke_connection(revoked).unwrap();
    let endpoint = ipc_endpoint(root.path());
    let listener = start_gui_ipc(&app, &endpoint).await;
    for principal in ["native-in-app-chat", "missing", revoked] {
        let mut socket = tokio::net::UnixStream::connect(&endpoint).await.unwrap();
        socket
            .write_all(format!("{principal}\n").as_bytes())
            .await
            .unwrap();
        let mut byte = [0];
        let closed =
            tokio::time::timeout(std::time::Duration::from_secs(2), socket.read(&mut byte))
                .await
                .expect("unauthorized connection must be closed promptly");
        assert!(
            matches!(closed, Ok(0) | Err(_)),
            "unauthorized principal received protocol data"
        );
    }
    listener.abort();
    let _ = listener.await;
}

#[cfg(unix)]
#[tokio::test]
async fn ipc_never_replaces_a_regular_file_or_symlink() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let app =
        NativeApplication::isolated(&root.path().join("vault"), &root.path().join("db.sqlite"))
            .unwrap();
    let file = root.path().join("precious.txt");
    std::fs::write(&file, "preserve me").unwrap();
    let symlink = root.path().join("mcp.sock");
    std::os::unix::fs::symlink(&file, &symlink).unwrap();
    for path in [&file, &symlink] {
        let result = llm_wiki_desktop::mcp_ipc::run_gui_listener_at(
            app.work_tracking_service(),
            path.to_string_lossy().into_owned(),
        )
        .await;
        assert!(result.unwrap_err().contains("not a socket"));
    }
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "preserve me");
    assert!(std::fs::symlink_metadata(&symlink)
        .unwrap()
        .file_type()
        .is_symlink());
}

#[cfg(unix)]
#[tokio::test]
async fn ipc_shutdown_removes_its_owned_socket() {
    use std::os::unix::fs::PermissionsExt;

    let root = tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let app =
        NativeApplication::isolated(&root.path().join("vault"), &root.path().join("db.sqlite"))
            .unwrap();
    let endpoint = ipc_endpoint(root.path());
    let listener = start_gui_ipc(&app, &endpoint).await;
    assert!(std::path::Path::new(&endpoint).exists());

    listener.abort();
    let _ = listener.await;

    assert!(!std::path::Path::new(&endpoint).exists());
}

#[cfg(unix)]
#[tokio::test]
async fn ipc_registered_shutdown_closes_and_removes_its_socket() {
    use std::os::unix::fs::PermissionsExt;

    let root = tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let app =
        NativeApplication::isolated(&root.path().join("vault"), &root.path().join("db.sqlite"))
            .unwrap();
    let endpoint = ipc_endpoint(root.path());
    let lifecycle = llm_wiki_desktop::mcp_ipc::McpListenerShutdown::default();
    let service = app.work_tracking_service();
    let listener_endpoint = endpoint.clone();
    let listener_lifecycle = lifecycle.clone();
    let listener = tokio::spawn(async move {
        llm_wiki_desktop::mcp_ipc::run_gui_listener_at_with_shutdown(
            service,
            listener_endpoint,
            listener_lifecycle,
        )
        .await
        .unwrap();
    });
    for _ in 0..100 {
        if std::path::Path::new(&endpoint).exists() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert!(std::path::Path::new(&endpoint).exists());

    lifecycle.shutdown();
    tokio::time::timeout(std::time::Duration::from_secs(2), listener)
        .await
        .expect("registered shutdown must stop the listener")
        .unwrap();

    assert!(!std::path::Path::new(&endpoint).exists());
}

#[cfg(unix)]
#[tokio::test]
async fn ipc_shutdown_preserves_replacement_file_and_symlink() {
    use std::os::unix::fs::{symlink, PermissionsExt};

    let root = tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let app =
        NativeApplication::isolated(&root.path().join("vault"), &root.path().join("db.sqlite"))
            .unwrap();
    for replacement in ["file", "symlink"] {
        let endpoint = ipc_endpoint(root.path());
        let listener = start_gui_ipc(&app, &endpoint).await;
        std::fs::remove_file(&endpoint).unwrap();
        let replacement_path = root.path().join(format!("replacement-{replacement}"));
        std::fs::write(&replacement_path, "preserve me").unwrap();
        if replacement == "file" {
            std::fs::rename(&replacement_path, &endpoint).unwrap();
        } else {
            symlink(&replacement_path, &endpoint).unwrap();
        }

        listener.abort();
        let _ = listener.await;

        let metadata = std::fs::symlink_metadata(&endpoint).unwrap();
        if replacement == "file" {
            assert!(metadata.file_type().is_file());
        } else {
            assert!(metadata.file_type().is_symlink());
        }
        assert_eq!(std::fs::read_to_string(&endpoint).unwrap(), "preserve me");
        std::fs::remove_file(&endpoint).unwrap();
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn stdio_initializes_lists_closed_tools_and_opens_idempotently() {
    let root = tempdir().unwrap();
    let vault = root.path().join("vault");
    let db = root.path().join("db.sqlite");
    let app = NativeApplication::isolated(&vault, &db).unwrap();
    std::fs::write(
        vault.join("allowed.md"),
        "# LLM Wiki decision\n\nScoped persistence guidance",
    )
    .unwrap();
    std::fs::write(
        vault.join("canary.md"),
        "# Secret roadmap\n\nScoped persistence guidance CANARY",
    )
    .unwrap();
    let indexed = app.execute_domain(
        "vault",
        NativeOperation {
            name: "vault.index".into(),
            input: json!({}),
        },
    );
    assert_eq!(indexed.status, 200, "{}", indexed.body);
    app.work_tracking_service().set_topic_membership(&json!({"topicId":"LLM Wiki","entityType":"vault","entityId":"allowed.md","included":true})).unwrap();
    for index in 0..51 {
        let response = app.execute_domain(
            "workflow",
            NativeOperation {
                name: "capture.create".into(),
                input: json!({"operationId":format!("overview-capture-{index}"),"text":format!("overview capture {index}")}),
            },
        );
        assert_eq!(response.status, 201, "{}", response.body);
    }
    let created=app.execute_work_tracking(NativeOperation{name:"work_tracking.connection.create".into(),input:json!({"name":"Contract test","scopes":["session:read","session:write","topic:read","workbench:current:read","workbench:overview:read","vault:search:lexical","vault:evidence:read","knowledge:draft:write","knowledge:publish"],"topicIds":["LLM Wiki"],"checkpointPolicy":"confirm_each"})});
    assert_eq!(created.status, 201, "{}", created.body);
    let id = created.body["id"].as_str().unwrap();
    let endpoint = ipc_endpoint(root.path());
    let listener = start_gui_ipc(&app, &endpoint).await;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&endpoint).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    let forbidden_db = root.path().join("must-not-be-opened.sqlite");
    let mut child = Command::new(env!("CARGO_BIN_EXE_llm-wiki-desktop"))
        .args(["--mcp", "--connection", id])
        .env("LLM_WIKI_MCP_ENDPOINT", &endpoint)
        .env("LLM_WIKI_DB", &forbidden_db)
        .env("LLM_WIKI_VAULT", root.path().join("must-not-be-read"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut reader = BufReader::new(child.stdout.take().unwrap());
    let meta = json!({"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{"elicitation":{}}});
    send(
        &mut stdin,
        json!({"jsonrpc":"2.0","id":1,"method":"server/discover","params":{"_meta":meta.clone()}}),
    );
    let initialized = receive(&mut reader);
    assert_eq!(initialized["id"], 1);
    assert!(initialized["result"]["supportedVersions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == "2026-07-28"));
    send(
        &mut stdin,
        json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{"_meta":meta.clone()}}),
    );
    let tools = receive(&mut reader);
    let names = tools["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v["name"].as_str())
        .collect::<Vec<_>>();
    assert!(names.contains(&"inbound_work_open"));
    assert!(names.contains(&"knowledge_publish"));
    assert!(!names.iter().any(|name| name.contains("sql")));
    for tool in tools["result"]["tools"].as_array().unwrap() {
        assert_eq!(
            tool["inputSchema"]["additionalProperties"], false,
            "{} must have a closed root schema",
            tool["name"]
        );
    }
    send(
        &mut stdin,
        json!({"jsonrpc":"2.0","id":20,"method":"tools/call","params":{"name":"vault_search_lexical","arguments":{"query":"Scoped persistence guidance","scope":"topic","targetId":"LLM Wiki","limit":10},"_meta":meta.clone()}}),
    );
    let search = receive(&mut reader);
    let search_text = search.to_string();
    assert!(search_text.contains("LLM Wiki decision"), "{search_text}");
    assert!(!search_text.contains("CANARY"));
    send(
        &mut stdin,
        json!({"jsonrpc":"2.0","id":21,"method":"tools/call","params":{"name":"vault_search_lexical","arguments":{"query":"Scoped persistence guidance","scope":"topic","targetId":"Secret","limit":10},"_meta":meta.clone()}}),
    );
    let denied_topic = receive(&mut reader);
    assert_eq!(denied_topic["result"]["isError"], true);
    assert!(!denied_topic.to_string().contains("CANARY"));
    let arguments = json!({"operationId":"open","lineageKey":"stdio-chat","mode":"create","capture":{"title":"MCP stdio","summary":"Track without Workbench"}});
    send(
        &mut stdin,
        json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"inbound_work_open","arguments":arguments,"_meta":meta.clone()}}),
    );
    let preview = receive(&mut reader);
    assert_eq!(preview["result"]["resultType"], "input_required");
    let capture_count: i64 = rusqlite::Connection::open(&db)
        .unwrap()
        .query_row("SELECT count(*) FROM captures", [], |row| row.get(0))
        .unwrap();
    assert_eq!(capture_count, 51);
    send(
        &mut stdin,
        json!({"jsonrpc":"2.0","id":30,"method":"tools/call","params":{"name":"inbound_work_open","arguments":arguments,"requestState":preview["result"]["requestState"],"inputResponses":{"decision":{"action":"accept","content":{"decision":"accept"}}},"_meta":meta.clone()}}),
    );
    let opened = receive(&mut reader);
    let structured = &opened["result"]["structuredContent"];
    assert_eq!(structured["created"], true);
    assert_eq!(structured["persistenceStatus"], "durable");
    send(
        &mut stdin,
        json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"inbound_work_open","arguments":{"operationId":"open","lineageKey":"stdio-chat","mode":"create","capture":{"title":"MCP stdio","summary":"Track without Workbench"}},"_meta":meta.clone()}}),
    );
    let replay = receive(&mut reader);
    assert_eq!(replay["result"]["structuredContent"]["deduplicated"], true);
    let session = structured["sessionId"].clone();
    send(
        &mut stdin,
        json!({"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"inbound_work_append","arguments":{"operationId":"task","sessionId":session,"expectedHeadRevision":1,"event":{"kind":"task_created","title":"One Task","detail":"Review in chat"}},"_meta":meta.clone()}}),
    );
    let appended = receive(&mut reader);
    let event = &appended["result"]["structuredContent"];
    assert!(event["eventId"].as_str().is_some());
    send(
        &mut stdin,
        json!({"jsonrpc":"2.0","id":9,"method":"resources/list","params":{"_meta":meta.clone()}}),
    );
    let resources = receive(&mut reader);
    let uris = resources["result"]["resources"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v["uri"].as_str())
        .collect::<Vec<_>>();
    assert!(uris
        .iter()
        .any(|uri| uri.starts_with("llm-wiki://work-session/")));
    assert!(uris.contains(&"llm-wiki://workbench/current"));
    assert!(uris.contains(&"llm-wiki://workbench/overview"));
    send(
        &mut stdin,
        json!({"jsonrpc":"2.0","id":91,"method":"resources/templates/list","params":{"_meta":meta.clone()}}),
    );
    let templates = receive(&mut reader);
    assert!(templates["result"]["resourceTemplates"]
        .as_array()
        .unwrap()
        .iter()
        .any(|template| template["uriTemplate"]
            == "llm-wiki://workbench/overview{?snapshotRevision,cursor,limit}"));
    send(
        &mut stdin,
        json!({"jsonrpc":"2.0","id":92,"method":"resources/read","params":{"uri":"llm-wiki://workbench/overview?limit=50","_meta":meta.clone()}}),
    );
    let first_overview = receive(&mut reader);
    let first_overview: Value = serde_json::from_str(
        first_overview["result"]["contents"][0]["text"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(first_overview["items"].as_array().unwrap().len(), 50);
    for field in [
        "captures",
        "tasks",
        "inProgressTasks",
        "completedTasks",
        "total",
    ] {
        assert!(first_overview["summary"].get(field).is_some(), "{field}");
    }
    assert!(first_overview["recentlyCompleted"].is_array());
    let cursor = first_overview["nextCursor"].as_str().unwrap();
    let snapshot_revision = first_overview["snapshotRevision"].as_i64().unwrap();
    send(
        &mut stdin,
        json!({"jsonrpc":"2.0","id":93,"method":"resources/read","params":{"uri":format!("llm-wiki://workbench/overview?snapshotRevision={snapshot_revision}&cursor={cursor}&limit=50"),"_meta":meta.clone()}}),
    );
    let second_overview = receive(&mut reader);
    let second_overview: Value = serde_json::from_str(
        second_overview["result"]["contents"][0]["text"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(second_overview["items"].as_array().unwrap().len(), 2);
    assert!(second_overview["nextCursor"].is_null());
    rusqlite::Connection::open(&db).unwrap().execute("UPDATE work_tracking_sessions SET state='completed',publication_state='offered',publication_offer_revision=head_revision WHERE id=?",[session.as_str().unwrap()]).unwrap();
    // This protocol-only fixture supplies a completion decision; canonical
    // completion and Lineage are exercised by work_tracking_release_acceptance.
    rusqlite::Connection::open(&db).unwrap().execute("INSERT INTO work_tracking_decisions(id,session_id,event_id,decision,accepted_payload_hash,result_entity_type,result_entity_id,decision_channel,created_at) SELECT 'protocol-completion',session_id,id,'accepted',payload_hash,'completions','protocol-completion','test_fixture',ingested_at FROM work_tracking_events WHERE id=?",[event["eventId"].as_str().unwrap()]).unwrap();
    let draft_args = json!({"operationId":"draft","sessionId":session,"completionEventId":event["eventId"],"draftId":"draft-contract","expectedDraftRevision":0,"title":"Contract knowledge","summary":"Reviewed summary","bodyMarkdown":"# Contract knowledge\n\nExact reviewed body","evidenceRefs":[]});
    send(
        &mut stdin,
        json!({"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"knowledge_draft_save","arguments":draft_args,"_meta":meta.clone()}}),
    );
    let preview = receive(&mut reader);
    assert_eq!(preview["result"]["resultType"], "input_required");
    send(
        &mut stdin,
        json!({"jsonrpc":"2.0","id":100,"method":"tools/call","params":{"name":"knowledge_draft_save","arguments":draft_args,"requestState":preview["result"]["requestState"],"inputResponses":{"decision":{"action":"accept","content":{"decision":"accept"}}},"_meta":meta.clone()}}),
    );
    let draft = receive(&mut reader);
    let saved = &draft["result"]["structuredContent"];
    assert_eq!(saved["draftRevision"], 1);
    std::fs::create_dir_all(vault.join("Knowledge")).unwrap();
    let target = vault.join("Knowledge/contract-knowledge-draft-co.md");
    std::fs::write(&target, "external edit").unwrap();
    let publish_args = json!({"operationId":"publish","draftId":"draft-contract","expectedDraftRevision":1,"expectedContentHash":saved["contentHash"]});
    send(
        &mut stdin,
        json!({"jsonrpc":"2.0","id":11,"method":"tools/call","params":{"name":"knowledge_publish","arguments":publish_args,"_meta":meta.clone()}}),
    );
    let preview = receive(&mut reader);
    assert_eq!(preview["result"]["resultType"], "input_required");
    let publish_state = preview["result"]["requestState"].clone();
    send(
        &mut stdin,
        json!({"jsonrpc":"2.0","id":110,"method":"tools/call","params":{"name":"knowledge_publish","arguments":publish_args,"requestState":publish_state,"inputResponses":{"decision":{"action":"accept","content":{"decision":"accept"}}},"_meta":meta.clone()}}),
    );
    let conflict = receive(&mut reader);
    assert!(
        conflict.get("error").is_some() || conflict["result"]["isError"] == true,
        "{conflict}"
    );
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "external edit");
    std::fs::remove_file(&target).unwrap();
    send(
        &mut stdin,
        json!({"jsonrpc":"2.0","id":12,"method":"tools/call","params":{"name":"knowledge_publish","arguments":publish_args,"requestState":publish_state,"inputResponses":{"decision":{"action":"accept","content":{"decision":"accept"}}},"_meta":meta.clone()}}),
    );
    let published = receive(&mut reader);
    assert_eq!(
        published["result"]["structuredContent"]["status"],
        "published"
    );
    assert!(std::fs::read_to_string(&target)
        .unwrap()
        .contains("Exact reviewed body"));
    let revoked = app.execute_work_tracking(NativeOperation {
        name: "work_tracking.connection.revoke".into(),
        input: json!({"connectionId":id}),
    });
    assert_eq!(revoked.status, 204);
    send(
        &mut stdin,
        json!({"jsonrpc":"2.0","id":13,"method":"tools/call","params":{"name":"inbound_work_session_read","arguments":{"sessionId":session},"_meta":meta}}),
    );
    let denied = receive(&mut reader);
    assert_ne!(denied["result"]["isError"], false);
    assert!(!denied.to_string().contains("Need one state machine"));
    drop(stdin);
    let _ = child.wait();
    assert!(!forbidden_db.exists());
    listener.abort();
}

#[tokio::test(flavor = "multi_thread")]
async fn legacy_client_cannot_bypass_governed_elicitation() {
    let root = tempdir().unwrap();
    let vault = root.path().join("vault");
    let db = root.path().join("db.sqlite");
    let app = NativeApplication::isolated(&vault, &db).unwrap();
    let created=app.execute_work_tracking(NativeOperation{name:"work_tracking.connection.create".into(),input:json!({"name":"Legacy","scopes":["session:read","session:write"],"checkpointPolicy":"confirm_each"})});
    let id = created.body["id"].as_str().unwrap();
    let endpoint = ipc_endpoint(root.path());
    let listener = start_gui_ipc(&app, &endpoint).await;
    let forbidden_db = root.path().join("must-not-be-opened.sqlite");
    let mut child = Command::new(env!("CARGO_BIN_EXE_llm-wiki-desktop"))
        .args(["--mcp", "--connection", id])
        .env("LLM_WIKI_MCP_ENDPOINT", &endpoint)
        .env("LLM_WIKI_DB", &forbidden_db)
        .env("LLM_WIKI_VAULT", root.path().join("must-not-be-read"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut reader = BufReader::new(child.stdout.take().unwrap());
    send(
        &mut stdin,
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"legacy","version":"1"}}}),
    );
    let initialized = receive(&mut reader);
    assert_eq!(initialized["result"]["protocolVersion"], "2025-06-18");
    send(
        &mut stdin,
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
    );
    send(
        &mut stdin,
        json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"inbound_work_open","arguments":{"operationId":"legacy-open","lineageKey":"legacy","mode":"create","capture":{"title":"Legacy","summary":"Must not bypass review"}}}}),
    );
    let opened = receive(&mut reader);
    assert_eq!(
        opened["result"]["structuredContent"]["error"]["code"],
        "elicitation_required"
    );
    let session = json!("unconsented-session");
    send(
        &mut stdin,
        json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"inbound_work_append","arguments":{"operationId":"legacy-problem","sessionId":session,"expectedHeadRevision":1,"event":{"kind":"problem_draft","statement":"Review me"}}}}),
    );
    let appended = receive(&mut reader);
    assert!(appended.get("error").is_some() || appended["result"]["isError"] == true);
    let event = json!("unconsented-event");
    send(
        &mut stdin,
        json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"inbound_work_advance","arguments":{"operationId":"legacy-advance","sessionId":session,"expectedHeadRevision":2,"sourceEventId":event,"action":"adopt_problem","proposedPayload":{"statement":"Review me"}}}}),
    );
    let denied = receive(&mut reader);
    assert_eq!(denied["result"]["isError"], true);
    assert_eq!(
        denied["result"]["structuredContent"]["error"]["code"],
        "elicitation_required"
    );
    let connection = rusqlite::Connection::open(&db).unwrap();
    let decisions: i64 = connection
        .query_row("SELECT count(*) FROM work_tracking_decisions", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(decisions, 0);
    drop(stdin);
    let _ = child.wait();
    assert!(!forbidden_db.exists());
    listener.abort();
}
