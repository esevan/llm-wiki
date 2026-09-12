use rusqlite::{
    params, types::ValueRef, Connection, OptionalExtension, Transaction, TransactionBehavior,
};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;

pub const CURRENT_SCHEMA_VERSION: i64 = 8;

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
    Migration {
        version: 6,
        name: "track workbench entity update timestamps",
        run: add_workbench_entity_update_timestamps,
    },
    Migration {
        version: 7,
        name: "propagate solution child updates to overview timestamps",
        run: propagate_solution_child_update_timestamps,
    },
    Migration {
        version: 8,
        name: "migrate legacy work into task aggregates",
        run: migrate_task_centered_workbench,
    },
];

fn migrate_task_centered_workbench(tx: &Transaction<'_>) -> Result<(), String> {
    if !table_exists(tx, "problems")? || !table_exists(tx, "captures")? {
        return Ok(());
    }
    drop_legacy_workbench_triggers(tx)?;
    if table_exists(tx, "features")? {
        for (column, declaration) in [
            ("problem_id", "TEXT NOT NULL DEFAULT ''"),
            ("title", "TEXT NOT NULL DEFAULT ''"),
            ("outcome", "TEXT NOT NULL DEFAULT ''"),
            ("non_goals", "TEXT NOT NULL DEFAULT ''"),
            ("validation_criteria", "TEXT NOT NULL DEFAULT ''"),
            ("state", "TEXT NOT NULL DEFAULT 'proposed'"),
            ("created_at", "TEXT NOT NULL DEFAULT ''"),
        ] {
            add_missing_column(tx, "features", column, declaration)?;
        }
    }
    for (column, declaration) in [
        ("statement", "TEXT NOT NULL DEFAULT ''"),
        ("detail", "TEXT NOT NULL DEFAULT ''"),
        ("state", "TEXT NOT NULL DEFAULT 'open'"),
        ("created_at", "TEXT NOT NULL DEFAULT ''"),
    ] {
        add_missing_column(tx, "problems", column, declaration)?;
    }
    add_missing_column(tx, "problems", "capture_id", "TEXT")?;
    add_missing_column(
        tx,
        "captures",
        "source_mode",
        "TEXT NOT NULL DEFAULT 'capture'",
    )?;
    add_missing_column(
        tx,
        "captures",
        "last_user_activity_at",
        "TEXT NOT NULL DEFAULT ''",
    )?;
    rebuild_problems_without_capture_uniqueness(tx)?;
    tx.execute_batch(include_str!("task_schema.sql"))
        .map_err(|error| error.to_string())?;
    tx.execute_batch(include_str!("task_assistance_schema.sql"))
        .map_err(|error| error.to_string())?;
    let snapshot = MigrationSnapshot::capture(tx)?;

    tx.execute(
        "UPDATE problems SET current_revision=1 WHERE current_revision=0",
        [],
    )
    .map_err(|error| error.to_string())?;
    tx.execute("UPDATE problems SET state=CASE WHEN state='completed' THEN 'resolved' WHEN state='archived' THEN 'archived' ELSE 'open' END", [])
        .map_err(|error| error.to_string())?;
    let problems = read_rows(tx, "SELECT id,statement,detail,created_at FROM problems")?;
    for row in problems {
        let id = string_field(&row, "id")?;
        let statement = string_field(&row, "statement")?;
        let detail = string_field(&row, "detail")?;
        let created = string_field(&row, "created_at")?;
        let hash = crate::domain::task::content_hash(&[&statement, &detail]);
        tx.execute("INSERT OR IGNORE INTO problem_revisions(problem_id,revision,statement,detail,content_hash,created_at) VALUES(?,1,?,?,?,?)", params![id,statement,detail,hash,created])
            .map_err(|error| error.to_string())?;
    }

    if !table_exists(tx, "features")? {
        return snapshot.validate(tx);
    }
    for (column, declaration) in [
        ("problem_id", "TEXT NOT NULL DEFAULT ''"),
        ("title", "TEXT NOT NULL DEFAULT ''"),
        ("outcome", "TEXT NOT NULL DEFAULT ''"),
        ("non_goals", "TEXT NOT NULL DEFAULT ''"),
        ("validation_criteria", "TEXT NOT NULL DEFAULT ''"),
        ("state", "TEXT NOT NULL DEFAULT 'proposed'"),
        ("created_at", "TEXT NOT NULL DEFAULT ''"),
    ] {
        add_missing_column(tx, "features", column, declaration)?;
    }
    let features = read_rows(tx, "SELECT id,problem_id,title,outcome,non_goals,validation_criteria,state,created_at FROM features")?;
    for row in features {
        let id = string_field(&row, "id")?;
        let problem = string_field(&row, "problem_id")?;
        let title = string_field(&row, "title")?;
        let outcome = string_field(&row, "outcome")?;
        let non_goals = string_field(&row, "non_goals")?;
        let criteria = string_field(&row, "validation_criteria")?;
        let state = string_field(&row, "state")?;
        let created = string_field(&row, "created_at")?;
        let new_state = match state.as_str() {
            "proposed" => "task",
            "approved" | "in_progress" => "in_progress",
            "completed" | "archived" => "completed",
            _ => return Err(format!("unknown legacy feature state: {state}")),
        };
        let (origin, category): (Option<String>, String) = tx.query_row(
            "SELECT p.capture_id,COALESCE((SELECT category FROM workbench_category_overrides WHERE entity_type='features' AND entity_id=?),(SELECT category FROM workbench_priorities WHERE entity_type='features' AND entity_id=?),'General') FROM problems p WHERE p.id=?",
            params![id,id,problem], |record| Ok((record.get(0)?,record.get(1)?)),
        ).map_err(|error| error.to_string())?;
        let deleted_at: Option<String> = tx.query_row("SELECT deleted_at FROM deleted_entities WHERE entity_type='features' AND entity_id=?", [&id], |record| record.get(0)).optional().map_err(|error| error.to_string())?;
        let archived_at = if state == "archived" {
            Some(created.clone())
        } else {
            deleted_at
        };
        tx.execute("INSERT OR IGNORE INTO tasks(id,origin_capture_id,current_revision,state,archived_at,category,created_at,last_user_activity_at,started_at,completed_at) VALUES(?,?,1,?,?,?,?,?,CASE WHEN ? IN ('approved','in_progress') THEN ? ELSE NULL END,CASE WHEN ? IN ('completed','archived') THEN ? ELSE NULL END)",params![id,origin,new_state,archived_at,category,created,created,state,created,state,created]).map_err(|error|error.to_string())?;
        let hash =
            crate::domain::task::content_hash(&[&title, "", &outcome, "", &non_goals, &criteria]);
        tx.execute("INSERT OR IGNORE INTO task_revisions(task_id,revision,title,detail,outcome,scope,non_goals,validation_criteria,content_hash,source_reference,created_at) VALUES(?,1,?,'',?,'',?,?,?,?,?)",params![id,title,outcome,non_goals,criteria,hash,format!("legacy:features:{id}"),created]).map_err(|error|error.to_string())?;
        tx.execute("INSERT OR IGNORE INTO task_problem_links(id,task_id,problem_id,problem_revision,relationship,note,created_at) VALUES(?,?,?,1,'context','legacy feature problem link',?)",params![format!("legacy-link-{id}"),id,problem,created]).map_err(|error|error.to_string())?;
        tx.execute("INSERT OR IGNORE INTO task_work_log_entries(id,task_id,body,image_data,image_media_type,image_summary,source_legacy_id,created_at) SELECT id,?,body,image_data,image_media_type,image_summary,id,created_at FROM solution_progress_entries WHERE feature_id=?",params![id,id]).map_err(|error|error.to_string())?;
        let attachments = read_rows_bound(tx, "SELECT id,image_data,image_media_type,created_at FROM solution_progress_entries WHERE feature_id=? AND image_data<>''", &id)?;
        for attachment in attachments {
            let entry_id = string_field(&attachment, "id")?;
            let data = string_field(&attachment, "image_data")?;
            let media = string_field(&attachment, "image_media_type")?;
            let attachment_created = string_field(&attachment, "created_at")?;
            let byte_hash = crate::domain::task::content_hash(&[&data]);
            tx.execute("INSERT OR IGNORE INTO task_attachments(id,task_id,entry_id,name,media_type,data,byte_hash,created_at) VALUES(?,?,?,?,?,?,?,?)",params![format!("legacy-attachment-{entry_id}"),id,entry_id,"legacy-work-log-image",media,data,byte_hash,attachment_created]).map_err(|error|error.to_string())?;
        }
        tx.execute("INSERT OR IGNORE INTO task_checklist_items(id,task_id,body,checked,created_at,updated_at) SELECT id,?,body,checked,created_at,updated_at FROM solution_checklist_items WHERE feature_id=?",params![id,id]).map_err(|error|error.to_string())?;
        tx.execute("INSERT OR IGNORE INTO task_completions(id,task_id,task_revision,evidence,report,operation_id,created_at) SELECT id,?,1,evidence,report,?,created_at FROM completions WHERE feature_id=?",params![id,format!("legacy-completion-{id}"),id]).map_err(|error|error.to_string())?;
    }
    for table in LEGACY_LEDGER_TABLES {
        preserve_legacy_table(tx, table)?;
    }
    tx.execute_batch("INSERT OR IGNORE INTO task_work_log_comments(id,entry_id,body,created_at) SELECT c.id,c.entry_id,c.body,c.created_at FROM solution_progress_comments c JOIN task_work_log_entries e ON e.id=c.entry_id;
      INSERT OR IGNORE INTO problem_resolution_decisions(id,problem_id,problem_revision,rationale,evidence_refs_json,operation_id,created_at) SELECT id,problem_id,1,reason,json_array(review_id),'legacy-problem-resolution-'||id,created_at FROM problem_completion_decisions;
      INSERT OR IGNORE INTO problem_resolution_decisions(id,problem_id,problem_revision,rationale,evidence_refs_json,operation_id,created_at) SELECT 'legacy-group-resolution-'||p.id,p.id,1,'legacy_group_completion','[]','legacy-group-resolution-'||p.id,p.created_at FROM problems p WHERE p.state='resolved' AND NOT EXISTS(SELECT 1 FROM problem_resolution_decisions d WHERE d.problem_id=p.id);
      INSERT OR IGNORE INTO task_decisions(id,task_id,task_revision,kind,payload_json,operation_id,created_at) SELECT 'legacy-approval-'||id,entity_id,1,'legacy_approval',json_object('sourceId',id,'action',action),'legacy-approval-'||id,created_at FROM approvals WHERE entity_type='features';
      INSERT OR IGNORE INTO task_decisions(id,task_id,task_revision,kind,payload_json,operation_id,created_at) SELECT 'legacy-conflict-resolution-'||id,feature_id,1,'legacy_conflict_resolution',json_object('sourceId',id,'runId',run_id,'conflictId',conflict_id,'action',action,'rationale',rationale),'legacy-conflict-resolution-'||id,resolved_at FROM conflict_resolutions;
      INSERT OR IGNORE INTO task_decisions(id,task_id,task_revision,kind,payload_json,operation_id,created_at) SELECT 'legacy-lineage-'||id,feature_id,1,'legacy_lineage_snapshot',json_object('sourceId',id,'version',version,'sourceHash',source_hash,'status',status),'legacy-lineage-'||id,created_at FROM lineage_snapshots;
      INSERT OR IGNORE INTO refinement_items(id,problem_id,capture_id,problem_revision,source_kind,created_at) SELECT 'legacy-problem-'||p.id,p.id,p.capture_id,1,'legacy_problem',p.created_at FROM problems p WHERE NOT EXISTS(SELECT 1 FROM task_problem_links l WHERE l.problem_id=p.id AND l.unlinked_at IS NULL);
      UPDATE captures SET last_user_activity_at=COALESCE(NULLIF(last_user_activity_at,''),created_at);
      INSERT OR IGNORE INTO localized_content(entity_type,entity_id,field_name,locale,value,origin,source_hash,created_at,updated_at) SELECT 'tasks',entity_id,field_name,locale,value,origin,source_hash,created_at,updated_at FROM localized_content WHERE entity_type='features';
      INSERT OR IGNORE INTO workbench_priority_overrides(entity_type,entity_id,manual_priority) SELECT 'tasks',entity_id,manual_priority FROM workbench_priority_overrides WHERE entity_type='features';
      INSERT OR IGNORE INTO workbench_category_overrides(entity_type,entity_id,category) SELECT 'tasks',entity_id,category FROM workbench_category_overrides WHERE entity_type='features';
      INSERT OR IGNORE INTO workbench_priorities(entity_type,entity_id,category,attention_rank,rationale,updated_at) SELECT 'tasks',entity_id,category,attention_rank,rationale,updated_at FROM workbench_priorities WHERE entity_type='features';
      INSERT OR IGNORE INTO mirror_files(entity_type,entity_id,path,source_hash) SELECT 'tasks',entity_id,path,source_hash FROM mirror_files WHERE entity_type='features';
      INSERT OR IGNORE INTO deleted_entities(entity_type,entity_id,deleted_at) SELECT 'tasks',entity_id,deleted_at FROM deleted_entities WHERE entity_type='features';
      UPDATE work_tracking_links SET entity_type='tasks' WHERE entity_type='features';
      INSERT OR IGNORE INTO work_tracking_entity_versions(entity_type,entity_id,revision,updated_at) SELECT 'tasks',entity_id,revision,updated_at FROM work_tracking_entity_versions WHERE entity_type='features';")
        .map_err(|error| error.to_string())?;
    snapshot.validate(tx)
}

const LEGACY_LEDGER_TABLES: &[&str] = &[
    "approvals",
    "conflict_reports",
    "conflict_review_runs",
    "conflict_review_conflicts",
    "conflict_resolutions",
    "completion_reviews",
    "completion_playbooks",
    "lineage_snapshots",
    "lineage_claims",
    "lineage_evidence",
    "lineage_revisions",
    "importance_assessments",
    "mirror_files",
    "patch_proposals",
    "deleted_entities",
    "workbench_priorities",
    "workbench_priority_overrides",
    "workbench_category_overrides",
    "localized_content",
    "work_tracking_sessions",
    "work_tracking_events",
    "work_tracking_decisions",
    "work_tracking_links",
    "work_tracking_idempotency_records",
];

fn drop_legacy_workbench_triggers(tx: &Transaction<'_>) -> Result<(), String> {
    let mut statement = tx.prepare("SELECT name FROM sqlite_master WHERE type='trigger' AND tbl_name IN ('problems','features','solution_progress_entries','solution_progress_comments','solution_checklist_items','completions')").map_err(|error|error.to_string())?;
    let names = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    drop(statement);
    for name in names {
        tx.execute_batch(&format!(
            "DROP TRIGGER IF EXISTS \"{}\"",
            name.replace('"', "\"\"")
        ))
        .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn rebuild_problems_without_capture_uniqueness(tx: &Transaction<'_>) -> Result<(), String> {
    tx.execute_batch(
        "PRAGMA defer_foreign_keys=ON;
         CREATE TABLE problems_task_v8 (
           id TEXT PRIMARY KEY, capture_id TEXT REFERENCES captures(id), statement TEXT NOT NULL,
           detail TEXT NOT NULL DEFAULT '', state TEXT NOT NULL DEFAULT 'open',
           created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, current_revision INTEGER NOT NULL DEFAULT 1
         );
         INSERT INTO problems_task_v8(id,capture_id,statement,detail,state,created_at,current_revision)
           SELECT id,capture_id,statement,detail,state,created_at,1 FROM problems;
         DROP TABLE problems;
         ALTER TABLE problems_task_v8 RENAME TO problems;
         CREATE INDEX IF NOT EXISTS problems_capture_lookup ON problems(capture_id);",
    )
    .map_err(|error| error.to_string())
}

fn json_value(value: ValueRef<'_>) -> Value {
    match value {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(value) => json!(value),
        ValueRef::Real(value) => json!(value),
        ValueRef::Text(value) => Value::String(String::from_utf8_lossy(value).into_owned()),
        ValueRef::Blob(value) => Value::String(format!(
            "hex:{}",
            value
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        )),
    }
}

fn read_rows(tx: &Transaction<'_>, sql: &str) -> Result<Vec<Map<String, Value>>, String> {
    let mut statement = tx.prepare(sql).map_err(|error| error.to_string())?;
    let names = statement
        .column_names()
        .iter()
        .map(|name| (*name).to_owned())
        .collect::<Vec<_>>();
    let rows = statement
        .query_map([], |row| {
            let mut value = Map::new();
            for (index, name) in names.iter().enumerate() {
                value.insert(name.clone(), json_value(row.get_ref(index)?));
            }
            Ok(value)
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    Ok(rows)
}

fn read_rows_bound(
    tx: &Transaction<'_>,
    sql: &str,
    parameter: &str,
) -> Result<Vec<Map<String, Value>>, String> {
    let mut statement = tx.prepare(sql).map_err(|error| error.to_string())?;
    let names = statement
        .column_names()
        .iter()
        .map(|name| (*name).to_owned())
        .collect::<Vec<_>>();
    let rows = statement
        .query_map([parameter], |row| {
            let mut value = Map::new();
            for (index, name) in names.iter().enumerate() {
                value.insert(name.clone(), json_value(row.get_ref(index)?));
            }
            Ok(value)
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    Ok(rows)
}

fn string_field(row: &Map<String, Value>, field: &str) -> Result<String, String> {
    row.get(field)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("legacy row missing {field}"))
}

fn preserve_legacy_table(tx: &Transaction<'_>, table: &str) -> Result<(), String> {
    if !table_exists(tx, table)? {
        return Ok(());
    }
    for row in read_rows(tx, &format!("SELECT * FROM {table}"))? {
        let payload = Value::Object(row.clone()).to_string();
        let source_id = row
            .get("id")
            .or_else(|| row.get("storage_id"))
            .and_then(Value::as_str)
            .map(str::to_owned)
            .unwrap_or_else(|| crate::domain::task::content_hash(&[&payload]));
        let task_id = row.get("feature_id").and_then(Value::as_str).or_else(|| {
            if row.get("entity_type").and_then(Value::as_str) == Some("features") {
                row.get("entity_id").and_then(Value::as_str)
            } else {
                None
            }
        });
        let problem_id = row.get("problem_id").and_then(Value::as_str);
        let created = row
            .get("created_at")
            .or_else(|| row.get("updated_at"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let hash = crate::domain::task::content_hash(&[&payload]);
        tx.execute("INSERT OR IGNORE INTO legacy_task_migration_records(source_table,source_id,task_id,problem_id,payload_json,source_hash,created_at) VALUES(?,?,?,?,?,?,?)", params![table,source_id,task_id,problem_id,payload,hash,created]).map_err(|error|error.to_string())?;
    }
    Ok(())
}

#[derive(Default)]
struct MigrationSnapshot {
    counts: BTreeMap<String, i64>,
}

impl MigrationSnapshot {
    fn capture(tx: &Transaction<'_>) -> Result<Self, String> {
        let mut counts = BTreeMap::new();
        for table in [
            "features",
            "solution_progress_entries",
            "solution_progress_comments",
            "solution_checklist_items",
            "completions",
        ] {
            counts.insert(table.to_owned(), table_count(tx, table)?);
        }
        for table in LEGACY_LEDGER_TABLES {
            if table_exists(tx, table)? {
                counts.insert((*table).to_owned(), table_count(tx, table)?);
            }
        }
        Ok(Self { counts })
    }

    fn validate(&self, tx: &Transaction<'_>) -> Result<(), String> {
        for (source, target) in [
            ("features", "tasks"),
            ("solution_progress_entries", "task_work_log_entries"),
            ("solution_progress_comments", "task_work_log_comments"),
            ("solution_checklist_items", "task_checklist_items"),
            ("completions", "task_completions"),
        ] {
            if table_count(tx, target)? < self.counts.get(source).copied().unwrap_or(0) {
                return Err(format!("migration_invariant_count:{source}:{target}"));
            }
        }
        let missing: i64 = tx.query_row("SELECT count(*) FROM features f LEFT JOIN tasks t ON t.id=f.id LEFT JOIN task_revisions r ON r.task_id=f.id AND r.revision=1 LEFT JOIN task_problem_links l ON l.task_id=f.id AND l.problem_id=f.problem_id AND l.problem_revision=1 WHERE t.id IS NULL OR r.task_id IS NULL OR l.id IS NULL OR r.title<>f.title OR r.outcome<>f.outcome OR r.non_goals<>f.non_goals OR r.validation_criteria<>f.validation_criteria", [], |row| row.get(0)).map_err(|error|error.to_string())?;
        if missing != 0 {
            return Err("migration_invariant_task_mapping".into());
        }
        let changed_work_log:i64=tx.query_row("SELECT count(*) FROM solution_progress_entries s LEFT JOIN task_work_log_entries t ON t.id=s.id WHERE t.id IS NULL OR t.body<>s.body OR t.image_data<>s.image_data OR t.image_media_type<>s.image_media_type OR t.image_summary<>s.image_summary OR t.created_at<>s.created_at",[],|row|row.get(0)).map_err(|error|error.to_string())?;
        if changed_work_log != 0 {
            return Err("migration_invariant_work_log_hash".into());
        }
        for (name, sql) in [
            ("problem_revision", "SELECT count(*) FROM problems p LEFT JOIN problem_revisions r ON r.problem_id=p.id AND r.revision=1 WHERE r.problem_id IS NULL OR r.statement<>p.statement OR r.detail<>p.detail"),
            ("comment", "SELECT count(*) FROM solution_progress_comments s LEFT JOIN task_work_log_comments t ON t.id=s.id WHERE t.id IS NULL OR t.entry_id<>s.entry_id OR t.body<>s.body OR t.created_at<>s.created_at"),
            ("checklist", "SELECT count(*) FROM solution_checklist_items s LEFT JOIN task_checklist_items t ON t.id=s.id WHERE t.id IS NULL OR t.task_id<>s.feature_id OR t.body<>s.body OR t.checked<>s.checked OR t.created_at<>s.created_at OR t.updated_at<>s.updated_at"),
            ("completion", "SELECT count(*) FROM completions s LEFT JOIN task_completions t ON t.id=s.id WHERE t.id IS NULL OR t.task_id<>s.feature_id OR t.evidence<>s.evidence OR t.report<>s.report OR t.created_at<>s.created_at"),
            ("attachment", "SELECT count(*) FROM solution_progress_entries s LEFT JOIN task_attachments t ON t.entry_id=s.id WHERE s.image_data<>'' AND (t.id IS NULL OR t.data<>s.image_data OR t.media_type<>s.image_media_type)"),
            ("completed_evidence", "SELECT count(*) FROM tasks t WHERE t.state='completed' AND (SELECT count(*) FROM task_work_log_entries w WHERE w.task_id=t.id)<>(SELECT count(*) FROM solution_progress_entries w WHERE w.feature_id=t.id)"),
        ] {
            let mismatches: i64 = tx
                .query_row(sql, [], |row| row.get(0))
                .map_err(|error| error.to_string())?;
            if mismatches != 0 {
                return Err(format!("migration_invariant_{name}"));
            }
        }
        for (table, count) in &self.counts {
            if LEGACY_LEDGER_TABLES.contains(&table.as_str()) {
                let ledger: i64 = tx
                    .query_row(
                        "SELECT count(*) FROM legacy_task_migration_records WHERE source_table=?",
                        [table],
                        |row| row.get(0),
                    )
                    .map_err(|error| error.to_string())?;
                if ledger != *count {
                    return Err(format!("migration_invariant_ledger:{table}"));
                }
            }
        }
        let foreign_key_errors: i64 = tx
            .query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |row| {
                row.get(0)
            })
            .map_err(|error| error.to_string())?;
        if foreign_key_errors != 0 {
            return Err("migration_invariant_foreign_keys".into());
        }
        let problem_sql: String = tx
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type='table' AND name='problems'",
                [],
                |row| row.get(0),
            )
            .map_err(|error| error.to_string())?;
        if problem_sql
            .to_ascii_lowercase()
            .replace(' ', "")
            .contains("capture_idtextunique")
        {
            return Err("migration_invariant_problem_capture_unique".into());
        }
        Ok(())
    }
}

fn table_count(tx: &Transaction<'_>, table: &str) -> Result<i64, String> {
    tx.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
        row.get(0)
    })
    .map_err(|error| error.to_string())
}

fn propagate_solution_child_update_timestamps(tx: &Transaction<'_>) -> Result<(), String> {
    for (table, feature_id) in [
        ("solution_progress_entries", "{row}.feature_id"),
        ("solution_checklist_items", "{row}.feature_id"),
        ("completions", "{row}.feature_id"),
        (
            "solution_progress_comments",
            "(SELECT feature_id FROM solution_progress_entries WHERE id={row}.entry_id)",
        ),
    ] {
        if !table_exists(tx, table)? {
            continue;
        }
        for (action, row) in [("INSERT", "NEW"), ("UPDATE", "NEW"), ("DELETE", "OLD")] {
            tx.execute_batch(&format!(
                "CREATE TRIGGER overview_feature_timestamp_{table}_{action} AFTER {action} ON {table} BEGIN
                   UPDATE work_tracking_entity_versions
                   SET revision=revision+1
                   WHERE entity_type='features' AND entity_id={};
                 END;",
                feature_id.replace("{row}", row),
            ))
            .map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

fn add_workbench_entity_update_timestamps(tx: &Transaction<'_>) -> Result<(), String> {
    add_missing_column(
        tx,
        "work_tracking_entity_versions",
        "updated_at",
        "TEXT NOT NULL DEFAULT ''",
    )?;
    tx.execute_batch(
        "CREATE TRIGGER tracked_entity_version_inserted AFTER INSERT ON work_tracking_entity_versions BEGIN
           UPDATE work_tracking_entity_versions
           SET updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now')
           WHERE entity_type=NEW.entity_type AND entity_id=NEW.entity_id;
         END;
         CREATE TRIGGER tracked_entity_version_changed AFTER UPDATE OF revision ON work_tracking_entity_versions BEGIN
           UPDATE work_tracking_entity_versions
           SET updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now')
           WHERE entity_type=NEW.entity_type AND entity_id=NEW.entity_id;
         END;",
    )
    .map_err(|error| error.to_string())?;
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
        for (action, row) in [("INSERT", "NEW"), ("UPDATE", "NEW"), ("DELETE", "OLD")] {
            tx.execute_batch(&format!(
                "CREATE TRIGGER tracked_updated_at_{table}_{action} AFTER {action} ON {table} BEGIN
                   UPDATE work_tracking_entity_versions
                   SET updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now')
                   WHERE entity_type='{table}' AND entity_id={row}.id;
                 END;"
            ))
            .map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

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

pub(crate) fn schema_version(connection: &Connection) -> Result<i64, String> {
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
    fn schema_eight_preserves_rich_legacy_records_and_is_idempotent() {
        let mut connection = Connection::open_in_memory().unwrap();
        apply_plan(&mut connection, &MIGRATIONS[..7], 7).unwrap();
        connection.execute_batch("INSERT INTO captures(id,text,created_at) VALUES('c1','solution already stated','2026-01-01');
          INSERT INTO problems(id,capture_id,statement,detail,state,created_at) VALUES('p1','c1','Problem','Detail','completed','2026-01-02');
          INSERT INTO problems(id,capture_id,statement,detail,state,created_at) VALUES('orphan-problem',NULL,'Unassigned','Detail','draft','2026-01-02');
          INSERT INTO features(id,problem_id,title,outcome,non_goals,validation_criteria,state,created_at) VALUES('f1','p1','Task title','Outcome','No scope creep','Evidence exists','archived','2026-01-03');
          INSERT INTO solution_progress_entries(id,feature_id,body,image_data,image_media_type,image_summary,created_at) VALUES('e1','f1','work','aGVsbG8=','image/png','screen','2026-01-04');
          INSERT INTO solution_progress_comments(id,entry_id,body,created_at) VALUES('comment1','e1','note','2026-01-05');
          INSERT INTO solution_checklist_items(id,feature_id,body,checked,created_at,updated_at) VALUES('check1','f1','verified',1,'2026-01-05','2026-01-06');
          INSERT INTO completions(id,feature_id,evidence,report,created_at) VALUES('complete1','f1','evidence','report','2026-01-07');
          INSERT INTO approvals(id,entity_type,entity_id,action,created_at) VALUES('approval1','features','f1','approve','2026-01-03');
          INSERT INTO conflict_reports(id,feature_id,state,citation,created_at) VALUES('report1','f1','conflicted','Doc.md','2026-01-04');
          INSERT INTO lineage_snapshots(id,feature_id,version,schema_version,source_hash,status,document_json,created_at) VALUES('lineage1','f1',1,1,'hash','ready','{}','2026-01-07');
          INSERT INTO localized_content(entity_type,entity_id,field_name,locale,value,origin,source_hash,created_at,updated_at) VALUES('features','f1','title','ko','작업','user','lh','2026-01-03','2026-01-03');
          INSERT INTO workbench_category_overrides(entity_type,entity_id,category) VALUES('features','f1','Delivery');
          INSERT INTO workbench_priority_overrides(entity_type,entity_id,manual_priority) VALUES('features','f1',7);
          INSERT INTO deleted_entities(entity_type,entity_id,deleted_at) VALUES('features','f1','2026-01-08');").unwrap();

        apply(&mut connection).unwrap();
        apply(&mut connection).unwrap();

        assert_eq!(schema_version(&connection).unwrap(), 8);
        let task: (String, String, Option<String>) = connection
            .query_row(
                "SELECT state,category,archived_at FROM tasks WHERE id='f1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(task.0, "completed");
        assert_eq!(task.1, "Delivery");
        assert!(task.2.is_some());
        assert_eq!(
            connection
                .query_row(
                    "SELECT image_data FROM task_work_log_entries WHERE id='e1'",
                    [],
                    |row| row.get::<_, String>(0)
                )
                .unwrap(),
            "aGVsbG8="
        );
        assert_eq!(connection.query_row("SELECT count(*) FROM legacy_task_migration_records WHERE source_id IN ('report1','lineage1')",[],|row|row.get::<_,i64>(0)).unwrap(),2);
        assert_eq!(connection.query_row("SELECT value FROM localized_content WHERE entity_type='tasks' AND entity_id='f1' AND field_name='title' AND locale='ko'",[],|row|row.get::<_,String>(0)).unwrap(),"작업");
        assert_eq!(connection.query_row("SELECT problem_id FROM refinement_items WHERE id='legacy-problem-orphan-problem'",[],|row|row.get::<_,String>(0)).unwrap(),"orphan-problem");
        connection.execute("INSERT INTO problems(id,capture_id,statement,created_at,current_revision) VALUES('p2','c1','Second independent problem','2026-01-09',1)",[]).unwrap();
        assert_eq!(
            connection
                .query_row(
                    "SELECT count(*) FROM problems WHERE capture_id='c1'",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            2
        );
    }

    #[test]
    fn invalid_schema_eight_mapping_rolls_back_every_table_and_version() {
        let mut connection = Connection::open_in_memory().unwrap();
        apply_plan(&mut connection, &MIGRATIONS[..7], 7).unwrap();
        connection.execute_batch("INSERT INTO captures(id,text,created_at) VALUES('c1','x','2026-01-01');
          INSERT INTO problems(id,capture_id,statement,state,created_at) VALUES('p1','c1','P','draft','2026-01-01');
          INSERT INTO features(id,problem_id,title,outcome,state,created_at) VALUES('bad','p1','Bad','Bad','unknown-state','2026-01-01');").unwrap();
        let error = apply(&mut connection).unwrap_err();
        assert!(error.contains("unknown legacy feature state"));
        assert_eq!(schema_version(&connection).unwrap(), 7);
        assert!(!table_exists(&connection, "tasks").unwrap());
        assert_eq!(
            connection
                .query_row("SELECT state FROM features WHERE id='bad'", [], |row| {
                    row.get::<_, String>(0)
                })
                .unwrap(),
            "unknown-state"
        );
    }

    #[test]
    fn large_attachment_fixture_preserves_every_id_and_byte() {
        let mut connection = Connection::open_in_memory().unwrap();
        apply_plan(&mut connection, &MIGRATIONS[..7], 7).unwrap();
        connection.execute_batch("INSERT INTO captures(id,text,created_at) VALUES('large-capture','large','2026-01-01'); INSERT INTO problems(id,capture_id,statement,state,created_at) VALUES('large-problem','large-capture','Large','draft','2026-01-01');").unwrap();
        let image = "A".repeat(32 * 1024);
        let transaction = connection.transaction().unwrap();
        for index in 0..128 {
            let feature = format!("large-task-{index}");
            let entry = format!("large-entry-{index}");
            transaction.execute("INSERT INTO features(id,problem_id,title,outcome,state,created_at) VALUES(?,'large-problem',?,'outcome','proposed','2026-01-02')",params![feature,feature]).unwrap();
            transaction.execute("INSERT INTO solution_progress_entries(id,feature_id,body,image_data,image_media_type,image_summary,created_at) VALUES(?,?,?,?,'image/png','large','2026-01-03')",params![entry,feature,format!("body-{index}"),image]).unwrap();
        }
        transaction.commit().unwrap();
        apply(&mut connection).unwrap();
        assert_eq!(
            connection
                .query_row(
                    "SELECT count(*) FROM tasks WHERE id LIKE 'large-task-%'",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            128
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT count(*) FROM task_attachments WHERE length(data)=32768",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            128
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT count(*) FROM task_work_log_entries WHERE image_data<>?",
                    [image],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            0
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
