//! Exact Task-scoped reference use, including explicitly adopted Capture work.
use rusqlite::{params, Connection};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

/// Recover the recorded statement that a source grounded, never an invented
/// explanation derived from a role label. The immutable preview owns this text.
pub(crate) fn claim_statement(fields: &Value, claim_id: &str) -> Option<String> {
    let (field, _) = claim_id.split_once(':')?;
    let value = fields.get(field)?;
    let text = if let Some(text) = value.as_str() {
        text.to_owned()
    } else {
        value
            .as_array()?
            .iter()
            .filter_map(|item| {
                item.as_str()
                    .or_else(|| item.get("text").and_then(Value::as_str))
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    (!text.trim().is_empty()).then_some(text)
}

#[derive(Default)]
struct Usage {
    reasons: BTreeSet<String>,
    scopes: BTreeSet<String>,
    counterevidence: bool,
    adopted: bool,
    excluded: bool,
    has_use: bool,
}

pub(crate) fn for_task(c: &Connection, task_id: &str) -> Result<Vec<Value>, String> {
    // Do not infer adoption from origin_capture_id: separate Tasks may retain
    // the same origin. Only a recorded successful decision binds Capture work.
    let mut query = c.prepare(
        "SELECT i.document_id,i.document_version,i.section,i.kind,i.reason,
                i.use_scope,i.preview_version,v.fields_json,r.role,r.metadata_json
         FROM reference_interactions i JOIN work_previews p ON p.id=i.preview_id
         LEFT JOIN work_preview_versions v ON v.preview_id=p.id AND v.version=i.preview_version
         LEFT JOIN work_preview_references r ON r.preview_id=p.id
           AND r.version=COALESCE(i.preview_version,p.current_version)
           AND r.document_id=i.document_id AND r.document_version=i.document_version AND r.section=i.section
         WHERE i.kind IN ('used','adopted','excluded') AND
           ((p.subject_type='task' AND p.subject_id=?1) OR
            (p.subject_type='capture' AND EXISTS (
              SELECT 1 FROM refinement_proposal_decisions d
              WHERE d.session_id=p.session_id AND d.decision IN ('accept','apply')
                AND json_extract(d.result_json,'$.id')=?1
                AND json_type(d.result_json,'$.taskRevision')='integer'
                AND NOT EXISTS (SELECT 1 FROM task_subtasks h WHERE h.child_task_id=?1))))
         ORDER BY i.rowid,r.rowid"
    ).map_err(|error| error.to_string())?;
    let records = query
        .query_map(params![task_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, Option<i64>>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, Option<String>>(8)?,
                row.get::<_, Option<String>>(9)?,
            ))
        })
        .map_err(|error| error.to_string())?;
    let mut uses: BTreeMap<(String, String, String), Usage> = BTreeMap::new();
    for record in records {
        let (
            document,
            revision,
            section,
            kind,
            reason,
            scope,
            preview_version,
            fields,
            role,
            metadata,
        ) = record.map_err(|error| error.to_string())?;
        let usage = uses.entry((document, revision, section)).or_default();
        if kind == "excluded" {
            usage.excluded = true;
            continue;
        }
        // Automatic preview generation or adoption cannot silently undo an
        // explicit exclusion. A later explicit Mark used can restore it.
        if kind == "used" && preview_version.is_none() && scope != "reply" {
            usage.excluded = false;
        }
        usage.has_use = true;
        usage.adopted |= kind == "adopted";
        let metadata: Value = metadata
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or(Value::Null);
        let recorded_role = metadata["claimUses"]
            .as_array()
            .and_then(|claims| claims.iter().find(|claim| claim["claimId"] == scope))
            .and_then(|claim| claim["role"].as_str())
            .or(role.as_deref())
            .unwrap_or("");
        usage.counterevidence |= recorded_role == "counterevidence"
            || (preview_version.is_some() && reason == "counterevidence");
        let fields: Value = fields
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or(Value::Null);
        let scopes: Vec<String> = if kind == "adopted" {
            serde_json::from_str(&scope).unwrap_or_default()
        } else {
            vec![scope.clone()]
        };
        let mut recovered = false;
        if preview_version.is_some() {
            for claim_id in scopes {
                if let Some(statement) = claim_statement(&fields, &claim_id) {
                    usage.reasons.insert(statement);
                    usage.scopes.insert(claim_id);
                    recovered = true;
                }
            }
        }
        // User-provided reasons and actual reply-finding summaries are already
        // recorded evidence. Legacy generated role-only strings are not.
        if !recovered && preview_version.is_none() && !reason.trim().is_empty() {
            usage.reasons.insert(reason);
            if !scope.is_empty() {
                usage.scopes.insert(scope);
            }
        }
    }
    uses.into_iter().filter(|(_,usage)| usage.has_use && !usage.excluded)
        .map(|((document,revision,section),usage)| {
            if usage.reasons.is_empty() {
                return Err(format!("reference_provenance_unresolved: {document} has no recorded use statement"));
            }
            Ok(json!({"documentId":document,"documentVersion":revision,"section":section,
                "disposition":if usage.counterevidence {"counterevidence"} else if usage.adopted {"adopted"} else {"used"},
                "reason":usage.reasons.into_iter().collect::<Vec<_>>().join("\n"),
                "claimIds":usage.scopes.into_iter().collect::<Vec<_>>()}))
        }).collect()
}

#[cfg(test)]
#[path = "reference_provenance_tests.rs"]
mod tests;
