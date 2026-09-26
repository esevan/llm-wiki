use crate::domain::retrieval::{is_indexable_path, parse_document_at_revision};
use crate::native::{database, retrieval_index, semantic::SemanticEngine};
use rusqlite::params;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
use std::path::{Component, Path};
use std::time::Instant;
use std::time::UNIX_EPOCH;
use walkdir::WalkDir;

#[cfg(windows)]
pub(crate) fn replace_file(temporary: &Path, target: &Path) -> Result<(), std::io::Error> {
    use std::os::windows::ffi::OsStrExt;
    use std::ptr;
    use windows_sys::Win32::Storage::FileSystem::ReplaceFileW;

    if !target.exists() {
        return fs::rename(temporary, target);
    }
    let target_wide: Vec<u16> = target.as_os_str().encode_wide().chain(Some(0)).collect();
    let temporary_wide: Vec<u16> = temporary.as_os_str().encode_wide().chain(Some(0)).collect();
    let replaced = unsafe {
        ReplaceFileW(
            target_wide.as_ptr(),
            temporary_wide.as_ptr(),
            ptr::null(),
            0,
            ptr::null_mut(),
            ptr::null_mut(),
        )
    };
    if replaced == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
pub(crate) fn replace_file(temporary: &Path, target: &Path) -> Result<(), std::io::Error> {
    fs::rename(temporary, target)
}

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

pub fn resolve_markdown(
    vault: &Path,
    relative: &str,
    must_exist: bool,
) -> Result<std::path::PathBuf, String> {
    let relative_path = Path::new(relative);
    if relative_path.is_absolute()
        || relative_path.components().any(|part| {
            matches!(
                part,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
        || relative_path.extension().and_then(|value| value.to_str()) != Some("md")
    {
        return Err("Knowledge document not found".into());
    }
    let candidate = vault.join(relative_path);
    if must_exist && !candidate.is_file() {
        return Err("Knowledge document not found".into());
    }
    Ok(candidate)
}

pub fn atomic_write(vault: &Path, relative: &str, content: &str) -> Result<(), String> {
    let target = resolve_markdown(vault, relative, false)?;
    let parent = target.parent().ok_or("Knowledge path has no parent")?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let temporary = parent.join(format!(
        ".{}.{}.tmp",
        target.file_name().unwrap_or_default().to_string_lossy(),
        uuid::Uuid::new_v4()
    ));
    fs::write(&temporary, content).map_err(|error| error.to_string())?;
    if let Err(error) = replace_file(&temporary, &target) {
        let _ = fs::remove_file(&temporary);
        return Err(error.to_string());
    }
    Ok(())
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

pub fn index(
    db_path: &Path,
    vault: &Path,
    semantic: &SemanticEngine,
    semantic_enabled: bool,
    force_embeddings: bool,
) -> Result<Value, String> {
    let started = Instant::now();
    let mut seen = HashSet::new();
    let mut scanned = Vec::new();
    let mut changed = 0_u64;
    let mut reused_embeddings = 0_usize;
    let mut pending_embeddings = Vec::new();
    for entry in WalkDir::new(vault).follow_links(false) {
        let entry = entry.map_err(|error| error.to_string())?;
        if !entry.file_type().is_file()
            || entry.path().extension().and_then(|value| value.to_str()) != Some("md")
        {
            continue;
        }
        let path = relative_path(vault, entry.path())?;
        if !is_indexable_path(&path) {
            continue;
        }
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
        let parsed = parse_document_at_revision(&path, &body, source_hash.clone());
        let document_title = title(entry.path(), &body);
        seen.insert(path.clone());
        scanned.push((
            path,
            document_title,
            body,
            source_hash,
            modified_at,
            parsed.units,
        ));
    }
    let scan_elapsed = started.elapsed();
    let persistence_started = Instant::now();
    let mut connection = database::open(db_path)?;
    let transaction = database::immediate_transaction(&mut connection)?;
    for (path, document_title, body, source_hash, modified_at, units) in scanned {
        let previous: Option<String> = transaction
            .query_row(
                "SELECT source_hash FROM vault_documents WHERE path=?",
                [&path],
                |row| row.get(0),
            )
            .ok();
        let has_units: bool = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM vault_search_units WHERE path=?)",
                [&path],
                |row| row.get(0),
            )
            .unwrap_or(false);
        if previous.as_deref() != Some(&source_hash) || !has_units {
            changed += 1;
        }
        let (mut pending, reused) = retrieval_index::persist_document(
            &transaction,
            &path,
            &document_title,
            &body,
            &source_hash,
            modified_at,
            &units,
            semantic,
            semantic_enabled,
            force_embeddings,
        )?;
        pending_embeddings.append(&mut pending);
        reused_embeddings += reused;
    }
    let existing = transaction
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
            transaction
                .execute("DELETE FROM vault_documents WHERE path=?", [&path])
                .map_err(|error| error.to_string())?;
            removed += 1;
        }
    }
    transaction.commit().map_err(|error| error.to_string())?;
    let persistence_elapsed = persistence_started.elapsed();
    let mut applied_embeddings = 0_usize;
    let mut stale_embeddings = 0_usize;
    if semantic_enabled && semantic.available() && !pending_embeddings.is_empty() {
        let texts = pending_embeddings
            .iter()
            .map(|pending| pending.text.clone())
            .collect();
        for (pending, vector) in pending_embeddings.into_iter().zip(semantic.embed(texts)?) {
            if retrieval_index::apply_embedding_if_current(
                &connection,
                &pending,
                &vector,
                semantic,
            )? {
                applied_embeddings += 1;
            } else {
                stale_embeddings += 1;
            }
        }
    }
    Ok(json!({
        "changed":changed,
        "removed":removed,
        "elapsed_ms":started.elapsed().as_secs_f64() * 1000.0,
        "scan_ms":scan_elapsed.as_secs_f64() * 1000.0,
        "persistence_ms":persistence_elapsed.as_secs_f64() * 1000.0,
        "semantic_available":semantic.available(),
        "embeddings_applied":applied_embeddings,
        "embeddings_reused":reused_embeddings,
        "embeddings_stale":stale_embeddings
    }))
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
        return Ok(json!({
            "results":[],"offset":offset,"limit":limit,"has_more":false,
            "semantic_available":semantic.available(),"semantic_complete":false
        }));
    }
    let connection = database::open(db_path)?;
    retrieval_index::search(
        &connection,
        semantic,
        query,
        limit,
        offset,
        semantic_requested,
        |_, _, _| Ok(true),
    )
}

