//! Evidence-backed Task journey projection.
//!
//! This deliberately models recorded changes and work as a trace.  An edge means
//! only that one event followed another in the Task record; it never implies that
//! a message, work-log entry, or decision caused a later change.

use rusqlite::{OptionalExtension, Transaction};
use serde_json::{json, Value};
use std::collections::HashSet;

pub(crate) fn build_tx(connection: &Transaction<'_>, task_id: &str) -> Result<Value, String> {
    let task: (Option<String>, String, Option<String>, Option<String>) = connection
        .query_row(
            "SELECT origin_capture_id,created_at,started_at,completed_at FROM tasks WHERE id=?",
            [task_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .ok_or("Task not found")?;

    let mut events = Vec::<Value>::new();
    let mut context_edges = Vec::<Value>::new();
    if let Some(capture_id) = task.0 {
        let capture: Option<(String, String)> = connection
            .query_row(
                "SELECT text,created_at FROM captures WHERE id=?",
                [&capture_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|error| error.to_string())?;
        if let Some((text, created_at)) = capture {
            events.push(event(
                format!("capture:{capture_id}"),
                "origin_capture",
                created_at,
                json!({"captureId":capture_id,"summary":text}),
            ));
        }
    }

    let mut revisions = connection
        .prepare("SELECT revision,title,detail,outcome,scope,non_goals,validation_criteria,source_reference,created_at FROM task_revisions WHERE task_id=? ORDER BY revision")
        .map_err(|error| error.to_string())?;
    let revisions = revisions
        .query_map([task_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
            ))
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    let mut prior: Option<Vec<String>> = None;
    for (revision, title, detail, outcome, scope, non_goals, validation_criteria, source, created_at) in revisions {
        let fields = vec![title.clone(), detail, outcome, scope, non_goals, validation_criteria];
        let changes = prior
            .as_ref()
            .map(|previous| revision_changes(previous, &fields))
            .unwrap_or_default();
        let event_type = if revision == 1 {
            "task_created"
        } else if source == "refinement" {
            "refinement_applied"
        } else {
            "task_updated"
        };
        events.push(event(
            format!("revision:{task_id}:{revision}"),
            event_type,
            created_at,
            json!({"taskRevision":revision,"title":title,"changes":changes,"source":source}),
        ));
        prior = Some(fields);
    }

    if let Some(started_at) = task.2 {
        events.push(event(
            format!("started:{task_id}"),
            "execution_started",
            started_at,
            json!({}),
        ));
    }
    append_rows(
        connection,
        "SELECT id,body,created_at FROM task_work_log_entries WHERE task_id=? ORDER BY created_at,id",
        task_id,
        |id, summary, created_at| event(format!("work:{id}"), "work_recorded", created_at, json!({"workLogId":id,"summary":summary})),
        &mut events,
    )?;
    let mut seen_refinement_messages = HashSet::new();
    let mut messages = connection.prepare("SELECT m.id,m.content,m.created_at,d.result_json FROM refinement_messages m JOIN refinement_sessions s ON s.id=m.session_id JOIN refinement_proposal_decisions d ON d.session_id=s.id WHERE (s.task_id=? OR s.capture_id=(SELECT origin_capture_id FROM tasks WHERE id=?)) AND m.role='user' AND m.created_at<=d.created_at AND d.decision!='reject' AND json_extract(d.result_json,'$.id')=? AND json_extract(d.result_json,'$.taskRevision') IS NOT NULL ORDER BY m.created_at,m.id").map_err(|error| error.to_string())?;
    for row in messages.query_map([task_id, task_id, task_id], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?))).map_err(|error| error.to_string())? {
        let (id, content, created_at, result_json) = row.map_err(|error| error.to_string())?;
        let message_id = format!("refinement-message:{id}");
        if seen_refinement_messages.insert(message_id.clone()) {
            events.push(event(message_id.clone(), "refinement_input", created_at, json!({"messageId":id,"summary":content})));
        }
        if let Some(revision) = serde_json::from_str::<Value>(&result_json).ok().and_then(|result| result["taskRevision"].as_i64()) {
            // The session relationship is explicit, but a message still does not prove
            // that it caused the applied patch.  The edge is intentionally contextual.
            context_edges.push(json!({"from":message_id,"to":format!("revision:{task_id}:{revision}"),"kind":"refinement_context"}));
        }
    }
    // Decision rows include their own timestamp.  Keep it outside the generic helper so
    // malformed legacy payloads cannot replace a trustworthy database timestamp.
    let mut decisions = connection.prepare("SELECT id,kind,payload_json,created_at FROM task_decisions WHERE task_id=? ORDER BY created_at,id").map_err(|error| error.to_string())?;
    for row in decisions.query_map([task_id], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?))).map_err(|error| error.to_string())? {
        let (id, kind, payload, created_at) = row.map_err(|error| error.to_string())?;
        events.push(event(format!("decision:{id}"), "decision_recorded", created_at, json!({"decisionId":id,"decisionKind":kind,"payload":serde_json::from_str::<Value>(&payload).unwrap_or(Value::Null)})));
    }
    let mut completions = connection.prepare("SELECT id,evidence,report,created_at FROM task_completions WHERE task_id=? ORDER BY created_at,id").map_err(|error| error.to_string())?;
    for row in completions.query_map([task_id], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?))).map_err(|error| error.to_string())? {
        let (id, evidence, report, created_at) = row.map_err(|error| error.to_string())?;
        events.push(event(format!("completion:{id}"), "task_completed", created_at, json!({"completionId":id,"summary":evidence,"report":report})));
    }
    if task.3.is_some() && !events.iter().any(|item| item["type"] == "task_completed") {
        events.push(event(format!("completed:{task_id}"), "task_completed", task.3.unwrap_or_default(), json!({"summary":""})));
    }

    // SQLite timestamps can tie. `sort_by` is stable, so tied events retain the
    // deterministic record order established above instead of inventing an order
    // from opaque IDs.
    events.sort_by(|left, right| left["occurredAt"].as_str().cmp(&right["occurredAt"].as_str()).then_with(|| event_priority(&left["type"]).cmp(&event_priority(&right["type"]))));
    let mut edges = events.windows(2).map(|pair| json!({"from":pair[0]["id"],"to":pair[1]["id"],"kind":"followed_by"})).collect::<Vec<_>>();
    edges.extend(context_edges);
    Ok(json!({"events":events,"edges":edges,"semantics":{"edge":"followed_by","causalInference":false}}))
}

