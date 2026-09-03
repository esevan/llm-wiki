use crate::native::{database, semantic::SemanticEngine};
use rusqlite::params;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
use std::path::{Component, Path};
use std::time::UNIX_EPOCH;
use walkdir::WalkDir;

fn relative_path(root: &Path, path: &Path) -> Result<String, String> {
    let relative = path.strip_prefix(root).map_err(|error| error.to_string())?;
    if relative
        .components()
        .any(|part| matches!(part, Component::ParentDir))
    {
        return Err("Vault path escapes the configured root".into());
    }
    Ok(relative.to_string_lossy().replace('\\', "/"))
}

fn title(path: &Path, body: &str) -> String {
    body.lines()
        .find_map(|line| line.strip_prefix("# ").map(str::trim))
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .or_else(|| {
            path.file_stem()
                .map(|value| value.to_string_lossy().into_owned())
        })
        .unwrap_or_default()
}

pub fn index(db_path: &Path, vault: &Path, semantic: &SemanticEngine) -> Result<Value, String> {
    let connection = database::open(db_path)?;
    let mut seen = HashSet::new();
    let mut changed = 0_u64;
    let mut pending_embeddings = Vec::new();
    for entry in WalkDir::new(vault).follow_links(false) {
        let entry = entry.map_err(|error| error.to_string())?;
        if !entry.file_type().is_file()
            || entry.path().extension().and_then(|value| value.to_str()) != Some("md")
            || entry
                .path()
                .components()
                .any(|part| part.as_os_str() == "Translations")
        {
            continue;
        }
        let path = relative_path(vault, entry.path())?;
        let body = fs::read_to_string(entry.path()).map_err(|error| error.to_string())?;
        let source_hash = format!("{:x}", Sha256::digest(body.as_bytes()));
        let modified_at = entry
            .metadata()
            .map_err(|error| error.to_string())?
            .modified()
            .map_err(|error| error.to_string())?
            .duration_since(UNIX_EPOCH)
            .map_err(|error| error.to_string())?
            .as_secs() as i64;
        let previous: Option<String> = connection
            .query_row(
                "SELECT source_hash FROM vault_documents WHERE path=?",
                [&path],
                |row| row.get(0),
            )
            .ok();
        if previous.as_deref() != Some(&source_hash) {
            connection.execute(
                "INSERT INTO vault_documents(path,title,body,source_hash,modified_at) VALUES (?,?,?,?,?) ON CONFLICT(path) DO UPDATE SET title=excluded.title,body=excluded.body,source_hash=excluded.source_hash,modified_at=excluded.modified_at",
                params![path,title(entry.path(),&body),body,source_hash,modified_at],
            ).map_err(|error| error.to_string())?;
            changed += 1;
        }
        let embedded_hash = if semantic.available() {
            connection
                .query_row(
                    "SELECT source_hash FROM vault_document_embeddings WHERE path=?",
                    [&path],
                    |row| row.get::<_, String>(0),
                )
                .ok()
        } else {
            None
        };
        if semantic.available() && embedded_hash.as_deref() != Some(&source_hash) {
            pending_embeddings.push((
                path.clone(),
                source_hash.clone(),
                format!(
                    "{}\n{}",
                    title(entry.path(), &body),
                    body.chars().take(4000).collect::<String>()
                ),
            ));
        }
        seen.insert(path);
    }
    let existing = connection
        .prepare("SELECT path FROM vault_documents")
        .and_then(|mut statement| {
            statement
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|error| error.to_string())?;
    let mut removed = 0_u64;
    for path in existing {
        if !seen.contains(&path) {
            connection
                .execute("DELETE FROM vault_documents WHERE path=?", [&path])
                .map_err(|error| error.to_string())?;
            connection
                .execute(
                    "DELETE FROM vault_document_embeddings WHERE path=?",
                    [&path],
                )
                .map_err(|error| error.to_string())?;
            removed += 1;
        }
    }
    if semantic.available() && !pending_embeddings.is_empty() {
        let texts = pending_embeddings
            .iter()
            .map(|(_, _, text)| text.clone())
            .collect();
        for ((path, source_hash, _), vector) in
            pending_embeddings.into_iter().zip(semantic.embed(texts)?)
        {
            let bytes = vector
                .iter()
                .flat_map(|value| value.to_le_bytes())
                .collect::<Vec<_>>();
            connection
                .execute(
                    "INSERT INTO vault_document_embeddings(path,source_hash,dimensions,vector) VALUES (?,?,?,?) ON CONFLICT(path) DO UPDATE SET source_hash=excluded.source_hash,dimensions=excluded.dimensions,vector=excluded.vector",
                    params![path, source_hash, vector.len() as i64, bytes],
                )
                .map_err(|error| error.to_string())?;
        }
    }
    Ok(json!({
        "changed":changed,
        "removed":removed,
        "elapsed_ms":0.0,
        "semantic_available":semantic.available()
    }))
}

fn cosine(left: &[f32], bytes: &[u8]) -> f32 {
    if bytes.len() != left.len() * 4 {
        return 0.0;
    }
    let mut dot = 0.0;
    let mut right_norm = 0.0;
    let left_norm = left.iter().map(|value| value * value).sum::<f32>().sqrt();
    let (chunks, remainder) = bytes.as_chunks::<4>();
    debug_assert!(remainder.is_empty());
    for (left_value, chunk) in left.iter().zip(chunks) {
        let right_value = f32::from_le_bytes(*chunk);
        dot += left_value * right_value;
        right_norm += right_value * right_value;
    }
    let denominator = left_norm * right_norm.sqrt();
    if denominator == 0.0 {
        0.0
    } else {
        dot / denominator
    }
}

pub fn search(
    db_path: &Path,
    semantic: &SemanticEngine,
    query: &str,
    limit: usize,
    offset: usize,
    semantic_requested: bool,
) -> Result<Value, String> {
    if query.trim().is_empty() {
        return Ok(
            json!({"results":[],"offset":offset,"limit":limit,"has_more":false,"semantic_available":semantic.available()}),
        );
    }
    let connection = database::open(db_path)?;
    let terms = query
        .split_whitespace()
        .map(|term| format!("\"{}\"", term.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(" AND ");
    let mut statement = connection.prepare(
        "SELECT d.path,d.title,snippet(vault_documents_fts,2,'<mark>','</mark>',' … ',24),d.body,d.source_hash
         FROM vault_documents_fts JOIN vault_documents d ON d.rowid=vault_documents_fts.rowid
         WHERE vault_documents_fts MATCH ? ORDER BY bm25(vault_documents_fts) LIMIT ? OFFSET ?"
    ).map_err(|error| error.to_string())?;
    let results = statement.query_map(params![terms,(limit+1) as i64,offset as i64], |row| Ok(json!({
        "path":row.get::<_,String>(0)?,"title":row.get::<_,String>(1)?,"snippet":row.get::<_,String>(2)?,"body":row.get::<_,String>(3)?,"source_hash":row.get::<_,String>(4)?,"score":1.0,"semantic_score":null
    }))).map_err(|error| error.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|error| error.to_string())?;
    let has_more = results.len() > limit;
    let mut results = results.into_iter().take(limit).collect::<Vec<_>>();
    if semantic_requested && semantic.available() && !results.is_empty() {
        let query_vector = semantic.embed(vec![query.to_owned()])?.remove(0);
        for result in &mut results {
            let Some(path) = result.get("path").and_then(Value::as_str) else {
                continue;
            };
            let embedding = connection.query_row(
                "SELECT e.vector FROM vault_document_embeddings e JOIN vault_documents d ON d.path=e.path AND d.source_hash=e.source_hash WHERE e.path=?",
                [path],
                |row| row.get::<_, Vec<u8>>(0),
            );
            if let Ok(vector) = embedding {
                let score = cosine(&query_vector, &vector);
                result["score"] = json!(score);
                result["semantic_score"] = json!(score);
            }
        }
        results.sort_by(|left, right| {
            right["semantic_score"]
                .as_f64()
                .partial_cmp(&left["semantic_score"].as_f64())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }
    Ok(
        json!({"results":results,"offset":offset,"limit":limit,"has_more":has_more,"semantic_available":semantic.available()}),
    )
}

pub fn health(db_path: &Path, semantic: &SemanticEngine) -> Result<Value, String> {
    let connection = database::open(db_path)?;
    let documents: i64 = connection
        .query_row("SELECT count(*) FROM vault_documents", [], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    let semantic_documents: i64 = connection
        .query_row(
            "SELECT count(*) FROM vault_document_embeddings e JOIN vault_documents d ON d.path=e.path AND d.source_hash=e.source_hash",
            [],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    Ok(json!({
        "status":"ok",
        "documents":documents,
        "semantic_documents":semantic_documents,
        "semantic_available":semantic.available()
    }))
}

pub fn read(vault: &Path, relative: &str) -> Result<Value, String> {
    let candidate = vault.join(relative);
    let canonical_root = vault.canonicalize().map_err(|error| error.to_string())?;
    let canonical = candidate
        .canonicalize()
        .map_err(|_| "Knowledge document not found".to_string())?;
    if !canonical.starts_with(&canonical_root)
        || canonical.extension().and_then(|value| value.to_str()) != Some("md")
    {
        return Err("Knowledge path is outside the Vault".into());
    }
    let content = fs::read_to_string(&canonical).map_err(|error| error.to_string())?;
    Ok(
        json!({"path":relative,"content":content,"source_hash":format!("{:x}",Sha256::digest(content.as_bytes()))}),
    )
}
