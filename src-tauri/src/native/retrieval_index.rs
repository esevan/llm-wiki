use crate::domain::retrieval::{
    embedding_is_compatible, fuse_candidates, resolve_successor, DecisionStatus, EmbeddingIdentity,
    RankedCandidate, SearchUnit, SuccessorResolution, MAX_PASSAGES,
};
use crate::native::semantic::SemanticEngine;
use rusqlite::{params, Connection};
use serde_json::{json, Value};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug)]
pub(crate) struct PendingEmbedding {
    pub unit_id: String,
    pub source_revision: String,
    pub input_hash: String,
    pub text: String,
}

#[derive(Clone)]
struct StoredUnit {
    unit: SearchUnit,
    title: String,
    body: String,
    source_hash: String,
    modified_at: i64,
}

pub(crate) fn persist_document(
    connection: &Connection,
    path: &str,
    title: &str,
    body: &str,
    source_hash: &str,
    modified_at: i64,
    units: &[SearchUnit],
    semantic: &SemanticEngine,
    semantic_enabled: bool,
    force_embeddings: bool,
) -> Result<(Vec<PendingEmbedding>, usize), String> {
    let (model_id, model_version, dimensions) = semantic.identity();
    connection
        .prepare_cached(
            "INSERT INTO vault_documents(path,title,body,source_hash,modified_at) VALUES (?,?,?,?,?)
         ON CONFLICT(path) DO UPDATE SET title=excluded.title,body=excluded.body,
         source_hash=excluded.source_hash,modified_at=excluded.modified_at",
        )
        .and_then(|mut statement| statement.execute(params![path,title,body,source_hash,modified_at]))
        .map_err(|error| error.to_string())?;
    let ids = units
        .iter()
        .map(|unit| unit.unit_id.clone())
        .collect::<HashSet<_>>();
    let prior = connection
        .prepare("SELECT unit_id FROM vault_search_units WHERE path=?")
        .and_then(|mut statement| {
            statement
                .query_map([path], |row| row.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|error| error.to_string())?;
    for id in prior {
        if !ids.contains(&id) {
            connection
                .execute("DELETE FROM vault_search_units WHERE unit_id=?", [&id])
                .map_err(|error| error.to_string())?;
        }
    }
    let mut pending = Vec::new();
    let mut reused = 0;
    for unit in units {
        let unit_json = serde_json::to_string(unit).map_err(|error| error.to_string())?;
        connection.prepare_cached(
            "INSERT INTO vault_search_units(unit_id,document_id,path,title,source_revision,declared_revision,section,chunk_index,chunk_count,aspect,information_type,status,input_hash,text,unit_json)
             VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)
             ON CONFLICT(unit_id) DO UPDATE SET document_id=excluded.document_id,path=excluded.path,
             title=excluded.title,
             source_revision=excluded.source_revision,declared_revision=excluded.declared_revision,
             section=excluded.section,chunk_index=excluded.chunk_index,chunk_count=excluded.chunk_count,
             aspect=excluded.aspect,information_type=excluded.information_type,status=excluded.status,
             input_hash=excluded.input_hash,text=excluded.text,unit_json=excluded.unit_json,indexed_at=CURRENT_TIMESTAMP",
        ).and_then(|mut statement| statement.execute(params![unit.unit_id,unit.document_id,unit.path,title,unit.source_revision,unit.declared_revision,
                unit.section,unit.chunk_index as i64,unit.chunk_count as i64,unit.aspect.as_str(),
                unit.information_type.as_str(),unit.status.map(|status|status.as_str()),unit.input_hash,
                unit.text,unit_json])).map_err(|error| error.to_string())?;
        if semantic_enabled && semantic.available() {
            let stored = connection.query_row(
                "SELECT source_revision,input_hash,dimensions,vector FROM vault_search_unit_embeddings
                 WHERE unit_id=? AND model_id=? AND model_version=?",
                params![unit.unit_id,model_id,model_version],
                |row| Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,i64>(2)? as usize,row.get::<_,Vec<u8>>(3)?)),
            ).ok();
            let compatible = stored.as_ref().is_some_and(|stored| {
                embedding_is_compatible(
                    &EmbeddingIdentity {
                        unit_id: &unit.unit_id,
                        source_revision: &stored.0,
                        input_hash: &stored.1,
                        model_id,
                        model_version,
                        dimensions: stored.2,
                    },
                    &EmbeddingIdentity {
                        unit_id: &unit.unit_id,
                        source_revision: &unit.source_revision,
                        input_hash: &unit.input_hash,
                        model_id,
                        model_version,
                        dimensions,
                    },
                    &stored.3,
                )
            });
            let reusable = stored.as_ref().is_some_and(|stored| {
                stored.1 == unit.input_hash
                    && stored.2 == dimensions
                    && stored.3.len() == dimensions * 4
            });
            if !force_embeddings && (compatible || reusable) {
                if reusable && !compatible {
                    connection.execute(
                        "UPDATE vault_search_unit_embeddings SET source_revision=?,indexed_at=CURRENT_TIMESTAMP
                         WHERE unit_id=? AND model_id=? AND model_version=?",
                        params![unit.source_revision,unit.unit_id,model_id,model_version],
                    ).map_err(|error| error.to_string())?;
                }
                reused += 1;
            } else {
                pending.push(PendingEmbedding {
                    unit_id: unit.unit_id.clone(),
                    source_revision: unit.source_revision.clone(),
                    input_hash: unit.input_hash.clone(),
                    text: unit.text.clone(),
                });
            }
        }
    }
    Ok((pending, reused))
}

