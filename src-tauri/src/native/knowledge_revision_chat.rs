//! Evidence-bound revision chat. A turn proposes; explicit apply appends a private
//! Knowledge revision. Publication remains exclusively owned by knowledge_archive.
use super::{database, knowledge_distillation as kd, task_assistance};
use crate::workflow_foundation::PromptId;
use rusqlite::{params, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Reply {
    pub message: String,
    pub missing_evidence: Vec<String>,
    pub candidate: Option<kd::GenerationResult>,
}
fn required<'a>(v: &'a Value, k: &str) -> Result<&'a str, String> {
    v[k].as_str()
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| format!("invalid_input: {k} is required"))
}
fn revision(v: &Value) -> Result<i64, String> {
    v["baseRevision"]
        .as_i64()
        .filter(|n| *n > 0)
        .ok_or("baseRevision is required".into())
}
fn hash(v: &Value) -> String {
    format!("sha256:{:x}", Sha256::digest(v.to_string().as_bytes()))
}
fn base(tx: &Transaction<'_>, input: &Value) -> Result<Value, String> {
    let task = required(input, "taskId")?;
    let n = revision(input)?;
    tx.query_row("SELECT v.result_json,v.content_hash,v.body_markdown,s.manifest_json FROM knowledge_draft_versions v LEFT JOIN knowledge_evidence_snapshots s ON s.id=v.generation_snapshot_id WHERE v.task_id=? AND v.revision=?",params![task,n],|r|Ok(json!({"result":serde_json::from_str::<Value>(&r.get::<_,String>(0)?).unwrap_or(Value::Null),"contentHash":r.get::<_,String>(1)?,"bodyMarkdown":r.get::<_,String>(2)?,"snapshot":r.get::<_,Option<String>>(3)?.and_then(|s|serde_json::from_str::<Value>(&s).ok())}))).optional().map_err(|e|e.to_string())?.ok_or("knowledge_revision_not_found".into())
}
fn head(tx: &Transaction<'_>, task: &str) -> Result<Option<i64>, String> {
    tx.query_row(
        "SELECT current_private_revision FROM knowledge_pointers WHERE task_id=?",
        [task],
        |r| r.get(0),
    )
    .optional()
    .map(|v| v.flatten())
    .map_err(|e| e.to_string())
}
fn leaf_text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Array(a) => a
            .iter()
            .map(leaf_text)
            .max_by_key(|s| s.chars().count())
            .unwrap_or_default(),
        Value::Object(o) => o
            .values()
            .map(leaf_text)
            .max_by_key(|s| s.chars().count())
            .unwrap_or_default(),
        _ => String::new(),
    }
}
fn short(s: &str) -> String {
    s.chars().take(240).collect()
}
fn suggestion(kind: &str, ko: bool) -> &'static str {
    match (kind,ko){
        ("material_omission" | "validation_material_omission",true)=>"조건·실패·미해결 내용을 복원하세요. 해결 증거가 없다면 미해결로 남겨 주세요.",
        ("material_omission" | "validation_material_omission",false)=>"Restore the missing condition, failure or unresolved matter; keep it unresolved without new evidence.",
        ("invalid_finality"|"invalid_verification",true)=>"완료 상태와 실적 증거를 구분하세요. 확정·검증 표현을 근거 수준에 맞추거나 해당 주장을 삭제하세요.",
        ("invalid_finality"|"invalid_verification",false)=>"Match the claim to the recorded evidence state; remove unsupported decided or verified wording.",
        ("applicability_overreach",true)=>"활용 범위를 근거 안으로 좁히고 해석은 출처와 연결해 '활용 해석'으로 구분하세요.",
        ("applicability_overreach",false)=>"Narrow reuse to supported scope and label source-bound interpretations separately.",
        (_,true)=>"인용문을 원문에 맞추거나 직접 인용 표시를 제거하세요. 사용자 경험은 진술로 남기고 AI 해석과 구분할 수 있습니다.",
        (_,false)=>"Correct the direct quote or remove its quotation attribution. Keep user experience as a user report and distinguish AI interpretation.",
    }
}
/// Diagnostics are a projection of stored findings plus deterministic validation,
/// not new model-approved evidence. Rejected quotes stay outside authenticated refs.
pub(crate) fn diagnostics(snapshot: &Value, result: &Value, issue: &str, ko: bool) -> Vec<Value> {
    if issue == "invalid_knowledge: idea disposition is invalid" {
        return result["ideas"].as_array().into_iter().flatten().enumerate()
            .filter(|(_, idea)| !kd::IDEA_DISPOSITIONS.contains(&idea["disposition"].as_str().unwrap_or("")))
            .map(|(i, idea)| json!({"kind":"invalid_idea_disposition","locator":format!("ideas[{i}].disposition"),"statement":idea["title"].as_str().unwrap_or(""),"reason":if ko {"아이디어 분류가 지원되는 형식과 다릅니다."} else {"The idea classification is outside the supported contract."},"direction":if ko {"기존 아이디어와 수정 내용을 보존하며 제안을 갱신하세요. 분류는 unverified, deferred, out_of_scope, rejected 중 원래 의미에 맞게 지정해야 합니다."} else {"Request an updated proposal preserving the idea and edits. Use unverified, deferred, out_of_scope or rejected according to its recorded meaning."},"evidence":[]})).collect();
    }
    let mut findings = result["qualityFindings"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    if !issue.is_empty()
        && issue != "Knowledge candidate has blocking quality findings"
        && !findings.iter().any(|f| f["message"] == issue)
    {
        findings.push(json!({"kind":issue.split(':').next().unwrap_or("missing_reference"),"severity":"blocking","locator":"article","message":issue,"sourceRefs":[]}));
    }
    findings.iter().filter(|f|f["severity"]=="blocking").map(|f|{
        let locator=f["locator"].as_str().unwrap_or("article");
        let path=locator.replace('[',".").replace(']',"").replace('.',"/");
        let located=result.pointer(&format!("/{path}"));
        let statement=if locator.contains("claimBindings") && located.is_some(){
            let index=locator.split("claimBindings[").nth(1).and_then(|s|s.split(']').next()).and_then(|s|s.parse::<usize>().ok());
            index.and_then(|i|result["article"]["claimBindings"][i]["statement"].as_str()).unwrap_or("").to_owned()
        }else if locator.contains("applicability"){result["article"]["applicability"]["summary"].as_str().unwrap_or("").to_owned()}
            else{result["article"]["claimBindings"].as_array().into_iter().flatten().find(|claim|format!("{locator} {issue}").contains(claim["claimId"].as_str().unwrap_or("\0"))).and_then(|v|v["statement"].as_str()).unwrap_or("").to_owned()};
        let message=f["message"].as_str().unwrap_or(issue);
        let mut refs=f["sourceRefs"].as_array().cloned().unwrap_or_default();
        if refs.is_empty() {
            refs=result["article"]["claimBindings"].as_array().into_iter().flatten().find(|claim|claim["statement"]==statement)
                .and_then(|claim|claim["sourceRefs"].as_array()).cloned().unwrap_or_default();
        }
        if refs.is_empty() && !message.starts_with("Rejected citation:") {
            refs=result["article"]["applicability"]["sourceRefs"].as_array().cloned().unwrap_or_default();
        }
        let evidence=snapshot["sourceManifest"].as_array().into_iter().flatten().filter(|source|{
            refs.iter().any(|r|r["type"]==source["type"]&&r["id"]==source["id"]&&r["revision"]==source["revision"]&&r["locator"]==source["locator"])
                || message.contains(&format!("{}:{} revision {} at {}",source["type"].as_str().unwrap_or(""),source["id"].as_str().unwrap_or(""),source["revision"].as_str().unwrap_or(""),source["locator"].as_str().unwrap_or("")))
        }).take(2).map(|s|json!({"type":s["type"],"id":s["id"],"revision":s["revision"],"locator":s["locator"],"quote":short(refs.iter().find(|r|r["type"]==s["type"]&&r["id"]==s["id"]&&r["revision"]==s["revision"]&&r["locator"]==s["locator"]).and_then(|r|r["quote"].as_str()).unwrap_or(&leaf_text(&s["content"])))})).collect::<Vec<_>>();
        let reason=match (f["kind"].as_str().unwrap_or(""),ko) {
            ("missing_reference",true)=>"인용 문구나 문장과 출처의 연결이 정확한 원본 기록과 맞지 않습니다.",
            ("invalid_finality",true)=>"현재 근거에는 이 문장을 확정 결정으로 뒷받침할 기록이 없습니다.",
            ("invalid_verification",true)=>"주장 표현이나 검증 수준이 연결된 원본 근거와 다릅니다.",
            ("applicability_overreach",true)=>"활용 설명이 근거 범위를 벗어났거나 명시적인 해석 연결이 빠졌습니다.",
            ("material_omission" | "validation_material_omission",true)=>"중요한 조건·실패·미해결 사항이 초안에서 빠졌습니다.",
            _=>message
        };
        json!({"kind":f["kind"],"locator":locator,"statement":short(if statement.is_empty(){result["article"]["bodyMarkdown"].as_str().unwrap_or("")}else{&statement}),"reason":reason,"technicalReason":message,"direction":suggestion(f["kind"].as_str().unwrap_or(""),ko),"evidence":evidence})
    }).collect()
}
fn diagnostic_base(b: &Value, ko: bool) -> Vec<Value> {
    match serde_json::from_value::<kd::GenerationResult>(b["result"].clone()).and_then(|r| {
        kd::prepared(b["snapshot"].clone(), r, "enhanced", "").map_err(serde::de::Error::custom)
    }) {
        Ok(p) => diagnostics(
            &b["snapshot"],
            &p["generationResult"],
            p["modelError"].as_str().unwrap_or(""),
            ko,
        ),
        Err(e) => diagnostics(&b["snapshot"], &b["result"], &e.to_string(), ko),
    }
}
fn turns(tx: &Transaction<'_>, task: &str, _n: i64, ko: bool) -> Result<Vec<Value>, String> {
    let mut s=tx.prepare("SELECT id,user_message,reply_json,state,error,applied_revision,created_at,source_snapshot_json FROM knowledge_revision_turns WHERE task_id=? ORDER BY rowid").map_err(|e|e.to_string())?;
    let mut out=s.query_map(params![task],|r|Ok(json!({"id":r.get::<_,String>(0)?,"userMessage":r.get::<_,String>(1)?,"reply":r.get::<_,Option<String>>(2)?.and_then(|s|serde_json::from_str::<Value>(&s).ok()),"state":r.get::<_,String>(3)?,"error":r.get::<_,String>(4)?,"appliedRevision":r.get::<_,Option<i64>>(5)?,"createdAt":r.get::<_,String>(6)?,"validationSnapshot":r.get::<_,String>(7)?}))).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
    let current=head(tx,task)?;
    for turn in &mut out {
        if matches!(turn["state"].as_str(), Some("proposed" | "failed")) && turn["reply"]["candidate"].is_object() {
            let snapshot: Value = serde_json::from_str(turn["validationSnapshot"].as_str().unwrap_or("{}" )).unwrap_or(json!({}));
            let check = serde_json::from_value::<kd::GenerationResult>(turn["reply"]["candidate"].clone()).map_err(|e|e.to_string()).and_then(|r|kd::prepared(snapshot.clone(), r, "enhanced", ""));
            if let Err(error) = check {
                turn["reply"]["applicable"] = json!(false);
                turn["reply"]["preparationError"] = json!(error);
                turn["reply"]["diagnostics"] = json!(diagnostics(&snapshot, &turn["reply"]["candidate"], &error, ko));
            }
        }
        if let Some(reply)=turn["reply"].as_object_mut() {
            if let Some(expected)=reply.get("expectedPrivateRevision").and_then(Value::as_i64) {
                if current != Some(expected) && turn["state"]=="proposed" {
                    turn["reply"]["applicable"]=json!(false);
                    turn["reply"]["applicationState"]=json!("stale");
                }
            }
        }
    }
    for turn in &mut out { turn.as_object_mut().unwrap().remove("validationSnapshot"); }
    Ok(out)
}
pub(crate) fn get(db: &Path, input: &Value) -> Result<Value, String> {
    let mut c = database::open(db)?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let b = base(&tx, input)?;
    let result = json!({"baseRevision":revision(input)?,"diagnostics":diagnostic_base(&b,input["locale"]=="ko"),"turns":turns(&tx,required(input,"taskId")?,revision(input)?,input["locale"]=="ko")?});
    tx.commit().map_err(|e| e.to_string())?;
    Ok(result)
}
#[cfg(test)]
async fn send(db: &Path, settings: &Path, input: &Value) -> Result<Value, String> {
    send_registered(db, settings, input, &super::jobs::JobRegistry::default()).await
}
pub(crate) async fn send_registered(
    db: &Path,
    settings: &Path,
    input: &Value,
    registry: &super::jobs::JobRegistry,
) -> Result<Value, String> {
    let task = required(input, "taskId")?;
    let n = revision(input)?;
    let id = required(input, "operationId")?;
    let message = required(input, "message")?;
    if message.chars().count() > 20000 {
        return Err("invalid_input: message exceeds 20000 characters".into());
    }
    let request_hash = hash(input);
    let (b, snapshot, history, expected_head) = {
        let mut c = database::open(db)?;
        let tx = database::immediate_transaction(&mut c)?;
        if let Some((old_hash, state, reply)) = tx
            .query_row(
                "SELECT request_hash,state,reply_json FROM knowledge_revision_turns WHERE id=?",
                [id],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, Option<String>>(2)?,
                    ))
                },
            )
            .optional()
            .map_err(|e| e.to_string())?
        {
            if old_hash != request_hash {
                return Err("operation_conflict".into());
            }
            return if state == "running" {
                Err("revision_chat_pending".into())
            } else {
                reply
                    .and_then(|s| serde_json::from_str::<Value>(&s).ok())
                    .ok_or("revision_chat_failed: retry with a new request".into())
            };
        }
        let pending:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM knowledge_revision_turns WHERE task_id=? AND base_revision=? AND state='running')",params![task,n],|r|r.get(0)).map_err(|e|e.to_string())?;
        if pending {
            return Err("revision_chat_pending: cancel the pending turn before retrying".into());
        }
        let b = base(&tx, input)?;
        if b["contentHash"] != input["expectedContentHash"] {
            return Err("content_hash_conflict".into());
        }
        let actual: i64 = tx
            .query_row(
                "SELECT current_revision FROM tasks WHERE id=?",
                [task],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        let expected_head = head(&tx, task)?;
        let history = turns(&tx, task, n, input["locale"]=="ko")?;
        tx.execute("INSERT INTO knowledge_revision_turns(id,task_id,base_revision,base_content_hash,expected_private_revision,request_hash,user_message,source_snapshot_json,prompt_version,state) VALUES(?,?,?,?,?,?,?,'{}',?,'running')",params![id,task,n,b["contentHash"].as_str(),expected_head,request_hash,message,crate::workflow_foundation::prompt_definition(PromptId::KnowledgeRevisionChat).version]).map_err(|e|e.to_string())?;
        let snapshot = kd::capture_snapshot_tx(&tx, task, actual, input["locale"].as_str().unwrap_or("en"))?;
        tx.execute("UPDATE knowledge_revision_turns SET source_snapshot_json=? WHERE id=?",params![snapshot.to_string(),id]).map_err(|e|e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        (b, snapshot, history, expected_head)
    };
    let mut context = kd::generation_context(&snapshot);
    context["reviewedVersion"] = json!({"revision":n,"contentHash":b["contentHash"],"bodyMarkdown":b["bodyMarkdown"],"result":b["result"],"diagnostics":diagnostic_base(&b,input["locale"]=="ko")});
    context["conversation"]=json!(history.iter().rev().take(16).collect::<Vec<_>>().into_iter().rev().map(|turn|
        json!({"userMessage":turn["userMessage"],"message":turn["reply"]["message"],"missingEvidence":turn["reply"]["missingEvidence"],"state":turn["state"]})
    ).collect::<Vec<_>>());
    context["previousProposal"] = history
        .iter()
        .rev()
        .find(|turn| if let Some(previous)=input["previousTurnId"].as_str() { turn["id"]==previous } else { turn["state"] == "proposed" && turn["reply"]["baseRevision"] == n })
        .map(|turn| turn["reply"]["candidate"].clone())
        .unwrap_or(Value::Null);
    context["previousProposalId"] = input["previousTurnId"].clone();
    if let Some(previous) = history.iter().find(|turn| turn["id"] == input["previousTurnId"]) {
        context["previousProposalValidation"] = json!({"preparationError":previous["reply"]["preparationError"],"diagnostics":previous["reply"]["diagnostics"],"baseRevision":previous["reply"]["baseRevision"]});
    }
    context["request"] = json!(message);
    let scope = format!("knowledge-revision:{task}:{id}");
    let mut tracking=None;
    let result = async {
        let model=super::settings::provider_credentials_for(settings,"completion_report").map(|(_,model,_)|model).unwrap_or_default();
        tracking=Some(super::foreground_jobs::Tracking::start(db,registry,"knowledge_revision",&model,json!({"taskId":task,"revision":n,"turnId":id,"view":"review"}))?);
        let observed=tracking.as_ref().unwrap();
        let token = observed.token.clone();
        let (text_context, images) = task_assistance::image_context(&context);
        let response = task_assistance::provider_json_with_images_tracked(
            settings,
            "completion_report",
            PromptId::KnowledgeRevisionChat,
            text_context,
            &images,
            Some(&token),
            Some(observed),
        )
        .await?;
        prepare_reply(&snapshot, &b, response, input["locale"] == "ko")
    }
    .await;
    registry.cancel(&format!("ephemeral:{scope}"));
    let c = database::open(db)?;
    let outcome = match result {
        Ok(mut reply) => {
            let prepared = reply
                .as_object_mut()
                .unwrap()
                .remove("prepared")
                .unwrap_or(Value::Null);
            reply["id"] = json!(id);
            reply["baseRevision"] = json!(n);
            reply["expectedPrivateRevision"] = json!(expected_head);
            reply["applicable"] = json!(prepared.is_object());
            let state = if prepared.is_object() {
                "proposed"
            } else if reply["candidate"].is_object() {
                "failed"
            } else {
                "needs_evidence"
            };
            let changed=c.execute("UPDATE knowledge_revision_turns SET reply_json=?,prepared_json=?,state=? WHERE id=? AND state='running'",params![reply.to_string(),prepared.as_object().map(|_|prepared.to_string()),state,id]).map_err(|e|e.to_string())?;
            let _ = expected_head;
            if changed == 0 { Err("revision_chat_cancelled".into()) } else { Ok(reply) }
        }
        Err(error) => {
            c.execute("UPDATE knowledge_revision_turns SET state='failed',error=? WHERE id=? AND state='running'",params![error,id]).map_err(|e|e.to_string())?;
            Err(error)
        }
    };
    if let Some(tracking)=tracking {
        let error=match &outcome {Err(error)=>Some(error.as_str()),Ok(reply) if reply["candidate"].is_object() && reply["applicable"]==false=>reply["preparationError"].as_str(),_=>None};
        tracking.finish(error)?;
    }
    outcome
}
fn prepare_reply(snapshot: &Value, b: &Value, response: Value, ko: bool) -> Result<Value, String> {
    let reply: Reply =
        serde_json::from_value(response).map_err(|e| format!("invalid_revision_reply: {e}"))?;
    if reply.message.trim().is_empty() {
        return Err("invalid_revision_reply: message required".into());
    }
    if reply.candidate.is_some() && !reply.missing_evidence.is_empty() {
        return Err(
            "invalid_revision_reply: missing evidence must not produce an applicable candidate"
                .into(),
        );
    }
    let original_candidate = reply
        .candidate
        .as_ref()
        .map(|candidate| serde_json::to_value(candidate).unwrap_or(Value::Null));
    let preparation = reply
        .candidate
        .map(|mut r| {
            // A model may repair bindings, not erase unchanged known blocking findings.
            let mut before = b["result"]["article"].clone();
            let mut after = serde_json::to_value(&r.article).map_err(|e| e.to_string())?;
            if let Some(o) = before.as_object_mut() {
                o.remove("title");
            }
            if let Some(o) = after.as_object_mut() {
                o.remove("title");
            }
            if after == before {
                for f in b["result"]["qualityFindings"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter(|f| f["severity"] == "blocking")
                {
                    if let Ok(f) = serde_json::from_value::<kd::QualityFinding>(f.clone()) {
                        r.quality_findings.push(f);
                    }
                }
            }
            kd::prepared(snapshot.clone(), r, "enhanced", "")
        })
        .transpose();
    // Invalid proposals remain repairable chat material, never canonical drafts
    // or authenticated evidence. Only a prepared, valid candidate can apply.
    let (prepared, error) = match preparation {
        Ok(value) => (value, String::new()),
        Err(error) => (None, error),
    };
    let candidate = prepared
        .as_ref()
        .map(|p| p["generationResult"].clone())
        .or(original_candidate);
    let quality = prepared
        .as_ref()
        .map(|p| p["qualityState"].clone())
        .or_else(|| candidate.as_ref().map(|_| json!("blocked_unsupported")));
    let issue = if error.is_empty() {
        prepared
            .as_ref()
            .and_then(|p| p["modelError"].as_str())
            .unwrap_or("")
    } else {
        &error
    };
    let ds = candidate
        .as_ref()
        .map(|candidate| diagnostics(snapshot, candidate, issue, ko))
        .unwrap_or_default();
    Ok(
        json!({"message":reply.message,"missingEvidence":reply.missing_evidence,"candidate":candidate,"qualityState":quality,"diagnostics":ds,"preparationError":error,"applicable":prepared.is_some(),"prepared":prepared}),
    )
}
pub(crate) fn cancel_registered(
    db: &Path,
    input: &Value,
    registry: &super::jobs::JobRegistry,
) -> Result<Value, String> {
    let result = decide(db, input, false)?;
    registry.cancel(&format!(
        "ephemeral:knowledge-revision:{}:{}",
        required(input, "taskId")?,
        required(input, "turnId")?
    ));
    let observed_id=super::foreground_jobs::revision_id(required(input,"turnId")?);
    registry.cancel(&observed_id);
    database::open(db)?.execute("UPDATE ai_jobs_v2 SET status='cancelled',execution_outcome='cancelled',finished_at=CURRENT_TIMESTAMP WHERE id=? AND status='running'",[observed_id]).map_err(|e|e.to_string())?;
    Ok(result)
}
pub(crate) fn decide(db: &Path, input: &Value, apply: bool) -> Result<Value, String> {
    let task = required(input, "taskId")?;
    let id = required(input, "turnId")?;
    let mut c = database::open(db)?;
    let tx = database::immediate_transaction(&mut c)?;
    let (n,base_hash,expected,prepared,state,applied):(i64,String,Option<i64>,Option<String>,String,Option<i64>)=tx.query_row("SELECT base_revision,base_content_hash,expected_private_revision,prepared_json,state,applied_revision FROM knowledge_revision_turns WHERE id=? AND task_id=?",params![id,task],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?))).optional().map_err(|e|e.to_string())?.ok_or("revision_turn_not_found")?;
    if state == "applied" && apply {
        return Ok(json!({"draftRevision":applied}));
    }
    if !apply {
        if state == "applied" {
            return Err("revision_already_applied".into());
        }
        tx.execute(
            "UPDATE knowledge_revision_turns SET state='cancelled' WHERE id=?",
            [id],
        )
        .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        return Ok(json!({"state":"cancelled"}));
    }
    if state != "proposed" {
        return Err("revision_proposal_unavailable".into());
    }
    if head(&tx, task)? != expected {
        return Err("current_private_conflict: draft changed; request a new proposal".into());
    }
    let b = base(&tx, &json!({"taskId":task,"baseRevision":n}))?;
    if b["contentHash"] != base_hash {
        return Err("content_hash_conflict".into());
    }
    let p: Value = match prepared {
        Some(text) => serde_json::from_str(&text).map_err(|e|e.to_string())?,
        None => {
            // Recover old proposed rows that exposed a candidate without prepared bytes.
            let (reply,source):(String,String)=tx.query_row("SELECT reply_json,source_snapshot_json FROM knowledge_revision_turns WHERE id=? AND task_id=?",params![id,task],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|e|e.to_string())?;
            let reply:Value=serde_json::from_str(&reply).map_err(|e|e.to_string())?;
            let source:Value=serde_json::from_str(&source).map_err(|e|e.to_string())?;
            let candidate=serde_json::from_value::<kd::GenerationResult>(reply["candidate"].clone()).map_err(|e|format!("revision_proposal_unavailable: {e}"))?;
            kd::prepared(source,candidate,"enhanced","").map_err(|e|format!("revision_proposal_unavailable: {e}"))?
        }
    };
    // Quality findings remain attached to the new version, never an apply gate.
    let saved = kd::append_generated_tx(&tx, &p)?;
    let next = saved["draftRevision"].as_i64().ok_or("invalid_revision")?;
    tx.execute("UPDATE knowledge_draft_versions SET derivation_kind='edited',derived_from_revision=? WHERE task_id=? AND revision=?",params![n,task,next]).map_err(|e|e.to_string())?;
    tx.execute(
        "UPDATE knowledge_revision_turns SET state='applied',applied_revision=? WHERE id=?",
        params![next, id],
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(saved)
}