pub fn health(db_path: &Path, semantic: &SemanticEngine) -> Result<Value, String> {
    let connection = database::open(db_path)?;
    let documents: i64 = connection
        .query_row("SELECT count(*) FROM vault_documents", [], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    let units: i64 = connection
        .query_row("SELECT count(*) FROM vault_search_units", [], |row| {
            row.get(0)
        })
        .map_err(|error| error.to_string())?;
    let semantic_units: i64 = connection
        .query_row(
            "SELECT count(*) FROM vault_search_unit_embeddings e
             JOIN vault_search_units u ON u.unit_id=e.unit_id
              AND u.source_revision=e.source_revision AND u.input_hash=e.input_hash
             JOIN vault_documents d ON d.path=u.path AND d.source_hash=u.source_revision
             WHERE e.model_id=? AND e.model_version=? AND e.dimensions=? AND length(e.vector)=?",
            params![
                semantic.identity().0,
                semantic.identity().1,
                semantic.identity().2 as i64,
                (semantic.identity().2 * 4) as i64
            ],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    Ok(json!({
        "status":"ok",
        "documents":documents,
        "units":units,
        "semantic_units":semantic_units,
        "semantic_available":semantic.available()
    }))
}

pub fn read(vault: &Path, relative: &str, requested_locale: &str) -> Result<Value, String> {
    let candidate = resolve_markdown(vault, relative, true)?;
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
    let managed =
        content.contains("llm_wiki_managed: true") && content.contains("canonical_locale: en");
    let source_hash = format!("{:x}", Sha256::digest(content.as_bytes()));
    if managed && requested_locale.to_ascii_lowercase().starts_with("ko") {
        let derived_path = vault.join("Translations").join("ko").join(relative);
        if let Ok(translated) = fs::read_to_string(derived_path) {
            let quoted_hash = format!("source_hash: \"{source_hash}\"");
            let plain_hash = format!("source_hash: {source_hash}");
            if translated.contains(&quoted_hash) || translated.contains(&plain_hash) {
                return Ok(json!({
                    "path":relative,"content":translated,"markdown":translated,
                    "canonical_locale":"en","served_locale":"ko","translated":true,
                    "cache_status":"hit","source_hash":source_hash
                }));
            }
        }
        return Ok(json!({
            "path":relative,"content":content,"markdown":content,
            "canonical_locale":"en","served_locale":"en","translated":false,
            "cache_status":"pending","source_hash":source_hash
        }));
    }
    Ok(json!({
        "path":relative,
        "content":content,
        "markdown":content,
        "canonical_locale":if managed { "en" } else { "original" },
        "served_locale":if managed { "en" } else { "original" },
        "translated":false,
        "cache_status":"not_applicable",
        "source_hash":source_hash
    }))
}
