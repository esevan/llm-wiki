//! Review exact archive bytes, journal every filesystem step, and advance pointers
//! only after the same bytes have a structural search receipt.
use crate::domain::knowledge_archive::{Artifact, Organization};
use crate::domain::retrieval::{parse_document_at_revision, MAX_CONTEXT_TOKENS, MAX_PASSAGES};
use crate::native::{
    database, knowledge_distillation, retrieval_index, semantic::SemanticEngine, task_assistance,
    vault,
};
use crate::workflow_foundation::PromptId;
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Write,
    path::{Component, Path, PathBuf},
};
use walkdir::WalkDir;

type Result<T> = std::result::Result<T, String>;
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn uid() -> String {
    uuid::Uuid::new_v4().to_string()
}
fn required<'a>(v: &'a Value, key: &str) -> Result<&'a str> {
    v[key]
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| format!("{key} is required"))
}
fn integer(v: &Value, key: &str) -> Result<i64> {
    v[key].as_i64().ok_or_else(|| format!("{key} is required"))
}
fn parse(text: &str) -> Result<Value> {
    serde_json::from_str(text).map_err(|e| e.to_string())
}
fn db_err(e: rusqlite::Error) -> String {
    e.to_string()
}
fn bytes_at(root: &Path, relative: &str) -> Result<Option<Vec<u8>>> {
    let path = safe_path(root, relative)?;
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}
/// Check every existing ancestor, not just the final extension.
fn safe_path(root: &Path, relative: &str) -> Result<PathBuf> {
    let path = Path::new(relative);
    if relative.contains(['[', ']', '#', '?'])
        || relative.contains('\\')
        || relative.contains('\0')
        || path.is_absolute()
        || path.extension().and_then(|s| s.to_str()) != Some("md")
        || path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
        || relative
            .split('/')
            .any(|s| s.starts_with('.') || s.is_empty())
    {
        return Err("archive_path_invalid".into());
    }
    let mut current = root.canonicalize().map_err(|e| e.to_string())?;
    for part in path.components() {
        current.push(part.as_os_str());
        match fs::symlink_metadata(&current) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err("archive_path_invalid: symlink".into())
            }
            Ok(meta) if !meta.is_file() && !meta.is_dir() => {
                return Err("archive_path_invalid: non-file".into())
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(current)
}
fn artifact(
    root: &Path,
    path: String,
    document_id: String,
    kind: &str,
    bytes: Option<String>,
) -> Result<Artifact> {
    let old = bytes_at(root, &path)?;
    Ok(Artifact {
        document_id,
        path,
        kind: kind.into(),
        sha256: bytes.as_ref().map(|b| hash(b.as_bytes())),
        bytes,
        expected_hash: old.map(|b| hash(&b)),
        idea_id: None,
        idea_revision: None,
    })
}
fn inventory(root: &Path) -> Result<Vec<Value>> {
    let mut result = Vec::new();
    let mut paths = BTreeSet::new();
    let mut document_ids = BTreeSet::new();
    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| e.depth() == 0 || !e.file_name().to_string_lossy().starts_with('.'))
    {
        let entry = entry.map_err(|e| e.to_string())?;
        if !entry.file_type().is_file()
            || entry.path().extension().and_then(|s| s.to_str()) != Some("md")
        {
            continue;
        }
        let path = entry
            .path()
            .strip_prefix(root)
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        if path.split('/').any(|p| p == "Translations") {
            continue;
        }
        safe_path(root, &path)?;
        if !paths.insert(path.to_lowercase()) {
            return Err("archive_path_invalid: case collision".into());
        }
        let body = fs::read_to_string(entry.path()).map_err(|e| e.to_string())?;
        let parsed = parse_document_at_revision(&path, &body, hash(body.as_bytes()));
        let document_id = parsed.document_id;
        if !document_ids.insert(document_id.clone()) {
            return Err("archive_target_conflict: duplicate document identity".into());
        }
        let metadata = frontmatter(&body);
        result.push(json!({"path":path,"documentId":document_id,"revision":hash(body.as_bytes()),"title":body.lines().find_map(|l|l.strip_prefix("# ")).unwrap_or(""),"tags":metadata["tags"],"aliases":metadata["aliases"],"moc":metadata["moc"],"excerpt":body.chars().take(600).collect::<String>()}));
    }
    result.sort_by_key(|v| v["path"].as_str().unwrap_or("").to_owned());
    Ok(result)
}
fn frontmatter(body: &str) -> Value {
    body.strip_prefix("---\n")
        .and_then(|v| v.split_once("\n---"))
        .and_then(|(head, _)| serde_yaml::from_str(head).ok())
        .unwrap_or(json!({}))
}
fn markdown(metadata: &Value, body: &str) -> Result<String> {
    Ok(format!(
        "---\n{}---\n\n{}\n",
        serde_yaml::to_string(metadata).map_err(|e| e.to_string())?,
        body.trim()
    ))
}
fn snapshot(tx: &Transaction<'_>, input: &Value) -> Result<Value> {
    let task = required(input, "taskId")?;
    let revision = integer(input, "knowledgeRevision")?;
    let projection = knowledge_distillation::review_projection(tx, task, Some(revision))?;
    let version = projection["versions"]
        .as_array()
        .and_then(|vs| vs.iter().find(|v| v["revision"] == revision))
        .ok_or("knowledge_revision_not_found")?;
    if projection["pointers"]["currentPrivateRevision"] != revision
        || version["contentHash"] != input["expectedKnowledgeContentHash"]
        || version["generationSnapshotHash"] != input["expectedGenerationSnapshotHash"]
        || version["freshness"] != "current"
    {
        return Err("generation_snapshot_conflict".into());
    }
    let mut ideas = Vec::new();
    for selection in input["selectedIdeaRevisionIds"]
        .as_array()
        .into_iter()
        .flatten()
    {
        let idea = projection["ideas"]
            .as_array()
            .and_then(|vs| {
                vs.iter().find(|v| {
                    v["id"] == selection["id"]
                        && v["revision"] == selection["revision"]
                        && v["knowledgeRevision"] == revision
                })
            })
            .ok_or("idea_revision_not_found")?;
        ideas.push(idea.clone());
    }
    let sources:String=tx.query_row("SELECT s.manifest_json FROM knowledge_draft_versions v JOIN knowledge_evidence_snapshots s ON s.id=v.generation_snapshot_id WHERE v.task_id=? AND v.revision=?",params![task,revision],|r|r.get(0)).map_err(db_err)?;
    Ok(
        json!({"version":version,"ideas":ideas,"sources":parse(&sources)?,"pointers":projection["pointers"]}),
    )
}
fn proposal_replay(c: &Connection, input: &Value) -> Result<Option<Value>> {
    let row: Option<(String, String)> = c
        .query_row(
            "SELECT request_hash,payload_json FROM knowledge_archive_proposals WHERE request_id=?",
            [required(input, "operationId")?],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .map_err(db_err)?;
    row.map(|(h, p)| {
        if h == hash(input.to_string().as_bytes()) {
            parse(&p)
        } else {
            Err("operation_conflict".into())
        }
    })
    .transpose()
}
fn bounded_overlap_context(mut search: Value) -> Value {
    let candidates = search["results"]
        .take()
        .as_array()
        .cloned()
        .unwrap_or_default();
    let candidate_count = candidates.len();
    let mut estimated_tokens = 0_usize;
    let mut results = Vec::new();
    for mut candidate in candidates.into_iter().take(MAX_PASSAGES) {
        let passage = candidate["snippet"].as_str().unwrap_or_default();
        let candidate_tokens = passage.chars().count().div_ceil(4);
        if estimated_tokens + candidate_tokens > MAX_CONTEXT_TOKENS {
            break;
        }
        estimated_tokens += candidate_tokens;
        if let Some(object) = candidate.as_object_mut() {
            // Search reads retain the complete document for viewers. The archive
            // proposal needs the exact ranked passage and source identity only.
            object.remove("body");
        }
        results.push(candidate);
    }
    let truncated = results.len() < candidate_count;
    search["results"] = json!(results);
    search["retrievedContextEstimatedTokens"] = json!(estimated_tokens);
    if truncated {
        search["has_more"] = json!(true);
    }
    search
}
pub async fn prepare(
    db: &Path,
    settings: &Path,
    root: &Path,
    semantic: &SemanticEngine,
    input: &Value,
) -> Result<Value> {
    let evidence = {
        let mut c = database::open(db)?;
        if let Some(replay) = proposal_replay(&c, input)? {
            return Ok(replay);
        }
        let tx = c.transaction().map_err(db_err)?;
        let evidence = snapshot(&tx, input)?;
        tx.commit().map_err(db_err)?;
        evidence
    };
    let taxonomy = inventory(root)?;
    let taxonomy_prompt = taxonomy
        .iter()
        .cloned()
        .map(|mut note| {
            note.as_object_mut().unwrap().remove("excerpt");
            note
        })
        .collect::<Vec<_>>();
    let overlap = bounded_overlap_context(vault::search(
        db,
        semantic,
        evidence["version"]["title"].as_str().unwrap_or(""),
        MAX_PASSAGES,
        0,
        true,
    )?);
    let plan = task_assistance::provider_json(
        settings,
        "knowledge_draft",
        PromptId::KnowledgeArchiveProposal,
        json!({"knowledge":evidence["version"],"selectedIdeas":evidence["ideas"],"taxonomy":taxonomy_prompt,"overlapCandidates":overlap,"locale":input["locale"]}),
        None,
    )
    .await?;
    prepare_from_plan(db, root, input, &evidence, &taxonomy, plan)
}
fn reference_links(c: &Connection, evidence: &Value, taxonomy: &[Value]) -> Result<Vec<Value>> {
    // Only retained actual-use bindings qualify. Clicking or discovering a note is insufficient.
    let sources = &evidence["sources"];
    let refs = sources["actualReferenceUsages"]
        .as_array()
        .cloned()
        .unwrap_or_else(|| {
            sources
                .as_array()
                .into_iter()
                .flatten()
                .filter(|v| v["type"] == "reference")
                .map(|v| v["content"].clone())
                .collect()
        });
    let mut links = BTreeMap::new();
    for r in refs {
        let Some(id) = r["documentId"].as_str() else {
            continue;
        };
        let revision = r["documentVersion"].as_str().unwrap_or("");
        let role = r["disposition"].as_str().unwrap_or("");
        if !matches!(role, "used" | "adopted" | "counterevidence") {
            continue;
        }
        let reason = r["reason"]
            .as_str()
            .or_else(|| r["metadata"]["rationale"].as_str())
            .or_else(|| r["metadata"]["reason"].as_str())
            .unwrap_or("");
        if reason.trim().is_empty() || revision.is_empty() {
            return Err(format!(
                "archive_reference_unresolved: {id} needs its exact revision and use rationale"
            ));
        }
        let path=taxonomy.iter().find(|v|v["documentId"]==id).and_then(|v|v["path"].as_str()).map(str::to_owned).or_else(||c.query_row("SELECT path FROM knowledge_archive_revisions WHERE document_id=? AND revision=?",params![id,revision],|r|r.get::<_,String>(0)).ok());
        let Some(path) = path else {
            return Err(format!("archive_reference_unresolved: locate retained source {id} before preparing publication"));
        };
        links.insert(format!("{id}:{revision}:{}",r["section"]),json!({"documentId":id,"documentVersion":revision,"section":r["section"],"path":path,"role":if role=="counterevidence"{"counterevidence"}else{"support"},"rationale":reason}));
    }
    Ok(links.into_values().collect())
}
fn links_markdown(links: &[Value]) -> String {
    if links.is_empty() {
        return String::new();
    }
    let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for link in links {
        let path = link["path"].as_str().unwrap_or("").trim_end_matches(".md");
        let section = link["section"].as_str().unwrap_or("");
        let line = format!(
            "- [[{path}{}]] — {} ({}; revision `{}`)",
            if section.is_empty() {
                String::new()
            } else {
                format!("#{section}")
            },
            link["rationale"].as_str().unwrap_or(""),
            link["role"].as_str().unwrap_or("support"),
            link["documentVersion"].as_str().unwrap_or("")
        );
        groups
            .entry(link["documentId"].as_str().unwrap_or("").into())
            .or_default()
            .push(line);
    }
    let mut text = "\n\n## References\n".to_owned();
    for (id, lines) in groups {
        text.push_str(&format!(
            "\n<!-- llm-wiki:{id}:start -->\n{}\n<!-- llm-wiki:{id}:end -->\n",
            lines.join("\n")
        ));
    }
    text
}
fn managed_patch(body: &str, document_id: &str, entry: Option<&str>) -> Result<String> {
    let start = format!("<!-- llm-wiki:{document_id}:start -->");
    let end = format!("<!-- llm-wiki:{document_id}:end -->");
    let counts = (body.matches(&start).count(), body.matches(&end).count());
    let replacement = entry
        .map(|v| format!("{start}\n{v}\n{end}"))
        .unwrap_or_default();
    match counts {
        (0, 0) => Ok(if replacement.is_empty() {
            body.to_owned()
        } else {
            format!("{}\n\n{replacement}\n", body.trim_end())
        }),
        (1, 1) => {
            let a = body.find(&start).unwrap();
            let b = body.find(&end).unwrap() + end.len();
            if a >= b {
                return Err("archive_managed_region_conflict".into());
            }
            Ok(format!("{}{}{}", &body[..a], replacement, &body[b..]))
        }
        _ => Err("archive_managed_region_conflict".into()),
    }
}
fn prepare_from_plan(
    db: &Path,
    root: &Path,
    input: &Value,
    evidence: &Value,
    taxonomy: &[Value],
    plan: Value,
) -> Result<Value> {
    let plan: Organization =
        serde_json::from_value(plan).map_err(|e| format!("invalid_archive_plan: {e}"))?;
    if !plan.validate() {
        return Err("invalid_archive_plan".into());
    }
    if plan.tags.len() > 24 || plan.aliases.len() > 24 || plan.moc_paths.len() > 8 {
        return Err("invalid_archive_plan: excessive taxonomy".into());
    }
    safe_path(root, &plan.path)?;
    let directory = Path::new(&plan.path)
        .parent()
        .unwrap()
        .to_string_lossy()
        .to_string();
    if !taxonomy.is_empty()
        && !taxonomy.iter().any(|v| {
            Path::new(v["path"].as_str().unwrap_or("")).parent() == Some(Path::new(&directory))
        })
        && plan
            .new_category_rationale
            .as_deref()
            .unwrap_or("")
            .trim()
            .is_empty()
    {
        return Err("archive_taxonomy_conflict: new category needs rationale".into());
    }
    let target = plan
        .target_path
        .as_ref()
        .and_then(|path| taxonomy.iter().find(|v| v["path"] == *path));
    if matches!(plan.outcome.as_str(), "update" | "merge" | "supersede") && target.is_none() {
        return Err("archive_target_conflict".into());
    }
    if matches!(plan.outcome.as_str(), "update" | "merge") && target.unwrap()["path"] != plan.path {
        return Err("archive_target_conflict: update must preserve canonical path".into());
    }
    if matches!(plan.outcome.as_str(), "new" | "supersede")
        && taxonomy.iter().any(|v| {
            v["path"]
                .as_str()
                .unwrap_or("")
                .eq_ignore_ascii_case(&plan.path)
        })
    {
        return Err("archive_target_conflict".into());
    }
    let document_id = if matches!(plan.outcome.as_str(), "update" | "merge") {
        target.unwrap()["documentId"].as_str().unwrap().to_owned()
    } else {
        uid()
    };
    let c = database::open(db)?;
    let links = reference_links(&c, evidence, taxonomy)?;
    let mut artifacts = Vec::new();
    let mut moc_patches = Vec::new();
    if plan.outcome != "conflict" {
        let version = &evidence["version"];
        let mut decisions = Vec::new();
        for outcome in version["result"]["article"]["finalOutcomes"]
            .as_array()
            .into_iter()
            .flatten()
        {
            decisions.push(json!({"id":format!("{}:{}",document_id,outcome["topicKey"].as_str().unwrap_or("decision")),"topic":outcome["topicKey"],"status":"adopted","confirmed_final":true,"conditions":version["applicability"]["conditions"].as_array().cloned().unwrap_or_default()}));
        }
        let metadata = json!({"title":version["title"],"tags":plan.tags,"aliases":plan.aliases,"llm_wiki":{"schema":1,"document_id":document_id,"information_type":"knowledge","status":"current","task_id":input["taskId"],"source_revision":version["contentHash"],"applicability":{"summary":version["applicability"]["summary"],"representative_questions":version["applicability"]["representativeQuestions"],"helps_with":version["applicability"]["helpsWith"],"conditions":version["applicability"]["conditions"],"exclusions":version["applicability"]["exclusions"]},"decisions":decisions,"references":links}});
        let mut metadata = metadata;
        if let Some(previous) = bytes_at(root, &plan.path)? {
            let previous = String::from_utf8(previous).map_err(|e| e.to_string())?;
            let mut retained = frontmatter(&previous);
            if !retained.is_object() {
                retained = json!({});
            }
            for key in ["tags", "aliases"] {
                let mut values = retained[key].as_array().cloned().unwrap_or_default();
                for value in metadata[key].as_array().into_iter().flatten() {
                    if !values.contains(value) {
                        values.push(value.clone());
                    }
                }
                metadata[key] = json!(values);
            }
            let mut retained_wiki = retained["llm_wiki"].as_object().cloned().unwrap_or_default();
            if plan.outcome == "merge" {
                let mut combined = metadata["llm_wiki"]["decisions"].as_array().cloned().unwrap_or_default();
                for old in retained_wiki.get("decisions").and_then(Value::as_array).into_iter().flatten() {
                    if !combined.iter().any(|new| new["topic"] == old["topic"]) {
                        combined.push(old.clone());
                    }
                }
                metadata["llm_wiki"]["decisions"] = json!(combined);
            }
            for (key, value) in metadata["llm_wiki"].as_object().unwrap() {
                retained_wiki.insert(key.clone(), value.clone());
            }
            metadata["llm_wiki"] = json!(retained_wiki);
            for (key, value) in metadata.as_object().unwrap() {
                retained[key] = value.clone();
            }
            metadata = retained;
        }
        let mut body = required(version, "bodyMarkdown")?.to_owned();
        if plan.outcome == "merge" {
            let old =
                String::from_utf8(bytes_at(root, &plan.path)?.ok_or("archive_target_conflict")?)
                    .map_err(|e| e.to_string())?;
            let old_body = old
                .strip_prefix("---\n")
                .and_then(|v| v.split_once("\n---"))
                .map(|(_, v)| v)
                .unwrap_or(&old);
            body = format!("{}\n\n## Reviewed addition\n\n{body}", old_body.trim());
        }
        body.push_str(&links_markdown(&links));
        artifacts.push(artifact(
            root,
            plan.path.clone(),
            document_id.clone(),
            "knowledge",
            Some(markdown(&metadata, &body)?),
        )?);
        for idea in evidence["ideas"].as_array().into_iter().flatten() {
            let id = required(idea, "id")?;
            let idea_revision = integer(idea, "revision")?;
            let path = format!(
                "idea/{}.md",
                hash(format!("{}:{id}", input["taskId"]).as_bytes())
            );
            let idea_doc = format!("idea:{id}");
            let metadata = json!({"llm_wiki":{"schema":1,"document_id":idea_doc,"information_type":"idea","status":if idea["disposition"]=="out_of_scope"{"deferred"}else{idea["disposition"].as_str().unwrap_or("unverified")},"disposition":idea["disposition"],"task_id":input["taskId"],"reconsideration_conditions":idea["reconsiderationConditions"],"sources":idea["sourceRefs"]}});
            let body = format!(
                "# {}\n\n{}\n\n## Revisit when\n{}",
                required(idea, "title")?,
                required(idea, "bodyMarkdown")?,
                idea["reconsiderationConditions"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .map(|s| format!("- {s}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            );
            let body = managed_patch(&body, &document_id, Some(&format!("[[{}]]", plan.path.trim_end_matches(".md"))))?;
            let mut item = artifact(
                root,
                path,
                idea_doc,
                "idea",
                Some(markdown(&metadata, &body)?),
            )?;
            item.idea_id = Some(id.into());
            item.idea_revision = Some(idea_revision);
            artifacts.push(item);
        }
        if plan.outcome == "supersede" {
            let old = target.unwrap();
            let path = required(old, "path")?;
            let text = String::from_utf8(bytes_at(root, path)?.ok_or("archive_target_conflict")?)
                .map_err(|e| e.to_string())?;
            let mut metadata = frontmatter(&text);
            if !metadata.is_object() {
                metadata = json!({});
            }
            if !metadata["llm_wiki"].is_object() {
                metadata["llm_wiki"] = json!({});
            }
            metadata["llm_wiki"]["schema"] = json!(1);
            metadata["llm_wiki"]["document_id"] = old["documentId"].clone();
            let successor_topics: BTreeSet<&str> = version["result"]["article"]["finalOutcomes"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|o| o["topicKey"].as_str())
                .collect();
            let mut replaced = 0;
            let mut retained_current = false;
            if let Some(decisions) = metadata["llm_wiki"]["decisions"].as_array_mut() {
                for decision in decisions {
                    if !successor_topics.contains(decision["topic"].as_str().unwrap_or("")) {
                        retained_current |= decision["status"] == "adopted";
                        continue;
                    }
                    replaced += 1;
                    decision["status"] = json!("superseded");
                    decision["confirmed_final"] = json!(false);
                    decision["successor_id"] = json!(format!(
                        "{}:{}",
                        document_id,
                        decision["topic"].as_str().unwrap_or("decision")
                    ));
                    decision["reason"] = json!(plan.rationale);
                }
            }
            if replaced == 0 {
                return Err(
                    "archive_target_conflict: supersede needs a matching confirmed decision topic"
                        .into(),
                );
            }
            if !retained_current {
                metadata["llm_wiki"]["status"] = json!("historical");
            }
            let body = text
                .strip_prefix("---\n")
                .and_then(|v| v.split_once("\n---"))
                .map(|(_, v)| v)
                .unwrap_or(&text);
            artifacts.push(artifact(
                root,
                path.into(),
                required(old, "documentId")?.into(),
                "knowledge",
                Some(markdown(&metadata, body)?),
            )?);
        }
        for path in &plan.moc_paths {
            let existing = bytes_at(root, path)?
                .map(String::from_utf8)
                .transpose()
                .map_err(|e| e.to_string())?;
            let id = taxonomy
                .iter()
                .find(|v| v["path"] == *path)
                .and_then(|v| v["documentId"].as_str())
                .map(str::to_owned)
                .unwrap_or_else(|| format!("moc:{}", hash(path.as_bytes())));
            let old = existing.clone().unwrap_or_default();
            let base = match existing {
                Some(text) => text,
                None => markdown(
                    &json!({"title":"Map of content","llm_wiki":{"schema":1,"document_id":id,"information_type":"knowledge","status":"current"}}),
                    "# Map of content",
                )?,
            };
            let entry = format!(
                "- [[{}|{}]]",
                plan.path.trim_end_matches(".md"),
                evidence["version"]["title"].as_str().unwrap_or("Knowledge")
            );
            let after = managed_patch(&base, &document_id, Some(&entry))?;
            moc_patches.push(json!({"path":path,"before":old,"after":after}));
            artifacts.push(artifact(root, path.clone(), id, "moc", Some(after))?);
        }
    }
    let mut paths = BTreeSet::new();
    for a in &artifacts {
        if !paths.insert(a.path.to_lowercase()) {
            return Err("archive_path_invalid: duplicate artifact".into());
        }
    }
    let mut payload = json!({"proposalId":uid(),"proposalVersion":1,"state":"review_needed","taskId":input["taskId"],"knowledgeRevision":input["knowledgeRevision"],"outcome":plan.outcome,"rationale":plan.rationale,"input":input,"taxonomySnapshotHash":hash(serde_json::to_string(taxonomy).unwrap().as_bytes()),"target":{"documentId":document_id,"path":plan.path},"artifacts":artifacts,"mocPatches":moc_patches,"referenceLinks":links,"unresolvedConflicts":if plan.outcome=="conflict"{vec![json!({"path":plan.path,"reason":plan.rationale})]}else{vec![]}});
    payload["proposalHash"] = json!(hash(payload.to_string().as_bytes()));
    if inventory(root)? != taxonomy {
        return Err("archive_taxonomy_conflict".into());
    }
    let mut c = database::open(db)?;
    let tx = c
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(db_err)?;
    if let Some(replay) = proposal_replay(&tx, input)? {
        return Ok(replay);
    }
    snapshot(&tx, input)?;
    tx.execute("INSERT INTO knowledge_archive_proposals(proposal_id,request_id,request_hash,task_id,knowledge_revision,proposal_hash,outcome,payload_json) VALUES(?,?,?,?,?,?,?,?)",params![payload["proposalId"].as_str(),required(input,"operationId")?,hash(input.to_string().as_bytes()),required(input,"taskId")?,integer(input,"knowledgeRevision")?,payload["proposalHash"].as_str(),plan.outcome,payload.to_string()]).map_err(db_err)?;
    tx.commit().map_err(db_err)?;
    Ok(payload)
}

fn load_proposal(c: &Connection, id: &str) -> Result<Value> {
    let text: String = c
        .query_row(
            "SELECT payload_json FROM knowledge_archive_proposals WHERE proposal_id=?",
            [id],
            |r| r.get(0),
        )
        .map_err(|_| "archive_proposal_not_found".to_owned())?;
    parse(&text)
}
pub fn review_state(db: &Path, task_id: &str) -> Result<Value> {
    let c=database::open(db)?;
    let mut proposals=Vec::new();
    let mut statement=c.prepare("SELECT json_set(payload_json,'$.state',state) FROM knowledge_archive_proposals WHERE task_id=? OR json_extract(payload_json,'$.target.documentId') IN (SELECT publication_document_id FROM knowledge_pointers WHERE task_id=?) ORDER BY rowid DESC LIMIT 50").map_err(db_err)?;
    for row in statement.query_map(params![task_id,task_id],|r|r.get::<_,String>(0)).map_err(db_err)? { proposals.push(parse(&row.map_err(db_err)?)?); }
    let mut operations=Vec::new();
    for proposal in &proposals {
        let mut query=c.prepare("SELECT operation_id FROM knowledge_archive_operations WHERE proposal_id=? ORDER BY rowid DESC").map_err(db_err)?;
        for row in query.query_map([required(proposal,"proposalId")?],|r|r.get::<_,String>(0)).map_err(db_err)? {
            operations.push(status(db,&json!({"operationId":row.map_err(db_err)?}))?);
        }
    }
    Ok(json!({"proposals":proposals,"operations":operations}))
}

pub fn index_queue_failed(db: &Path, operation_id: &str) -> Result<()> {
    set_state(&database::open(db)?,operation_id,"index_failed","publication_index_failed: index job could not be queued")
}

pub fn retry_index(db: &Path, input: &Value) -> Result<Value> {
    let current=status(db,input)?;
    match current["state"].as_str() {
        Some("complete" | "compensated") => return Ok(current),
        Some("repair_required") => return Err("archive_repair_required: review recovery choices before retry".into()),
        Some("approved" | "index_failed" | "index_pending") => {},
        _ => return Err("archive_operation_state_invalid".into()),
    }
    let c=database::open(db)?;
    set_state(&c,required(input,"operationId")?,"index_pending","")?;
    status(db,input)
}

pub fn status(db: &Path, input: &Value) -> Result<Value> {
    let c = database::open(db)?;
    let id = required(input, "operationId")?;
    c.query_row("SELECT o.state,o.error,o.proposal_id,o.proposal_version FROM knowledge_archive_operations o WHERE operation_id=?",[id],|r|Ok(json!({"operationId":id,"state":r.get::<_,String>(0)?,"error":r.get::<_,String>(1)?,"proposalId":r.get::<_,String>(2)?,"proposalVersion":r.get::<_,i64>(3)?}))).map_err(db_err)
}
fn set_state(c: &Connection, id: &str, state: &str, error: &str) -> Result<()> {
    c.execute("UPDATE knowledge_archive_operations SET state=?,error=?,updated_at=CURRENT_TIMESTAMP WHERE operation_id=?",params![state,error,id]).map_err(db_err)?;
    Ok(())
}
fn validate_artifacts(root: &Path, artifacts: &[Artifact], resume: bool) -> Result<()> {
    for item in artifacts {
        let current = bytes_at(root, &item.path)?.map(|b| hash(&b));
        if current != item.expected_hash && !(resume && current == item.sha256) {
            return Err("archive_external_edit".into());
        }
        if item.bytes.as_ref().map(|b| hash(b.as_bytes())) != item.sha256 {
            return Err("archive_proposal_stale: artifact hash".into());
        }
    }
    Ok(())
}
fn guard_proposal(tx: &Transaction<'_>, taxonomy: &[Value], proposal: &Value) -> Result<()> {
    if proposal["outcome"] == "conflict" {
        return Err("archive_target_conflict".into());
    }
    if proposal["input"]["intent"].is_null() {
        snapshot(tx, &proposal["input"])?;
    } else {
        let id = required(&proposal["input"], "documentId")?;
        let revision: String = taxonomy.iter()
            .find(|v| v["documentId"] == id)
            .and_then(|v| v["revision"].as_str().map(str::to_owned))
            .ok_or("archive_document_not_found")?;
        if proposal["input"]["expectedRevision"] != revision {
            return Err("archive_proposal_stale".into());
        }
    }
    if proposal["taxonomySnapshotHash"]
        != hash(serde_json::to_string(taxonomy).unwrap().as_bytes())
    {
        return Err("archive_taxonomy_conflict".into());
    }
    Ok(())
}
/// Journal before any file changes. Replayed operation IDs are payload-bound.
pub fn publish(db: &Path, root: &Path, input: &Value) -> Result<Value> {
    let id = required(input, "operationId")?;
    // Expensive inventory is outside the writer lock. Target bytes are checked
    // again while journaling and immediately before each filesystem operation.
    let taxonomy = inventory(root)?;
    let mut c = database::open(db)?;
    let tx = c
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(db_err)?;
    let input_hash = hash(input.to_string().as_bytes());
    let prior: Option<String> = tx
        .query_row(
            "SELECT payload_hash FROM knowledge_archive_operations WHERE operation_id=?",
            [id],
            |r| r.get(0),
        )
        .optional()
        .map_err(db_err)?;
    if let Some(prior) = prior {
        if prior != input_hash {
            return Err("operation_conflict".into());
        }
        tx.commit().map_err(db_err)?;
        return status(db, input);
    }
    let proposal = load_proposal(&tx, required(input, "proposalId")?)?;
    if proposal["proposalVersion"] != input["proposalVersion"]
        || proposal["proposalHash"] != input["proposalHash"]
    {
        return Err("archive_proposal_stale".into());
    }
    let applied: bool = tx
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM knowledge_archive_operations WHERE proposal_id=?)",
            [required(input, "proposalId")?],
            |r| r.get(0),
        )
        .map_err(db_err)?;
    if applied {
        return Err("archive_proposal_already_applied".into());
    }
    guard_proposal(&tx, &taxonomy, &proposal)?;
    let artifacts: Vec<Artifact> =
        serde_json::from_value(proposal["artifacts"].clone()).map_err(|e| e.to_string())?;
    validate_artifacts(root, &artifacts, false)?;
    tx.execute("INSERT INTO knowledge_archive_operations(operation_id,proposal_id,proposal_version,payload_hash) VALUES(?,?,?,?)",params![id,required(input,"proposalId")?,integer(input,"proposalVersion")?,input_hash]).map_err(db_err)?;
    for (index, item) in artifacts.iter().enumerate() {
        let before = bytes_at(root, &item.path)?;
        tx.execute("INSERT INTO knowledge_archive_steps(operation_id,ordinal,document_id,path,kind,before_hash,after_hash,before_bytes,after_bytes) VALUES(?,?,?,?,?,?,?,?,?)",params![id,index as i64,item.document_id,item.path,item.kind,item.expected_hash,item.sha256,before,item.bytes.as_ref().map(|s|s.as_bytes())]).map_err(db_err)?;
    }
    tx.commit().map_err(db_err)?;
    // The durable operation is recoverable even if the caller disappears now.
    apply_files(db, root, id)?;
    status(db, input)
}
fn recovery_dir(root: &Path, operation: &str) -> Result<PathBuf> {
    let base = root
        .canonicalize()
        .map_err(|e| e.to_string())?
        .join(".llm-wiki-recovery");
    for path in [&base, &base.join(hash(operation.as_bytes()))] {
        if fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err("archive_path_invalid: recovery symlink".into());
        }
        fs::create_dir_all(path).map_err(|e| e.to_string())?;
    }
    Ok(base.join(hash(operation.as_bytes())))
}
fn durable_copy(path: &Path, bytes: &[u8]) -> Result<()> {
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
    {
        Ok(mut f) => {
            f.write_all(bytes).map_err(|e| e.to_string())?;
            f.sync_all().map_err(|e| e.to_string())
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            if fs::symlink_metadata(path)
                .map_err(|e| e.to_string())?
                .file_type()
                .is_symlink()
                || fs::read(path).map_err(|e| e.to_string())? != bytes
            {
                return Err("archive_repair_required: recovery bytes changed".into());
            }
            Ok(())
        }
        Err(e) => Err(e.to_string()),
    }
}
fn apply_files(db: &Path, root: &Path, id: &str) -> Result<()> {
    let c = database::open(db)?;
    let (phase, pid): (String, String) = c
        .query_row(
            "SELECT state,proposal_id FROM knowledge_archive_operations WHERE operation_id=?",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(db_err)?;
    if matches!(phase.as_str(), "complete" | "compensated") {
        return Ok(());
    }
    let proposal = load_proposal(&c, &pid)?;
    let artifacts: Vec<Artifact> =
        serde_json::from_value(proposal["artifacts"].clone()).map_err(|e| e.to_string())?;
    let outcome = (|| -> Result<()> {
        validate_artifacts(root, &artifacts, true)?;
        let recovery = recovery_dir(root, id)?;
        for (ordinal, item) in artifacts.iter().enumerate() {
            let before:Option<Vec<u8>>=c.query_row("SELECT before_bytes FROM knowledge_archive_steps WHERE operation_id=? AND ordinal=?",params![id,ordinal as i64],|r|r.get(0)).map_err(db_err)?;
            if let Some(bytes) = &before {
                durable_copy(&recovery.join(format!("{ordinal}.before")), bytes)?;
            }
            if let Some(bytes) = &item.bytes {
                durable_copy(&recovery.join(format!("{ordinal}.after")), bytes.as_bytes())?;
            }
            let current = bytes_at(root, &item.path)?.map(|b| hash(&b));
            if current != item.sha256 {
                if current != item.expected_hash {
                    return Err("archive_external_edit".into());
                }
                let target = safe_path(root, &item.path)?;
                if let Some(bytes) = &item.bytes {
                    let parent = target.parent().ok_or("archive_path_invalid")?;
                    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                    safe_path(root, &item.path)?;
                    let staged =
                        parent.join(format!(".archive-{}-{ordinal}.tmp", hash(id.as_bytes())));
                    durable_copy(&staged, bytes.as_bytes())?;
                    // Recheck after staging and before the atomic replacement.
                    if bytes_at(root, &item.path)?.map(|b| hash(&b)) != item.expected_hash {
                        return Err("archive_external_edit".into());
                    }
                    if item.expected_hash.is_none() {
                        // A no-clobber hard link prevents a competing create from being replaced.
                        fs::hard_link(&staged, &target)
                            .map_err(|_| "archive_target_conflict".to_owned())?;
                        fs::remove_file(&staged).map_err(|e| e.to_string())?;
                    } else {
                        vault::replace_file(&staged, &target).map_err(|e| e.to_string())?;
                    }
                    if let Ok(dir) = fs::File::open(parent) {
                        let _ = dir.sync_all();
                    }
                } else {
                    fs::remove_file(&target).map_err(|e| e.to_string())?;
                }
            }
            if bytes_at(root, &item.path)?.map(|b| hash(&b)) != item.sha256 {
                return Err("archive_external_edit".into());
            }
            c.execute("UPDATE knowledge_archive_steps SET state='applied' WHERE operation_id=? AND ordinal=?",params![id,ordinal as i64]).map_err(db_err)?;
        }
        let mut c = database::open(db)?;
        let tx = c
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(db_err)?;
        for item in &artifacts {
            let revision = item.sha256.clone().unwrap_or_else(|| {
                format!("withdrawn:{}", item.expected_hash.as_deref().unwrap_or(""))
            });
            tx.execute("INSERT OR IGNORE INTO knowledge_archive_revisions(document_id,revision,path,kind,body,operation_id,withdrawn) VALUES(?,?,?,?,?,?,?)",params![item.document_id,revision,item.path,item.kind,item.bytes.as_deref().unwrap_or(""),id,item.bytes.is_none()]).map_err(db_err)?;
        }
        set_state(&tx, id, "index_pending", "")?;
        tx.commit().map_err(db_err)?;
        Ok(())
    })();
    if let Err(error) = outcome {
        set_state(&c, id, "repair_required", &error)?;
        return Err(error);
    }
    Ok(())
}
/// Index exact operation artifacts and move pointers in one short transaction.
/// Optional embeddings are scheduled separately and never fake this receipt.
pub fn finish_index(db: &Path, root: &Path, semantic: &SemanticEngine, id: &str) -> Result<Value> {
    let c = database::open(db)?;
    let phase = status(db, &json!({"operationId":id}))?;
    if phase["state"] == "complete" || phase["state"] == "compensated" {
        return Ok(phase);
    }
    apply_files(db, root, id)?;
    let pid: String = c
        .query_row(
            "SELECT proposal_id FROM knowledge_archive_operations WHERE operation_id=?",
            [id],
            |r| r.get(0),
        )
        .map_err(db_err)?;
    let proposal = load_proposal(&c, &pid)?;
    let artifacts: Vec<Artifact> =
        serde_json::from_value(proposal["artifacts"].clone()).map_err(|e| e.to_string())?;
    let parsed = artifacts
        .iter()
        .map(|a| {
            a.bytes
                .as_ref()
                .map(|b| parse_document_at_revision(&a.path, b, a.sha256.clone().unwrap()).units)
        })
        .collect::<Vec<_>>();
    let outcome = (|| -> Result<()> {
        let mut c = database::open(db)?;
        let tx = c
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(db_err)?;
        if proposal["input"]["intent"].is_null() {
            snapshot(&tx, &proposal["input"])?;
        }
        for (a, units) in artifacts.iter().zip(&parsed) {
            if bytes_at(root, &a.path)?.map(|b| hash(&b)) != a.sha256 {
                return Err("archive_external_edit".into());
            }
            match (&a.bytes, units) {
                (Some(body), Some(units)) => {
                    retrieval_index::persist_document(
                        &tx,
                        &a.path,
                        frontmatter(body)["title"].as_str().unwrap_or(&a.path),
                        body,
                        a.sha256.as_deref().unwrap(),
                        0,
                        units,
                        semantic,
                        false,
                        false,
                    )?;
                }
                _ => {
                    tx.execute("DELETE FROM vault_documents WHERE path=?", [&a.path])
                        .map_err(db_err)?;
                }
            }
        }
        for a in &artifacts {
            let indexed: Option<String> = tx
                .query_row(
                    "SELECT source_hash FROM vault_documents WHERE path=?",
                    [&a.path],
                    |r| r.get(0),
                )
                .optional()
                .map_err(db_err)?;
            if indexed != a.sha256 || bytes_at(root, &a.path)?.map(|b| hash(&b)) != a.sha256 {
                return Err("publication_index_failed: exact receipt mismatch".into());
            }
        }
        // Delete old path records before upserting moved identities.
        for a in artifacts.iter().filter(|a| a.bytes.is_none()) {
            tx.execute(
                "DELETE FROM knowledge_archive_current WHERE document_id=?",
                [&a.document_id],
            )
            .map_err(db_err)?;
        }
        for a in artifacts.iter().filter(|a| a.bytes.is_some()) {
            tx.execute("INSERT INTO knowledge_archive_current(document_id,revision,path,kind,withdrawn) VALUES(?,?,?,?,0) ON CONFLICT(document_id) DO UPDATE SET revision=excluded.revision,path=excluded.path,kind=excluded.kind,withdrawn=0",params![a.document_id,a.sha256,a.path,a.kind]).map_err(db_err)?;
            if let (Some(idea), Some(revision)) = (&a.idea_id, a.idea_revision) {
                tx.execute("UPDATE knowledge_idea_revisions SET publication_state='published' WHERE id=? AND revision=?",params![idea,revision]).map_err(db_err)?;
            }
        }
        if proposal["input"]["intent"].is_null() {
            let doc = required(&proposal["target"], "documentId")?;
            let a = artifacts
                .iter()
                .find(|a| a.document_id == doc)
                .ok_or("archive_target_conflict")?;
            let changed = tx.execute("UPDATE knowledge_pointers SET published_revision=?,publication_document_id=?,publication_path=?,publication_content_hash=?,updated_at=CURRENT_TIMESTAMP WHERE task_id=? AND current_private_revision=?",params![integer(&proposal,"knowledgeRevision")?,doc,a.path,a.sha256,required(&proposal,"taskId")?,integer(&proposal,"knowledgeRevision")?]).map_err(db_err)?;
            if changed != 1 { return Err("generation_snapshot_conflict: publication pointer changed".into()); }
            for link in proposal["referenceLinks"].as_array().into_iter().flatten() {
                tx.execute("INSERT OR IGNORE INTO knowledge_archive_links(operation_id,source_document_id,target_document_id,target_revision,section,role,rationale) VALUES(?,?,?,?,?,?,?)",params![id,doc,required(link,"documentId")?,required(link,"documentVersion")?,link["section"].as_str().unwrap_or(""),required(link,"role")?,required(link,"rationale")?]).map_err(db_err)?;
            }
        } else {
            let doc = required(&proposal["input"], "documentId")?;
            if let Some(a) = artifacts
                .iter()
                .find(|a| a.document_id == doc && a.bytes.is_some())
            {
                tx.execute("UPDATE knowledge_pointers SET publication_path=?,publication_content_hash=? WHERE publication_document_id=?",params![a.path,a.sha256,doc]).map_err(db_err)?;
            } else {
                tx.execute("UPDATE knowledge_pointers SET published_revision=NULL,publication_path=NULL,publication_content_hash=NULL WHERE publication_document_id=?",[doc]).map_err(db_err)?;
                tx.execute(
                    "UPDATE knowledge_idea_revisions SET publication_state='withdrawn' WHERE id=?",
                    [doc.strip_prefix("idea:").unwrap_or(doc)],
                )
                .map_err(db_err)?;
            }
        }
        set_state(&tx, id, "complete", "")?;
        tx.execute(
            "UPDATE knowledge_archive_proposals SET state='applied' WHERE proposal_id=?",
            [pid.as_str()],
        )
        .map_err(db_err)?;
        tx.commit().map_err(db_err)?;
        Ok(())
    })();
    if let Err(error) = outcome {
        set_state(
            &c,
            id,
            if error.contains("external") || error.contains("conflict") {
                "repair_required"
            } else {
                "index_failed"
            },
            &error,
        )?;
        return Err(error);
    }
    status(db, &json!({"operationId":id}))
}
/// Startup recovery never invokes a model or overwrites bytes that differ from
/// both journal hashes. Such operations remain visible for explicit repair.
pub fn recover_pending(db: &Path, root: &Path, semantic: &SemanticEngine) -> Result<usize> {
    let c = database::open(db)?;
    let ids=c.prepare("SELECT operation_id FROM knowledge_archive_operations WHERE state IN ('approved','index_pending','index_failed') ORDER BY created_at LIMIT 100").map_err(db_err)?.query_map([],|r|r.get::<_,String>(0)).map_err(db_err)?.collect::<std::result::Result<Vec<_>,_>>().map_err(db_err)?;
    let mut recovered = 0;
    for id in ids {
        if finish_index(db, root, semantic, &id).is_ok() {
            recovered += 1;
        }
    }
    Ok(recovered)
}

pub fn organize(db: &Path, root: &Path, input: &Value) -> Result<Value> {
    let c = database::open(db)?;
    if let Some(replay) = proposal_replay(&c, input)? {
        return Ok(replay);
    }
    let intent = required(input, "intent")?;
    if !matches!(intent, "move" | "rename" | "withdraw" | "repair") {
        return Err("archive_intent_invalid".into());
    }
    let taxonomy = inventory(root)?;
    let doc = required(input, "documentId")?;
    let current = taxonomy
        .iter()
        .find(|v| v["documentId"] == doc)
        .ok_or("archive_document_not_found")?;
    if current["revision"] != input["expectedRevision"] {
        return Err("archive_external_edit".into());
    }
    let old_path = required(current, "path")?;
    let old = String::from_utf8(bytes_at(root, old_path)?.ok_or("archive_document_not_found")?)
        .map_err(|e| e.to_string())?;
    let mut artifacts = Vec::new();
    let mut repairs = Vec::new();
    let mut unresolved = Vec::new();
    let new_path = if intent == "withdraw" {
        None
    } else {
        Some(input["requestedPath"].as_str().unwrap_or(old_path))
    };
    if let Some(path) = new_path {
        safe_path(root, path)?;
        if path != old_path
            && taxonomy
                .iter()
                .any(|v| v["path"].as_str().unwrap_or("").eq_ignore_ascii_case(path))
        {
            return Err("archive_target_conflict".into());
        }
        let mut metadata = frontmatter(&old);
        if !metadata.is_object() {
            metadata = json!({});
        }
        if !metadata["llm_wiki"].is_object() {
            metadata["llm_wiki"] = json!({});
        }
        metadata["llm_wiki"]["schema"] = json!(1);
        metadata["llm_wiki"]["document_id"] = json!(doc);
        let body = old
            .strip_prefix("---\n")
            .and_then(|s| s.split_once("\n---"))
            .map(|(_, b)| b)
            .unwrap_or(&old);
        artifacts.push(artifact(
            root,
            path.into(),
            doc.into(),
            "knowledge",
            Some(markdown(&metadata, body)?),
        )?);
    }
    if new_path != Some(old_path) {
        artifacts.push(artifact(
            root,
            old_path.into(),
            doc.into(),
            "knowledge",
            None,
        )?);
    }
    let needle = format!("[[{}", old_path.trim_end_matches(".md"));
    for note in &taxonomy {
        let path = required(note, "path")?;
        if path == old_path || Some(path) == new_path {
            continue;
        }
        let body = String::from_utf8(bytes_at(root, path)?.ok_or("archive_external_edit")?)
            .map_err(|e| e.to_string())?;
        if !body.contains(&needle) {
            continue;
        }
        let start = format!("<!-- llm-wiki:{doc}:start -->");
        let end = format!("<!-- llm-wiki:{doc}:end -->");
        let after = if let (Some(a), Some(b)) = (body.find(&start), body.find(&end)) {
            if body.matches(&start).count() != 1 || body.matches(&end).count() != 1 || a >= b {
                return Err("archive_managed_region_conflict".into());
            }
            let content = &body[a + start.len()..b];
            let replacement = new_path.map(|path| {
                content.replace(&needle, &format!("[[{}", path.trim_end_matches(".md")))
            });
            managed_patch(&body, doc, replacement.as_deref())?
        } else {
            unresolved.push(json!({"path":path,"reason":"unmanaged inbound link preserved"}));
            continue;
        };
        if after.contains(&needle) && new_path != Some(old_path) {
            unresolved.push(json!({"path":path,"reason":"unmanaged inbound link preserved"}));
        }
        if after != body {
            repairs.push(json!({"path":path,"before":body,"after":after}));
            artifacts.push(artifact(
                root,
                path.into(),
                required(note, "documentId")?.into(),
                "moc",
                Some(after),
            )?);
        }
    }
    let mut payload = json!({"proposalId":uid(),"proposalVersion":1,"state":"review_needed","taskId":"","knowledgeRevision":0,"outcome":intent,"input":input,"rationale":format!("Reviewed {intent} of {old_path}"),"taxonomySnapshotHash":hash(serde_json::to_string(&taxonomy).unwrap().as_bytes()),"target":{"documentId":doc,"path":new_path},"artifacts":artifacts,"mocPatches":repairs,"referenceLinks":[],"unresolvedConflicts":unresolved});
    payload["proposalHash"] = json!(hash(payload.to_string().as_bytes()));
    c.execute("INSERT INTO knowledge_archive_proposals(proposal_id,request_id,request_hash,task_id,knowledge_revision,proposal_hash,outcome,payload_json) VALUES(?,?,?,'',0,?,?,?)",params![required(&payload,"proposalId")?,required(input,"operationId")?,hash(input.to_string().as_bytes()),required(&payload,"proposalHash")?,intent,payload.to_string()]).map_err(db_err)?;
    Ok(payload)
}
pub fn recover(db: &Path, root: &Path, semantic: &SemanticEngine, input: &Value) -> Result<Value> {
    let id = required(input, "operationId")?;
    if input["choice"] == "finish" {
        return finish_index(db, root, semantic, id);
    }
    if input["choice"] != "compensate" {
        return Err("archive_recovery_choice_required".into());
    }
    let c = database::open(db)?;
    let state = status(db, input)?;
    if state["state"] == "complete" {
        return Err("archive_already_complete: prepare a new reviewed operation".into());
    }
    let steps=c.prepare("SELECT path,before_bytes,after_hash FROM knowledge_archive_steps WHERE operation_id=? ORDER BY ordinal DESC").map_err(db_err)?.query_map([id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,Option<Vec<u8>>>(1)?,r.get::<_,Option<String>>(2)?))).map_err(db_err)?.collect::<std::result::Result<Vec<_>,_>>().map_err(db_err)?;
    for (path, before, after) in &steps {
        let current = bytes_at(root, path)?.map(|b| hash(&b));
        if current != *after && current != before.as_ref().map(|b| hash(b)) {
            return Err("archive_external_edit: compensation refused".into());
        }
    }
    for (path, before, after) in steps {
        if bytes_at(root, &path)?.map(|b| hash(&b)) != after {
            continue;
        }
        match before {
            Some(bytes) => {
                safe_path(root, &path)?;
                vault::atomic_write(
                    root,
                    &path,
                    &String::from_utf8(bytes).map_err(|e| e.to_string())?,
                )?;
            }
            None => {
                if safe_path(root, &path)?.exists() {
                    fs::remove_file(safe_path(root, &path)?).map_err(|e| e.to_string())?;
                }
            }
        }
    }
    set_state(&c, id, "compensated", "")?;
    status(db, input)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (tempfile::TempDir, PathBuf, PathBuf, Value) {
        let temporary = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join(".tmp");
        fs::create_dir_all(&temporary).unwrap();
        let root = tempfile::tempdir_in(temporary).unwrap();
        let db = root.path().join("state.sqlite");
        database::initialize(&db).unwrap();
        let vault = root.path().join("vault");
        fs::create_dir(&vault).unwrap();
        let body=markdown(&json!({"llm_wiki":{"schema":1,"document_id":"document-one","information_type":"knowledge","status":"current"}}),"# Approval\n\nExplicit approval is required.").unwrap();
        fs::write(vault.join("Approval.md"), &body).unwrap();
        let request = json!({"operationId":"review-move","documentId":"document-one","expectedRevision":hash(body.as_bytes()),"intent":"move","requestedPath":"Guides/Approval.md"});
        (root, db, vault, request)
    }
    fn publish_input(proposal: &Value) -> Value {
        json!({"operationId":"publish-one","proposalId":proposal["proposalId"],"proposalVersion":proposal["proposalVersion"],"proposalHash":proposal["proposalHash"]})
    }
    #[test]
    fn archive_moves_only_after_review_and_waits_for_exact_index_receipts() {
        let (_r, db, root, input) = fixture();
        let moc="# Manual notes\n\nKeep this introduction.\n\n<!-- llm-wiki:document-one:start -->\n- [[Approval]]\n<!-- llm-wiki:document-one:end -->\n\nKeep this conclusion.\n";
        fs::write(root.join("Index.md"), moc).unwrap();
        let proposal = organize(&db, &root, &input).unwrap();
        assert!(root.join("Approval.md").exists());
        assert!(!root.join("Guides/Approval.md").exists());
        assert_eq!(proposal, organize(&db, &root, &input).unwrap());
        let publish = publish_input(&proposal);
        let result = super::publish(&db, &root, &publish).unwrap();
        assert_eq!(result["state"], "index_pending");
        let c = database::open(&db).unwrap();
        assert_eq!(
            c.query_row("SELECT count(*) FROM knowledge_archive_current", [], |r| {
                r.get::<_, i64>(0)
            })
            .unwrap(),
            0
        );
        assert!(!root.join("Approval.md").exists());
        assert!(fs::read_to_string(root.join("Index.md"))
            .unwrap()
            .contains("Keep this conclusion."));
        let result = finish_index(&db, &root, &SemanticEngine::new(None), "publish-one").unwrap();
        assert_eq!(result["state"], "complete");
        assert_eq!(
            c.query_row(
                "SELECT path FROM knowledge_archive_current WHERE document_id='document-one'",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
            "Guides/Approval.md"
        );
        let actual = hash(&fs::read(root.join("Guides/Approval.md")).unwrap());
        assert_eq!(
            c.query_row(
                "SELECT source_hash FROM vault_documents WHERE path='Guides/Approval.md'",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
            actual
        );
        assert_eq!(
            super::publish(&db, &root, &publish).unwrap()["state"],
            "complete"
        );
    }
    #[test]
    fn archive_changed_target_and_case_collisions_never_overwrite() {
        let (_r, db, root, input) = fixture();
        let proposal = organize(&db, &root, &input).unwrap();
        fs::write(root.join("Approval.md"), "external edit").unwrap();
        assert!(super::publish(&db, &root, &publish_input(&proposal)).is_err());
        assert_eq!(
            fs::read_to_string(root.join("Approval.md")).unwrap(),
            "external edit"
        );
        fs::write(root.join("approval.md"), "different case").unwrap();
        // Case-insensitive filesystems may alias the first file; the existing target still conflicts.
        let mut collision = input.clone();
        collision["requestedPath"] = json!("Approval.md");
        collision["expectedRevision"] = json!("wrong");
        assert!(organize(&db, &root, &collision).is_err());
    }
    #[test]
    fn archive_restart_finishes_durable_files_without_model_or_duplicate_revision() {
        let (_r, db, root, input) = fixture();
        let proposal = organize(&db, &root, &input).unwrap();
        let publish = publish_input(&proposal);
        super::publish(&db, &root, &publish).unwrap();
        let c = database::open(&db).unwrap();
        let before: i64 = c
            .query_row(
                "SELECT count(*) FROM knowledge_archive_revisions",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            recover_pending(&db, &root, &SemanticEngine::new(None)).unwrap(),
            1
        );
        assert_eq!(
            recover_pending(&db, &root, &SemanticEngine::new(None)).unwrap(),
            0
        );
        assert_eq!(
            c.query_row(
                "SELECT count(*) FROM knowledge_archive_revisions",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            before
        );
    }
    #[test]
    fn archive_post_write_external_edit_preserves_bytes_and_blocks_current_pointer() {
        let (_r, db, root, input) = fixture();
        let proposal = organize(&db, &root, &input).unwrap();
        super::publish(&db, &root, &publish_input(&proposal)).unwrap();
        fs::write(root.join("Guides/Approval.md"), "external after write").unwrap();
        assert!(finish_index(&db, &root, &SemanticEngine::new(None), "publish-one").is_err());
        assert_eq!(
            status(&db, &json!({"operationId":"publish-one"})).unwrap()["state"],
            "repair_required"
        );
        assert!(recover(
            &db,
            &root,
            &SemanticEngine::new(None),
            &json!({"operationId":"publish-one","choice":"compensate"})
        )
        .is_err());
        assert_eq!(
            fs::read_to_string(root.join("Guides/Approval.md")).unwrap(),
            "external after write"
        );
    }
    #[test]
    fn archive_withdraw_repairs_managed_links_and_removes_search_without_deleting_history() {
        let (_r, db, root, mut input) = fixture();
        vault::index(&db, &root, &SemanticEngine::new(None), false, false).unwrap();
        input["intent"] = json!("withdraw");
        fs::write(root.join("Index.md"),"# Index\n<!-- llm-wiki:document-one:start -->\n[[Approval]]\n<!-- llm-wiki:document-one:end -->\nuser notes").unwrap();
        let proposal = organize(&db, &root, &input).unwrap();
        super::publish(&db, &root, &publish_input(&proposal)).unwrap();
        finish_index(&db, &root, &SemanticEngine::new(None), "publish-one").unwrap();
        let c = database::open(&db).unwrap();
        assert_eq!(
            c.query_row(
                "SELECT count(*) FROM vault_documents WHERE path='Approval.md'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
        assert!(fs::read_to_string(root.join("Index.md"))
            .unwrap()
            .contains("user notes"));
        assert!(recovery_dir(&root, "publish-one")
            .unwrap()
            .join("0.before")
            .exists());
    }
    #[test]
    fn archive_compensation_restores_only_operation_owned_bytes() {
        let (_r, db, root, input) = fixture();
        let original = fs::read(root.join("Approval.md")).unwrap();
        let proposal = organize(&db, &root, &input).unwrap();
        super::publish(&db, &root, &publish_input(&proposal)).unwrap();
        assert_eq!(
            recover(
                &db,
                &root,
                &SemanticEngine::new(None),
                &json!({"operationId":"publish-one","choice":"compensate"})
            )
            .unwrap()["state"],
            "compensated"
        );
        assert_eq!(fs::read(root.join("Approval.md")).unwrap(), original);
        assert!(!root.join("Guides/Approval.md").exists());
    }
    #[test]
    fn archive_paths_and_malformed_managed_regions_fail_closed() {
        let (_r, _db, root, _input) = fixture();
        for path in [
            "../escape.md",
            "/tmp/escape.md",
            ".llm-wiki-recovery/x.md",
            "bad#heading.md",
            "bad.txt",
        ] {
            assert!(safe_path(&root, path).is_err());
        }
        assert!(managed_patch("<!-- llm-wiki:x:start -->bad", "x", Some("new")).is_err());
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(root.parent().unwrap(), root.join("outside")).unwrap();
            assert!(safe_path(&root, "outside/escape.md").is_err());
        }
    }
    fn seed_knowledge_context() -> (tempfile::TempDir, std::path::PathBuf, Value) {
        let root = tempfile::tempdir_in(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .join(".tmp"),
        )
        .unwrap();
        let db = root.path().join("db.sqlite");
        database::initialize(&db).unwrap();
        let c = database::open(&db).unwrap();
        c.execute("INSERT INTO captures(id,text) VALUES('capture','raw')", [])
            .unwrap();
        c.execute("INSERT INTO tasks(id,origin_capture_id,current_revision,state,created_at,last_user_activity_at) VALUES('task','capture',1,'completed','2026-01-01','2026-01-01')",[]).unwrap();
        c.execute("INSERT INTO task_revisions(task_id,revision,title,detail,outcome,scope,non_goals,validation_criteria,content_hash,created_at) VALUES('task',1,'Approval flow','Keep approval explicit','Approved behavior','offline','online','tests','task-hash','2026-01-01')",[]).unwrap();
        c.execute("INSERT INTO task_completions(id,task_id,task_revision,evidence,report,operation_id,created_at) VALUES('completion','task',1,'cargo test passed','Approval remains required','complete-op','2026-01-02')",[]).unwrap();
        c.execute("INSERT INTO task_work_log_entries(id,task_id,body,created_at) VALUES('log','task','work','2026-01-02')",[]).unwrap();
        c.execute("INSERT INTO task_distillation_revisions(owner_type,owner_id,revision,task_id,work_log_entry_id,source_set_hash,prompt_id,prompt_version,rules_version,result_schema_version,locale,result_json,freshness,job_id) VALUES('work_log','log',1,'task','log','work-hash','run_report_distillation',2,'rules','schema','en','{}','current','work-job')",[]).unwrap();
        c.execute("INSERT INTO task_distillation_current(owner_type,owner_id,revision,source_set_hash,freshness) VALUES('work_log','log',1,'work-hash','current')",[]).unwrap();
        c.execute("INSERT INTO task_distillation_revisions(owner_type,owner_id,revision,task_id,source_set_hash,prompt_id,prompt_version,rules_version,result_schema_version,locale,result_json,freshness,job_id) VALUES('task_journey','task',1,'task','journey-hash','task_journey_increment',2,'rules','schema','en','{}','current','journey-job')",[]).unwrap();
        c.execute("INSERT INTO task_distillation_current(owner_type,owner_id,revision,source_set_hash,freshness) VALUES('task_journey','task',1,'journey-hash','current')",[]).unwrap();
        c.execute("INSERT INTO task_distillation_completion_snapshots(task_id,snapshot_id,completion_id,completion_revision,status,snapshot_json,projection_revision) VALUES('task','snapshot','completion','1','current','{\"id\":\"snapshot\",\"completionId\":\"completion\",\"completionRevision\":\"1\",\"status\":\"current\",\"claimIds\":[]}',1)",[]).unwrap();
        let mut c = database::open(&db).unwrap();
        let tx = c.transaction().unwrap();
        let snapshot = knowledge_distillation::capture_snapshot_tx(&tx, "task", 1, "en").unwrap();
        tx.commit().unwrap();
        (root, db, snapshot)
    }

    fn knowledge_fixture() -> (tempfile::TempDir, PathBuf, PathBuf, Value, Value) {
        let (temp, db, context) = seed_knowledge_context();
        let root = temp.path().join("vault");
        fs::create_dir(&root).unwrap();
        let source = json!({"type":"task_revision","id":"task","revision":"1","locator":"definition","quote":"Approval flow"});
        let output:knowledge_distillation::GenerationResult=serde_json::from_value(json!({"article":{"type":"concept","title":"Approval flow","bodyMarkdown":"# Approval flow\n\nApproval flow","applicability":{"summary":"Approval flow","representativeQuestions":["When is approval required?"],"helpsWith":["Approval flow"],"conditions":[],"exclusions":[],"sourceRefs":[source]},"claimBindings":[{"claimId":"claim","statement":"Approval flow","epistemicState":"reported","sourceRefs":[source]}]},"ideas":[]})).unwrap();
        let prepared =
            knowledge_distillation::prepared(context.clone(), output, "enhanced", "").unwrap();
        let mut c = database::open(&db).unwrap();
        let tx = c.transaction().unwrap();
        let saved = knowledge_distillation::append_generated_tx(&tx, &prepared).unwrap();
        tx.commit().unwrap();
        let input = json!({"operationId":"prepare-knowledge","taskId":"task","knowledgeRevision":saved["draftRevision"],"expectedKnowledgeContentHash":saved["contentHash"],"expectedGenerationSnapshotHash":context["generationSnapshotHash"],"selectedIdeaRevisionIds":[]});
        let tx = c.transaction().unwrap();
        let evidence = snapshot(&tx, &input).unwrap();
        tx.commit().unwrap();
        (temp, db, root, input, evidence)
    }

    #[test]
    fn archive_final_knowledge_uses_reviewed_bytes_and_defers_f_pointer_until_receipt() {
        let (_temp, db, root, input, evidence) = knowledge_fixture();
        let c = database::open(&db).unwrap();
        let proposal=prepare_from_plan(&db,&root,&input,&evidence,&[],json!({"outcome":"new","path":"Knowledge/Approval.md","rationale":"Approval guide","tags":["approval"],"aliases":["Approval guide"],"mocPaths":["Index.md"]})).unwrap();
        assert_eq!(review_state(&db,"task").unwrap()["proposals"][0]["proposalId"], proposal["proposalId"]);
        let reviewed = proposal["artifacts"][0]["bytes"]
            .as_str()
            .unwrap()
            .to_owned();
        super::publish(&db, &root, &publish_input(&proposal)).unwrap();
        assert_eq!(review_state(&db,"task").unwrap()["operations"][0]["state"],"index_pending");
        assert_eq!(
            c.query_row(
                "SELECT published_revision FROM knowledge_pointers WHERE task_id='task'",
                [],
                |r| r.get::<_, Option<i64>>(0)
            )
            .unwrap(),
            None
        );
        assert_eq!(
            fs::read_to_string(root.join("Knowledge/Approval.md")).unwrap(),
            reviewed
        );
        finish_index(&db, &root, &SemanticEngine::new(None), "publish-one").unwrap();
        assert_eq!(
            c.query_row(
                "SELECT published_revision FROM knowledge_pointers WHERE task_id='task'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            1
        );
        let units = parse_document_at_revision(
            "Knowledge/Approval.md",
            &reviewed,
            hash(reviewed.as_bytes()),
        )
        .units;
        assert!(units
            .iter()
            .any(|u| u.aspect == crate::domain::retrieval::SearchAspect::Applicability));
        assert!(units
            .iter()
            .all(|u| u.document_id == proposal["target"]["documentId"].as_str().unwrap()));
        let moc = &proposal["artifacts"][1];
        let parsed = parse_document_at_revision("Index.md", moc["bytes"].as_str().unwrap(), moc["sha256"].as_str().unwrap().into());
        assert_eq!(parsed.document_id, moc["documentId"].as_str().unwrap());
    }
    #[test]
    fn archive_outcomes_preserve_topics_metadata_and_explicit_conflict() {
        for outcome in ["update", "merge", "conflict", "supersede"] {
            let (_temp, db, root, input, mut evidence) = knowledge_fixture();
            let original = markdown(&json!({"custom":{"preserve":"exact value"},"tags":["existing"],"llm_wiki":{"schema":1,"document_id":"old","status":"current","decisions":[{"id":"old:approval","topic":"approval","status":"adopted","confirmed_final":true,"conditions":["internal only"]},{"id":"old:billing","topic":"billing","status":"adopted","confirmed_final":true}]}}), "# Old\n\nKeep billing guidance.").unwrap();
            fs::write(root.join("Old.md"), &original).unwrap();
            evidence["version"]["result"]["article"]["finalOutcomes"] = json!([{"topicKey":"approval","statement":"Explicit approval","claimIds":["claim"]}]);
            let taxonomy = inventory(&root).unwrap();
            let path = if outcome == "supersede" { "New.md" } else { "Old.md" };
            let proposal = prepare_from_plan(&db, &root, &input, &evidence, &taxonomy, json!({"outcome":outcome,"path":path,"targetPath":"Old.md","rationale":"Same approval topic","tags":["new"]})).unwrap();
            assert_eq!(fs::read_to_string(root.join("Old.md")).unwrap(), original);
            if outcome == "conflict" {
                assert!(proposal["artifacts"].as_array().unwrap().is_empty());
                assert!(super::publish(&db,&root,&publish_input(&proposal)).is_err());
            } else if outcome == "supersede" {
                let predecessor = proposal["artifacts"].as_array().unwrap().iter().find(|a|a["path"]=="Old.md").unwrap()["bytes"].as_str().unwrap();
                let metadata = frontmatter(predecessor);
                assert_eq!(metadata["llm_wiki"]["status"],"current");
                assert_eq!(metadata["llm_wiki"]["decisions"][0]["status"],"superseded");
                assert_eq!(metadata["llm_wiki"]["decisions"][1]["status"],"adopted");
                assert_eq!(metadata["llm_wiki"]["decisions"][1]["confirmed_final"],true);
                assert!(metadata["llm_wiki"]["decisions"][1]["successor_id"].is_null());
                assert_eq!(metadata["custom"]["preserve"],"exact value");
            } else {
                let body=proposal["artifacts"][0]["bytes"].as_str().unwrap();
                let metadata=frontmatter(body);
                assert_eq!(metadata["custom"]["preserve"],"exact value");
                assert_eq!(metadata["tags"],json!(["existing","new"]));
                assert_eq!(metadata["llm_wiki"]["document_id"],"old");
                if outcome=="merge" { assert!(body.contains("Keep billing guidance.")); }
            }
        }
    }

    #[test]
    fn archive_reference_eligibility_keeps_exact_versions_and_repairable_links() {
        let (_temp, db, root, _input) = fixture();
        let c=database::open(&db).unwrap();
        let taxonomy=inventory(&root).unwrap();
        let evidence=json!({"sources":{"actualReferenceUsages":[
            {"documentId":"document-one","documentVersion":"v1","section":"Limits","disposition":"used","reason":"Applicable to internal approval"},
            {"documentId":"document-one","documentVersion":"v2","section":"Exceptions","disposition":"counterevidence","reason":"External requests differ"},
            {"documentId":"missing","documentVersion":"v3","disposition":"viewed"}
        ]}});
        let links=reference_links(&c,&evidence,&taxonomy).unwrap();
        assert_eq!(links.len(),2);
        let markdown=links_markdown(&links);
        assert!(markdown.contains("[[Approval#Limits]]"));
        assert!(markdown.contains("<!-- llm-wiki:document-one:start -->"));
        assert!(markdown.contains("v1") && markdown.contains("v2"));
        let mut invalid=evidence;
        invalid["sources"]["actualReferenceUsages"][0]["reason"]=json!("");
        assert!(reference_links(&c,&invalid,&taxonomy).unwrap_err().contains("reference_unresolved"));
    }

    #[test]
    fn archive_taxonomy_and_source_heads_reject_stale_review() {
        let (_temp, db, root, input, evidence)=knowledge_fixture();
        let plan=json!({"outcome":"new","path":"Guide.md","rationale":"Reuse guide"});
        let proposal=prepare_from_plan(&db,&root,&input,&evidence,&[],plan).unwrap();
        fs::write(root.join("New external.md"),"External").unwrap();
        assert!(super::publish(&db,&root,&publish_input(&proposal)).unwrap_err().contains("taxonomy_conflict"));
        assert!(!root.join("Guide.md").exists());
        let mut stale=input.clone();stale["expectedKnowledgeContentHash"]=json!("stale");
        let mut c=database::open(&db).unwrap();let tx=c.transaction().unwrap();
        assert!(snapshot(&tx,&stale).is_err());
    }

    #[test]
    fn archive_overlap_context_preserves_sources_within_the_token_budget() {
        let passage = "evidence".repeat(500);
        assert_eq!(passage.chars().count().div_ceil(4), 1_000);
        let candidates = (0..9)
            .map(|index| {
                json!({
                    "documentId":format!("document-{index}"),
                    "path":format!("Knowledge/document-{index}.md"),
                    "source_hash":format!("sha256:{index}"),
                    "section":format!("Decision {index}"),
                    "snippet":passage,
                    "body":format!("complete document {index} {}", "private ".repeat(4_000))
                })
            })
            .collect::<Vec<_>>();

        let bounded = bounded_overlap_context(json!({
            "results":candidates,
            "has_more":false,
            "semantic_available":true
        }));
        let results = bounded["results"].as_array().unwrap();
        assert_eq!(results.len(), 6);
        assert_eq!(bounded["retrievedContextEstimatedTokens"], 6_000);
        assert_eq!(bounded["has_more"], true);
        for (index, candidate) in results.iter().enumerate() {
            assert_eq!(candidate["documentId"], format!("document-{index}"));
            assert_eq!(candidate["source_hash"], format!("sha256:{index}"));
            assert_eq!(candidate["snippet"], passage);
            assert!(candidate.get("body").is_none());
        }
        assert!(results
            .iter()
            .all(|candidate| candidate["documentId"] != "document-6"));
    }

    #[tokio::test]
    async fn archive_prepare_dispatches_registered_prompt_and_replays_without_provider() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let (temp, db, root, input, _) = knowledge_fixture();
        let settings_path = temp.path().join("settings.json");
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        crate::native::settings::save_provider(&settings_path, &json!({"base_url":format!("http://{address}"),"model":"fixture","api_key":"fixture-key"})).unwrap();
        let response_content = json!({"outcome":"new","path":"Approval.md","rationale":"Approval guide","tags":["approval"]}).to_string();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            let mut buffer = [0_u8; 4096];
            loop {
                let count = socket.read(&mut buffer).await.unwrap();
                if count == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..count]);
                let Some(header_end) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n")
                else {
                    continue;
                };
                let headers = String::from_utf8_lossy(&request[..header_end]);
                let length = headers
                    .lines()
                    .find_map(|line| {
                        line.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .and_then(|value| value.trim().parse::<usize>().ok())
                    })
                    .unwrap_or(0);
                if request.len() >= header_end + 4 + length {
                    break;
                }
            }
            let payload = json!({"choices":[{"message":{"content":response_content}}]}).to_string();
            socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",payload.len(),payload).as_bytes()).await.unwrap();
            String::from_utf8(request).unwrap()
        });

        let proposal = prepare(&db,&settings_path,&root,&SemanticEngine::new(None),&input).await.unwrap();
        let request=server.await.unwrap();
        assert!(request.contains("Organize the supplied reviewed Knowledge"));
        assert!(request.contains("overlapCandidates") && request.contains("taxonomy"));
        assert!(request.contains("retrievedContextEstimatedTokens"));
        assert!(request.contains("Approval flow"));
        assert!(!root.join("Approval.md").exists());
        // The one-response server has exited; replay must not call the model.
        assert_eq!(prepare(&db,&settings_path,&root,&SemanticEngine::new(None),&input).await.unwrap(),proposal);
    }

    #[test]
    fn archive_inventory_budget_keeps_thousand_notes_outside_write_locks() {
        let (_temp,_db,root,_)=fixture();
        let body=format!("# Existing note\n{}", "useful knowledge ".repeat(625));
        for n in 0..1000 { fs::write(root.join(format!("note-{n:04}.md")),&body).unwrap(); }
        let started=std::time::Instant::now();
        let notes=inventory(&root).unwrap();
        let elapsed=started.elapsed();
        assert_eq!(notes.len(),1001);
        eprintln!("archive inventory 1001 notes/~10MB: {}ms",elapsed.as_millis());
        assert!(elapsed < std::time::Duration::from_secs(3));
    }

    #[test]
    fn archive_recovery_finishes_each_partial_file_boundary() {
        for completed_steps in 0..=2 {
            let (_temp,db,root,input)=fixture();
            let proposal=organize(&db,&root,&input).unwrap();
            super::publish(&db,&root,&publish_input(&proposal)).unwrap();
            let c=database::open(&db).unwrap();
            let steps=c.prepare("SELECT path,before_bytes FROM knowledge_archive_steps WHERE operation_id='publish-one' ORDER BY ordinal").unwrap().query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,Option<Vec<u8>>>(1)?))).unwrap().collect::<std::result::Result<Vec<_>,_>>().unwrap();
            // Reproduce a process stopping after each durable per-file boundary.
            for (path,before) in steps.iter().skip(completed_steps) {
                let target=root.join(path);
                if let Some(bytes)=before { fs::write(target,bytes).unwrap(); }
                else if target.exists() { fs::remove_file(target).unwrap(); }
            }
            c.execute("UPDATE knowledge_archive_operations SET state='approved' WHERE operation_id='publish-one'",[]).unwrap();
            assert_eq!(recover_pending(&db,&root,&SemanticEngine::new(None)).unwrap(),1);
            assert!(!root.join("Approval.md").exists());
            assert!(root.join("Guides/Approval.md").exists());
            assert_eq!(status(&db,&json!({"operationId":"publish-one"})).unwrap()["state"],"complete");
        }
    }

    #[test]
    fn archive_reopen_overlays_proposal_state_and_retry_restarts_pending_status() {
        let (_temp,db,root,input,evidence)=knowledge_fixture();
        let proposal=prepare_from_plan(&db,&root,&input,&evidence,&[],json!({"outcome":"new","path":"Guide.md","rationale":"Reuse guide"})).unwrap();
        super::publish(&db,&root,&publish_input(&proposal)).unwrap();
        let c=database::open(&db).unwrap();
        set_state(&c,"publish-one","index_failed","temporary index failure").unwrap();
        assert_eq!(retry_index(&db,&json!({"operationId":"publish-one"})).unwrap()["state"],"index_pending");
        finish_index(&db,&root,&SemanticEngine::new(None),"publish-one").unwrap();
        let reopened=review_state(&db,"task").unwrap();
        assert_eq!(reopened["proposals"][0]["state"],"applied");
        assert_eq!(reopened["operations"][0]["state"],"complete");
    }

}