pub(crate) fn apply_embedding_if_current(
    connection: &Connection,
    pending: &PendingEmbedding,
    vector: &[f32],
    semantic: &SemanticEngine,
) -> Result<bool, String> {
    let (model_id, model_version, dimensions) = semantic.identity();
    if vector.len() != dimensions {
        return Ok(false);
    }
    let bytes = vector
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect::<Vec<_>>();
    let applied=connection.execute(
        "INSERT INTO vault_search_unit_embeddings(unit_id,source_revision,input_hash,model_id,model_version,dimensions,vector)
         SELECT u.unit_id,u.source_revision,u.input_hash,?,?,?,? FROM vault_search_units u
         JOIN vault_documents d ON d.path=u.path WHERE u.unit_id=? AND u.source_revision=?
         AND u.input_hash=? AND d.source_hash=u.source_revision
         ON CONFLICT(unit_id,model_id,model_version) DO UPDATE SET source_revision=excluded.source_revision,
         input_hash=excluded.input_hash,dimensions=excluded.dimensions,vector=excluded.vector,indexed_at=CURRENT_TIMESTAMP",
        params![model_id,model_version,dimensions as i64,bytes,pending.unit_id,pending.source_revision,pending.input_hash],
    ).map_err(|error|error.to_string())?;
    Ok(applied == 1)
}

fn decode(value: String) -> rusqlite::Result<SearchUnit> {
    serde_json::from_str(&value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            value.len(),
            rusqlite::types::Type::Text,
            Box::new(error),
        )
    })
}

