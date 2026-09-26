use base64::{engine::general_purpose::STANDARD, Engine};
use rusqlite::{params, OptionalExtension, Transaction};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;
use uuid::Uuid;

use crate::workflow_foundation::{
    prompt_definition, ApplicationDisposition, PromptId, SourceAttribution, VersionOwner,
    VersionProvenance,
};

fn id() -> String {
    Uuid::new_v4().to_string()
}

fn image_parts(image: &Value) -> Result<(&str, &str, &str), String> {
    Ok((
        image["name"]
            .as_str()
            .ok_or("Capture image name is required")?,
        image["mediaType"]
            .as_str()
            .ok_or("Capture image media type is required")?,
        image["data"]
            .as_str()
            .ok_or("Capture image data is required")?,
    ))
}

pub(crate) fn source_hash(text: &str, images: &[Value]) -> Result<(String, Vec<String>), String> {
    let mut source = Sha256::new();
    source.update((text.len() as u64).to_be_bytes());
    source.update(text.as_bytes());
    let mut image_hashes = Vec::with_capacity(images.len());
    for image in images {
        let (name, media_type, data) = image_parts(image)?;
        let bytes = STANDARD
            .decode(data)
            .map_err(|_| "Capture image data is not valid base64")?;
        let content_hash = format!("sha256:{:x}", Sha256::digest(&bytes));
        for value in [name, media_type, content_hash.as_str()] {
            source.update((value.len() as u64).to_be_bytes());
            source.update(value.as_bytes());
        }
        image_hashes.push(content_hash);
    }
    Ok((format!("sha256:{:x}", source.finalize()), image_hashes))
}

pub(crate) fn initialize_tx(
    tx: &Transaction<'_>,
    capture_id: &str,
    text: &str,
    images: &[Value],
    locale: &str,
    at: &str,
) -> Result<Value, String> {
    let (source_hash, image_hashes) = source_hash(text, images)?;
    let logical_operation_id = format!("capture-distillation:{capture_id}");
    let source_revision = format!("{capture_id}:source:1:context:0:current:1:{source_hash}");
    tx.execute(
        "INSERT INTO capture_source_heads(
           capture_id,source_revision,current_revision,context_revision,source_hash,
           initial_distillation_eligible,logical_operation_id,last_user_activity_at)
         VALUES(?,1,1,0,?,1,?,?)",
        params![capture_id, source_hash, logical_operation_id, at],
    )
    .map_err(|error| error.to_string())?;
    tx.execute(
        "INSERT INTO capture_source_snapshots(
           capture_id,source_revision,authored_text,source_hash,created_at)
         VALUES(?,1,?,?,?)",
        params![capture_id, text, source_hash, at],
    )
    .map_err(|error| error.to_string())?;
    let mut image_refs = Vec::with_capacity(images.len());
    for (index, (image, content_hash)) in images.iter().zip(image_hashes.iter()).enumerate() {
        let (name, media_type, data) = image_parts(image)?;
        tx.execute(
            "INSERT INTO capture_source_images(
               capture_id,source_revision,image_index,name,media_type,content_hash,data)
             VALUES(?,1,?,?,?,?,?)",
            params![
                capture_id,
                index as i64,
                name,
                media_type,
                content_hash,
                data
            ],
        )
        .map_err(|error| error.to_string())?;
        image_refs.push(
            json!({"index":index,"name":name,"mediaType":media_type,"contentHash":content_hash}),
        );
    }
    let prompt = prompt_definition(PromptId::CaptureDistillation);
    let job_id = id();
    let safe_locale = if locale.starts_with("ko") { "ko" } else { "en" };
    let input = json!({
        "schemaVersion":1,
        "operationId":logical_operation_id,
        "logicalOperationId":logical_operation_id,
        "taskKind":"capture_distillation",
        "entityType":"captures",
        "entityId":capture_id,
        "captureId":capture_id,
        "sourceRevision":source_revision,
        "sourceNumber":1,
        "currentRevision":1,
        "contextRevision":0,
        "sourceHash":source_hash,
        "locale":safe_locale,
        "text":text,
        "images":image_refs,
    });
    tx.execute(
        "INSERT INTO ai_jobs_v2(
           id,task_kind,entity_type,entity_id,status,input_json,idempotency_key,
           result_interface,prompt_id,prompt_version,source_revision,
           execution_outcome,application_disposition,created_at,available_at)
         VALUES(?,'capture_distillation','captures',?,'queued',?,?,
           'capture_distillation',?,?,?,'pending','pending',?,?)",
        params![
            job_id,
            capture_id,
            input.to_string(),
            logical_operation_id,
            prompt.id.as_str(),
            prompt.version,
            source_revision,
            at,
            at,
        ],
    )
    .map_err(|error| error.to_string())?;
    Ok(json!({
        "sourceRevision":source_revision,
        "currentRevision":1,
        "source":{"text":text,"images":image_refs},
        "display":{"title":if text.is_empty() { if safe_locale == "ko" { "이미지 캡처" } else { "Image capture" } } else { text },"content":text,"revision":0,"placeholder":true},
        "distillation":{"jobId":job_id,"logicalOperationId":logical_operation_id,"status":"queued","executionOutcome":"pending","applicationDisposition":"pending","retryAllowed":false,"attempt":0}
    }))
}

