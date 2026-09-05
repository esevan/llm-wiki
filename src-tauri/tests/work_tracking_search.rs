use llm_wiki_desktop::{NativeApplication, NativeOperation};
use serde_json::json;
use tempfile::tempdir;

#[test]
fn lexical_search_returns_bounded_ids_and_evidence_revalidates_revision() {
    let root = tempdir().unwrap();
    let vault = root.path().join("vault");
    let db = root.path().join("db.sqlite");
    std::fs::create_dir_all(&vault).unwrap();
    std::fs::write(
        vault.join("decision.md"),
        "# Earlier decision\n\nUse one persistence boundary.",
    )
    .unwrap();
    let app = NativeApplication::isolated(&vault, &db).unwrap();
    let indexed = app.execute_domain(
        "vault",
        NativeOperation {
            name: "vault.index".into(),
            input: json!({}),
        },
    );
    assert_eq!(indexed.status, 200, "{}", indexed.body);
    let search = app.execute_work_tracking(NativeOperation {
        name: "work_tracking.vault.lexical".into(),
        input: json!({"query":"persistence boundary","limit":5}),
    });
    assert_eq!(search.status, 200, "{}", search.body);
    let hit = &search.body["hits"][0];
    assert!(hit["evidenceId"].as_str().unwrap().starts_with("ev_"));
    assert!(!hit["evidenceId"].as_str().unwrap().contains("decision"));
    assert!(hit.get("body").is_none());
    let evidence=app.execute_work_tracking(NativeOperation{name:"work_tracking.vault.evidence".into(),input:json!({"items":[{"evidenceId":hit["evidenceId"],"expectedRevision":hit["revision"]}]})});
    assert_eq!(evidence.status, 200, "{}", evidence.body);
    assert!(evidence.body["items"][0]["content"]
        .as_str()
        .unwrap()
        .contains("one persistence"));
    std::fs::write(vault.join("decision.md"), "# Changed\n\nA newer decision.").unwrap();
    let stale=app.execute_work_tracking(NativeOperation{name:"work_tracking.vault.evidence".into(),input:json!({"items":[{"evidenceId":hit["evidenceId"],"expectedRevision":hit["revision"]}]})});
    assert_eq!(stale.status, 409);
    assert_eq!(stale.body["error"]["code"], "evidence_revision_changed");
}

#[test]
fn evidence_grants_are_bound_to_the_connection_that_searched() {
    let root = tempdir().unwrap();
    let vault = root.path().join("vault");
    let db = root.path().join("db.sqlite");
    std::fs::create_dir_all(&vault).unwrap();
    std::fs::write(vault.join("private.md"), "# Private\n\nScoped evidence").unwrap();
    let app = NativeApplication::isolated(&vault, &db).unwrap();
    app.execute_domain(
        "vault",
        NativeOperation {
            name: "vault.index".into(),
            input: json!({}),
        },
    );
    let search = app.execute_work_tracking(NativeOperation {
        name: "work_tracking.vault.lexical".into(),
        input: json!({"query":"Scoped evidence"}),
    });
    let evidence_id = search.body["hits"][0]["evidenceId"].as_str().unwrap();
    let external=app.execute_work_tracking(NativeOperation{name:"work_tracking.connection.create".into(),input:json!({"name":"Other","scopes":["vault:evidence:read"],"checkpointPolicy":"confirm_each"})});
    rusqlite::Connection::open(&db)
        .unwrap()
        .execute(
            "UPDATE work_tracking_evidence_grants SET connection_id=? WHERE evidence_id=?",
            [external.body["id"].as_str().unwrap(), evidence_id],
        )
        .unwrap();
    let denied = app.execute_work_tracking(NativeOperation {
        name: "work_tracking.vault.evidence".into(),
        input: json!({"items":[{"evidenceId":evidence_id}]}),
    });
    assert_eq!(denied.status, 404, "{}", denied.body);
    assert!(!denied.body.to_string().contains("private.md"));
}

#[test]
fn fabricated_evidence_ids_are_rejected_without_disclosure() {
    let root = tempdir().unwrap();
    let app =
        NativeApplication::isolated(&root.path().join("vault"), &root.path().join("db.sqlite"))
            .unwrap();
    let denied = app.execute_work_tracking(NativeOperation {
        name: "work_tracking.vault.evidence".into(),
        input: json!({"items":[{"evidenceId":"ev_fabricated","expectedRevision":"guessed"}]}),
    });
    assert_eq!(denied.status, 404);
    assert_eq!(denied.body["error"]["code"], "not_found_or_not_visible");
    assert!(!denied.body.to_string().contains("sqlite"));
}

#[test]
fn semantic_unavailability_is_explicit_and_lexical_remains_available() {
    let root = tempdir().unwrap();
    let app =
        NativeApplication::isolated(&root.path().join("vault"), &root.path().join("db.sqlite"))
            .unwrap();
    let semantic = app.execute_work_tracking(NativeOperation {
        name: "work_tracking.vault.semantic".into(),
        input: json!({"query":"related idea"}),
    });
    assert_eq!(semantic.status, 503);
    assert_eq!(semantic.body["error"]["code"], "semantic_index_not_ready");
    let lexical = app.execute_work_tracking(NativeOperation {
        name: "work_tracking.vault.lexical".into(),
        input: json!({"query":"related idea"}),
    });
    assert_eq!(lexical.status, 200);
}