fn event_priority(event_type: &Value) -> u8 {
    match event_type.as_str().unwrap_or("") {
        "origin_capture" => 0,
        "task_created" => 1,
        "refinement_input" => 2,
        "refinement_applied" | "task_updated" => 3,
        "execution_started" => 4,
        "work_recorded" => 5,
        "decision_recorded" => 6,
        "task_completed" => 7,
        _ => 8,
    }
}

fn event(id: String, event_type: &str, occurred_at: String, detail: Value) -> Value {
    json!({"id":id,"type":event_type,"occurredAt":occurred_at,"detail":detail})
}

fn revision_changes(previous: &[String], current: &[String]) -> Vec<Value> {
    ["title", "detail", "outcome", "scope", "nonGoals", "validationCriteria"]
        .into_iter()
        .enumerate()
        .filter_map(|(index, name)| (previous.get(index) != current.get(index)).then(|| json!({"field":name,"before":excerpt(previous.get(index).map(String::as_str).unwrap_or("")),"after":excerpt(current.get(index).map(String::as_str).unwrap_or(""))})))
        .collect()
}

fn excerpt(value: &str) -> String {
    let mut result = value.chars().take(140).collect::<String>();
    if value.chars().count() > 140 { result.push('…'); }
    result
}

fn append_rows<F>(
    connection: &Transaction<'_>, sql: &str, task_id: &str, mut map: F, events: &mut Vec<Value>,
) -> Result<(), String>
where F: FnMut(String, String, String) -> Value {
    let mut statement = connection.prepare(sql).map_err(|error| error.to_string())?;
    for row in statement.query_map([task_id], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))).map_err(|error| error.to_string())? {
        let (id, summary, created_at) = row.map_err(|error| error.to_string())?;
        events.push(map(id, summary, created_at));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{build_tx, revision_changes};
    use rusqlite::Connection;

    #[test]
    fn reports_only_fields_that_actually_changed() {
        let before = vec!["Title".into(), "".into(), "Done".into(), "Local".into(), "".into(), "Tests".into()];
        let after = vec!["Title".into(), "".into(), "Done".into(), "Broader".into(), "".into(), "Tests".into()];
        assert_eq!(revision_changes(&before, &after)[0]["field"], "scope");
    }

    #[test]
    fn projects_refinement_context_and_execution_records_without_claiming_causality() {
        let mut connection = Connection::open_in_memory().unwrap();
        connection.execute_batch("CREATE TABLE tasks(id TEXT PRIMARY KEY,origin_capture_id TEXT,created_at TEXT,started_at TEXT,completed_at TEXT); CREATE TABLE captures(id TEXT PRIMARY KEY,text TEXT,created_at TEXT); CREATE TABLE task_revisions(task_id TEXT,revision INTEGER,title TEXT,detail TEXT,outcome TEXT,scope TEXT,non_goals TEXT,validation_criteria TEXT,source_reference TEXT,created_at TEXT); CREATE TABLE task_work_log_entries(id TEXT,task_id TEXT,body TEXT,created_at TEXT); CREATE TABLE task_decisions(id TEXT,task_id TEXT,kind TEXT,payload_json TEXT,created_at TEXT); CREATE TABLE task_completions(id TEXT,task_id TEXT,evidence TEXT,report TEXT,created_at TEXT); CREATE TABLE refinement_sessions(id TEXT,task_id TEXT,capture_id TEXT); CREATE TABLE refinement_messages(id TEXT,session_id TEXT,role TEXT,content TEXT,created_at TEXT); CREATE TABLE refinement_proposal_decisions(session_id TEXT,decision TEXT,result_json TEXT,created_at TEXT);").unwrap();
        connection.execute_batch("INSERT INTO captures VALUES('capture','새 아이디어: 내보내기를 추가','2026-01-01T09:00:00Z'); INSERT INTO tasks VALUES('task','capture','2026-01-01T09:01:00Z','2026-01-01T10:00:00Z','2026-01-01T12:00:00Z'); INSERT INTO task_revisions VALUES('task',1,'내보내기','','','파일','','','capture','2026-01-01T09:01:00Z'); INSERT INTO task_revisions VALUES('task',2,'안전한 내보내기','','','빈 파일도 처리','','','refinement','2026-01-01T09:30:00Z'); INSERT INTO refinement_sessions VALUES('session',NULL,'capture'); INSERT INTO refinement_messages VALUES('message','session','user','Edge Case: 빈 파일이면 기존 결정을 번복','2026-01-01T09:20:00Z'); INSERT INTO refinement_proposal_decisions VALUES('session','accept','{\"id\":\"task\",\"taskRevision\":2}','2026-01-01T09:30:00Z'); INSERT INTO task_decisions VALUES('decision','task','release','{\"rationale\":\"빈 파일 처리 후 출시\"}','2026-01-01T10:30:00Z'); INSERT INTO task_work_log_entries VALUES('work','task','빈 파일 테스트를 추가','2026-01-01T11:00:00Z'); INSERT INTO task_completions VALUES('completion','task','테스트 통과','','2026-01-01T12:00:00Z');").unwrap();
        let tx = connection.transaction().unwrap();
        let journey = build_tx(&tx, "task").unwrap();
        assert!(journey["events"].as_array().unwrap().iter().any(|event| event["type"] == "refinement_input" && event["detail"]["summary"] == "Edge Case: 빈 파일이면 기존 결정을 번복"));
        assert!(journey["events"].as_array().unwrap().iter().any(|event| event["type"] == "refinement_applied" && event["detail"]["changes"][0]["field"] == "title"));
        assert!(journey["edges"].as_array().unwrap().iter().any(|edge| edge["kind"] == "refinement_context"));
        let input_position = journey["events"].as_array().unwrap().iter().position(|event| event["type"] == "refinement_input").unwrap();
        let applied_position = journey["events"].as_array().unwrap().iter().position(|event| event["type"] == "refinement_applied").unwrap();
        assert!(input_position < applied_position);
        assert_eq!(journey["semantics"]["causalInference"], false);
    }
}