#[cfg(test)]
mod tests {
    use super::super::settings;
    use super::*;
    fn fixture(blocked: bool) -> (tempfile::TempDir, std::path::PathBuf, Value, Value) {
        let parent = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join(".tmp");
        std::fs::create_dir_all(&parent).unwrap();
        let root = tempfile::tempdir_in(parent).unwrap();
        let db = root.path().join("fixture.sqlite");
        database::initialize(&db).unwrap();
        let mut c = database::open(&db).unwrap();
        c.execute_batch("INSERT INTO captures(id,text) VALUES('capture','raw');
            INSERT INTO tasks(id,origin_capture_id,current_revision,state,created_at,last_user_activity_at) VALUES('task','capture',1,'completed','2026-01-01','2026-01-01');
            INSERT INTO task_revisions(task_id,revision,title,detail,outcome,scope,non_goals,validation_criteria,content_hash,created_at) VALUES('task',1,'Approval flow','Keep approval explicit','Recorded outcome','offline','online','tests','task-hash','2026-01-01');
            INSERT INTO task_completions(id,task_id,task_revision,evidence,report,operation_id,created_at) VALUES('completion','task',1,'Receipt recorded','Approval remains required','complete-op','2026-01-02');").unwrap();
        let tx = c.transaction().unwrap();
        let snapshot = kd::capture_snapshot_tx(&tx, "task", 1, "en").unwrap();
        tx.commit().unwrap();
        let source = json!({"type":"task_revision","id":"task","revision":"1","locator":"definition","quote":"Approval flow"});
        let good = json!({"article":{"type":"concept","title":"Approval flow","bodyMarkdown":"Approval flow","finalOutcomes":[],"applicability":{"summary":"Approval flow","representativeQuestions":["What was recorded?"],"helpsWith":["Approval flow"],"conditions":[],"exclusions":[],"sourceRefs":[source]},"claimBindings":[{"claimId":"reported","statement":"Approval flow","epistemicState":"reported","sourceRefs":[source]}],"assumptionBindings":[]},"ideas":[],"qualityFindings":[]});
        let mut first = good.clone();
        if blocked {
            first["article"]["applicability"]["sourceRefs"][0]["quote"] = json!("invented quote");
        }
        let p = kd::prepared(
            snapshot.clone(),
            serde_json::from_value(first).unwrap(),
            "enhanced",
            "",
        )
        .unwrap();
        let tx = c.transaction().unwrap();
        let saved = kd::append_generated_tx(&tx, &p).unwrap();
        tx.commit().unwrap();
        let input = json!({"taskId":"task","baseRevision":1,"expectedContentHash":saved["contentHash"],"operationId":"chat-1","message":"Replace the rejected citation and keep the article","locale":"en"});
        (root, db, snapshot, json!({"input":input,"good":good}))
    }
    async fn provider(settings_path: &Path, response: Value) -> tokio::task::JoinHandle<String> {
        controlled_provider(settings_path, response, None).await
    }
    async fn controlled_provider(
        settings_path: &Path,
        response: Value,
        control: Option<(
            std::sync::Arc<tokio::sync::Notify>,
            std::sync::Arc<tokio::sync::Notify>,
        )>,
    ) -> tokio::task::JoinHandle<String> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        settings::save_provider(settings_path,&json!({"base_url":format!("http://{}",listener.local_addr().unwrap()),"model":"fixture","api_key":"test-only"})).unwrap();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut bytes = Vec::new();
            let mut buf = [0u8; 4096];
            loop {
                let n = socket.read(&mut buf).await.unwrap();
                assert!(n > 0);
                bytes.extend_from_slice(&buf[..n]);
                if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                    let len = String::from_utf8_lossy(&bytes[..end])
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .map(|v| v.trim().parse::<usize>().unwrap())
                        })
                        .unwrap();
                    if bytes.len() >= end + 4 + len {
                        break;
                    }
                }
            }
            if let Some((received, release)) = control {
                received.notify_one();
                release.notified().await;
            }
            let body =
                json!({"choices":[{"message":{"content":response.to_string()}}]}).to_string();
            let _ = socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body).as_bytes()).await;
            String::from_utf8(bytes).unwrap()
        })
    }
    #[tokio::test]
    async fn knowledge_revision_unknown_citation_and_legacy_unprepared_rows_apply_atomically() {
        for legacy in [false,true] {
            let (root,db,_,data)=fixture(false);let settings=root.path().join("settings.json");
            let mut candidate=data["good"].clone();candidate["article"]["claimBindings"][0]["sourceRefs"][0]["id"]=json!("invented-source");
            let server=provider(&settings,json!({"message":"Keep the useful edit and mark the invalid citation as a note","missingEvidence":[],"candidate":candidate})).await;
            let sent=send(&db,&settings,&data["input"]).await.unwrap();server.await.unwrap();
            assert_eq!(sent["applicable"],true);assert_eq!(sent["qualityState"],"blocked_unsupported");
            if legacy {
                let mut reply=sent.clone();reply.as_object_mut().unwrap().remove("applicable");reply["candidate"]=candidate;
                database::open(&db).unwrap().execute("UPDATE knowledge_revision_turns SET prepared_json=NULL,reply_json=? WHERE id='chat-1'",[reply.to_string()]).unwrap();
            }
            let reopened=get(&db,&data["input"]).unwrap();assert_eq!(reopened["turns"][0]["id"],"chat-1");assert!(reopened["turns"][0]["reply"]["candidate"].is_object());
            let app=crate::NativeApplication::isolated(&root.path().join("vault"),&db).unwrap();
            let applied=app.execute_workflow(crate::NativeOperation{name:"task-knowledge.revision-apply".into(),input:json!({"taskId":"task","turnId":"chat-1"})}).await;
            assert_eq!(applied.status,200,"{}",applied.body);assert_eq!(applied.body["draftRevision"],2);
            assert_eq!(decide(&db,&json!({"taskId":"task","turnId":"chat-1"}),true).unwrap()["draftRevision"],2);
            let c=database::open(&db).unwrap();let stored:String=c.query_row("SELECT result_json FROM knowledge_draft_versions WHERE revision=2",[],|r|r.get(0)).unwrap();
            let stored:Value=serde_json::from_str(&stored).unwrap();assert!(stored["article"]["claimBindings"][0]["sourceRefs"].as_array().unwrap().is_empty());assert!(stored["qualityFindings"].to_string().contains("invented-source"));
            assert_eq!(c.query_row("SELECT count(*) FROM knowledge_draft_versions",[],|r|r.get::<_,i64>(0)).unwrap(),2);
        }
    }

    #[tokio::test]
    async fn knowledge_revision_stale_proposal_can_be_updated_without_losing_its_candidate() {
        let(root,db,_,data)=fixture(false);let settings=root.path().join("settings.json");
        let server=provider(&settings,json!({"message":"First proposal","missingEvidence":[],"candidate":data["good"]})).await;
        send(&db,&settings,&data["input"]).await.unwrap();server.await.unwrap();
        database::open(&db).unwrap().execute("UPDATE task_completions SET report='A newer completion report' WHERE id='completion'",[]).unwrap();
        assert!(decide(&db,&json!({"taskId":"task","turnId":"chat-1"}),true).is_err());
        assert!(get(&db,&data["input"]).unwrap()["turns"][0]["reply"]["candidate"].is_object());
        let mut input=data["input"].clone();input["operationId"]=json!("chat-updated");input["previousTurnId"]=json!("chat-1");
        let server=provider(&settings,json!({"message":"Adapted to the current source snapshot","missingEvidence":[],"candidate":data["good"]})).await;
        send(&db,&settings,&input).await.unwrap();let request=server.await.unwrap();
        assert!(request.contains("previousProposal")&&request.contains("chat-1")&&request.contains("A newer completion report"));
        assert_eq!(decide(&db,&json!({"taskId":"task","turnId":"chat-updated"}),true).unwrap()["draftRevision"],2);
        assert_eq!(get(&db,&input).unwrap()["turns"][0]["id"],"chat-1");
    }

    #[tokio::test]
    async fn invalid_idea_proposal_is_preserved_and_refresh_applies_privately() {
        let (root, db, _, data) = fixture(false);
        let settings = root.path().join("settings.json");
        let mut candidate = data["good"].clone();
        candidate["ideas"] = json!([{"id":"idea-1","title":"Later option","bodyMarkdown":"Keep this option","disposition":"approved","reconsiderationConditions":["New evidence"],"sourceRefs":[],"relatedTopicKeys":[]}]);
        let server = provider(&settings, json!({"message":"Preserved edit","missingEvidence":[],"candidate":candidate})).await;
        let sent = send(&db, &settings, &data["input"]).await.unwrap();
        let request = server.await.unwrap();
        assert!(request.contains("out_of_scope"));
        assert_eq!(sent["applicable"], false);
        assert_eq!(sent["preparationError"], "invalid_knowledge: idea disposition is invalid");
        assert_eq!(sent["candidate"]["ideas"][0]["disposition"], "approved");
        assert_eq!(sent["diagnostics"][0]["locator"], "ideas[0].disposition");
        assert!(decide(&db, &json!({"taskId":"task","turnId":"chat-1"}), true).is_err());
        assert_eq!(database::open(&db).unwrap().query_row("SELECT count(*) FROM knowledge_draft_versions WHERE task_id='task'", [], |r|r.get::<_,i64>(0)).unwrap(), 1);
        // A legacy row without readiness metadata must be revalidated on read.
        database::open(&db).unwrap().execute("UPDATE knowledge_revision_turns SET state='proposed',prepared_json=NULL,reply_json=json_remove(reply_json,'$.applicable','$.diagnostics','$.preparationError') WHERE id='chat-1'", []).unwrap();
        let mut ko_input = data["input"].clone(); ko_input["locale"] = json!("ko");
        let reopened = get(&db, &ko_input).unwrap();
        assert_eq!(reopened["turns"][0]["reply"]["applicable"], false);
        assert!(reopened["turns"][0]["reply"]["diagnostics"][0]["reason"].as_str().unwrap().contains("아이디어"));
        assert!(decide(&db, &json!({"taskId":"task","turnId":"chat-1"}), true).is_err());
        // Reproduce an old v1 failed proposal while current private content is v2.
        let mut newer = data["good"].clone();
        newer["article"]["bodyMarkdown"] = json!("Approval flow\n\nPreserve newer v2 edits");
        let mut c = database::open(&db).unwrap(); let tx = c.transaction().unwrap();
        let snapshot = kd::capture_snapshot_tx(&tx, "task", 1, "en").unwrap();
        let prepared = kd::prepared(snapshot, serde_json::from_value(newer.clone()).unwrap(), "enhanced", "").unwrap();
        let v2 = kd::append_generated_tx(&tx, &prepared).unwrap(); tx.commit().unwrap();
        candidate["article"]["bodyMarkdown"] = newer["article"]["bodyMarkdown"].clone();
        candidate["ideas"][0]["disposition"] = json!("deferred");
        let mut input = data["input"].clone(); input["operationId"] = json!("chat-fixed"); input["previousTurnId"] = json!("chat-1"); input["baseRevision"] = json!(2); input["expectedContentHash"] = v2["contentHash"].clone();
        let server = provider(&settings, json!({"message":"Repaired classification","missingEvidence":[],"candidate":candidate})).await;
        assert_eq!(send(&db, &settings, &input).await.unwrap()["applicable"], true);
        let repair_request = server.await.unwrap();
        assert!(repair_request.contains("previousProposalValidation"));
        assert!(repair_request.contains("invalid_knowledge: idea disposition is invalid"));
        assert!(repair_request.contains("approved"));
        assert!(repair_request.contains("Preserve newer v2 edits"));
        assert_eq!(decide(&db, &json!({"taskId":"task","turnId":"chat-fixed"}), true).unwrap()["draftRevision"], 3);
        let c = database::open(&db).unwrap();
        let published: Option<i64> = c.query_row("SELECT published_revision FROM knowledge_pointers WHERE task_id='task'", [], |r|r.get(0)).unwrap();
        assert_eq!(published, None);
        let failed=super::super::jobs::get(&db,&super::super::foreground_jobs::revision_id("chat-1")).unwrap();
        assert_eq!(failed["status"],"failed");assert_eq!(failed["calls_started"],1);assert_eq!(failed["observation_only"],true);
        let done=super::super::jobs::get(&db,&super::super::foreground_jobs::revision_id("chat-fixed")).unwrap();
        assert_eq!(done["status"],"completed");assert_eq!(done["calls_completed"],1);assert_eq!(done["origin"]["revision"],2);
        // Restart reconciliation trusts a durable applied outcome, not a new call.
        c.execute("UPDATE ai_jobs_v2 SET status='running' WHERE id=?",[super::super::foreground_jobs::revision_id("chat-fixed")]).unwrap();
        super::super::foreground_jobs::recover(&db).unwrap();
        let recovered=super::super::jobs::get(&db,&super::super::foreground_jobs::revision_id("chat-fixed")).unwrap();
        assert_eq!(recovered["status"],"completed");assert_eq!(recovered["calls_started"],1);
    }

    #[tokio::test]
    async fn knowledge_revision_structural_failure_is_not_advertised_as_applicable() {
        let (root,db,_,data)=fixture(false);let settings=root.path().join("settings.json");let mut candidate=data["good"].clone();candidate["article"]["bodyMarkdown"]=json!("");
        let server=provider(&settings,json!({"message":"Incomplete proposal preserved","missingEvidence":[],"candidate":candidate})).await;
        let sent=send(&db,&settings,&data["input"]).await.unwrap();server.await.unwrap();assert_eq!(sent["applicable"],false);assert!(sent["preparationError"].as_str().unwrap().contains("bodyMarkdown"));
        let saved=get(&db,&data["input"]).unwrap();assert_eq!(saved["turns"][0]["state"],"failed");assert!(saved["turns"][0]["reply"]["candidate"].is_object());
        assert!(decide(&db,&json!({"taskId":"task","turnId":"chat-1"}),true).is_err());
    }

    #[tokio::test]
    async fn knowledge_work_log_pixels_reach_generation_and_revision_requests() {
        let (root,db,_,data)=fixture(false);
        // Seed the screenshot as pre-completion evidence, then freeze the fixture.
        database::open(&db).unwrap().execute("UPDATE tasks SET state='in_progress' WHERE id='task'", []).unwrap();
        let png="iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=";
        crate::application::task_service::TaskApplicationService::new(&db).execute("task.work-log.create",&json!({"operationId":"image-log","taskId":"task","expectedTaskRevision":1,"body":"My screenshot","attachment":{"name":"pixel.png","mediaType":"image/png","data":png}})).unwrap();
        database::open(&db).unwrap().execute("UPDATE tasks SET state='completed' WHERE id='task'", []).unwrap();
        let settings=root.path().join("settings.json");
        let server=provider(&settings,data["good"].clone()).await;
        let generated=task_assistance::prepare_knowledge_draft(&db,&settings,&json!({"taskId":"task","expectedTaskRevision":1,"locale":"en"})).await.unwrap();
        assert!(generated["bodyMarkdown"].as_str().unwrap().contains("Approval flow"));
        let request=server.await.unwrap();
        let payload:Value=serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap();
        assert!(payload["messages"].to_string().contains(&format!("data:image/png;base64,{png}")));
        let content=payload["messages"][0]["content"].as_array().unwrap();
        assert!(!content[0]["text"].as_str().unwrap().contains(png),"binary must not enter textual prompt/catalog");
        let server=provider(&settings,json!({"message":"Your statement is a primary user report","missingEvidence":[],"candidate":data["good"]})).await;
        let mut input=data["input"].clone();input["message"]=json!("I experienced this myself; no external proof is needed");
        send(&db,&settings,&input).await.unwrap();
        let request=server.await.unwrap();
        let payload:Value=serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap();
        assert!(payload["messages"].to_string().contains(&format!("data:image/png;base64,{png}")));
        assert!(payload["messages"].to_string().contains("user_statement"));
        assert!(payload["messages"].to_string().contains("no external proof is needed"));
    }

    #[tokio::test]
    async fn knowledge_revision_repairs_rejected_citation_atomically_without_touching_publication()
    {
        for published in [false, true] {
            let (root, db, _snapshot, data) = fixture(true);
            let settings = root.path().join("settings.json");
            let c = database::open(&db).unwrap();
            if published {
                c.execute("UPDATE knowledge_pointers SET published_revision=1,publication_path='Knowledge/Approval.md',publication_content_hash='published-hash' WHERE task_id='task'",[]).unwrap();
            }
            kd::add_review_note(&db,&json!({"taskId":"task","knowledgeRevision":1,"expectedContentHash":data["input"]["expectedContentHash"],"body":"PRIVATE OPINION IS NOT CHAT","operationId":"note"})).unwrap();
            let server=provider(&settings,json!({"message":"Replace the invented quote with Approval flow from the Task definition.","missingEvidence":[],"candidate":data["good"]})).await;
            let response = send(&db, &settings, &data["input"]).await.unwrap();
            let sent = server.await.unwrap();
            assert!(
                sent.contains("citationCatalog")
                    && sent.contains("reviewedVersion")
                    && sent.contains("Rejected citation")
            );
            assert!(!sent.contains("PRIVATE OPINION IS NOT CHAT"));
            assert_eq!(response["qualityState"], "valid");
            assert!(response.get("prepared").is_none());
            assert_eq!(
                c.query_row("SELECT count(*) FROM knowledge_draft_versions", [], |r| r
                    .get::<_, i64>(
                    0
                ))
                .unwrap(),
                1
            );
            let applied = decide(&db, &json!({"taskId":"task","turnId":"chat-1"}), true).unwrap();
            assert_eq!(applied["draftRevision"], 2);
            assert_eq!(
                decide(&db, &json!({"taskId":"task","turnId":"chat-1"}), true).unwrap()
                    ["draftRevision"],
                2
            );
            let row:(String,String)=c.query_row("SELECT quality_state,result_json FROM knowledge_draft_versions WHERE task_id='task' AND revision=2",[],|r|Ok((r.get(0)?,r.get(1)?))).unwrap();
            assert_eq!(row.0, "valid");
            assert!(!row.1.contains("Rejected citation"));
            assert_eq!(c.query_row("SELECT quality_state FROM knowledge_draft_versions WHERE task_id='task' AND revision=1",[],|r|r.get::<_,String>(0)).unwrap(),"blocked_unsupported");
            assert_eq!(
                c.query_row(
                    "SELECT published_revision FROM knowledge_pointers WHERE task_id='task'",
                    [],
                    |r| r.get::<_, Option<i64>>(0)
                )
                .unwrap(),
                published.then_some(1)
            );
        }
    }
    #[test]
    fn knowledge_revision_sentence_replacement_deletion_and_interpretation_rebuild_bindings() {
        let (_root, db, snapshot, data) = fixture(true);
        let mut c = database::open(&db).unwrap();
        let tx = c.transaction().unwrap();
        let b = base(&tx, &data["input"]).unwrap();
        tx.commit().unwrap();
        for mode in ["replace", "delete", "interpret"] {
            let mut candidate = data["good"].clone();
            if mode == "replace" {
                candidate["article"]["bodyMarkdown"] =
                    json!("Approval flow\nKeep approval explicit");
                candidate["article"]["claimBindings"][0]["statement"] =
                    json!("Keep approval explicit");
                candidate["article"]["claimBindings"][0]["sourceRefs"][0]["quote"] =
                    json!("Keep approval explicit");
            } else if mode == "delete" {
                candidate["article"]["claimBindings"] = json!([]);
            } else {
                let phrase = "Planning a review while preserving approval";
                candidate["article"]["bodyMarkdown"] =
                    json!(format!("Approval flow\nReuse interpretation: {phrase}"));
                candidate["article"]["applicability"]["summary"] = json!(phrase);
                candidate["article"]["assumptionBindings"] = json!([{"assumptionId":"applicability:review","bodyLocator":phrase,"sourceRefs":candidate["article"]["applicability"]["sourceRefs"]}]);
            }
            let p = prepare_reply(
                &snapshot,
                &b,
                json!({"message":"Repair proposal","missingEvidence":[],"candidate":candidate}),
                false,
            )
            .unwrap();
            assert_eq!(p["qualityState"], "valid");
            assert!(p["candidate"]["qualityFindings"]
                .as_array()
                .unwrap()
                .is_empty());
        }
        let mut fake = data["good"].clone();
        fake["article"]["claimBindings"][0]["sourceRefs"][0]["quote"] = json!("fake approval");
        let p = prepare_reply(
            &snapshot,
            &b,
            json!({"message":"Bad proposal","missingEvidence":[],"candidate":fake}),
            false,
        )
        .unwrap();
        assert_ne!(p["qualityState"], "valid");
        let mut unknown = data["good"].clone();
        unknown["article"]["claimBindings"][0]["sourceRefs"][0]["id"] = json!("outside-snapshot");
        let unavailable=prepare_reply(&snapshot,&b,json!({"message":"Correct this invalid source next","missingEvidence":[],"candidate":unknown}),false).unwrap();
        assert_ne!(unavailable["qualityState"], "valid");
        assert!(unavailable["prepared"].is_object());assert_eq!(unavailable["applicable"],true);assert!(unavailable["prepared"]["generationResult"]["article"]["claimBindings"][0]["sourceRefs"].as_array().unwrap().is_empty());
        assert!(!unavailable["diagnostics"].as_array().unwrap().is_empty());
        let mut material_snapshot = snapshot.clone();
        material_snapshot["journey"]["result"]["claims"] = json!([{"id":"required-unresolved","statement":"Approval flow","kind":"unresolved","status":"current"}]);
        let mut omitted = data["good"].clone();
        omitted["article"]["claimBindings"] = json!([]);
        assert_ne!(prepare_reply(&material_snapshot,&b,json!({"message":"Cannot omit unresolved work","missingEvidence":[],"candidate":omitted}),false).unwrap()["qualityState"],"valid");
        let mut semantic_base = b.clone();
        semantic_base["result"] = data["good"].clone();
        semantic_base["result"]["qualityFindings"] = json!([{"kind":"unsupported_addition","severity":"blocking","locator":"article","message":"Known unsupported conclusion","sourceRefs":[]}]);
        let mut retitled = data["good"].clone();
        retitled["article"]["title"] = json!("Only a new title");
        assert_ne!(prepare_reply(&snapshot,&semantic_base,json!({"message":"Cannot erase a finding","missingEvidence":[],"candidate":retitled}),false).unwrap()["qualityState"],"valid");
        let mut verified = data["good"].clone();
        verified["article"]["claimBindings"][0]["epistemicState"] = json!("verified");
        assert_ne!(
            prepare_reply(
                &snapshot,
                &b,
                json!({"message":"Cannot promote","missingEvidence":[],"candidate":verified}),
                false
            )
            .unwrap()["qualityState"],
            "valid"
        );
    }
    #[tokio::test]
    async fn queue_cancel_stops_revision_without_applying_or_retrying_it() {
        let (root,db,_,data)=fixture(false);let settings=root.path().join("settings.json");
        let received=std::sync::Arc::new(tokio::sync::Notify::new());let release=std::sync::Arc::new(tokio::sync::Notify::new());
        let server=controlled_provider(&settings,json!({"message":"Review only","missingEvidence":[],"candidate":data["good"]}),Some((received.clone(),release.clone()))).await;
        let registry=super::super::jobs::JobRegistry::default();
        let worker=registry.clone();let request=data["input"].clone();let worker_db=db.clone();let worker_settings=settings.clone();
        let pending=tokio::spawn(async move {send_registered(&worker_db,&worker_settings,&request,&worker).await});
        received.notified().await;
        let publication=super::super::task_publication_status::project(&database::open(&db).unwrap()).unwrap();
        assert_eq!(publication["task"]["proposal"],"generating");
        let id=super::super::foreground_jobs::revision_id("chat-1");
        super::super::jobs::cancel(&db,&registry,&id).unwrap();
        assert!(super::super::jobs::cancel(&db,&registry,&id).is_err());
        release.notify_one();assert!(pending.await.unwrap().is_err());server.await.unwrap();
        assert_eq!(get(&db,&data["input"]).unwrap()["turns"][0]["state"],"cancelled");
        assert_eq!(super::super::jobs::get(&db,&id).unwrap()["status"],"cancelled");
        let publication=super::super::task_publication_status::project(&database::open(&db).unwrap()).unwrap();
        assert_ne!(publication["task"]["proposal"],"generating");
        assert_eq!(publication["task"]["published"],false);
        assert!(decide(&db,&json!({"taskId":"task","turnId":"chat-1"}),true).is_err());
        let c=database::open(&db).unwrap();assert_eq!(c.query_row("SELECT count(*) FROM knowledge_draft_versions",[],|r|r.get::<_,i64>(0)).unwrap(),1);
    }

    #[tokio::test]
    async fn knowledge_revision_cancelled_provider_result_and_invalid_candidate_cannot_apply() {
        let (root, db, _snapshot, data) = fixture(false);
        let settings = root.path().join("settings.json");
        let received = std::sync::Arc::new(tokio::sync::Notify::new());
        let release = std::sync::Arc::new(tokio::sync::Notify::new());
        let server = controlled_provider(
            &settings,
            json!({"message":"Proposed repair","missingEvidence":[],"candidate":data["good"]}),
            Some((received.clone(), release.clone())),
        )
        .await;
        let registry = super::super::jobs::JobRegistry::default();
        let worker_registry = registry.clone();
        let cloned_db = db.clone();
        let request = data["input"].clone();
        let cloned_settings = settings.clone();
        let pending = tokio::spawn(async move {
            send_registered(&cloned_db, &cloned_settings, &request, &worker_registry).await
        });
        received.notified().await;
        cancel_registered(&db, &json!({"taskId":"task","turnId":"chat-1"}), &registry).unwrap();
        release.notify_one();
        assert!(pending.await.unwrap().unwrap_err().contains("cancelled"));
        server.await.unwrap();
        assert_eq!(
            get(&db, &data["input"]).unwrap()["turns"][0]["state"],
            "cancelled"
        );
        assert!(decide(&db, &json!({"taskId":"task","turnId":"chat-1"}), true).is_err());
        let mut fake = data["good"].clone();
        fake["article"]["applicability"]["sourceRefs"][0]["quote"] = json!("fabricated approval");
        let server=provider(&settings,json!({"message":"Invalid proposal retained for correction","missingEvidence":[],"candidate":fake})).await;
        let mut input = data["input"].clone();
        input["operationId"] = json!("chat-invalid");
        assert_ne!(
            send(&db, &settings, &input).await.unwrap()["qualityState"],
            "valid"
        );
        server.await.unwrap();
        let applied=decide(&db,&json!({"taskId":"task","turnId":"chat-invalid"}),true).unwrap();
        assert_eq!(applied["draftRevision"],2);
        let c=database::open(&db).unwrap();
        let (quality,result):(String,String)=c.query_row("SELECT quality_state,result_json FROM knowledge_draft_versions WHERE revision=2",[],|r|Ok((r.get(0)?,r.get(1)?))).unwrap();
        assert_eq!(quality,"blocked_unsupported");
        let result:Value=serde_json::from_str(&result).unwrap();
        assert!(result["article"]["applicability"]["sourceRefs"].as_array().unwrap().is_empty());
        assert!(result["qualityFindings"].to_string().contains("fabricated approval"));
    }

    #[tokio::test]
    async fn knowledge_revision_missing_evidence_cancel_failure_and_stale_guards_preserve_versions()
    {
        let (root, db, _snapshot, data) = fixture(false);
        let settings = root.path().join("settings.json");
        assert!(send(&db, &settings, &data["input"]).await.is_err());
        let conversation = get(&db, &data["input"]).unwrap();
        assert_eq!(conversation["turns"][0]["state"], "failed");
        assert_eq!(
            conversation["turns"][0]["userMessage"],
            data["input"]["message"]
        );
        let mut input = data["input"].clone();
        input["operationId"] = json!("chat-2");
        let server=provider(&settings,json!({"message":"No observed approval is present. Supply the actual decision record.","missingEvidence":["The original approval decision and its source"],"candidate":null})).await;
        assert!(send(&db, &settings, &input).await.unwrap()["candidate"].is_null());
        server.await.unwrap();
        assert!(decide(&db, &json!({"taskId":"task","turnId":"chat-2"}), true).is_err());
        for (id, cancel, stale) in [
            ("chat-3", true, false),
            ("chat-4", false, true),
            ("chat-5", false, false),
        ] {
            input["operationId"] = json!(id);
            let server=provider(&settings,json!({"message":"Grounded revision","missingEvidence":[],"candidate":data["good"]})).await;
            send(&db, &settings, &input).await.unwrap();
            server.await.unwrap();
            if cancel {
                decide(&db, &json!({"taskId":"task","turnId":id}), false).unwrap();
            }
            if stale {
                database::open(&db).unwrap().execute("UPDATE task_completions SET report='Changed evidence' WHERE id='completion'",[]).unwrap();
            }
            if !cancel && !stale {
                database::open(&db).unwrap().execute("UPDATE knowledge_pointers SET current_private_revision=NULL WHERE task_id='task'",[]).unwrap();
            }
            assert!(decide(&db, &json!({"taskId":"task","turnId":id}), true).is_err());
        }
        assert_eq!(
            database::open(&db)
                .unwrap()
                .query_row("SELECT count(*) FROM knowledge_draft_versions", [], |r| r
                    .get::<_, i64>(
                    0
                ))
                .unwrap(),
            1
        );
    }
}
