use super::super::{database, semantic::SemanticEngine, task_assistance as assistance, vault};
use super::*;
use crate::workflow_foundation::PromptId;
use rusqlite::{Connection, Transaction};

fn string<'a>(v: &'a Value, key: &str) -> Result<&'a str, String> {
    v[key]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("invalid_input: {key}"))
}
fn number(v: &Value, key: &str) -> Result<i64, String> {
    v[key]
        .as_i64()
        .ok_or_else(|| format!("invalid_input: {key}"))
}
fn list(v: &Value) -> Result<&Vec<Value>, String> {
    v.as_array()
        .filter(|a| a.len() <= 32)
        .ok_or_else(|| "invalid_preview: array".into())
}
fn keys(v: &Value, allowed: &[&str]) -> Result<(), String> {
    if v.as_object()
        .is_none_or(|o| o.keys().any(|k| !allowed.contains(&k.as_str())))
    {
        Err("invalid_preview: unknown fields".into())
    } else {
        Ok(())
    }
}
pub(crate) fn validate_planner_output(v: &Value) -> Result<(), String> {
    keys(v, &["retrieval", "preliminaryAssumptions"])?;
    validate_planner(v)?;
    let mut ids = std::collections::HashSet::new();
    for assumption in list(&v["preliminaryAssumptions"])? {
        keys(assumption, &["id", "text", "basis"])?;
        if !ids.insert(string(assumption, "id")?)
            || string(assumption, "text")?.len() > 12000
            || !matches!(
                assumption["basis"].as_str(),
                Some("user_context" | "source" | "model_inference")
            )
        {
            return Err("invalid_preview: preliminary assumption".into());
        }
    }
    Ok(())
}
pub(crate) fn validate_output(v: &Value) -> Result<(), String> {
    keys(v, &["preview", "claimSources", "optionalInvestigation"])?;
    let p = &v["preview"];
    keys(
        p,
        &[
            "description",
            "background",
            "goal",
            "scope",
            "nonGoals",
            "constraints",
            "completionCriteria",
            "initialApproach",
            "assumptions",
        ],
    )?;
    required_fields(p)?;
    for k in ["description", "background", "goal", "scope", "nonGoals"] {
        if p[k].as_str().is_none_or(|s| s.len() > 12000) {
            return Err("invalid_preview: text".into());
        }
    }
    for k in ["constraints", "completionCriteria", "initialApproach"] {
        for s in list(&p[k])? {
            if s.as_str().is_none_or(|s| s.len() > 12000) {
                return Err("invalid_preview: list text".into());
            }
        }
    }
    let mut ids = std::collections::HashSet::new();
    for a in list(&p["assumptions"])? {
        keys(a, &["id", "text", "basis", "sourceClaimIds"])?;
        if !ids.insert(string(a, "id")?)
            || string(a, "text")?.len() > 12000
            || !matches!(
                a["basis"].as_str(),
                Some("user_context" | "source" | "model_inference")
            )
        {
            return Err("invalid_preview: assumption".into());
        }
        list(&a["sourceClaimIds"])?;
    }
    let claims = list(&v["claimSources"])?;
    for c in claims {
        keys(
            c,
            &[
                "claimId",
                "documentId",
                "documentVersion",
                "section",
                "role",
            ],
        )?;
        for k in ["claimId", "documentId", "documentVersion", "role"] {
            string(c, k)?;
        }
        if !c["section"].is_string() {
            return Err("invalid_reference_binding".into());
        }
        let field = string(c, "claimId")?.split(':').next().unwrap_or("");
        if p.get(field).is_none() {
            return Err("invalid_reference_binding".into());
        }
    }
    for a in list(&p["assumptions"])? {
        for cid in list(&a["sourceClaimIds"])? {
            if !claims.iter().any(|c| c["claimId"] == *cid) {
                return Err("invalid_reference_binding".into());
            }
        }
    }
    validate_planner(&json!({"retrieval":v["optionalInvestigation"]}))?;
    Ok(())
}
pub(crate) fn head(c: &Connection, session: &str) -> Result<i64, String> {
    c.query_row(
        "SELECT current_version FROM work_previews WHERE session_id=?",
        [session],
        |r| r.get(0),
    )
    .optional()
    .map(|v| v.unwrap_or(0))
    .map_err(|e| e.to_string())
}
fn context_key(v: &Value) -> Value {
    json!({"messages":v["messages"],"taskSnapshot":v["taskSnapshot"],"originalCapture":v["originalCapture"],"hierarchyContext":v["hierarchyContext"],"draftRevision":v["draftRevision"]})
}
fn source_current(c: &Connection, r: &Value) -> Result<bool, String> {
    c.query_row("SELECT EXISTS(SELECT 1 FROM vault_search_units u JOIN vault_documents d ON d.path=u.path AND d.source_hash=u.source_revision WHERE u.document_id=? AND u.source_revision=? AND u.section=?)",params![string(r,"documentId")?,string(r,"documentVersion")?,r["section"].as_str().unwrap_or("")],|r|r.get(0)).map_err(|e|e.to_string())
}
pub(crate) fn is_current(
    tx: &Transaction<'_>,
    input: &Value,
    payload: &Value,
) -> Result<bool, String> {
    let session = string(input, "sessionId")?;
    if head(tx, session)? != number(input, "expectedPreviewVersion")?
        || context_key(&assistance::image_context(&assistance::refinement_context(tx, session)?).0)
            != context_key(&input["promptContext"])
    {
        return Ok(false);
    }
    for r in list(&payload["references"])? {
        if !source_current(tx, r)? {
            return Ok(false);
        }
        if let Some(root) = payload["sourceRoot"].as_str() {
            if vault::read(Path::new(root), string(r, "path")?, "en")
                .ok()
                .is_none_or(|v| v["source_hash"] != r["documentVersion"])
            {
                return Ok(false);
            }
        }
    }
    Ok(true)
}
pub(super) fn retrieve(
    db: &Path,
    vault_root: &Path,
    semantic: &SemanticEngine,
    intent: &RetrievalIntent,
) -> Result<(String, Vec<Value>), String> {
    if !intent.needed {
        return Ok(("not_needed".into(), vec![]));
    }
    if intent.queries.len() > 4 || intent.queries.iter().any(|q| q.len() > 500) {
        return Err("invalid_retrieval_intent: query budget".into());
    }
    let request_hash = hash(&serde_json::to_value(intent).map_err(|e| e.to_string())?);
    let mut refs = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut queries = intent.queries.clone();
    if let Some(requery) = &intent.requery {
        queries.extend(requery.queries.iter().take(2).cloned());
    }
    for query in queries {
        let results = vault::search(db, semantic, &query, 16, 0, true)
            .map_err(|_| "necessary_retrieval_failed")?;
        for item in results["results"].as_array().into_iter().flatten() {
            if !intent.aspects.is_empty()
                && !intent
                    .aspects
                    .iter()
                    .any(|a| Some(a.as_str()) == item["aspect"].as_str())
            {
                continue;
            }
            if [
                ("informationTypes", "informationType"),
                ("statuses", "status"),
            ]
            .iter()
            .any(|(f, k)| {
                intent
                    .filters
                    .get(*f)
                    .and_then(Value::as_array)
                    .is_some_and(|a| !a.is_empty() && !a.contains(&item[*k]))
            }) {
                continue;
            }
            let identity = format!(
                "{}:{}:{}",
                item["documentId"], item["source_hash"], item["section"]
            );
            if !seen.insert(identity.clone()) {
                continue;
            }
            let exact = vault::read(vault_root, string(item, "path")?, "en")
                .map_err(|_| "reference_version_unavailable")?;
            if exact["source_hash"] != item["source_hash"] {
                return Err("reference_version_unavailable".into());
            }
            // Internal desktop scope is trusted; no MCP connection grant is reused.
            refs.push(json!({"referenceId":identity,"documentId":item["documentId"],"documentVersion":item["source_hash"],"section":item["section"],"path":item["path"],"title":item["title"],"aspect":item["aspect"],"informationType":item["informationType"],"status":item["status"],"excerpt":item["snippet"],"retrievalRequestHash":request_hash,"evidenceScope":"local_exact_revision"}));
            if refs.len() == 16 {
                break;
            }
        }
        if refs.len() == 16 {
            break;
        }
    }
    Ok((
        if refs.is_empty() {
            "no_suitable_result"
        } else {
            "results"
        }
        .into(),
        refs,
    ))
}
pub(crate) async fn prepare(
    db: &Path,
    settings: &Path,
    vault_root: &Path,
    semantic: &SemanticEngine,
    input: &Value,
) -> Result<Value, String> {
    let started = std::time::Instant::now();
    let context = json!({"locale":input["locale"],"session":input["promptContext"]});
    let images = input["images"].as_array().map(Vec::as_slice).unwrap_or(&[]);
    let task = string(input, "modelTask")?;
    let planner = assistance::provider_json_with_images(
        settings,
        task,
        PromptId::WorkPreviewPlanner,
        context.clone(),
        images,
        None,
    )
    .await?;
    let planner_ms = started.elapsed().as_millis();
    let intent = validate_planner(&planner)?;
    let (mut outcome, mut refs) = retrieve(db, vault_root, semantic, &intent)?;
    for reference in input["promptContext"]["documentMentions"].as_array().into_iter().flatten(){
        if refs.iter().any(|r|r["documentId"]==reference["documentId"]&&r["documentVersion"]==reference["documentVersion"]&&r["section"]==reference["section"]){continue}
        let exact=vault::read(vault_root,string(reference,"path")?,"en")?;
        if exact["source_hash"]!=reference["documentVersion"]{return Err("reference_version_unavailable".into())}
        refs.push(reference.clone());
    }
    if !refs.is_empty(){outcome="results".into();}
    let retrieval_ms = started.elapsed().as_millis() - planner_ms;
    let output = assistance::provider_json_with_images(
        settings,
        task,
        PromptId::WorkPreviewFinalizer,
        json!({"context":context,"planner":planner,"retrievalOutcome":outcome,"references":refs}),
        images,
        None,
    )
    .await?;
    let finalizer_ms = started.elapsed().as_millis() - planner_ms - retrieval_ms;
    let fields = finalize(&output, &refs)?;
    // Preserve existing Task/Capture proposal consumers while exposing structured fields.
    let patch = task_fields(&fields);
    let proposal = if input["taskBinding"]["id"].is_string() {
        json!({"id":"work-preview","type":"task_patch","payload":{"patch":patch,"expectedTaskRevision":input["taskBinding"]["taskRevision"],"hierarchyContextHash":input["hierarchyContextHash"]}})
    } else {
        json!({"id":"work-preview","type":"new_task","payload":patch})
    };
    Ok(
        json!({"stageTimings":{"plannerMs":planner_ms,"retrievalMs":retrieval_ms,"finalizerMs":finalizer_ms},"sourceRoot":vault_root,"proposals":[proposal],"structuredPreview":fields,"claimSources":output["claimSources"],"references":refs,"retrievalOutcome":outcome,"optionalInvestigation":output["optionalInvestigation"]}),
    )
}
fn task_fields(f: &Value) -> Value {
    let lines = |k: &str| {
        f[k].as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join("\n")
    };
    json!({"title":f["description"].as_str().unwrap_or("").chars().take(180).collect::<String>(),"detail":format!("{}\n\n{}\n\n{}\n\n{}",f["description"].as_str().unwrap_or(""),f["background"].as_str().unwrap_or(""),lines("constraints"),lines("initialApproach")),"outcome":f["goal"],"scope":f["scope"],"nonGoals":f["nonGoals"],"validationCriteria":lines("completionCriteria")})
}
pub(crate) fn save_generated_tx(
    tx: &Transaction<'_>,
    input: &Value,
    p: &Value,
    job: &str,
    context: i64,
) -> Result<Value, String> {
    let session = string(input, "sessionId")?;
    let preview = tx
        .query_row(
            "SELECT id FROM work_previews WHERE session_id=?",
            [session],
            |r| r.get::<_, String>(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .unwrap_or_else(id);
    let next = head(tx, session)? + 1;
    let fields = &p["structuredPreview"];
    tx.execute("INSERT OR IGNORE INTO work_previews(id,session_id,subject_type,subject_id) SELECT ?,id,CASE WHEN task_id IS NULL THEN 'capture' ELSE 'task' END,COALESCE(task_id,capture_id) FROM refinement_sessions WHERE id=?",params![preview,session]).map_err(|e|e.to_string())?;
    tx.execute("INSERT INTO work_preview_versions(preview_id,version,derivation_kind,generation_job_id,task_revision,context_revision,locale,fields_json,content_hash,prompt_id,prompt_version) VALUES(?,?,'generated',?,?,?,?,?,?,'work_preview_finalizer',1)",params![preview,next,job,input["taskBinding"]["taskRevision"].as_i64(),context,input["locale"].as_str().unwrap_or("en"),fields.to_string(),hash(fields)]).map_err(|e|e.to_string())?;
    for a in list(&fields["assumptions"])? {
        tx.execute("INSERT INTO work_preview_assumptions(preview_id,version,id,text,status,basis,source_claim_ids_json) VALUES(?,?,?,?,'open',?,?)",params![preview,next,string(a,"id")?,string(a,"text")?,string(a,"basis")?,a["sourceClaimIds"].to_string()]).map_err(|e|e.to_string())?;
    }
    for r in list(&p["references"])? {
        let claims = list(&p["claimSources"])?
            .iter()
            .filter(|c| {
                c["documentId"] == r["documentId"]
                    && c["documentVersion"] == r["documentVersion"]
                    && c["section"] == r["section"]
            })
            .collect::<Vec<_>>();
        let ids = claims
            .iter()
            .map(|c| c["claimId"].clone())
            .collect::<Vec<_>>();
        let role = claims
            .first()
            .and_then(|c| c["role"].as_str())
            .unwrap_or("discovered");
        let mut metadata = r.clone();
        metadata["claimUses"] = json!(claims.iter().map(|claim| {
            let statement = super::super::reference_provenance::claim_statement(fields, claim["claimId"].as_str().unwrap_or("")).ok_or("invalid_reference_binding: empty grounded statement")?;
            Ok(json!({"claimId":claim["claimId"],"role":claim["role"],"statement":statement}))
        }).collect::<Result<Vec<Value>, &str>>()?);
        tx.execute("INSERT INTO work_preview_references(id,preview_id,version,retrieval_request_hash,source_bundle_hash,document_id,document_version,section,metadata_json,role,claim_ids_json) VALUES(?,?,?,?,?,?,?,?,?,?,?)",params![id(),preview,next,string(r,"retrievalRequestHash")?,hash(&p["references"]),string(r,"documentId")?,string(r,"documentVersion")?,r["section"].as_str().unwrap_or(""),metadata.to_string(),role,json!(ids).to_string()]).map_err(|e|e.to_string())?;
        for claim in claims {
            let statement = super::super::reference_provenance::claim_statement(fields, string(claim,"claimId")?).ok_or("invalid_reference_binding: empty grounded statement")?;
            tx.execute("INSERT INTO reference_interactions(id,preview_id,context_revision,document_id,document_version,section,kind,use_scope,reason,preview_version) VALUES(?,?,?,?,?,?,'used',?,?,?)",params![id(),preview,context,string(r,"documentId")?,string(r,"documentVersion")?,r["section"].as_str().unwrap_or(""),string(claim,"claimId")?,statement,next]).map_err(|e|e.to_string())?;
        }
    }
    tx.execute("UPDATE work_previews SET current_version=?,context_revision=?,task_revision=?,updated_at=CURRENT_TIMESTAMP WHERE id=?",params![next,context,input["taskBinding"]["taskRevision"].as_i64(),preview]).map_err(|e|e.to_string())?;
    Ok(
        json!({"previewId":preview,"version":next,"contentHash":hash(fields),"fields":fields,"references":p["references"],"retrievalOutcome":p["retrievalOutcome"]}),
    )
}
fn rows(c: &Connection, sql: &str, args: impl rusqlite::Params) -> Result<Vec<Value>, String> {
    c.prepare(sql)
        .and_then(|mut s| {
            s.query_map(args, |r| {
                let raw: String = r.get(0)?;
                Ok(serde_json::from_str(&raw).unwrap_or(Value::Null))
            })?
            .collect()
        })
        .map_err(|e| e.to_string())
}
pub(crate) fn current_findings(c: &Connection, session: &str) -> Result<Value, String> {
    Ok(json!(rows(c,"SELECT json_object('priority',f.priority,'summary',f.summary,'references',json(f.reference_ids_json)) FROM work_preview_findings f JOIN work_preview_investigations i ON i.id=f.investigation_id JOIN work_previews p ON p.id=i.preview_id WHERE p.session_id=? AND i.state='completed' AND i.context_revision=p.context_revision ORDER BY f.rowid",[session])?))
}

pub(crate) fn schedule_investigation(
    db: &Path,
    settings: &Path,
    vault_root: &Path,
    semantic: &SemanticEngine,
    input: &Value,
    p: &Value,
    context: i64,
) -> Result<(), String> {
    let preview = database::open(db)?
        .query_row(
            "SELECT id FROM work_previews WHERE session_id=?",
            [string(input, "sessionId")?],
            |r| r.get::<_, String>(0),
        )
        .map_err(|e| e.to_string())?;
    let intent = validate_planner(&json!({"retrieval":p["optionalInvestigation"]}))?;
    let investigation = start_investigation(
        db,
        &preview,
        context,
        "speculative_search",
        &p["optionalInvestigation"],
    )?;
    if !intent.needed {
        database::open(db)?.execute("UPDATE work_preview_investigations SET state='not_needed',finished_at=CURRENT_TIMESTAMP WHERE id=?",[investigation]).map_err(|e|e.to_string())?;
        return Ok(());
    }
    let (db, settings, vault_root, semantic, input, p) = (
        db.to_owned(),
        settings.to_owned(),
        vault_root.to_owned(),
        semantic.clone(),
        input.clone(),
        p.clone(),
    );
    tokio::spawn(async move {
        let outcome = investigate(
            &db,
            &settings,
            &vault_root,
            &semantic,
            &input,
            &p,
            &investigation,
            context,
            &intent,
        )
        .await;
        if outcome.is_err() {
            if let Ok(c) = database::open(&db) {
                let _=c.execute("UPDATE work_preview_investigations SET state='failed',safe_error='optional_investigation_failed',finished_at=CURRENT_TIMESTAMP WHERE id=? AND state IN ('queued','running')",[investigation]);
            }
        }
    });
    Ok(())
}
async fn investigate(
    db: &Path,
    settings: &Path,
    vault_root: &Path,
    semantic: &SemanticEngine,
    input: &Value,
    p: &Value,
    investigation: &str,
    context: i64,
    intent: &RetrievalIntent,
) -> Result<(), String> {
    if database::open(db)?
        .execute(
            "UPDATE work_preview_investigations SET state='running' WHERE id=? AND state='queued'",
            [investigation],
        )
        .map_err(|e| e.to_string())?
        == 0
    {
        return Ok(());
    }
    let (outcome, refs) = retrieve(db, vault_root, semantic, intent)?;
    let findings = if refs.is_empty() {
        vec![]
    } else {
        let result=assistance::provider_json_with_images(settings,string(input,"modelTask")?,PromptId::WorkPreviewInvestigation,json!({"locale":input["locale"],"session":input["promptContext"],"preview":p["structuredPreview"],"references":refs}),&[],None).await?;
        let findings = list(&result["findings"])?.clone();
        if findings.len() > 16 {
            return Err("invalid findings".into());
        }
        for f in &findings {
            if !matches!(
                f["priority"].as_str(),
                Some("critical" | "supporting" | "ancillary")
            ) || string(f, "summary")?.len() > 2000
                || list(&f["referenceIds"])?.is_empty()
                || list(&f["referenceIds"])?
                    .iter()
                    .any(|id| !refs.iter().any(|r| r["referenceId"] == *id))
            {
                return Err("invalid_reference_binding".into());
            }
        }
        findings
    };
    let mut c = database::open(db)?;
    let tx = database::immediate_transaction(&mut c)?;
    let current:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM work_preview_investigations i JOIN work_previews p ON p.id=i.preview_id JOIN refinement_sessions s ON s.id=p.session_id WHERE i.id=? AND i.state='running' AND p.context_revision=? AND s.current_draft_revision=?)",params![investigation,context,context],|r|r.get(0)).map_err(|e|e.to_string())?;
    let now_context = assistance::refinement_context(&tx, string(input, "sessionId")?)?;
    let conversation_current = now_context["messages"] == input["promptContext"]["messages"]
        && now_context["taskSnapshot"] == input["promptContext"]["taskSnapshot"]
        && head(&tx, string(input, "sessionId")?)? == number(input, "expectedPreviewVersion")? + 1;
    let source_ok = refs
        .iter()
        .map(|r| source_current(&tx, r))
        .collect::<Result<Vec<_>, _>>()?
        .iter()
        .all(|v| *v);
    if !current || !source_ok || !conversation_current {
        tx.execute("UPDATE work_preview_investigations SET state='changed_context',finished_at=CURRENT_TIMESTAMP WHERE id=?",[investigation]).map_err(|e|e.to_string())?;
    } else {
        for f in findings {
            let bindings = list(&f["referenceIds"])?
                .iter()
                .filter_map(|id| refs.iter().find(|r| r["referenceId"] == *id))
                .collect::<Vec<_>>();
            tx.execute("INSERT INTO work_preview_findings(id,investigation_id,priority,summary,reference_ids_json) VALUES(?,?,?,?,?)",params![id(),investigation,string(&f,"priority")?,string(&f,"summary")?,json!(bindings).to_string()]).map_err(|e|e.to_string())?;
        }
        tx.execute("UPDATE work_preview_investigations SET state=?,source_bundle_hash=?,finished_at=CURRENT_TIMESTAMP WHERE id=?",params![if outcome=="no_suitable_result"{"no_suitable_result"}else{"completed"},hash(&json!(refs)),investigation]).map_err(|e|e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())
}
fn version(c: &Connection, preview: &str, v: i64) -> Result<Value, String> {
    let mut result=c.query_row("SELECT fields_json,content_hash,derivation_kind,derived_from_version,task_revision,context_revision FROM work_preview_versions WHERE preview_id=? AND version=?",params![preview,v],|r|Ok(json!({"previewId":preview,"version":v,"fields":serde_json::from_str::<Value>(&r.get::<_,String>(0)?).unwrap_or(Value::Null),"contentHash":r.get::<_,String>(1)?,"derivationKind":r.get::<_,String>(2)?,"derivedFromVersion":r.get::<_,Option<i64>>(3)?,"taskRevision":r.get::<_,Option<i64>>(4)?,"contextRevision":r.get::<_,i64>(5)?}))).map_err(|e|e.to_string())?;
    result["assumptions"] = json!(rows(c, "SELECT json_object('id',id,'text',text,'status',status,'basis',basis,'sourceClaimIds',json(source_claim_ids_json)) FROM work_preview_assumptions WHERE preview_id=? AND version=? ORDER BY rowid", params![preview,v])?);
    result["references"]=json!(rows(c,"SELECT json_set(metadata_json,'$.documentId',document_id,'$.documentVersion',document_version,'$.section',section,'$.role',role,'$.claimIds',json(claim_ids_json)) FROM work_preview_references WHERE preview_id=? AND version=? ORDER BY rowid",params![preview,v])?);
    Ok(result)
}
fn copy_bindings(
    tx: &Transaction<'_>,
    preview: &str,
    source: i64,
    next: i64,
) -> Result<(), String> {
    tx.execute("INSERT INTO work_preview_assumptions(preview_id,version,id,text,status,basis,source_claim_ids_json,supersedes_id) SELECT preview_id,?,id,text,status,basis,source_claim_ids_json,supersedes_id FROM work_preview_assumptions WHERE preview_id=? AND version=?",params![next,preview,source]).map_err(|e|e.to_string())?;
    tx.execute("INSERT INTO work_preview_references(id,preview_id,version,retrieval_request_hash,source_bundle_hash,document_id,document_version,section,chunk_index,metadata_json,role,claim_ids_json) SELECT lower(hex(randomblob(16))),preview_id,?,retrieval_request_hash,source_bundle_hash,document_id,document_version,section,chunk_index,metadata_json,role,claim_ids_json FROM work_preview_references WHERE preview_id=? AND version=?",params![next,preview,source]).map_err(|e|e.to_string())?;
    Ok(())
}
pub(crate) fn execute(
    db: &Path,
    vault_root: &Path,
    name: &str,
    input: &Value,
) -> Result<Value, String> {
    let session = string(input, "sessionId")?;
    let mut c = database::open(db)?;
    assistance::ensure_refinement_subject_visible(&c, session)?;
    let owner = c
        .query_row(
            "SELECT id,current_version,context_revision FROM work_previews WHERE session_id=?",
            [session],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, i64>(2)?,
                ))
            },
        )
        .optional()
        .map_err(|e| e.to_string())?;
    if name == "task-refinement.reference-list" && input["query"].as_str().is_some_and(|q|!q.is_empty()) {
        let intent=validate_planner(&json!({"retrieval":{"needed":true,"reason":"User reference lookup","queries":[input["query"]],"aspects":[],"filters":{},"requery":null}}))?;
        let (_,items)=retrieve(db,vault_root,&SemanticEngine::new(None),&intent)?;
        return Ok(json!({"items":items}));
    }
    if name == "task-refinement.reference-workspace" {
        let mention_draft=mention_draft(&c,session)?;
        let generation=rows(&c,"SELECT json_object('id',id,'status',status,'executionOutcome',execution_outcome,'applicationDisposition',application_disposition,'safeError',CASE WHEN status='failed' THEN 'preview_generation_failed' ELSE '' END) FROM ai_jobs_v2 WHERE task_kind='refinement_preview' AND entity_id=? ORDER BY rowid DESC LIMIT 1",[session])?.into_iter().next();
        let Some((preview, current, context)) = owner else {
            return Ok(
                json!({"mentionDraft":mention_draft,"generation":generation,"preview":null,"references":[],"investigations":[],"findings":[]}),
            );
        };
        let versions=rows(&c,"SELECT json_object('version',version,'contentHash',content_hash,'derivationKind',derivation_kind,'derivedFromVersion',derived_from_version,'createdAt',created_at) FROM work_preview_versions WHERE preview_id=? ORDER BY version",[&preview])?;
        let investigations=rows(&c,"SELECT json_object('id',id,'state',state,'contextRevision',context_revision,'safeError',safe_error) FROM work_preview_investigations WHERE preview_id=? ORDER BY rowid",[&preview])?;
        let interactions = rows(&c, "SELECT DISTINCT json_object('documentId',document_id,'documentVersion',document_version,'section',section,'kind',kind,'contextRevision',context_revision) FROM reference_interactions WHERE preview_id=? ORDER BY rowid", [&preview])?;
        let last_run=rows(&c,"SELECT payload_json FROM refinement_drafts WHERE session_id=? ORDER BY revision DESC LIMIT 1",[session])?.into_iter().next().unwrap_or(Value::Null);
        return Ok(
            json!({"mentionDraft":mention_draft,"generation":generation,"retrievalOutcome":last_run["retrievalOutcome"],"stageTimings":last_run["stageTimings"],"preview":{"id":preview,"currentVersion":current,"contextRevision":context,"versions":versions,"current":version(&c,&preview,current)?},"interactions":interactions,"investigations":investigations,"findings":current_findings(&c,session)?}),
        );
    }
    if name == "task-refinement.reference-list" && owner.is_none(){return Ok(json!({"items":[]}))}
    let (preview, current, context) = owner.ok_or("preview_not_found")?;
    if name == "task-refinement.reference-usage" {
        if !source_current(&c,input)?{return Err("invalid_reference_binding".into())}
        let kind=string(input,"kind")?;if !matches!(kind,"used"|"excluded"){return Err("invalid_reference_interaction".into())}
        let reason=string(input,"reason")?;
        record_use(db,&preview,context,input,kind,reason)?;
        return Ok(json!({"kind":kind,"documentId":input["documentId"]}));
    }
    if name == "task-refinement.reference-list" {
        let selected = input["version"].as_i64().unwrap_or(current);
        let snapshot = version(&c, &preview, selected)?;
        return Ok(json!({"items": snapshot["references"], "orderEpoch": context}));
    }
    if name == "task-refinement.reference-version" {
        return version(&c, &preview, number(input, "version")?);
    }
    if name == "task-refinement.reference-compare" {
        let left = number(input, "left")?;
        let right = number(input, "right")?;
        return Ok(
            json!({"left":version(&c,&preview,left)?,"right":version(&c,&preview,right)?,"comparison":compare(db,&preview,left,right)?}),
        );
    }
    if name == "task-refinement.reference-open" {
        let selected = input["version"].as_i64().unwrap_or(current);
        let v = version(&c, &preview, selected)?;
        let mut available = list(&v["references"])? .clone();
        let historical_findings = rows(&c,
            "SELECT f.reference_ids_json FROM work_preview_findings f JOIN work_preview_investigations i ON i.id=f.investigation_id WHERE i.preview_id=? AND i.context_revision=?",
            params![preview, v["contextRevision"].as_i64()],
        )?;
        for references in historical_findings {
            available.extend(list(&references)?.iter().cloned());
        }
        let r = available.iter().find(|r| {
            r["documentId"] == input["documentId"]
                && r["documentVersion"] == input["documentVersion"]
                && r["section"] == input["section"]
        }).ok_or("invalid_reference_binding")?;
        // A move changes the path, not the identity or exact source revision.
        // Resolve only through the current index; never trust a caller path.
        let path: Option<String> = c.query_row(
            "SELECT u.path FROM vault_search_units u JOIN vault_documents d ON d.path=u.path AND d.source_hash=u.source_revision WHERE u.document_id=? AND u.source_revision=? AND u.section=? LIMIT 1",
            params![string(r,"documentId")?, string(r,"documentVersion")?, r["section"].as_str().unwrap_or("")],
            |row| row.get(0),
        ).optional().map_err(|error| error.to_string())?;
        let path = path.ok_or("reference_version_unavailable")?;
        let exact = vault::read(vault_root, &path, "en")?;
        if exact["source_hash"] != r["documentVersion"] {
            return Err("reference_version_unavailable".into());
        }
        record_use(db, &preview, context, r, "viewed", "")?;
        return Ok(json!({"reference":r,"source":exact}));
    }
    let tx = database::immediate_transaction(&mut c)?;
    assistance::ensure_refinement_subject_visible(&tx, session)?;
    let mut operation = input.clone();
    operation["operationName"] = json!(name);
    if let Some(result) = assistance::operation_replay(&tx, &operation)? {
        return Ok(result);
    }
    let actual = head(&tx, session)?;
    if actual != number(input, "expectedCurrentPreviewVersion")? {
        return Err("preview_head_conflict".into());
    }
    let selected = number(input, "version")?;
    let stored = version(&tx, &preview, selected)?;
    let result = match name {
        "task-refinement.reference-restore" | "task-refinement.reference-edit" => {
            let edit = name.ends_with("-edit");
            if edit && (selected != actual || stored["contentHash"] != input["expectedContentHash"])
            {
                return Err("content_hash_conflict".into());
            }
            let fields = if edit {
                input["fields"].clone()
            } else {
                stored["fields"].clone()
            };
            if edit {
                let mut check = fields.clone();
                check["assumptions"] = json!([]);
                validate_output(
                    &json!({"preview":check,"claimSources":[],"optionalInvestigation":{"needed":false,"reason":"User edit","queries":[],"aspects":[],"filters":{},"requery":null}}),
                )?;
                if fields["assumptions"] != stored["fields"]["assumptions"] {
                    return Err("invalid_input: assumption bindings must be preserved".into());
                }
            }
            let next = actual + 1;
            tx.execute("INSERT INTO work_preview_versions(preview_id,version,derivation_kind,derived_from_version,generation_job_id,task_revision,context_revision,locale,fields_json,content_hash,prompt_id,prompt_version) SELECT preview_id,?,?,version,generation_job_id,task_revision,context_revision,locale,?,?,prompt_id,prompt_version FROM work_preview_versions WHERE preview_id=? AND version=?",params![next,if edit{"edited"}else{"restored"},fields.to_string(),hash(&fields),preview,selected]).map_err(|e|e.to_string())?;
            copy_bindings(&tx, &preview, selected, next)?;
            tx.execute("UPDATE work_previews SET current_version=?,updated_at=CURRENT_TIMESTAMP WHERE id=?",params![next,preview]).map_err(|e|e.to_string())?;
            json!({"previewId":preview,"version":next,"derivedFromVersion":selected,"contentHash":hash(&fields),"transitionCause":if edit{Value::Null}else{json!("version_restore")}})
        }
        "task-refinement.reference-apply" => {
            if selected != actual {
                return Err("preview_head_conflict".into());
            }
            if stored["contentHash"] != input["expectedContentHash"] {
                return Err("content_hash_conflict".into());
            }
            if let Some(hashes) = input["editedFieldHashes"].as_object() {
                for (k, v) in hashes {
                    if hash(&stored["fields"][k]) != v.as_str().unwrap_or("") {
                        return Err("content_hash_conflict".into());
                    }
                }
            }
            let (capture, mut task): (Option<String>, Option<String>) = tx
                .query_row(
                    "SELECT capture_id,task_id FROM refinement_sessions WHERE id=?",
                    [session],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .map_err(|e| e.to_string())?;
            task = assistance::refinement_context(&tx, session)?["taskId"]
                .as_str()
                .map(str::to_owned)
                .or(task);
            let expected = input["expectedTaskRevision"].as_i64();
            if task.is_some() && (expected.is_none() || expected != stored["taskRevision"].as_i64())
            {
                return Err("task_revision_conflict".into());
            }
            let latest:Option<String>=tx.query_row("SELECT id FROM task_assistance_jobs WHERE kind='refinement_response' AND subject_id=? ORDER BY rowid DESC LIMIT 1",[session],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
            let origin:Option<String>=tx.query_row("SELECT json_extract(j.input_json,'$.responseJobId') FROM work_preview_versions v LEFT JOIN ai_jobs_v2 j ON j.id=v.generation_job_id WHERE v.preview_id=? AND v.version=?",params![preview,actual],|r|r.get(0)).map_err(|e|e.to_string())?;
            if latest != origin {
                return Err("context_revision_conflict".into());
            }
            let session_context = assistance::refinement_context(&tx, session)?;
            if session_context["draftRevision"] != stored["contextRevision"] {
                return Err("context_revision_conflict".into());
            }
            if let Some(task_id) = task.as_deref() {
                let expected_hierarchy:Option<String>=tx.query_row("SELECT json_extract(j.input_json,'$.hierarchyContextHash') FROM work_preview_versions v JOIN ai_jobs_v2 j ON j.id=v.generation_job_id WHERE v.preview_id=? AND v.version=?",params![preview,actual],|r|r.get(0)).optional().map_err(|e|e.to_string())?.flatten();
                if expected_hierarchy.is_some_and(|h| {
                    super::super::task_hierarchy::context_hash(&tx, task_id)
                        .ok()
                        .as_ref()
                        != Some(&h)
                }) {
                    return Err("context_revision_conflict".into());
                }
            }
            for reference in list(&stored["references"])? {
                if !source_current(&tx, reference)?
                    || vault::read(vault_root, string(reference, "path")?, "en")
                        .ok()
                        .is_none_or(|v| v["source_hash"] != reference["documentVersion"])
                {
                    return Err("reference_version_unavailable".into());
                }
            }
            let patch = task_fields(&stored["fields"]);
            let payload = if task.is_some() {
                json!({"patch":patch,"expectedTaskRevision":expected})
            } else {
                patch
            };
            let result = assistance::apply_proposal_tx(
                &tx,
                if task.is_some() {
                    "task_patch"
                } else {
                    "new_task"
                },
                &payload,
                capture.as_deref(),
                task.as_deref(),
                &chrono::Utc::now().to_rfc3339(),
            )?;
            // Capture sessions retain their source identity; the existing decision record
            // resolves their promoted Task on subsequent refinement turns.
            tx.execute("INSERT INTO refinement_proposal_decisions(id,session_id,draft_revision,proposal_id,decision,result_json,operation_id) VALUES(?,?,?,'work-preview','apply',?,?)",params![id(),session,context,result.to_string(),string(input,"operationId")?]).map_err(|e|e.to_string())?;
            super::super::task_hierarchy::mark_refined(
                &tx,
                string(&result, "id")?,
                result["taskRevision"]
                    .as_i64()
                    .ok_or("invalid task revision")?,
            )?;
            crate::adapters::sqlite::task_repository::sync_linked_sessions_tx(
                &tx,
                string(input, "operationId")?,
                "task.refinement",
                None,
                &chrono::Utc::now().to_rfc3339(),
            )?;
            tx.execute("INSERT INTO reference_interactions(id,preview_id,context_revision,document_id,document_version,section,kind,use_scope,reason,preview_version) SELECT lower(hex(randomblob(16))),preview_id,?,document_id,document_version,section,'adopted',claim_ids_json,'Explicit preview apply',version FROM work_preview_references WHERE preview_id=? AND version=? AND claim_ids_json!='[]'",params![context,preview,actual]).map_err(|e|e.to_string())?;
            tx.execute("UPDATE ai_jobs_v2 SET application_disposition='applied' WHERE id=(SELECT generation_job_id FROM work_preview_versions WHERE preview_id=? AND version=?)",params![preview,actual]).map_err(|e|e.to_string())?;
            crate::adapters::sqlite::task_repository::record_activity_tx(
                &tx,
                "task",
                string(&result, "id")?,
                "refinement",
                string(input, "operationId")?,
                &chrono::Utc::now().to_rfc3339(),
            )?;
            json!({"task":result,"transitionCause":"preview_adopted","version":actual})
        }
        _ => return Err("unsupported reference operation".into()),
    };
    assistance::record_operation(&tx, &operation, &result)?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(result)
}

pub(crate) fn reply_findings(c: &Connection, session: &str) -> Result<Value, String> {
    Ok(json!(rows(c,"SELECT json_object('id',f.id,'priority',f.priority,'summary',f.summary,'references',json(f.reference_ids_json)) FROM work_preview_findings f JOIN work_preview_investigations i ON i.id=f.investigation_id JOIN work_previews p ON p.id=i.preview_id WHERE p.session_id=? AND i.state='completed' AND i.context_revision=p.context_revision AND f.delivered_in_message_id IS NULL AND f.priority IN ('critical','supporting') ORDER BY f.rowid",[session])?))
}
pub(crate) fn deliver_findings_tx(
    tx: &Transaction<'_>,
    session: &str,
    supplied: &Value,
    used: &Value,
    message: &str,
) -> Result<(), String> {
    let used = list(used)?;
    let supplied = list(supplied)?;
    if used
        .iter()
        .any(|id| !supplied.iter().any(|f| f["id"] == *id))
    {
        return Err("invalid_reference_binding".into());
    }
    let owner: Option<(String, i64)> = tx
        .query_row(
            "SELECT id,context_revision FROM work_previews WHERE session_id=?",
            [session],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    for f in supplied {
        let changed=tx.execute("UPDATE work_preview_findings SET delivered_in_message_id=? WHERE id=? AND delivered_in_message_id IS NULL",params![message,string(f,"id")?]).map_err(|e|e.to_string())?;
        if changed == 0 || !used.contains(&f["id"]) {
            continue;
        }
        if let Some((preview, context)) = &owner {
            for r in list(&f["references"])? {
                if !source_current(tx, r)? {
                    return Err("reference_version_unavailable".into());
                }
                tx.execute("INSERT INTO reference_interactions(id,preview_id,context_revision,document_id,document_version,section,kind,use_scope,reason,message_id) VALUES(?,?,?,?,?,?,'used','reply',?,?)",params![id(),preview,context,string(r,"documentId")?,string(r,"documentVersion")?,r["section"].as_str().unwrap_or(""),string(f,"summary")?,message]).map_err(|e|e.to_string())?;
            }
        }
    }
    Ok(())
}

pub(crate) fn mention_draft(c:&Connection,session:&str)->Result<Value,String>{
    Ok(rows(c,"SELECT result_json FROM task_assistance_operations WHERE json_extract(result_json,'$.kind')='mention_draft' AND json_extract(result_json,'$.sessionId')=? ORDER BY rowid DESC LIMIT 1",[session])?.into_iter().next().unwrap_or_else(||json!({"text":"","mentions":[]})))
}
fn validate_mentions(c:&Connection,mentions:&Value)->Result<(),String>{
    for mention in list(mentions)? {string(mention,"mentionId")?;string(mention,"displayLabel")?;if !source_current(c,mention)?{return Err("invalid_reference_binding".into())}}
    Ok(())
}
pub(crate) fn save_mention_draft(db:&Path,input:&Value)->Result<Value,String>{
    let mut c=database::open(db)?;let tx=database::immediate_transaction(&mut c)?;
    if let Some(result)=assistance::operation_replay(&tx,input)? {return Ok(result)}
    let session=string(input,"sessionId")?;assistance::ensure_refinement_subject_visible(&tx,session)?;
    let revision:i64=tx.query_row("SELECT current_draft_revision FROM refinement_sessions WHERE id=?",[session],|r|r.get(0)).map_err(|e|e.to_string())?;
    if input["expectedDraftRevision"].as_i64().is_some_and(|v|v!=revision){return Err("draft_conflict".into())}
    validate_mentions(&tx,&input["mentions"])?;
    let result=json!({"kind":"mention_draft","sessionId":session,"text":input["text"].as_str().unwrap_or(""),"mentions":input["mentions"]});
    assistance::record_operation(&tx,input,&result)?;tx.commit().map_err(|e|e.to_string())?;Ok(result)
}
pub(crate) fn bind_sent_mentions_tx(tx:&Transaction<'_>,session:&str,mentions:&Value)->Result<(),String>{
    validate_mentions(tx,mentions)?;
    for mention in list(mentions)?{
        tx.execute("INSERT INTO refinement_document_mentions(id,session_id,draft_revision,document_id,document_version,section,display_label,range_anchor_json) SELECT ?,id,current_draft_revision,?,?,?,?,? FROM refinement_sessions WHERE id=?",params![id(),string(mention,"documentId")?,string(mention,"documentVersion")?,mention["section"].as_str().unwrap_or(""),string(mention,"displayLabel")?,mention.to_string(),session]).map_err(|e|e.to_string())?;
        tx.execute("INSERT INTO reference_interactions(id,preview_id,context_revision,document_id,document_version,section,kind) SELECT ?,id,context_revision,?,?,?,'mentioned' FROM work_previews WHERE session_id=?",params![id(),string(mention,"documentId")?,string(mention,"documentVersion")?,mention["section"].as_str().unwrap_or(""),session]).map_err(|e|e.to_string())?;
    }
    let clear=json!({"kind":"mention_draft","sessionId":session,"text":"","mentions":[]});
    tx.execute("INSERT INTO task_assistance_operations(operation_id,payload_hash,result_json) VALUES(?,'sent-mention-draft',?)",params![id(),clear.to_string()]).map_err(|e|e.to_string())?;
    Ok(())
}


pub(crate) fn sent_mention_references(c:&Connection,session:&str)->Result<Value,String>{
    Ok(json!(rows(c,"SELECT DISTINCT json_object('documentId',u.document_id,'documentVersion',u.source_revision,'section',u.section,'path',u.path,'title',d.title,'excerpt',substr(u.text,1,500),'aspect',u.aspect,'status',u.status,'retrievalRequestHash','explicit_mention') FROM refinement_document_mentions m JOIN vault_search_units u ON u.document_id=m.document_id AND u.source_revision=m.document_version AND u.section=m.section JOIN vault_documents d ON d.path=u.path AND d.source_hash=u.source_revision WHERE m.session_id=? ORDER BY m.rowid DESC LIMIT 16",[session])?))
}
