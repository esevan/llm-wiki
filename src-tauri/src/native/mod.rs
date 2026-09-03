mod completion;
pub(crate) mod conversation_context;
mod database;
mod job_results;
pub mod jobs;
pub(crate) mod lineage;
mod localization;
mod patches;
mod projection;
mod refinement;
mod semantic;
pub mod settings;
mod vault;
mod workbench;
pub mod workflow;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct NativeApplication {
    db_path: PathBuf,
    vault: PathBuf,
    vault_setup_required: bool,
    semantic: semantic::SemanticEngine,
    jobs: jobs::JobRegistry,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeOperation {
    pub name: String,
    #[serde(default)]
    pub input: Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeResponse {
    pub status: u16,
    pub body: Value,
}

impl NativeApplication {
    pub fn new(
        vault: PathBuf,
        db_path: PathBuf,
        embedding_model_dir: Option<PathBuf>,
    ) -> Result<Self, String> {
        Self::build(vault, db_path, embedding_model_dir, false)
    }

    pub fn with_vault_setup(
        vault: PathBuf,
        db_path: PathBuf,
        embedding_model_dir: Option<PathBuf>,
        vault_setup_required: bool,
    ) -> Result<Self, String> {
        Self::build(vault, db_path, embedding_model_dir, vault_setup_required)
    }

    fn build(
        vault: PathBuf,
        db_path: PathBuf,
        embedding_model_dir: Option<PathBuf>,
        vault_setup_required: bool,
    ) -> Result<Self, String> {
        if !vault_setup_required {
            std::fs::create_dir_all(&vault).map_err(|error| error.to_string())?;
        }
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        database::initialize(&db_path)?;
        Ok(Self {
            db_path,
            vault,
            vault_setup_required,
            semantic: semantic::SemanticEngine::new(embedding_model_dir),
            jobs: jobs::JobRegistry::default(),
        })
    }

    pub fn isolated(vault: &Path, db_path: &Path) -> Result<Self, String> {
        Self::new(vault.to_owned(), db_path.to_owned(), None)
    }

    #[cfg(test)]
    pub fn isolated_with_model(
        vault: &Path,
        db_path: &Path,
        embedding_model_dir: &Path,
    ) -> Result<Self, String> {
        Self::new(
            vault.to_owned(),
            db_path.to_owned(),
            Some(embedding_model_dir.to_owned()),
        )
    }

    pub fn db_path(&self) -> PathBuf {
        self.db_path.clone()
    }

    pub fn vault_path(&self) -> PathBuf {
        self.vault.clone()
    }

    pub fn vault_setup_status(&self) -> Value {
        json!({
            "required": self.vault_setup_required,
            "path": if self.vault_setup_required { Value::Null } else { Value::String(self.vault.to_string_lossy().into_owned()) }
        })
    }

    pub fn save_vault_selection(&self, path: &Path) -> Result<(), String> {
        if !path.is_absolute() || !path.is_dir() {
            return Err("Choose an existing Vault folder".into());
        }
        let selected = path.canonicalize().map_err(|error| error.to_string())?;
        settings::save_vault_path(&self.db_path, &selected)
    }

    pub fn execute(&self, operation: NativeOperation) -> NativeResponse {
        match self.dispatch(&operation.name, &operation.input) {
            Ok((status, body)) => NativeResponse { status, body },
            Err(error) => NativeResponse {
                status: error_status(&error),
                body: json!({"detail":error}),
            },
        }
    }

    pub fn execute_domain(&self, domain: &str, operation: NativeOperation) -> NativeResponse {
        let belongs_to_domain = match domain {
            "system" => operation.name == "health.get",
            "vault" => matches!(
                operation.name.as_str(),
                "vault.index" | "vault.search" | "knowledge.read"
            ),
            "settings" => {
                operation.name.starts_with("locale.")
                    || operation.name.starts_with("provider.")
                    || operation.name.starts_with("i18n.")
            }
            "workflow" => matches!(
                operation.name.split('.').next().unwrap_or_default(),
                "capture"
                    | "board"
                    | "problem"
                    | "solution"
                    | "item"
                    | "workbench"
                    | "compass"
                    | "refinement"
                    | "transitions"
            ),
            "jobs" => {
                operation.name.starts_with("jobs.") || operation.name.starts_with("notifications.")
            }
            _ => false,
        };
        if !belongs_to_domain {
            return NativeResponse {
                status: 400,
                body: json!({"detail": format!("Operation {} is not available in the {domain} domain", operation.name)}),
            };
        }
        self.execute(operation)
    }

    pub async fn enqueue_job(&self, input: Value) -> NativeResponse {
        match jobs::enqueue(
            self.db_path.clone(),
            self.vault.clone(),
            self.jobs.clone(),
            self.semantic.clone(),
            input,
        )
        .await
        {
            Ok(body) => NativeResponse { status: 202, body },
            Err(error) => NativeResponse {
                status: error_status(&error),
                body: json!({"detail":error}),
            },
        }
    }

    pub async fn execute_workflow(&self, operation: NativeOperation) -> NativeResponse {
        let name = operation.name.clone();
        let input = operation.input.clone();
        let mut response = self.execute_domain("workflow", operation);
        if !(200..300).contains(&response.status) {
            return response;
        }
        let derived = match name.as_str() {
            "solution.progress.add" => Some(("solution_progress_entries", "body")),
            "solution.comment.add" => Some(("solution_progress_comments", "body")),
            "solution.checklist.add" => Some(("solution_checklist_items", "body")),
            _ => None,
        };
        if let Some((entity_type, field)) = derived {
            let source = input.get("body").and_then(Value::as_str).unwrap_or("");
            if !source.trim().is_empty() {
                let _ = self
                    .enqueue_job(json!({
                        "taskKind":"derived_translation","entityType":entity_type,
                        "entityId":response.body["id"],"entity_type":entity_type,
                        "entity_id":response.body["id"],"field":field,"source":source,
                        "source_locale":input.get("locale").and_then(Value::as_str).unwrap_or("en")
                    }))
                    .await;
            }
        }
        if name == "problem.complete" {
            let problem_id = input.get("problemId").and_then(Value::as_str).unwrap_or("");
            let job = self.enqueue_job(json!({
                "taskKind":"completion_report","entityType":"problems","entityId":problem_id,
                "refresh_lineage":false,"locale":input.get("locale").and_then(Value::as_str).unwrap_or("en")
            })).await;
            if job.status == 202 {
                response.body["report_job_id"] = job.body["id"].clone();
            }
        }
        response
    }

    fn dispatch(&self, name: &str, input: &Value) -> Result<(u16, Value), String> {
        let id = |key: &str| {
            input
                .get(key)
                .and_then(Value::as_str)
                .ok_or_else(|| format!("{key} is required"))
        };
        let result = match name {
            "health.get" => vault::health(&self.db_path, &self.semantic)?,
            "vault.index" => vault::index(
                &self.db_path,
                &self.vault,
                &self.semantic,
                input
                    .get("semantic")
                    .and_then(Value::as_bool)
                    .unwrap_or(true),
            )?,
            "vault.search" => vault::search(
                &self.db_path,
                &self.semantic,
                input.get("query").and_then(Value::as_str).unwrap_or(""),
                input.get("limit").and_then(Value::as_u64).unwrap_or(20) as usize,
                input.get("offset").and_then(Value::as_u64).unwrap_or(0) as usize,
                input
                    .get("semantic")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            )?,
            "knowledge.read" => vault::read(
                &self.vault,
                id("path")?,
                input.get("locale").and_then(Value::as_str).unwrap_or("en"),
            )?,
            "locale.get" => settings::locale(
                &self.db_path,
                input
                    .get("browserLocale")
                    .and_then(Value::as_str)
                    .unwrap_or("en"),
            )?,
            "locale.save" => settings::save_locale(&self.db_path, input)?,
            "i18n.get" => settings::resources(id("locale")?)?,
            "provider.get" => settings::provider(&self.db_path)?,
            "provider.save" => settings::save_provider(&self.db_path, input)?,
            "capture.create" => workflow::create_capture(&self.db_path, input)?,
            "board.get" => workflow::board_for_locale(
                &self.db_path,
                input.get("locale").and_then(Value::as_str).unwrap_or("en"),
            )?,
            "problem.record" => workflow::problem_record(&self.db_path, id("problemId")?)?,
            "capture.promote" => workflow::promote_capture(&self.db_path, id("captureId")?, input)?,
            "problem.approve" => workflow::approve_problem(&self.db_path, id("problemId")?)?,
            "solution.create" => workflow::create_feature(&self.db_path, id("problemId")?, input)?,
            "solution.conflict.save" => {
                workflow::set_conflict(&self.db_path, id("solutionId")?, input)?
            }
            "solution.approve" => workflow::approve_feature(&self.db_path, id("solutionId")?)?,
            "solution.stage.save" => workflow::set_stage(&self.db_path, id("solutionId")?, input)?,
            "solution.progress.get" => workflow::progress_for_locale(
                &self.db_path,
                id("solutionId")?,
                input.get("locale").and_then(Value::as_str).unwrap_or("en"),
            )?,
            "solution.progress.add" => {
                workflow::add_progress(&self.db_path, id("solutionId")?, input)?
            }
            "solution.comment.add" => workflow::add_comment(&self.db_path, id("entryId")?, input)?,
            "solution.checklist.add" => {
                workflow::add_checklist(&self.db_path, id("solutionId")?, input)?
            }
            "solution.checklist.update" => {
                workflow::update_checklist(&self.db_path, id("itemId")?, input)?
            }
            "solution.follow_up" => workflow::follow_up_problem(&self.db_path, id("solutionId")?)?,
            "problem.complete" => {
                completion::complete(&self.db_path, &self.vault, id("problemId")?, input)?
            }
            "problem.playbook.delete" => completion::remove(
                &self.db_path,
                &self.vault,
                id("problemId")?,
                input.get("force").and_then(Value::as_bool).unwrap_or(false),
            )?,
            "solution.lineage" => lineage::get(&self.db_path, id("solutionId")?)?,
            "solution.lineage.evidence" => {
                lineage::evidence(&self.db_path, id("solutionId")?, id("evidenceId")?)?
            }
            "solution.lineage.regenerate" => {
                lineage::create(&self.db_path, id("solutionId")?, true)?
            }
            "solution.lineage.correct" => {
                lineage::correct(&self.db_path, id("solutionId")?, id("claimId")?, input)?
            }
            "solution.patch.create" => {
                patches::propose(&self.db_path, &self.vault, id("solutionId")?, input)?
            }
            "solution.patch.apply" => patches::apply(&self.db_path, &self.vault, id("patchId")?)?,
            "solution.patch.undo" => patches::undo(&self.db_path, &self.vault, id("patchId")?)?,
            "solution.handoff" => workflow::handoff(&self.db_path, id("solutionId")?)?,
            "compass.goal.create" => workflow::create_goal(&self.db_path, input)?,
            "compass.dashboard" => workflow::dashboard(&self.db_path)?,
            "item.delete" => workflow::delete(&self.db_path, id("entityType")?, id("entityId")?)?,
            "item.restore" => workflow::restore(&self.db_path, id("entityType")?, id("entityId")?)?,
            "item.update" => {
                workflow::update_item(&self.db_path, id("entityType")?, id("entityId")?, input)?
            }
            "item.localization.save" => workflow::supplement_localization(
                &self.db_path,
                id("entityType")?,
                id("entityId")?,
                input,
            )?,
            "item.project" => projection::project(
                &self.db_path,
                &self.vault,
                id("entityType")?,
                id("entityId")?,
            )?,
            "item.archive" => projection::archive(
                &self.db_path,
                &self.vault,
                id("entityType")?,
                id("entityId")?,
            )?,
            "item.get" => workflow::item_for_locale(
                &self.db_path,
                id("entityType")?,
                id("entityId")?,
                input.get("locale").and_then(Value::as_str).unwrap_or("en"),
            )?,
            "workbench.category.save" => workflow::set_category(&self.db_path, input)?,
            "workbench.importance.save" => workflow::set_importance(&self.db_path, input)?,
            "refinement.context" => refinement::context(
                &self.db_path,
                id("entityType")?,
                id("entityId")?,
                input.get("locale").and_then(Value::as_str).unwrap_or("en"),
            )?,
            "transitions.list" => json!({"transitions":workflow::transitions(None)}),
            "transitions.entity" => {
                json!({"transitions":workflow::transitions(Some(id("entityType")?))})
            }
            "transitions.apply" => workflow::apply_transition(
                &self.db_path,
                &self.vault,
                id("entityType")?,
                id("entityId")?,
                input,
            )?,
            "workbench.recent" => workflow::recent_archive(
                &self.db_path,
                input.get("limit").and_then(Value::as_u64).unwrap_or(5),
            )?,
            "workbench.completed" => workflow::recent_completed(
                &self.db_path,
                &self.vault,
                input.get("limit").and_then(Value::as_u64).unwrap_or(20),
                input.get("locale").and_then(Value::as_str).unwrap_or("en"),
            )?,
            "problem.importance.save" => {
                workflow::assess_importance(&self.db_path, id("problemId")?, input)?
            }
            "solution.completion.create" => {
                workflow::record_completion(&self.db_path, id("solutionId")?, input)?
            }
            "solution.completion.verify" => {
                workflow::verify_completion(&self.db_path, id("solutionId")?)?
            }
            "jobs.list" => jobs::list(&self.db_path)?,
            "jobs.get" => jobs::get(&self.db_path, id("jobId")?)?,
            "jobs.result" => jobs::result(&self.db_path, id("jobId")?)?,
            "jobs.conflict.get" => jobs::conflict_review_status(&self.db_path, id("runId")?)?,
            "jobs.cancel" => jobs::cancel(&self.db_path, &self.jobs, id("jobId")?)?,
            "jobs.retry" => jobs::retry(
                &self.db_path,
                &self.vault,
                &self.jobs,
                &self.semantic,
                id("jobId")?,
            )?,
            "notifications.list" => jobs::notifications(
                &self.db_path,
                input
                    .get("unreadOnly")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            )?,
            "notifications.read" => {
                jobs::update_notification(&self.db_path, id("notificationId")?, false)?
            }
            "notifications.dismiss" => {
                jobs::update_notification(&self.db_path, id("notificationId")?, true)?
            }
            "solution.conflict.resolve" => {
                workflow::resolve_conflict_review(&self.db_path, id("runId")?, input)?
            }
            _ => return Err(format!("Native operation is not implemented: {name}")),
        };
        let status = match name {
            "capture.create"
            | "capture.promote"
            | "solution.create"
            | "solution.progress.add"
            | "solution.comment.add"
            | "solution.checklist.add"
            | "solution.follow_up"
            | "solution.completion.create"
            | "solution.patch.create"
            | "solution.lineage.regenerate"
            | "solution.lineage.correct"
            | "item.project"
            | "compass.goal.create" => 201,
            "problem.approve"
            | "solution.approve"
            | "solution.stage.save"
            | "solution.checklist.update"
            | "solution.completion.verify"
            | "solution.patch.apply"
            | "solution.patch.undo"
            | "item.delete"
            | "item.restore"
            | "item.update"
            | "item.localization.save"
            | "item.archive"
            | "problem.playbook.delete"
            | "workbench.category.save"
            | "workbench.importance.save" => 204,
            _ => 200,
        };
        Ok((status, result))
    }
}

fn error_status(error: &str) -> u16 {
    let normalized = error.to_ascii_lowercase();
    if normalized.contains("not found") || normalized.contains("no longer available") {
        404
    } else if normalized.contains("changed")
        || normalized.contains("modified outside")
        || normalized.contains("modified externally")
        || normalized.contains("cannot be")
        || normalized.contains("reload")
        || normalized.contains("already")
    {
        409
    } else if normalized.contains("provider") || normalized.contains("temporarily unavailable") {
        502
    } else {
        400
    }
}