pub(crate) fn advance_source_tx(
    tx: &Transaction<'_>,
    capture_id: &str,
    text: &str,
) -> Result<(), String> {
    let head = tx
        .query_row(
            "SELECT source_revision,current_revision FROM capture_source_heads WHERE capture_id=?",
            [capture_id],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let Some((source_revision, current_revision)) = head else {
        return Ok(());
    };
    let mut statement = tx
        .prepare("SELECT name,media_type,data FROM input_images WHERE capture_id=? ORDER BY rowid")
        .map_err(|error| error.to_string())?;
    let images = statement.query_map([capture_id], |row| Ok(json!({
        "name":row.get::<_,String>(0)?,"mediaType":row.get::<_,String>(1)?,"data":row.get::<_,String>(2)?
    }))).map_err(|error|error.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|error|error.to_string())?;
    drop(statement);
    let (source_hash, image_hashes) = source_hash(text, &images)?;
    let next_source = source_revision + 1;
    tx.execute(
        "INSERT INTO capture_source_snapshots(capture_id,source_revision,authored_text,source_hash)
         VALUES(?,?,?,?)",
        params![capture_id, next_source, text, source_hash],
    )
    .map_err(|error| error.to_string())?;
    for (index, (image, content_hash)) in images.iter().zip(image_hashes.iter()).enumerate() {
        let (name, media_type, data) = image_parts(image)?;
        tx.execute(
            "INSERT INTO capture_source_images(capture_id,source_revision,image_index,name,media_type,content_hash,data)
             VALUES(?,?,?,?,?,?,?)",
            params![capture_id,next_source,index as i64,name,media_type,content_hash,data],
        ).map_err(|error|error.to_string())?;
    }
    tx.execute(
        "UPDATE capture_source_heads SET source_revision=?,current_revision=?,source_hash=?,last_user_activity_at=CURRENT_TIMESTAMP WHERE capture_id=?",
        params![next_source,current_revision+1,source_hash,capture_id],
    ).map_err(|error|error.to_string())?;
    Ok(())
}

fn safe_error(code: &str, locale: &str) -> Value {
    if code.is_empty() {
        return Value::Null;
    }
    let message = match (locale, code) {
        ("ko", "invalid_result") => "정리 결과를 안전하게 적용할 수 없습니다.",
        ("ko", "retry_exhausted") => "정리를 완료하지 못했습니다. 다시 시도할 수 있습니다.",
        ("ko", _) => "정리를 완료하지 못했습니다.",
        (_, "invalid_result") => "The organized result could not be applied safely.",
        (_, "retry_exhausted") => "Organization did not finish. You can retry it.",
        _ => "Organization did not finish.",
    };
    json!({"code":code,"message":message})
}

pub(crate) fn projection(
    connection: &rusqlite::Connection,
    capture_id: &str,
    locale: &str,
) -> Result<Option<Value>, String> {
    let locale = if locale.starts_with("ko") { "ko" } else { "en" };
    let head = connection
        .query_row(
            "SELECT h.source_revision,h.current_revision,h.context_revision,h.source_hash,
                    h.initial_distillation_eligible,h.logical_operation_id,s.authored_text
             FROM capture_source_heads h JOIN capture_source_snapshots s
               ON s.capture_id=h.capture_id AND s.source_revision=h.source_revision
             WHERE h.capture_id=?",
            [capture_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, bool>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                ))
            },
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let Some((
        source_number,
        current_revision,
        context_revision,
        source_hash,
        eligible,
        logical_operation_id,
        text,
    )) = head
    else {
        return Ok(None);
    };
    let opaque_revision = format!(
        "{capture_id}:source:{source_number}:context:{context_revision}:current:{current_revision}:{source_hash}"
    );
    let mut image_statement = connection
        .prepare(
            "SELECT image_index,name,media_type,content_hash FROM capture_source_images
         WHERE capture_id=? AND source_revision=? ORDER BY image_index",
        )
        .map_err(|error| error.to_string())?;
    let images = image_statement
        .query_map(params![capture_id, source_number], |row| {
            Ok(json!({
                "index":row.get::<_,i64>(0)?,"name":row.get::<_,String>(1)?,
                "mediaType":row.get::<_,String>(2)?,"contentHash":row.get::<_,String>(3)?
            }))
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    drop(image_statement);
    let result = connection
        .query_row(
            "SELECT result_revision,title,content,context,explicit_requests_json,locale,job_id
         FROM capture_distillations WHERE capture_id=?",
            [capture_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                ))
            },
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let display = result.as_ref().map(|value| json!({
        "title":value.1,"content":value.2,"context":value.3,
        "explicitRequests":serde_json::from_str::<Value>(&value.4).unwrap_or_else(|_|json!([])),
        "locale":value.5,"revision":value.0,"placeholder":false,"jobId":value.6,
    })).unwrap_or_else(|| json!({
        "title":if text.trim().is_empty() { if locale == "ko" { "이미지 캡처" } else { "Image capture" } } else { text.as_str() },
        "content":text,"context":"","explicitRequests":[],"locale":locale,
        "revision":0,"placeholder":true,
    }));
    let job = connection.query_row(
        "SELECT id,status,execution_outcome,application_disposition,error_code,attempt,prompt_id,prompt_version,source_revision
         FROM ai_jobs_v2 WHERE task_kind='capture_distillation' AND entity_id=?
         ORDER BY created_at DESC,rowid DESC LIMIT 1",
        [capture_id],
        |row| Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,String>(2)?,row.get::<_,String>(3)?,row.get::<_,String>(4)?,row.get::<_,i64>(5)?,row.get::<_,String>(6)?,row.get::<_,i64>(7)?,row.get::<_,String>(8)?)),
    ).optional().map_err(|error| error.to_string())?;
    let distillation = job.map(|value| json!({
        "jobId":value.0,"logicalOperationId":logical_operation_id,"status":value.1,
        "executionOutcome":value.2,"applicationDisposition":value.3,
        "safeError":safe_error(&value.4,locale),
        "retryAllowed":matches!(value.1.as_str(),"failed"|"retryable") && value.8 == opaque_revision,
        "attempt":value.5,"prompt":{"id":value.6,"version":value.7},"sourceRevision":value.8,
    }));
    let proposal = connection.query_row(
        "SELECT id,title,content,context,explicit_requests_json,state,job_id
         FROM capture_distillation_proposals WHERE capture_id=? ORDER BY created_at DESC,rowid DESC LIMIT 1",
        [capture_id], |row| Ok(json!({
            "id":row.get::<_,String>(0)?,"title":row.get::<_,String>(1)?,"content":row.get::<_,String>(2)?,
            "context":row.get::<_,String>(3)?,"explicitRequests":serde_json::from_str::<Value>(&row.get::<_,String>(4)?).unwrap_or_else(|_|json!([])),
            "state":row.get::<_,String>(5)?,"jobId":row.get::<_,String>(6)?,
        })),
    ).optional().map_err(|error| error.to_string())?;
    Ok(Some(json!({
        "captureId":capture_id,"sourceRevision":opaque_revision,"sourceNumber":source_number,
        "currentRevision":current_revision,"contextRevision":context_revision,"eligible":eligible,
        "source":{"text":text,"images":images},"display":display,
        "distillation":distillation,"proposal":proposal,
    })))
}

