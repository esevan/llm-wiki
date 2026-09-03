use crate::native::database;
use rusqlite::{params, OptionalExtension, Row};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::Path;
use uuid::Uuid;

fn id() -> String {
    Uuid::new_v4().to_string()
}

fn required_text<'a>(input: &'a Value, key: &str) -> Result<&'a str, String> {
    input
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("{key} is required"))
}

fn capture_row(row: &Row<'_>) -> rusqlite::Result<Value> {
    Ok(json!({
        "id": row.get::<_, String>(0)?,
        "text": row.get::<_, String>(1)?,
        "created_at": row.get::<_, String>(2)?,
        "category": row.get::<_, Option<String>>(3)?.unwrap_or_else(|| "General".into()),
        "important": row.get::<_, i64>(4)? != 0,
        "localized_versions": {},
    }))
}

fn problem_row(row: &Row<'_>) -> rusqlite::Result<Value> {
    Ok(json!({
        "id": row.get::<_, String>(0)?,
        "capture_id": row.get::<_, Option<String>>(1)?,
        "statement": row.get::<_, String>(2)?,
        "detail": row.get::<_, String>(3)?,
        "state": row.get::<_, String>(4)?,
        "created_at": row.get::<_, String>(5)?,
        "category": row.get::<_, Option<String>>(6)?.unwrap_or_else(|| "General".into()),
        "important": row.get::<_, i64>(7)? != 0,
        "localized_versions": {},
    }))
}

fn feature_row(row: &Row<'_>) -> rusqlite::Result<Value> {
    Ok(json!({
        "id": row.get::<_, String>(0)?,
        "problem_id": row.get::<_, String>(1)?,
        "title": row.get::<_, String>(2)?,
        "outcome": row.get::<_, String>(3)?,
        "non_goals": row.get::<_, String>(4)?,
        "conflict_state": row.get::<_, String>(5)?,
        "validation_criteria": row.get::<_, String>(6)?,
        "state": row.get::<_, String>(7)?,
        "created_at": row.get::<_, String>(8)?,
        "category": row.get::<_, Option<String>>(9)?.unwrap_or_else(|| "General".into()),
        "important": row.get::<_, i64>(10)? != 0,
        "localized_versions": {},
    }))
}

pub fn create_capture(db_path: &Path, input: &Value) -> Result<Value, String> {
    let text = required_text(input, "text")?.trim();
    if text.len() > 20_000 {
        return Err("Capture text is too long".into());
    }
    let connection = database::open(db_path)?;
    let capture_id = id();
    connection
        .execute(
            "INSERT INTO captures(id,text) VALUES (?,?)",
            params![capture_id, text],
        )
        .map_err(|error| error.to_string())?;
    Ok(json!({"id": capture_id, "text": text}))
}

