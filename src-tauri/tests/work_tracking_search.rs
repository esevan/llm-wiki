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
    let vault = root.path().join("vault");
    std::fs::create_dir_all(&vault).unwrap();
    std::fs::write(vault.join("fallback.md"), "# Related idea\n\nLexical fallback evidence").unwrap();
    let app = NativeApplication::isolated(&vault, &root.path().join("db.sqlite")).unwrap();
    let indexed = app.execute_domain(
        "vault",
        NativeOperation { name: "vault.index".into(), input: json!({}) },
    );
    assert_eq!(indexed.status, 200, "{}", indexed.body);
    let semantic = app.execute_work_tracking(NativeOperation {
        name: "work_tracking.vault.semantic".into(),
        input: json!({"query":"related idea"}),
    });
    assert_eq!(semantic.status, 200, "{}", semantic.body);
    assert_eq!(semantic.body["indexState"], "partial");
    assert_eq!(semantic.body["hits"][0]["sourceIdentity"], "fallback.md");
    let lexical = app.execute_work_tracking(NativeOperation {
        name: "work_tracking.vault.lexical".into(),
        input: json!({"query":"related idea"}),
    });
    assert_eq!(lexical.status, 200);
}

#[test]
fn applicability_metadata_is_additive_and_internal_withdrawals_are_excluded() {
    let root = tempdir().unwrap();
    let vault = root.path().join("vault");
    let db = root.path().join("db.sqlite");
    std::fs::create_dir_all(vault.join(".llm-wiki-withdrawn")).unwrap();
    std::fs::write(
        vault.join("applicable.md"),
        "---\nllm_wiki:\n  schema: 1\n  document_id: applicable\n  applicability:\n    helps_with: [offline retrieval]\n---\n# Notes\nSaved body\n",
    )
    .unwrap();
    std::fs::write(
        vault.join(".llm-wiki-withdrawn/obsolete.md"),
        "# Obsolete\n\noffline retrieval",
    )
    .unwrap();
    let app = NativeApplication::isolated(&vault, &db).unwrap();
    let indexed = app.execute_domain(
        "vault",
        NativeOperation {
            name: "vault.index".into(),
            input: json!({"semantic":false}),
        },
    );
    assert_eq!(indexed.status, 200, "{}", indexed.body);
    let search = app.execute_work_tracking(NativeOperation {
        name: "work_tracking.vault.lexical".into(),
        input: json!({"query":"offline retrieval","limit":5}),
    });
    assert_eq!(search.status, 200, "{}", search.body);
    assert_eq!(search.body["hits"].as_array().unwrap().len(), 1);
    assert_eq!(search.body["hits"][0]["documentId"], "applicable");
    assert_eq!(search.body["hits"][0]["aspect"], "applicability");
    assert_eq!(search.body["hits"][0]["passage"]["kind"], "section");
}

#[test]
fn native_search_bounds_passages_after_ranking_and_reports_more() {
    let root = tempdir().unwrap();
    let vault = root.path().join("vault");
    let db = root.path().join("db.sqlite");
    std::fs::create_dir_all(&vault).unwrap();
    for index in 0..10 {
        std::fs::write(
            vault.join(format!("note-{index}.md")),
            format!("# Note {index}\n\nshared retrieval phrase"),
        )
        .unwrap();
    }
    let app = NativeApplication::isolated(&vault, &db).unwrap();
    let indexed = app.execute_domain(
        "vault",
        NativeOperation {
            name: "vault.index".into(),
            input: json!({"semantic":false}),
        },
    );
    assert_eq!(indexed.status, 200, "{}", indexed.body);
    let search = app.execute_domain(
        "vault",
        NativeOperation {
            name: "vault.search".into(),
            input: json!({"query":"shared retrieval phrase","limit":20,"semantic":false}),
        },
    );
    assert_eq!(search.status, 200, "{}", search.body);
    assert_eq!(search.body["results"].as_array().unwrap().len(), 8);
    assert_eq!(search.body["limit"], 8);
    assert_eq!(search.body["has_more"], true);
}