fn row(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoredUnit> {
    Ok(StoredUnit {
        unit: decode(row.get(0)?)?,
        title: row.get(1)?,
        body: row.get(2)?,
        source_hash: row.get(3)?,
        modified_at: row.get(4)?,
    })
}

fn cosine(left: &[f32], bytes: &[u8]) -> f32 {
    if bytes.len() != left.len() * 4 {
        return -1.0;
    }
    let mut dot = 0.0;
    let mut right_norm = 0.0;
    let left_norm = left.iter().map(|value| value * value).sum::<f32>().sqrt();
    for (left_value, chunk) in left.iter().zip(bytes.chunks_exact(4)) {
        let right = f32::from_le_bytes(chunk.try_into().expect("four-byte vector component"));
        dot += left_value * right;
        right_norm += right * right;
    }
    let denominator = left_norm * right_norm.sqrt();
    if denominator == 0.0 {
        -1.0
    } else {
        dot / denominator
    }
}

fn all_visible<F>(connection: &Connection, visible: &mut F) -> Result<Vec<SearchUnit>, String>
where
    F: FnMut(&str, &str, &str) -> Result<bool, String>,
{
    let rows=connection.prepare(
        "SELECT u.unit_json,d.title,d.body,d.source_hash,d.modified_at FROM vault_search_units u
         JOIN vault_documents d ON d.path=u.path AND d.source_hash=u.source_revision"
    ).and_then(|mut statement|statement.query_map([],row)?.collect::<Result<Vec<_>,_>>()).map_err(|error|error.to_string())?;
    let mut units = Vec::new();
    for item in rows {
        if visible(&item.unit.path, &item.title, &item.body)? {
            units.push(item.unit);
        }
    }
    Ok(units)
}

pub(crate) fn search<F>(
    connection: &Connection,
    semantic: &SemanticEngine,
    query: &str,
    limit: usize,
    offset: usize,
    semantic_requested: bool,
    mut visible: F,
) -> Result<Value, String>
where
    F: FnMut(&str, &str, &str) -> Result<bool, String>,
{
    let effective_limit = limit.clamp(1, MAX_PASSAGES);
    let pool_limit = (offset + effective_limit).saturating_mul(8).clamp(32, 256);
    let terms = query
        .split_whitespace()
        .filter(|term| !term.is_empty())
        .map(|term| format!("\"{}\"", term.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(" AND ");
    let rows=connection.prepare(
        "SELECT u.unit_json,d.title,d.body,d.source_hash,d.modified_at FROM vault_search_units_fts f
         JOIN vault_search_units u ON u.rowid=f.rowid JOIN vault_documents d ON d.path=u.path
         AND d.source_hash=u.source_revision WHERE vault_search_units_fts MATCH ?
         ORDER BY bm25(vault_search_units_fts),u.path,u.unit_id LIMIT ?"
    ).and_then(|mut statement|statement.query_map(params![terms,pool_limit as i64],row)?.collect::<Result<Vec<_>,_>>()).map_err(|error|error.to_string())?;
    let mut documents = HashMap::new();
    let mut lexical = Vec::new();
    for item in rows {
        if visible(&item.unit.path, &item.title, &item.body)? {
            documents.insert(
                item.unit.path.clone(),
                (item.title, item.body, item.source_hash, item.modified_at),
            );
            lexical.push(RankedCandidate::lexical(item.unit, lexical.len() + 1));
        }
    }
    let (model_id, model_version, dimensions) = semantic.identity();
    let mut semantic_candidates = Vec::new();
    let mut semantic_available = semantic.available();
    let mut semantic_complete = false;
    if semantic_requested && semantic_available {
        match semantic.embed(vec![query.to_owned()]) {
            Ok(mut vectors)
                if vectors
                    .first()
                    .is_some_and(|vector| vector.len() == dimensions) =>
            {
                let query_vector = vectors.remove(0);
                let rows=connection.prepare(
                    "SELECT u.unit_json,d.title,d.body,d.source_hash,d.modified_at,e.vector FROM vault_search_units u
                     JOIN vault_documents d ON d.path=u.path AND d.source_hash=u.source_revision
                     JOIN vault_search_unit_embeddings e ON e.unit_id=u.unit_id AND e.source_revision=u.source_revision
                     AND e.input_hash=u.input_hash AND e.model_id=? AND e.model_version=? AND e.dimensions=?
                     WHERE length(e.vector)=?"
                ).and_then(|mut statement|statement.query_map(params![model_id,model_version,dimensions as i64,(dimensions*4) as i64],|r|Ok((row(r)?,r.get::<_,Vec<u8>>(5)?)))?.collect::<Result<Vec<_>,_>>()).map_err(|error|error.to_string())?;
                let mut visible_rows = Vec::new();
                for row in rows {
                    if visible(&row.0.unit.path, &row.0.title, &row.0.body)? {
                        visible_rows.push(row);
                    }
                }
                visible_rows.sort_by(|left, right| {
                    cosine(&query_vector, &right.1)
                        .partial_cmp(&cosine(&query_vector, &left.1))
                        .unwrap_or(Ordering::Equal)
                        .then_with(|| left.0.unit.unit_id.cmp(&right.0.unit.unit_id))
                });
                visible_rows.truncate(pool_limit);
                for (item, vector) in visible_rows {
                    let score = cosine(&query_vector, &vector);
                    documents.insert(
                        item.unit.path.clone(),
                        (item.title, item.body, item.source_hash, item.modified_at),
                    );
                    semantic_candidates.push(RankedCandidate::semantic(
                        item.unit,
                        semantic_candidates.len() + 1,
                        score,
                    ));
                }
                semantic_complete=all_visible(connection,&mut visible)?.iter().all(|unit|{
                    connection.query_row("SELECT EXISTS(SELECT 1 FROM vault_search_unit_embeddings WHERE unit_id=? AND source_revision=? AND input_hash=? AND model_id=? AND model_version=? AND dimensions=? AND length(vector)=?)",params![unit.unit_id,unit.source_revision,unit.input_hash,model_id,model_version,dimensions as i64,(dimensions*4) as i64],|r|r.get::<_,bool>(0)).unwrap_or(false)
                });
            }
            _ => semantic_available = false,
        }
    }
    let mut ranked = fuse_candidates(lexical, semantic_candidates, effective_limit + 1, offset);
    let has_more = ranked.len() > effective_limit;
    ranked.truncate(effective_limit);
    if ranked.iter().any(|candidate| {
        candidate
            .unit
            .decision
            .as_ref()
            .is_some_and(|decision| decision.status == DecisionStatus::Superseded)
    }) {
        let all_units = all_visible(connection, &mut visible)?;
        for candidate in &mut ranked {
            if candidate
                .unit
                .decision
                .as_ref()
                .is_some_and(|decision| decision.status == DecisionStatus::Superseded)
            {
                match resolve_successor(&candidate.unit, &all_units, &[]) {
                    SuccessorResolution::Current(id) if id != candidate.unit.unit_id => {
                        candidate.historical_match = candidate
                            .unit
                            .decision
                            .as_ref()
                            .map(|decision| decision.id.clone());
                        if let Some(successor) = all_units.iter().find(|unit| unit.unit_id == id) {
                            candidate.unit = successor.clone();
                        }
                    }
                    SuccessorResolution::Unresolved(warning) => candidate.warning = Some(warning),
                    _ => {}
                }
            }
        }
    }
    for candidate in &ranked {
        if !documents.contains_key(&candidate.unit.path) {
            if let Ok(document) = connection.query_row(
                "SELECT title,body,source_hash,modified_at FROM vault_documents
                 WHERE path=? AND source_hash=?",
                params![candidate.unit.path, candidate.unit.source_revision],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            ) {
                documents.insert(candidate.unit.path.clone(), document);
            }
        }
    }
    let source_revision = documents.values().map(|value| value.3).max().unwrap_or(0);
    let results=ranked.into_iter().map(|candidate|{
        let(title,body,source_hash,modified_at)=documents.get(&candidate.unit.path).cloned().unwrap_or_default();
        let mut warnings=candidate.unit.warnings.clone();warnings.extend(candidate.warning);
        let lower=body.to_lowercase();let lower_title=title.to_lowercase();
        let matched=query.split_whitespace().filter(|term|{let term=term.to_lowercase();lower.contains(&term)||lower_title.contains(&term)}).collect::<Vec<_>>();
        json!({"path":candidate.unit.path,"title":title,"body":body,"source_hash":source_hash,"modified_at":modified_at,
            "snippet":candidate.unit.text.chars().take(500).collect::<String>(),"matchedTerms":matched,
            "passage":{"kind":"section","section":candidate.unit.section},"score":candidate.fused_score,"semantic_score":candidate.semantic_score,
            "documentId":candidate.unit.document_id,"section":candidate.unit.section,"chunkIndex":candidate.unit.chunk_index,"chunkCount":candidate.unit.chunk_count,
            "aspect":candidate.unit.aspect,"informationType":candidate.unit.information_type,"status":candidate.unit.status,"conditions":candidate.unit.conditions,
            "historicalMatch":candidate.historical_match,"warnings":warnings})
    }).collect::<Vec<_>>();
    Ok(
        json!({"results":results,"offset":offset,"limit":effective_limit,"has_more":has_more,"semantic_available":semantic_available,"semantic_complete":semantic_complete,"source_revision":source_revision}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::{database, vault};
    use std::fs;

    fn setup() -> (
        tempfile::TempDir,
        std::path::PathBuf,
        std::path::PathBuf,
        SemanticEngine,
    ) {
        let root = tempfile::tempdir().unwrap();
        let db = root.path().join("index.sqlite3");
        let vault_path = root.path().join("vault");
        fs::create_dir_all(vault_path.join("Knowledge")).unwrap();
        database::initialize(&db).unwrap();
        (root, db, vault_path, SemanticEngine::new(None))
    }

    #[test]
    fn index_survives_restart_and_tracks_change_move_delete_and_exclusions() {
        let (_root, db, vault_path, semantic) = setup();
        let original = "---\nllm_wiki:\n  schema: 1\n  document_id: stable-doc\n---\n# Alpha\nPersistent needle";
        fs::write(vault_path.join("Knowledge/alpha.md"), original).unwrap();
        fs::write(
            vault_path.join("Knowledge/uniquevaultpath.md"),
            "plain content without a heading",
        )
        .unwrap();
        fs::create_dir_all(vault_path.join("Translations/ko")).unwrap();
        fs::write(
            vault_path.join("Translations/ko/ignored.md"),
            "Persistent needle",
        )
        .unwrap();
        vault::index(&db, &vault_path, &semantic, false, false).unwrap();
        let first = vault::search(&db, &semantic, "Persistent", 8, 0, false).unwrap();
        assert_eq!(first["results"].as_array().unwrap().len(), 1);
        assert_eq!(
            vault::search(&db, &semantic, "uniquevaultpath", 8, 0, false).unwrap()["results"]
                [0]["path"],
            "Knowledge/uniquevaultpath.md"
        );
        drop(database::open(&db).unwrap());
        database::initialize(&db).unwrap();
        let restarted = vault::search(&db, &semantic, "needle", 8, 0, false).unwrap();
        assert_eq!(restarted["results"][0]["documentId"], "stable-doc");

        fs::write(
            vault_path.join("Knowledge/alpha.md"),
            original.replace("needle", "updated-token"),
        )
        .unwrap();
        vault::index(&db, &vault_path, &semantic, false, false).unwrap();
        assert!(
            vault::search(&db, &semantic, "needle", 8, 0, false).unwrap()["results"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            vault::search(&db, &semantic, "updated-token", 8, 0, false).unwrap()["results"]
                .as_array()
                .unwrap()
                .len(),
            1
        );

        fs::rename(
            vault_path.join("Knowledge/alpha.md"),
            vault_path.join("Knowledge/moved.md"),
        )
        .unwrap();
        vault::index(&db, &vault_path, &semantic, false, false).unwrap();
        assert_eq!(
            vault::search(&db, &semantic, "updated-token", 8, 0, false).unwrap()["results"][0]
                ["path"],
            "Knowledge/moved.md"
        );
        fs::remove_file(vault_path.join("Knowledge/moved.md")).unwrap();
        vault::index(&db, &vault_path, &semantic, false, false).unwrap();
        assert!(
            vault::search(&db, &semantic, "updated-token", 8, 0, false).unwrap()["results"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn stale_and_wrong_dimension_vectors_cannot_commit() {
        let (_root, db, vault_path, semantic) = setup();
        fs::write(vault_path.join("Knowledge/cas.md"), "# CAS\nfirst revision").unwrap();
        vault::index(&db, &vault_path, &semantic, false, false).unwrap();
        let connection = database::open(&db).unwrap();
        let old = connection
            .query_row(
                "SELECT unit_id,source_revision,input_hash,text FROM vault_search_units LIMIT 1",
                [],
                |row| {
                    Ok(PendingEmbedding {
                        unit_id: row.get(0)?,
                        source_revision: row.get(1)?,
                        input_hash: row.get(2)?,
                        text: row.get(3)?,
                    })
                },
            )
            .unwrap();
        drop(connection);
        fs::write(
            vault_path.join("Knowledge/cas.md"),
            "# CAS\nsecond revision",
        )
        .unwrap();
        vault::index(&db, &vault_path, &semantic, false, false).unwrap();
        let connection = database::open(&db).unwrap();
        assert!(
            !apply_embedding_if_current(&connection, &old, &vec![0.0; 384], &semantic).unwrap()
        );
        let current = connection
            .query_row(
                "SELECT unit_id,source_revision,input_hash,text FROM vault_search_units LIMIT 1",
                [],
                |row| {
                    Ok(PendingEmbedding {
                        unit_id: row.get(0)?,
                        source_revision: row.get(1)?,
                        input_hash: row.get(2)?,
                        text: row.get(3)?,
                    })
                },
            )
            .unwrap();
        assert!(
            !apply_embedding_if_current(&connection, &current, &vec![0.0; 3], &semantic).unwrap()
        );
        assert!(
            apply_embedding_if_current(&connection, &current, &vec![0.0; 384], &semantic).unwrap()
        );
        connection
            .execute(
                "UPDATE vault_search_unit_embeddings SET model_version='wrong'",
                [],
            )
            .unwrap();
        let raw = search(&connection, &semantic, "revision", 8, 0, true, |_, _, _| {
            Ok(true)
        })
        .unwrap();
        assert_eq!(raw["semantic_available"], false);
        assert_eq!(raw["semantic_complete"], false);
        assert_eq!(raw["results"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn structural_index_and_warm_lexical_search_meet_local_budget() {
        let (_root, db, vault_path, semantic) = setup();
        for index in 0..1_000 {
            fs::write(
                vault_path.join(format!("Knowledge/note-{index}.md")),
                format!(
                    "# Note {index}\nshared benchmark token {index}\n{}",
                    "x".repeat(10_000)
                ),
            )
            .unwrap();
        }
        let started = std::time::Instant::now();
        let profile = vault::index(&db, &vault_path, &semantic, false, false).unwrap();
        let structural = started.elapsed();
        eprintln!(
            "retrieval structural profile: {structural:?}, scan_ms={}, persistence_ms={}",
            profile["scan_ms"], profile["persistence_ms"]
        );
        assert!(structural < std::time::Duration::from_secs(3));
        let _ = vault::search(&db, &semantic, "benchmark", 8, 0, false).unwrap();
        let warm = std::time::Instant::now();
        let result = vault::search(&db, &semantic, "benchmark", 8, 0, false).unwrap();
        let result_count = result["results"].as_array().unwrap().len();
        assert!(result_count > 0 && result_count <= 8);
        let warm = warm.elapsed();
        eprintln!("retrieval profile: structural={structural:?}, warm_lexical={warm:?}");
        assert!(warm < std::time::Duration::from_millis(75));
    }
}