pub fn board(db_path: &Path) -> Result<Value, String> {
    let connection = database::open(db_path)?;
    let mut captures = connection
        .prepare(
            "SELECT c.id,c.text,c.created_at,w.category,
             EXISTS(SELECT 1 FROM workbench_priority_overrides o WHERE o.entity_type='captures' AND o.entity_id=c.id AND o.manual_priority=1)
             FROM captures c LEFT JOIN workbench_category_overrides w ON w.entity_type='captures' AND w.entity_id=c.id
             WHERE NOT EXISTS(SELECT 1 FROM problems p WHERE p.capture_id=c.id)
             AND NOT EXISTS(SELECT 1 FROM deleted_entities d WHERE d.entity_type='captures' AND d.entity_id=c.id)
             ORDER BY c.created_at",
        )
        .map_err(|error| error.to_string())?;
    let captures = captures
        .query_map([], capture_row)
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    let mut problems = connection
        .prepare(
            "SELECT p.id,p.capture_id,p.statement,p.detail,p.state,p.created_at,w.category,
             EXISTS(SELECT 1 FROM workbench_priority_overrides o WHERE o.entity_type='problems' AND o.entity_id=p.id AND o.manual_priority=1)
             FROM problems p LEFT JOIN workbench_category_overrides w ON w.entity_type='problems' AND w.entity_id=p.id
             WHERE NOT EXISTS(SELECT 1 FROM deleted_entities d WHERE d.entity_type='problems' AND d.entity_id=p.id)
             ORDER BY p.created_at",
        )
        .map_err(|error| error.to_string())?;
    let problems = problems
        .query_map([], problem_row)
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    let mut features = connection
        .prepare(
            "SELECT f.id,f.problem_id,f.title,f.outcome,f.non_goals,f.conflict_state,f.validation_criteria,f.state,f.created_at,w.category,
             EXISTS(SELECT 1 FROM workbench_priority_overrides o WHERE o.entity_type='features' AND o.entity_id=f.id AND o.manual_priority=1)
             FROM features f LEFT JOIN workbench_category_overrides w ON w.entity_type='features' AND w.entity_id=f.id
             WHERE NOT EXISTS(SELECT 1 FROM deleted_entities d WHERE d.entity_type='features' AND d.entity_id=f.id)
             ORDER BY f.created_at",
        )
        .map_err(|error| error.to_string())?;
    let features = features
        .query_map([], feature_row)
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    Ok(json!({"captures": captures, "problems": problems, "features": features}))
}

