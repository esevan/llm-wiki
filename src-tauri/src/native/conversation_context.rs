use crate::native::database;
use rusqlite::params;
use serde_json::{json, Value};
use std::path::Path;

pub struct ConversationRequest {
    pub model_task: &'static str,
    pub messages: Value,
}

pub fn build(
    db_path: &Path,
    entity_type: &str,
    entity_id: &str,
    mode: &str,
    locale: &str,
    message: &str,
) -> Result<ConversationRequest, String> {
    let connection = database::open(db_path)?;
    let (title, detail, state) = match entity_type {
        "captures" => connection.query_row(
            "SELECT text,'','inbox' FROM captures WHERE id=?",
            [entity_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        ),
        "problems" => connection.query_row(
            "SELECT statement,detail,state FROM problems WHERE id=?",
            [entity_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        ),
        "features" => connection.query_row(
            "SELECT title,outcome,state FROM features WHERE id=?",
            [entity_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        ),
        _ => return Err("Unsupported workflow item".into()),
    }
    .map_err(|_| "Item not found".to_string())?;
    if mode == "next" && entity_type == "features" {
        return Err("Solutions do not have a next workflow stage".into());
    }
    if mode == "completed" && entity_type != "features" {
        return Err("Completed chat requires a Solution".into());
    }
    let (system, model_task) = match (mode, entity_type) {
        ("completed", "features") => ("Explain this immutable completed Solution using its preserved record and evidence only. Clearly identify missing evidence and recommend a follow-up Problem for new work.", "completed_solution_chat"),
        ("next", "captures") => ("Help define the next Problem. Ask exactly one useful open-ended question and do not invent facts.", "problem_drafting"),
        ("next", "problems") => ("Help define the next Solution. Ask exactly one useful open-ended question and do not invent implementation details.", "solution_drafting"),
        (_, "captures") => ("Help clarify this Capture without advancing its workflow state. Ask one focused question at a time.", "capture_assistance"),
        (_, "problems") => ("Help refine this Problem using known evidence and boundaries. Ask one focused question at a time.", "problem_assistance"),
        (_, "features") => ("Help refine this Solution without changing its approved state. Preserve constraints and observable validation criteria.", "solution_assistance"),
        _ => unreachable!(),
    };
    if mode == "completed" && state != "completed" {
        let verified: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM completions WHERE feature_id=? AND state='verified')",
                [entity_id],
                |row| row.get(0),
            )
            .map_err(|error| error.to_string())?;
        if !verified {
            return Err("Completed Solution not found".into());
        }
    }
    let mut messages = vec![
        json!({"role":"system","content":system}),
        json!({"role":"system","content":if locale.to_ascii_lowercase().starts_with("ko") { "Respond in Korean unless the user explicitly requests another language." } else { "Respond in English unless the user explicitly requests another language." }}),
        json!({"role":"system","content":format!("Current {entity_type}: {title}\nKnown detail: {detail}\nState: {state}")}),
    ];
    let mut history = connection.prepare("SELECT input_text,output_text FROM ai_runs WHERE entity_type=? AND entity_id=? AND kind='workflow_chat' ORDER BY created_at DESC LIMIT 6").map_err(|error| error.to_string())?
        .query_map(params![entity_type,entity_id], |row| Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?))).map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>,_>>().map_err(|error| error.to_string())?;
    history.reverse();
    for (input, output) in history {
        messages.push(json!({"role":"user","content":input}));
        messages.push(json!({"role":"assistant","content":output}));
    }
    messages.push(json!({"role":"user","content":message}));
    Ok(ConversationRequest {
        model_task,
        messages: Value::Array(messages),
    })
}
