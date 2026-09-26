use crate::workflow_foundation::{
    validate_source_attributions, ApplicationDisposition, SourceAttribution, VersionOwner,
    VersionProvenance,
};
use rusqlite::{params, Transaction};
use uuid::Uuid;

#[allow(dead_code)]
pub fn record_version_provenance(
    tx: &Transaction<'_>,
    provenance: &VersionProvenance,
) -> Result<(), String> {
    validate_owner(&provenance.owner)?;
    validate_source_attributions(&provenance.source_references).map_err(|error| error.to_string())?;
    if provenance.prompt_id.is_some() != provenance.prompt_version.is_some() {
        return Err("prompt_id and prompt_version must be recorded together".into());
    }
    if provenance.prompt_version == Some(0) {
        return Err("prompt_version must be positive".into());
    }
    if let Some(source) = &provenance.source {
        validate_owner(source)?;
    }
    let (source_type, source_id, source_version) = provenance
        .source
        .as_ref()
        .map(|source| {
            (
                Some(source.entity_type.as_str()),
                Some(source.entity_id.as_str()),
                Some(source.version.as_str()),
            )
        })
        .unwrap_or((None, None, None));
    tx.execute(
        "INSERT INTO workflow_version_provenance(
           owner_type,owner_id,owner_version,source_owner_type,source_owner_id,
           source_owner_version,prompt_id,prompt_version,operation_id,
           restored_from_version,source_references_json
         ) VALUES(?,?,?,?,?,?,?,?,?,?,?)",
        params![
            provenance.owner.entity_type,
            provenance.owner.entity_id,
            provenance.owner.version,
            source_type,
            source_id,
            source_version,
            provenance.prompt_id.map(|id| id.as_str()).unwrap_or(""),
            provenance.prompt_version.unwrap_or(0),
            provenance.operation_id.as_deref().unwrap_or(""),
            provenance.restored_from_version,
            serde_json::to_string(&provenance.source_references)
                .map_err(|error| error.to_string())?,
        ],
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

#[allow(dead_code)]
pub fn record_document_references(
    tx: &Transaction<'_>,
    owner: &VersionOwner,
    references: &[SourceAttribution],
) -> Result<(), String> {
    validate_owner(owner)?;
    validate_source_attributions(references).map_err(|error| error.to_string())?;
    for reference in references {
        tx.execute(
            "INSERT OR IGNORE INTO workflow_document_references(
               id,owner_type,owner_id,owner_version,document_id,document_version,
               section,excerpt,claim_id
             ) VALUES(?,?,?,?,?,?,?,?,?)",
            params![
                Uuid::new_v4().to_string(),
                owner.entity_type,
                owner.entity_id,
                owner.version,
                reference.document_id,
                reference.document_version,
                reference.section.as_deref().unwrap_or(""),
                reference.excerpt.as_deref().unwrap_or(""),
                reference.claim_id.as_deref().unwrap_or(""),
            ],
        )
        .map_err(|error| error.to_string())?;
    }
    Ok(())
}

/// Records only queue metadata. The caller owns the canonical domain mutation and must call this
/// helper inside the same transaction after checking and writing that domain state.
#[allow(dead_code)]
pub fn record_job_application_disposition(
    tx: &Transaction<'_>,
    job_id: &str,
    expected_source_revision: &str,
    actual_source_revision: &str,
    disposition: ApplicationDisposition,
) -> Result<ApplicationDisposition, String> {
    let effective = if expected_source_revision != actual_source_revision {
        ApplicationDisposition::Superseded
    } else {
        disposition
    };
    if tx
        .execute(
            "UPDATE ai_jobs_v2
             SET application_disposition=?
             WHERE id=? AND source_revision=?",
            params![effective.as_str(), job_id, expected_source_revision],
        )
        .map_err(|error| error.to_string())?
        == 0
    {
        return Err("job freshness metadata changed".into());
    }
    Ok(effective)
}

fn validate_owner(owner: &VersionOwner) -> Result<(), String> {
    if owner.entity_type.trim().is_empty()
        || owner.entity_id.trim().is_empty()
        || owner.version.trim().is_empty()
    {
        return Err("version owner type, id, and version are required".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::{database, migrations};
    use crate::workflow_foundation::PromptId;
    use serde_json::json;

    #[test]
    fn provenance_and_references_attach_to_canonical_owner_versions() {
        let root = tempfile::tempdir().unwrap();
        let db = root.path().join("state.sqlite3");
        database::initialize(&db).unwrap();
        let mut connection = database::open(&db).unwrap();
        let tx = connection.transaction().unwrap();
        let owner = VersionOwner {
            entity_type: "task_knowledge_drafts".into(),
            entity_id: "task-1".into(),
            version: "2".into(),
        };
        let reference = SourceAttribution {
            document_id: "vault-note".into(),
            document_version: "sha256:abc".into(),
            section: Some("Decision".into()),
            excerpt: Some("Use the local queue".into()),
            claim_id: Some("claim-1".into()),
        };
        record_version_provenance(
            &tx,
            &VersionProvenance {
                owner: owner.clone(),
                source: None,
                prompt_id: Some(PromptId::KnowledgeDraft),
                prompt_version: Some(1),
                operation_id: Some("job-1".into()),
                restored_from_version: Some("1".into()),
                source_references: vec![reference.clone()],
            },
        )
        .unwrap();
        record_document_references(&tx, &owner, &[reference]).unwrap();
        let document_level = SourceAttribution {
            document_id: "whole-note".into(), document_version: "v1".into(),
            section: None, excerpt: None, claim_id: None,
        };
        record_document_references(&tx, &owner, &[document_level.clone(),document_level.clone()]).unwrap();
        record_document_references(&tx, &owner, &[document_level]).unwrap();
        tx.commit().unwrap();
        let stored: String = connection
            .query_row(
                "SELECT source_references_json FROM workflow_version_provenance
                 WHERE owner_type=? AND owner_id=? AND owner_version=?",
                ["task_knowledge_drafts", "task-1", "2"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(serde_json::from_str::<serde_json::Value>(&stored).unwrap()[0]["claim_id"], "claim-1");
        assert_eq!(migrations::schema_version(&connection).unwrap(), migrations::CURRENT_SCHEMA_VERSION);
        assert_eq!(connection.query_row(
            "SELECT count(*) FROM workflow_document_references WHERE document_id='whole-note'",[],
            |row|row.get::<_,i64>(0),
        ).unwrap(),1);
    }

    #[test]
    fn disposition_marks_stale_results_superseded_without_domain_writes() {
        let root = tempfile::tempdir().unwrap();
        let db = root.path().join("state.sqlite3");
        database::initialize(&db).unwrap();
        let mut connection = database::open(&db).unwrap();
        connection.execute(
            "INSERT INTO ai_jobs_v2(id,task_kind,status,source_revision,input_json)
             VALUES('job','user_generation','completed','r1',?)",
            [json!({}).to_string()],
        ).unwrap();
        let tx = connection.transaction().unwrap();
        assert_eq!(
            record_job_application_disposition(
                &tx,
                "job",
                "r1",
                "r2",
                ApplicationDisposition::Applied,
            ).unwrap(),
            ApplicationDisposition::Superseded
        );
        tx.commit().unwrap();
        let stored: String = connection.query_row(
            "SELECT application_disposition FROM ai_jobs_v2 WHERE id='job'", [], |row| row.get(0)
        ).unwrap();
        assert_eq!(stored, "superseded");
    }
}