pub(crate) fn workbench_projections(
    connection: &rusqlite::Connection,
    locale: &str,
) -> Result<HashMap<String, Value>, String> {
    let locale = if locale.starts_with("ko") { "ko" } else { "en" };
    let mut statement = connection
        .prepare(
            "WITH latest_jobs AS (
               SELECT *,row_number() OVER (
                 PARTITION BY entity_id ORDER BY created_at DESC,rowid DESC) AS latest_rank
               FROM ai_jobs_v2 WHERE task_kind='capture_distillation'
             ), latest_proposals AS (
               SELECT *,row_number() OVER (
                 PARTITION BY capture_id ORDER BY created_at DESC,rowid DESC) AS latest_rank
               FROM capture_distillation_proposals
             )
             SELECT h.capture_id,h.source_revision,h.current_revision,h.context_revision,
                h.source_hash,h.initial_distillation_eligible,h.logical_operation_id,
                s.authored_text,
                COALESCE((SELECT json_group_array(json_object(
                    'index',i.image_index,'name',i.name,'mediaType',i.media_type,
                    'contentHash',i.content_hash))
                  FROM capture_source_images i WHERE i.capture_id=h.capture_id
                    AND i.source_revision=h.source_revision),'[]'),
                d.result_revision,d.title,d.content,d.context,d.explicit_requests_json,
                d.locale,d.job_id,
                j.id,j.status,j.execution_outcome,j.application_disposition,j.error_code,
                j.attempt,j.prompt_id,j.prompt_version,j.source_revision,
                p.id,p.title,p.content,p.context,p.explicit_requests_json,p.state,p.job_id
         FROM capture_source_heads h
         JOIN capture_source_snapshots s ON s.capture_id=h.capture_id
           AND s.source_revision=h.source_revision
         LEFT JOIN capture_distillations d ON d.capture_id=h.capture_id
         LEFT JOIN latest_jobs j ON j.entity_id=h.capture_id AND j.latest_rank=1
         LEFT JOIN latest_proposals p ON p.capture_id=h.capture_id AND p.latest_rank=1",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement.query_map([], |row| {
        let capture_id = row.get::<_,String>(0)?;
        let source_number = row.get::<_,i64>(1)?;
        let current_revision = row.get::<_,i64>(2)?;
        let context_revision = row.get::<_,i64>(3)?;
        let source_hash = row.get::<_,String>(4)?;
        let logical_operation_id = row.get::<_,String>(6)?;
        let text = row.get::<_,String>(7)?;
        let images = serde_json::from_str::<Value>(&row.get::<_,String>(8)?)
            .unwrap_or_else(|_| json!([]));
        let opaque_revision = format!(
            "{capture_id}:source:{source_number}:context:{context_revision}:current:{current_revision}:{source_hash}"
        );
        let result_revision = row.get::<_,Option<i64>>(9)?;
        let display = if let Some(revision) = result_revision {
            json!({
                "title":row.get::<_,String>(10)?,"content":row.get::<_,String>(11)?,
                "context":row.get::<_,String>(12)?,
                "explicitRequests":serde_json::from_str::<Value>(&row.get::<_,String>(13)?).unwrap_or_else(|_|json!([])),
                "locale":row.get::<_,String>(14)?,"revision":revision,"placeholder":false,
                "jobId":row.get::<_,String>(15)?,
            })
        } else {
            json!({
                "title":if text.trim().is_empty() { if locale == "ko" { "이미지 캡처" } else { "Image capture" } } else { text.as_str() },
                "content":text,"context":"","explicitRequests":[],"locale":locale,
                "revision":0,"placeholder":true,
            })
        };
        let job_id = row.get::<_,Option<String>>(16)?;
        let distillation = if let Some(job_id) = job_id {
            let status = row.get::<_,String>(17)?;
            let error_code = row.get::<_,String>(20)?;
            let job_source_revision = row.get::<_,String>(24)?;
            Some(json!({
                "jobId":job_id,"logicalOperationId":logical_operation_id,"status":status,
                "executionOutcome":row.get::<_,String>(18)?,
                "applicationDisposition":row.get::<_,String>(19)?,
                "safeError":safe_error(&error_code,locale),
                "retryAllowed":matches!(status.as_str(),"failed"|"retryable") && job_source_revision == opaque_revision,
                "attempt":row.get::<_,i64>(21)?,
                "prompt":{"id":row.get::<_,String>(22)?,"version":row.get::<_,i64>(23)?},
                "sourceRevision":job_source_revision,
            }))
        } else { None };
        let proposal_id = row.get::<_,Option<String>>(25)?;
        let proposal = if let Some(proposal_id) = proposal_id {
            Some(json!({
                "id":proposal_id,"title":row.get::<_,String>(26)?,
                "content":row.get::<_,String>(27)?,"context":row.get::<_,String>(28)?,
                "explicitRequests":serde_json::from_str::<Value>(&row.get::<_,String>(29)?).unwrap_or_else(|_|json!([])),
                "state":row.get::<_,String>(30)?,"jobId":row.get::<_,String>(31)?,
            }))
        } else { None };
        Ok((capture_id.clone(),json!({
            "captureId":capture_id,"sourceRevision":opaque_revision,"sourceNumber":source_number,
            "currentRevision":current_revision,"contextRevision":context_revision,
            "eligible":row.get::<_,bool>(5)?,"source":{"text":text,"images":images},
            "display":display,"distillation":distillation,"proposal":proposal,
        })))
    }).map_err(|error| error.to_string())?;
    rows.collect::<Result<HashMap<_, _>, _>>()
        .map_err(|error| error.to_string())
}