pub fn promote_capture(db_path: &Path, capture_id: &str, input: &Value) -> Result<Value, String> {
    let connection = database::open(db_path)?;
    let source: String = connection
        .query_row(
            "SELECT text FROM captures WHERE id=?",
            [capture_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .ok_or("Capture not found")?;
    let statement = input
        .get("statement")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(&source);
    let detail = input.get("detail").and_then(Value::as_str).unwrap_or("");
    let existing: Option<String> = connection
        .query_row(
            "SELECT id FROM problems WHERE capture_id=?",
            [capture_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let problem_id = existing.unwrap_or_else(id);
    connection
        .execute(
            "INSERT OR IGNORE INTO problems(id,capture_id,statement,detail) VALUES (?,?,?,?)",
            params![problem_id, capture_id, statement, detail],
        )
        .map_err(|error| error.to_string())?;
    Ok(
        json!({"id": problem_id, "capture_id": capture_id, "statement": statement, "detail": detail, "state": "draft"}),
    )
}

pub fn approve_problem(db_path: &Path, problem_id: &str) -> Result<Value, String> {
    let connection = database::open(db_path)?;
    let changed = connection
        .execute(
            "UPDATE problems SET state='approved' WHERE id=?",
            [problem_id],
        )
        .map_err(|error| error.to_string())?;
    if changed == 0 {
        return Err("Problem not found".into());
    }
    connection
        .execute(
            "INSERT INTO approvals(id,entity_type,entity_id,action) VALUES (?,'problems',?,'approve')",
            params![id(), problem_id],
        )
        .map_err(|error| error.to_string())?;
    Ok(Value::Null)
}

pub fn create_feature(db_path: &Path, problem_id: &str, input: &Value) -> Result<Value, String> {
    let title = required_text(input, "title")?;
    let outcome = required_text(input, "outcome")?;
    let criteria = required_text(input, "validation_criteria")?;
    let non_goals = input.get("non_goals").and_then(Value::as_str).unwrap_or("");
    let connection = database::open(db_path)?;
    let state: Option<String> = connection
        .query_row(
            "SELECT state FROM problems WHERE id=?",
            [problem_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    if state.as_deref() != Some("approved") {
        return Err("Problem must be approved before creating a Solution".into());
    }
    let feature_id = id();
    connection
        .execute(
            "INSERT INTO features(id,problem_id,title,outcome,non_goals,validation_criteria) VALUES (?,?,?,?,?,?)",
            params![feature_id, problem_id, title, outcome, non_goals, criteria],
        )
        .map_err(|error| error.to_string())?;
    for line in criteria.lines() {
        let body = line
            .trim()
            .trim_start_matches('-')
            .trim()
            .trim_start_matches("[ ]")
            .trim()
            .trim_start_matches("[x]")
            .trim();
        if !body.is_empty() {
            connection.execute(
                "INSERT INTO solution_checklist_items(id,feature_id,body,checked) VALUES (?,?,?,?)",
                params![id(), feature_id, body, i64::from(line.contains("[x]"))],
            ).map_err(|error| error.to_string())?;
        }
    }
    Ok(
        json!({"id": feature_id, "problem_id": problem_id, "title": title, "outcome": outcome, "non_goals": non_goals, "validation_criteria": criteria, "state": "proposed", "conflict_state": "unknown"}),
    )
}

pub fn set_conflict(db_path: &Path, feature_id: &str, input: &Value) -> Result<Value, String> {
    let state = required_text(input, "state")?;
    if !matches!(state, "clear" | "conflict" | "unknown") {
        return Err("Unsupported conflict state".into());
    }
    let citation = input.get("citation").and_then(Value::as_str).unwrap_or("");
    if state != "unknown" && citation.trim().is_empty() {
        return Err("A citation is required for a reviewed conflict state".into());
    }
    let connection = database::open(db_path)?;
    if connection
        .execute(
            "UPDATE features SET conflict_state=? WHERE id=?",
            params![state, feature_id],
        )
        .map_err(|error| error.to_string())?
        == 0
    {
        return Err("Solution not found".into());
    }
    let report_id = id();
    connection
        .execute(
            "INSERT INTO conflict_reports(id,feature_id,state,citation) VALUES (?,?,?,?)",
            params![report_id, feature_id, state, citation],
        )
        .map_err(|error| error.to_string())?;
    Ok(json!({"id": report_id, "feature_id": feature_id, "state": state, "citation": citation}))
}

pub fn approve_feature(db_path: &Path, feature_id: &str) -> Result<Value, String> {
    let connection = database::open(db_path)?;
    let conflict: Option<String> = connection
        .query_row(
            "SELECT conflict_state FROM features WHERE id=?",
            [feature_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    if conflict.as_deref() != Some("clear") {
        return Err("Solution requires a clear conflict review before approval".into());
    }
    connection
        .execute(
            "UPDATE features SET state='approved' WHERE id=?",
            [feature_id],
        )
        .map_err(|error| error.to_string())?;
    connection
        .execute(
            "INSERT INTO approvals(id,entity_type,entity_id,action) VALUES (?,'features',?,'approve')",
            params![id(), feature_id],
        )
        .map_err(|error| error.to_string())?;
    Ok(Value::Null)
}

pub fn set_stage(db_path: &Path, feature_id: &str, input: &Value) -> Result<Value, String> {
    let state = required_text(input, "state")?;
    if !matches!(state, "proposed" | "in_progress" | "approved") {
        return Err("Unsupported Solution stage".into());
    }
    let connection = database::open(db_path)?;
    if connection
        .execute(
            "UPDATE features SET state=? WHERE id=?",
            params![state, feature_id],
        )
        .map_err(|error| error.to_string())?
        == 0
    {
        return Err("Solution not found".into());
    }
    Ok(Value::Null)
}

pub fn progress(db_path: &Path, feature_id: &str) -> Result<Value, String> {
    let connection = database::open(db_path)?;
    let mut statement = connection.prepare(
        "SELECT id,body,image_data,image_media_type,image_summary,created_at FROM solution_progress_entries WHERE feature_id=? ORDER BY created_at"
    ).map_err(|error| error.to_string())?;
    let entries = statement.query_map([feature_id], |row| {
        let entry_id: String = row.get(0)?;
        let mut comments_statement = connection.prepare("SELECT id,body,created_at FROM solution_progress_comments WHERE entry_id=? ORDER BY created_at")?;
        let comments = comments_statement.query_map([&entry_id], |comment| Ok(json!({"id": comment.get::<_,String>(0)?, "body": comment.get::<_,String>(1)?, "created_at": comment.get::<_,String>(2)?})))?.collect::<Result<Vec<_>,_>>()?;
        Ok(json!({"id":entry_id,"body":row.get::<_,String>(1)?,"image_data":row.get::<_,String>(2)?,"image_media_type":row.get::<_,String>(3)?,"image_summary":row.get::<_,String>(4)?,"created_at":row.get::<_,String>(5)?,"comments":comments,"localized_versions":{}}))
    }).map_err(|error| error.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|error| error.to_string())?;
    let mut checklist_statement = connection.prepare("SELECT id,body,checked,created_at,updated_at FROM solution_checklist_items WHERE feature_id=? ORDER BY created_at").map_err(|error| error.to_string())?;
    let checklist = checklist_statement.query_map([feature_id], |row| Ok(json!({"id":row.get::<_,String>(0)?,"body":row.get::<_,String>(1)?,"checked":row.get::<_,i64>(2)?,"created_at":row.get::<_,String>(3)?,"updated_at":row.get::<_,String>(4)?}))).map_err(|error| error.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|error| error.to_string())?;
    Ok(json!({"entries": entries, "checklist": checklist}))
}

pub fn add_progress(db_path: &Path, feature_id: &str, input: &Value) -> Result<Value, String> {
    let body = input.get("body").and_then(Value::as_str).unwrap_or("");
    let image_data = input
        .get("image_data")
        .and_then(Value::as_str)
        .unwrap_or("");
    if body.trim().is_empty() && image_data.is_empty() {
        return Err("Work Log requires text or an image".into());
    }
    let media_type = input
        .get("image_media_type")
        .and_then(Value::as_str)
        .unwrap_or("");
    let connection = database::open(db_path)?;
    let entry_id = id();
    connection.execute("INSERT INTO solution_progress_entries(id,feature_id,body,image_data,image_media_type) VALUES (?,?,?,?,?)", params![entry_id,feature_id,body,image_data,media_type]).map_err(|error| error.to_string())?;
    Ok(
        json!({"id":entry_id,"feature_id":feature_id,"body":body,"image_data":image_data,"image_media_type":media_type,"image_summary":"","comments":[]}),
    )
}

pub fn add_comment(db_path: &Path, entry_id: &str, input: &Value) -> Result<Value, String> {
    let body = required_text(input, "body")?;
    let connection = database::open(db_path)?;
    let comment_id = id();
    connection
        .execute(
            "INSERT INTO solution_progress_comments(id,entry_id,body) VALUES (?,?,?)",
            params![comment_id, entry_id, body],
        )
        .map_err(|error| error.to_string())?;
    Ok(json!({"id":comment_id,"entry_id":entry_id,"body":body}))
}

pub fn add_checklist(db_path: &Path, feature_id: &str, input: &Value) -> Result<Value, String> {
    let body = required_text(input, "body")?;
    let checked = input
        .get("checked")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let connection = database::open(db_path)?;
    let item_id = id();
    connection
        .execute(
            "INSERT INTO solution_checklist_items(id,feature_id,body,checked) VALUES (?,?,?,?)",
            params![item_id, feature_id, body, i64::from(checked)],
        )
        .map_err(|error| error.to_string())?;
    Ok(json!({"id":item_id,"feature_id":feature_id,"body":body,"checked":i64::from(checked)}))
}

pub fn update_checklist(db_path: &Path, item_id: &str, input: &Value) -> Result<Value, String> {
    let body = required_text(input, "body")?;
    let checked = input
        .get("checked")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let connection = database::open(db_path)?;
    if connection.execute("UPDATE solution_checklist_items SET body=?,checked=?,updated_at=CURRENT_TIMESTAMP WHERE id=?", params![body,i64::from(checked),item_id]).map_err(|error| error.to_string())? == 0 {
        return Err("Checklist item not found".into());
    }
    Ok(Value::Null)
}

pub fn create_goal(db_path: &Path, input: &Value) -> Result<Value, String> {
    let title = required_text(input, "title")?;
    let description = input
        .get("description")
        .and_then(Value::as_str)
        .unwrap_or("");
    let connection = database::open(db_path)?;
    let goal_id = id();
    connection
        .execute(
            "INSERT INTO compass_goals(id,title,description) VALUES (?,?,?)",
            params![goal_id, title, description],
        )
        .map_err(|error| error.to_string())?;
    Ok(json!({"id":goal_id,"title":title,"description":description,"active":1}))
}

pub fn dashboard(db_path: &Path) -> Result<Value, String> {
    let connection = database::open(db_path)?;
    let mut goals_statement = connection.prepare("SELECT id,title,description,active,created_at FROM compass_goals WHERE active=1 ORDER BY created_at").map_err(|error| error.to_string())?;
    let goals = goals_statement.query_map([], |row| Ok(json!({"id":row.get::<_,String>(0)?,"title":row.get::<_,String>(1)?,"description":row.get::<_,String>(2)?,"active":row.get::<_,i64>(3)?,"created_at":row.get::<_,String>(4)?}))).map_err(|error| error.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|error| error.to_string())?;
    let mut events_statement = connection.prepare("SELECT entity_type,entity_id,goal_id,points,event_type,created_at FROM score_events ORDER BY created_at DESC").map_err(|error| error.to_string())?;
    let events = events_statement.query_map([], |row| Ok(json!({"entity_type":row.get::<_,String>(0)?,"entity_id":row.get::<_,String>(1)?,"goal_id":row.get::<_,Option<String>>(2)?,"points":row.get::<_,f64>(3)?,"event_type":row.get::<_,String>(4)?,"created_at":row.get::<_,String>(5)?}))).map_err(|error| error.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|error| error.to_string())?;
    Ok(json!({"goals":goals,"events":events,"periods":[]}))
}

pub fn delete(db_path: &Path, entity_type: &str, entity_id: &str) -> Result<Value, String> {
    let connection = database::open(db_path)?;
    connection
        .execute(
            "INSERT OR REPLACE INTO deleted_entities(entity_type,entity_id) VALUES (?,?)",
            params![entity_type, entity_id],
        )
        .map_err(|error| error.to_string())?;
    Ok(Value::Null)
}

pub fn restore(db_path: &Path, entity_type: &str, entity_id: &str) -> Result<Value, String> {
    let connection = database::open(db_path)?;
    connection
        .execute(
            "DELETE FROM deleted_entities WHERE entity_type=? AND entity_id=?",
            params![entity_type, entity_id],
        )
        .map_err(|error| error.to_string())?;
    Ok(Value::Null)
}

pub fn update_item(
    db_path: &Path,
    entity_type: &str,
    entity_id: &str,
    input: &Value,
) -> Result<Value, String> {
    let title = required_text(input, "title")?;
    let detail = input.get("detail").and_then(Value::as_str).unwrap_or("");
    let connection = database::open(db_path)?;
    let changed = match entity_type {
        "captures" => connection.execute(
            "UPDATE captures SET text=? WHERE id=?",
            params![title, entity_id],
        ),
        "problems" => connection.execute(
            "UPDATE problems SET statement=?,detail=? WHERE id=?",
            params![title, detail, entity_id],
        ),
        "features" => connection.execute(
            "UPDATE features SET title=?,outcome=? WHERE id=?",
            params![title, detail, entity_id],
        ),
        _ => return Err("Unsupported item type".into()),
    }
    .map_err(|error| error.to_string())?;
    if changed == 0 {
        return Err("Item not found".into());
    }
    Ok(Value::Null)
}

pub fn item(db_path: &Path, entity_type: &str, entity_id: &str) -> Result<Value, String> {
    let board = board(db_path)?;
    let key = match entity_type {
        "captures" => "captures",
        "problems" => "problems",
        "features" => "features",
        _ => return Err("Unsupported item type".into()),
    };
    board[key]
        .as_array()
        .and_then(|items| items.iter().find(|item| item["id"] == entity_id))
        .cloned()
        .ok_or_else(|| "Item not found".into())
}

pub fn set_category(db_path: &Path, input: &Value) -> Result<Value, String> {
    let entity_type = required_text(input, "entity_type")?;
    let entity_id = required_text(input, "entity_id")?;
    let category = required_text(input, "category")?;
    let connection = database::open(db_path)?;
    connection.execute("INSERT INTO workbench_category_overrides(entity_type,entity_id,category) VALUES (?,?,?) ON CONFLICT(entity_type,entity_id) DO UPDATE SET category=excluded.category", params![entity_type,entity_id,category]).map_err(|error| error.to_string())?;
    Ok(Value::Null)
}

pub fn set_importance(db_path: &Path, input: &Value) -> Result<Value, String> {
    let entity_type = required_text(input, "entity_type")?;
    let entity_id = required_text(input, "entity_id")?;
    let important = input
        .get("important")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let connection = database::open(db_path)?;
    connection.execute("INSERT INTO workbench_priority_overrides(entity_type,entity_id,manual_priority) VALUES (?,?,?) ON CONFLICT(entity_type,entity_id) DO UPDATE SET manual_priority=excluded.manual_priority", params![entity_type,entity_id,i64::from(important)]).map_err(|error| error.to_string())?;
    Ok(Value::Null)
}

pub fn refinement_context(
    db_path: &Path,
    entity_type: &str,
    entity_id: &str,
) -> Result<Value, String> {
    let current = item(db_path, entity_type, entity_id)?;
    Ok(json!({"entries":[],"refinement_draft":null,"current_detail":current}))
}

pub fn record_ai_run(
    db_path: &Path,
    entity_type: &str,
    entity_id: &str,
    input: &str,
    output: &str,
) -> Result<(), String> {
    let table = match entity_type {
        "captures" | "problems" | "features" => entity_type,
        _ => return Err("Unsupported item type".into()),
    };
    let connection = database::open(db_path)?;
    let exists: bool = connection
        .query_row(
            &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE id=?)"),
            [entity_id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if !exists {
        return Err("Item not found".into());
    }
    connection.execute("INSERT INTO ai_runs(id,entity_type,entity_id,kind,input_text,output_text) VALUES (?,?,?,'workflow_chat',?,?)", params![id(),entity_type,entity_id,input,output]).map_err(|error| error.to_string())?;
    Ok(())
}

pub fn follow_up_problem(db_path: &Path, feature_id: &str) -> Result<Value, String> {
    let connection = database::open(db_path)?;
    let (title, outcome): (String, String) = connection
        .query_row(
            "SELECT title,outcome FROM features WHERE id=?",
            [feature_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .ok_or("Solution not found")?;
    let problem_id = id();
    connection
        .execute(
            "INSERT INTO problems(id,statement,detail,state) VALUES (?,?,?,'draft')",
            params![problem_id, format!("Follow up: {title}"), outcome],
        )
        .map_err(|error| error.to_string())?;
    connection
        .execute(
            "INSERT INTO follow_up_links(problem_id,source_feature_id) VALUES (?,?)",
            params![problem_id, feature_id],
        )
        .map_err(|error| error.to_string())?;
    Ok(
        json!({"id":problem_id,"statement":format!("Follow up: {title}"),"detail":outcome,"state":"draft"}),
    )
}

pub fn complete_problem(
    db_path: &Path,
    vault: &Path,
    problem_id: &str,
    input: &Value,
) -> Result<Value, String> {
    let reason = input.get("reason").and_then(Value::as_str).unwrap_or("");
    let connection = database::open(db_path)?;
    let (statement, detail): (String, String) = connection
        .query_row(
            "SELECT statement,detail FROM problems WHERE id=?",
            [problem_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .ok_or("Problem not found")?;
    let mut query = connection.prepare("SELECT title,outcome,validation_criteria FROM features WHERE problem_id=? ORDER BY created_at").map_err(|error| error.to_string())?;
    let features = query
        .query_map([problem_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    if features.is_empty() {
        return Err("Problem has no Solutions to complete".into());
    }
    let slug = statement
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-')
        .collect::<String>()
        .trim()
        .replace(' ', "-");
    let relative = format!(
        "90. Archive/{}-{}.md",
        chrono::Utc::now().format("%Y-%m-%d"),
        if slug.is_empty() {
            "completed-problem"
        } else {
            &slug
        }
    );
    let mut markdown = format!("# {statement}\n\n{detail}\n\n## Completion decision\n\n{reason}\n");
    for (title, outcome, criteria) in features {
        markdown.push_str(&format!(
            "\n## Solution: {title}\n\n{outcome}\n\n### Validation criteria\n\n{criteria}\n"
        ));
    }
    let target = vault.join(&relative);
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    std::fs::write(&target, &markdown).map_err(|error| error.to_string())?;
    let hash = format!("{:x}", Sha256::digest(markdown.as_bytes()));
    connection.execute("INSERT OR REPLACE INTO completion_playbooks(problem_id,path,source_hash) VALUES (?,?,?)", params![problem_id, relative, hash]).map_err(|error| error.to_string())?;
    connection
        .execute(
            "INSERT INTO problem_completion_decisions(id,problem_id,reason) VALUES (?,?,?)",
            params![id(), problem_id, reason],
        )
        .map_err(|error| error.to_string())?;
    connection
        .execute(
            "UPDATE problems SET state='completed' WHERE id=?",
            [problem_id],
        )
        .map_err(|error| error.to_string())?;
    Ok(json!({"path":relative,"problem_id":problem_id,"source_hash":hash}))
}

pub fn lineage(db_path: &Path, feature_id: &str) -> Result<Value, String> {
    let connection = database::open(db_path)?;
    let (problem_id, title): (String, String) = connection
        .query_row(
            "SELECT problem_id,title FROM features WHERE id=?",
            [feature_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .ok_or("Solution not found")?;
    let (capture_id, problem): (Option<String>, String) = connection
        .query_row(
            "SELECT capture_id,statement FROM problems WHERE id=?",
            [&problem_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|error| error.to_string())?;
    let capture = capture_id
        .as_ref()
        .and_then(|capture_id| {
            connection
                .query_row(
                    "SELECT text FROM captures WHERE id=?",
                    [capture_id],
                    |row| row.get::<_, String>(0),
                )
                .ok()
        })
        .unwrap_or_default();
    let completed: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM completion_playbooks WHERE problem_id=?)",
            [&problem_id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    let mut stages = vec![
        json!({"kind":"capture","id":capture_id,"title":capture}),
        json!({"kind":"problem","id":problem_id,"title":problem}),
        json!({"kind":"solution","id":feature_id,"title":title}),
    ];
    if completed {
        stages.push(json!({"kind":"complete","id":problem_id,"title":"Completed"}));
    }
    Ok(json!({"lineage":{"stages":stages},"claims":{},"evidence":{}}))
}

pub fn handoff(db_path: &Path, feature_id: &str) -> Result<Value, String> {
    let connection = database::open(db_path)?;
    let record: (String, String, String, String) = connection
        .query_row(
            "SELECT title,outcome,non_goals,validation_criteria FROM features WHERE id=?",
            [feature_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .ok_or("Solution not found")?;
    Ok(Value::String(format!(
        "# {}\n\n## Intended outcome\n{}\n\n## Non-goals\n{}\n\n## Done criteria\n{}\n",
        record.0, record.1, record.2, record.3
    )))
}

pub fn transitions(entity_type: Option<&str>) -> Value {
    let all = vec![
        json!({"id":"capture_to_problem","label":"Create Problem manually","submit_label":"Create","source_type":"captures","description":"Create the next Problem directly when AI assistance is unavailable.","fields":[{"name":"statement","label":"Problem statement","type":"text","required":true},{"name":"detail","label":"Context","type":"textarea","required":false}]}),
        json!({"id":"problem_to_solution","label":"Create Solution manually","submit_label":"Create","source_type":"problems","description":"Create a Solution directly from this approved Problem without using AI.","fields":[{"name":"title","label":"Solution name","type":"text","required":true},{"name":"outcome","label":"Intended outcome","type":"textarea","required":true},{"name":"non_goals","label":"Non-goals","type":"textarea","required":false},{"name":"validation_criteria","label":"Validation criteria","type":"textarea","required":true}]}),
        json!({"id":"solution_to_approved","label":"Start manually","submit_label":"Start work","source_type":"features","description":"Move this Solution to In progress without an AI conflict review.","fields":[{"name":"approval_path","label":"Conflict check","type":"select","required":true,"options":[{"value":"checked","label":"Already checked"},{"value":"skip","label":"Skip with a reason"}]},{"name":"citation","label":"Review basis","type":"textarea","required_when":{"approval_path":"checked"}},{"name":"skip_reason","label":"Skip reason","type":"textarea","required_when":{"approval_path":"skip"}}]}),
        json!({"id":"solution_to_completed","label":"Complete manually","submit_label":"Complete","source_type":"features","description":"Complete and archive this work directly without an AI completion review.","fields":[{"name":"evidence","label":"Completion evidence","type":"textarea","required":true},{"name":"completion_path","label":"Knowledge record","type":"select","required":true,"options":[{"value":"report","label":"Add completion note"},{"value":"no_update","label":"Skip note"}]},{"name":"report","label":"Completion note","type":"textarea","required_when":{"completion_path":"report"}},{"name":"reason","label":"Decision note","type":"textarea","required":false}]}),
    ];
    Value::Array(
        all.into_iter()
            .filter(|item| entity_type.is_none() || item["source_type"] == entity_type.unwrap())
            .collect(),
    )
}

pub fn apply_transition(
    db_path: &Path,
    vault: &Path,
    entity_type: &str,
    entity_id: &str,
    input: &Value,
) -> Result<Value, String> {
    let transition = required_text(input, "transition_id")?;
    let fields = input.get("fields").unwrap_or(&Value::Null);
    match (transition, entity_type) {
        ("capture_to_problem", "captures") => promote_capture(db_path, entity_id, fields),
        ("problem_to_solution", "problems") => create_feature(db_path, entity_id, fields),
        ("solution_to_approved", "features") => {
            let approval_path = required_text(fields, "approval_path")?;
            let citation = if approval_path == "skip" {
                required_text(fields, "skip_reason")?
            } else {
                required_text(fields, "citation")?
            };
            set_conflict(
                db_path,
                entity_id,
                &json!({"state":"clear","citation":citation}),
            )?;
            approve_feature(db_path, entity_id)?;
            Ok(json!({"approved":true}))
        }
        ("solution_to_completed", "features") => {
            required_text(fields, "evidence")?;
            let connection = database::open(db_path)?;
            let problem_id: String = connection
                .query_row(
                    "SELECT problem_id FROM features WHERE id=?",
                    [entity_id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|error| error.to_string())?
                .ok_or("Solution not found")?;
            complete_problem(db_path, vault, &problem_id, fields)
        }
        _ => Err("Unsupported workflow transition".into()),
    }
}