fn bounded_text(value: &Value, field: &str, limit: usize) -> Result<String, String> {
    let text = value
        .as_str()
        .ok_or_else(|| format!("invalid_result: {field} must be a string"))?
        .trim();
    if text.chars().count() > limit {
        return Err(format!("invalid_result: {field} is too long"));
    }
    Ok(text.to_owned())
}

pub(crate) fn validate_result(
    result: &Value,
    text_length: usize,
    image_count: usize,
) -> Result<Value, String> {
    let object = result
        .as_object()
        .ok_or("invalid_result: Capture distillation must be an object")?;
    let allowed = [
        "schemaVersion",
        "title",
        "content",
        "context",
        "explicitRequests",
        "grounding",
    ];
    if object.keys().any(|key| !allowed.contains(&key.as_str())) || result["schemaVersion"] != 1 {
        return Err(
            "invalid_result: Capture distillation contains unsupported fields or schema".into(),
        );
    }
    let title = bounded_text(&result["title"], "title", 120)?;
    let content = bounded_text(&result["content"], "content", 4_000)?;
    let context = bounded_text(&result["context"], "context", 2_000)?;
    let requests = result["explicitRequests"]
        .as_array()
        .ok_or("invalid_result: explicitRequests must be an array")?;
    if requests.len() > 8 {
        return Err("invalid_result: too many explicitRequests".into());
    }
    let requests = requests
        .iter()
        .map(|value| bounded_text(value, "explicitRequests", 500))
        .collect::<Result<Vec<_>, _>>()?;
    if title.is_empty() && content.is_empty() && context.is_empty() && requests.is_empty() {
        return Err("invalid_result: Capture distillation is empty".into());
    }
    let grounding = result["grounding"]
        .as_array()
        .ok_or("invalid_result: grounding must be an array")?;
    let mut grounded = std::collections::HashSet::new();
    for item in grounding {
        let field = item["field"]
            .as_str()
            .ok_or("invalid_result: grounding field is required")?;
        let refs = item["sourceRefs"]
            .as_array()
            .ok_or("invalid_result: sourceRefs must be an array")?;
        if refs.is_empty() {
            return Err("invalid_result: sourceRefs cannot be empty".into());
        }
        for source_ref in refs {
            let source_ref = source_ref
                .as_str()
                .ok_or("invalid_result: sourceRef must be a string")?;
            let valid = if let Some(range) = source_ref.strip_prefix("text:") {
                range
                    .split_once('-')
                    .and_then(|(start, end)| {
                        Some((start.parse::<usize>().ok()?, end.parse::<usize>().ok()?))
                    })
                    .is_some_and(|(start, end)| start < end && end <= text_length)
            } else if let Some(index) = source_ref.strip_prefix("image:") {
                index
                    .parse::<usize>()
                    .ok()
                    .is_some_and(|index| index < image_count)
            } else {
                false
            };
            if !valid {
                return Err(
                    "invalid_result: grounding sourceRef is not in the Capture source".into(),
                );
            }
        }
        grounded.insert(field.to_owned());
    }
    for (field, nonempty) in [
        ("title", !title.is_empty()),
        ("content", !content.is_empty()),
        ("context", !context.is_empty()),
    ] {
        if nonempty && !grounded.contains(field) {
            return Err(format!("invalid_result: {field} is not grounded"));
        }
    }
    for index in 0..requests.len() {
        if !grounded.contains(&format!("explicitRequests.{index}")) {
            return Err("invalid_result: explicit request is not grounded".into());
        }
    }
    Ok(
        json!({"schemaVersion":1,"title":title,"content":content,"context":context,"explicitRequests":requests,"grounding":grounding}),
    )
}

pub(crate) fn provider_content(
    connection: &rusqlite::Connection,
    capture_id: &str,
    source_number: i64,
    prompt: String,
) -> Result<Value, String> {
    let mut content = vec![json!({"type":"text","text":prompt})];
    let mut statement = connection.prepare(
        "SELECT media_type,data FROM capture_source_images WHERE capture_id=? AND source_revision=? ORDER BY image_index LIMIT 8"
    ).map_err(|error| error.to_string())?;
    let images = statement
        .query_map(params![capture_id, source_number], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    for (media_type, data) in images {
        content.push(json!({"type":"image_url","image_url":{"url":format!("data:{media_type};base64,{data}")}}));
    }
    Ok(Value::Array(content))
}

pub(crate) fn publish_result(
    db_path: &Path,
    job_id: &str,
    input: &Value,
    result: &Value,
) -> Result<Value, String> {
    let capture_id = input["captureId"].as_str().ok_or("captureId is required")?;
    let source_number = input["sourceNumber"]
        .as_i64()
        .ok_or("sourceNumber is required")?;
    let expected_current = input["currentRevision"]
        .as_i64()
        .ok_or("currentRevision is required")?;
    let expected_hash = input["sourceHash"]
        .as_str()
        .ok_or("sourceHash is required")?;
    let text_length = input["text"].as_str().unwrap_or("").chars().count();
    let image_count = input["images"].as_array().map_or(0, Vec::len);
    let result = validate_result(result, text_length, image_count)?;
    let mut connection = crate::native::database::open(db_path)?;
    let tx = connection
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(|error| error.to_string())?;
    let current = tx.query_row(
        "SELECT source_revision,current_revision,source_hash FROM capture_source_heads h
         WHERE capture_id=? AND NOT EXISTS(SELECT 1 FROM deleted_entities d WHERE d.entity_type='captures' AND d.entity_id=h.capture_id)",
        [capture_id], |row| Ok((row.get::<_,i64>(0)?,row.get::<_,i64>(1)?,row.get::<_,String>(2)?)),
    ).optional().map_err(|error| error.to_string())?;
    let exact = current.as_ref().is_some_and(|value| {
        value.0 == source_number && value.1 == expected_current && value.2 == expected_hash
    });
    let disposition = if exact {
        ApplicationDisposition::Applied
    } else {
        ApplicationDisposition::Superseded
    };
    let explicit = result["explicitRequests"].to_string();
    let source_references = result["grounding"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|grounding| {
            let claim_id = grounding["field"].as_str().unwrap_or_default().to_owned();
            grounding["sourceRefs"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(move |source_ref| {
                    Some(SourceAttribution {
                        document_id: capture_id.to_owned(),
                        document_version: source_number.to_string(),
                        section: Some(source_ref.as_str()?.to_owned()),
                        excerpt: None,
                        claim_id: Some(claim_id.clone()),
                    })
                })
        })
        .collect::<Vec<_>>();
    if exact {
        let next: i64 = tx.query_row("SELECT COALESCE(MAX(result_revision),0)+1 FROM capture_distillations WHERE capture_id=?",[capture_id],|row|row.get(0)).map_err(|error|error.to_string())?;
        tx.execute("INSERT INTO capture_distillations(capture_id,result_revision,source_revision,bound_current_revision,source_hash,title,content,context,explicit_requests_json,locale,job_id) VALUES(?,?,?,?,?,?,?,?,?,?,?) ON CONFLICT(capture_id) DO UPDATE SET result_revision=excluded.result_revision,source_revision=excluded.source_revision,bound_current_revision=excluded.bound_current_revision,source_hash=excluded.source_hash,title=excluded.title,content=excluded.content,context=excluded.context,explicit_requests_json=excluded.explicit_requests_json,locale=excluded.locale,job_id=excluded.job_id,applied_at=CURRENT_TIMESTAMP",params![capture_id,next,source_number,expected_current,expected_hash,result["title"].as_str().unwrap_or(""),result["content"].as_str().unwrap_or(""),result["context"].as_str().unwrap_or(""),explicit,input["locale"].as_str().unwrap_or("en"),job_id]).map_err(|error|error.to_string())?;
        crate::native::workflow_foundation::record_version_provenance(
            &tx,
            &VersionProvenance {
                owner: VersionOwner {
                    entity_type: "capture_distillation".into(),
                    entity_id: capture_id.into(),
                    version: next.to_string(),
                },
                source: Some(VersionOwner {
                    entity_type: "capture_source".into(),
                    entity_id: capture_id.into(),
                    version: source_number.to_string(),
                }),
                prompt_id: Some(PromptId::CaptureDistillation),
                prompt_version: Some(prompt_definition(PromptId::CaptureDistillation).version),
                operation_id: Some(input["logicalOperationId"].as_str().unwrap_or("").into()),
                restored_from_version: None,
                source_references: source_references.clone(),
            },
        )?;
    } else if current.is_some() {
        let proposal_id = id();
        tx.execute("INSERT INTO capture_distillation_proposals(id,capture_id,job_id,source_revision,bound_current_revision,source_hash,title,content,context,explicit_requests_json,locale) VALUES(?,?,?,?,?,?,?,?,?,?,?)",params![proposal_id,capture_id,job_id,source_number,expected_current,expected_hash,result["title"].as_str().unwrap_or(""),result["content"].as_str().unwrap_or(""),result["context"].as_str().unwrap_or(""),explicit,input["locale"].as_str().unwrap_or("en")]).map_err(|error|error.to_string())?;
        crate::native::workflow_foundation::record_version_provenance(
            &tx,
            &VersionProvenance {
                owner: VersionOwner {
                    entity_type: "capture_distillation_proposal".into(),
                    entity_id: proposal_id,
                    version: "1".into(),
                },
                source: Some(VersionOwner {
                    entity_type: "capture_source".into(),
                    entity_id: capture_id.into(),
                    version: source_number.to_string(),
                }),
                prompt_id: Some(PromptId::CaptureDistillation),
                prompt_version: Some(prompt_definition(PromptId::CaptureDistillation).version),
                operation_id: Some(input["logicalOperationId"].as_str().unwrap_or("").into()),
                restored_from_version: None,
                source_references,
            },
        )?;
    }
    crate::native::workflow_foundation::record_job_application_disposition(
        &tx,
        job_id,
        input["sourceRevision"].as_str().unwrap_or(""),
        input["sourceRevision"].as_str().unwrap_or(""),
        disposition,
    )?;
    tx.execute("UPDATE ai_jobs_v2 SET status='completed',execution_outcome='succeeded',result_json=?,progress_completed=1,finished_at=CURRENT_TIMESTAMP WHERE id=? AND status='running'",params![result.to_string(),job_id]).map_err(|error|error.to_string())?;
    tx.commit().map_err(|error| error.to_string())?;
    Ok(
        json!({"captureId":capture_id,"applicationDisposition":disposition.as_str(),"result":result}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn saved_capture() -> (tempfile::TempDir, std::path::PathBuf, Value) {
        let root = tempfile::tempdir().unwrap();
        let db = root.path().join("state.sqlite3");
        crate::native::database::initialize(&db).unwrap();
        let mut connection = crate::native::database::open(&db).unwrap();
        let tx = connection.transaction().unwrap();
        tx.execute("INSERT INTO captures(id,text,source_mode,last_user_activity_at) VALUES('capture-1','raw thought','capture','2026-09-01')",[]).unwrap();
        let projection =
            initialize_tx(&tx, "capture-1", "raw thought", &[], "en", "2026-09-01").unwrap();
        tx.commit().unwrap();
        (root, db, projection)
    }

    fn grounded_result() -> Value {
        json!({
            "schemaVersion":1,"title":"Readable thought","content":"raw thought",
            "context":"","explicitRequests":[],
            "grounding":[
                {"field":"title","sourceRefs":["text:0-3"]},
                {"field":"content","sourceRefs":["text:0-11"]}
            ]
        })
    }

    #[test]
    fn registered_runtime_validator_rejects_unbounded_or_invented_output() {
        let valid = grounded_result();
        crate::workflow_foundation::validate_prompt_output(PromptId::CaptureDistillation, &valid)
            .unwrap();
        validate_result(&valid, 11, 0).unwrap();

        let mut unexpected = valid.clone();
        unexpected["workflowState"] = json!("task");
        assert!(validate_result(&unexpected, 11, 0)
            .unwrap_err()
            .contains("unsupported"));
        let mut invented = valid.clone();
        invented["grounding"][0]["sourceRefs"] = json!(["image:0"]);
        assert!(validate_result(&invented, 11, 0)
            .unwrap_err()
            .contains("not in the Capture source"));
        let mut over_limit = valid;
        over_limit["title"] = json!("x".repeat(121));
        assert!(crate::workflow_foundation::validate_prompt_output(
            PromptId::CaptureDistillation,
            &over_limit
        )
        .is_err());
    }

    #[test]
    fn exact_head_publication_applies_without_changing_user_activity() {
        let (_root, db, created) = saved_capture();
        let connection = crate::native::database::open(&db).unwrap();
        let job_id = created["distillation"]["jobId"].as_str().unwrap();
        connection
            .execute(
                "UPDATE ai_jobs_v2 SET status='running' WHERE id=?",
                [job_id],
            )
            .unwrap();
        let input: String = connection
            .query_row(
                "SELECT input_json FROM ai_jobs_v2 WHERE id=?",
                [job_id],
                |row| row.get(0),
            )
            .unwrap();
        let input: Value = serde_json::from_str(&input).unwrap();

        let published = publish_result(&db, job_id, &input, &grounded_result()).unwrap();

        assert_eq!(published["applicationDisposition"], "applied");
        assert_eq!(
            connection
                .query_row(
                    "SELECT title FROM capture_distillations WHERE capture_id='capture-1'",
                    [],
                    |row| row.get::<_, String>(0)
                )
                .unwrap(),
            "Readable thought"
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT last_user_activity_at FROM captures WHERE id='capture-1'",
                    [],
                    |row| row.get::<_, String>(0)
                )
                .unwrap(),
            "2026-09-01"
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT application_disposition FROM ai_jobs_v2 WHERE id=?",
                    [job_id],
                    |row| row.get::<_, String>(0)
                )
                .unwrap(),
            "applied"
        );
        let references: String = connection
            .query_row(
                "SELECT source_references_json FROM workflow_version_provenance WHERE owner_type='capture_distillation' AND owner_id='capture-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(references.contains("text:0-3"));
    }

    #[test]
    fn changed_head_keeps_useful_result_as_proposal_without_overwrite() {
        let (_root, db, created) = saved_capture();
        let connection = crate::native::database::open(&db).unwrap();
        let job_id = created["distillation"]["jobId"].as_str().unwrap();
        connection
            .execute(
                "UPDATE ai_jobs_v2 SET status='running' WHERE id=?",
                [job_id],
            )
            .unwrap();
        let input: String = connection
            .query_row(
                "SELECT input_json FROM ai_jobs_v2 WHERE id=?",
                [job_id],
                |row| row.get(0),
            )
            .unwrap();
        let input: Value = serde_json::from_str(&input).unwrap();
        connection
            .execute(
                "UPDATE capture_source_heads SET current_revision=2 WHERE capture_id='capture-1'",
                [],
            )
            .unwrap();

        let published = publish_result(&db, job_id, &input, &grounded_result()).unwrap();

        assert_eq!(published["applicationDisposition"], "superseded");
        assert_eq!(
            connection
                .query_row("SELECT count(*) FROM capture_distillations", [], |row| row
                    .get::<_, i64>(
                    0
                ))
                .unwrap(),
            0
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT title FROM capture_distillation_proposals WHERE job_id=?",
                    [job_id],
                    |row| row.get::<_, String>(0)
                )
                .unwrap(),
            "Readable thought"
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT application_disposition FROM ai_jobs_v2 WHERE id=?",
                    [job_id],
                    |row| row.get::<_, String>(0)
                )
                .unwrap(),
            "superseded"
        );
    }

    #[test]
    fn workbench_batch_projection_matches_the_single_capture_projection() {
        let (_root, db, created) = saved_capture();
        let connection = crate::native::database::open(&db).unwrap();
        let job_id = created["distillation"]["jobId"].as_str().unwrap();
        connection
            .execute(
                "UPDATE ai_jobs_v2 SET status='failed',execution_outcome='failed',error_code='provider_unavailable' WHERE id=?",
                [job_id],
            )
            .unwrap();

        let single = projection(&connection, "capture-1", "ko").unwrap().unwrap();
        let batch = workbench_projections(&connection, "ko")
            .unwrap()
            .remove("capture-1")
            .unwrap();

        assert_eq!(batch, single);
    }
}
