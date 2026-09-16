mod completion;
pub(crate) mod conversation_context;
pub(crate) mod database;
mod job_results;
pub mod jobs;
pub(crate) mod lineage;
pub(crate) mod localization;
mod migrations;
mod patches;
mod projection;
mod refinement;
pub(crate) mod semantic;
pub mod settings;
pub(crate) mod task_assistance;
pub(crate) mod task_hierarchy;
pub(crate) mod vault;
pub(crate) mod work_tracking;
pub(crate) mod work_tracking_projector;
mod workbench;
pub mod workflow;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct NativeApplication {
    db_path: PathBuf,
    settings_path: PathBuf,
    vault: PathBuf,
    vault_setup_required: bool,
    semantic: semantic::SemanticEngine,
    jobs: jobs::JobRegistry,
    work_tracking: crate::application::work_tracking_service::WorkTrackingApplicationService,
    task_service: crate::application::task_service::TaskApplicationService,
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
        let settings_path = dirs::home_dir()
            .ok_or("The user home directory is unavailable")?
            .join(".llm-workbench/settings.json");
        Self::build(vault, db_path, settings_path, embedding_model_dir, false)
    }

    pub fn with_vault_setup(
        vault: PathBuf,
        db_path: PathBuf,
        settings_path: PathBuf,
        embedding_model_dir: Option<PathBuf>,
        vault_setup_required: bool,
    ) -> Result<Self, String> {
        Self::build(
            vault,
            db_path,
            settings_path,
            embedding_model_dir,
            vault_setup_required,
        )
    }

    fn build(
        vault: PathBuf,
        db_path: PathBuf,
        settings_path: PathBuf,
        embedding_model_dir: Option<PathBuf>,
        vault_setup_required: bool,
    ) -> Result<Self, String> {
        if !vault_setup_required {
            std::fs::create_dir_all(&vault).map_err(|error| error.to_string())?;
        }
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        // A failed migration leaves a verified recovery marker. Construct only
        // recovery-capable state until the person explicitly restores or retries.
        let recovery_pending = database::migration_failure(&db_path)?.is_some();
        if !recovery_pending {
            database::initialize(&db_path)?;
            task_assistance::recover_interrupted_refinement(&db_path)?;
        }
        let semantic = semantic::SemanticEngine::new(embedding_model_dir);
        let store = crate::adapters::sqlite::SqliteWorkTrackingStore::new(&db_path);
        if !recovery_pending {
            store
                .ensure_native_connection()
                .map_err(|error| error.message.clone())?;
        }
        let vault_adapter =
            crate::adapters::vault::MarkdownVaultAdapter::new(&db_path, &vault, semantic.clone());
        let jobs = jobs::JobRegistry::default();
        let task_assistance =
            crate::application::task_assistance_service::TaskAssistanceApplicationService::new(
                db_path.clone(),
                settings_path.clone(),
                vault.clone(),
                semantic.clone(),
            )
            .with_job_registry(jobs.clone());
        if !recovery_pending && !vault_setup_required { let _ = task_hierarchy::publish_pending(&db_path,&vault); }
        Ok(Self {
            db_path: db_path.clone(),
            settings_path,
            vault,
            vault_setup_required,
            semantic,
            jobs,
            work_tracking:
                crate::application::work_tracking_service::WorkTrackingApplicationService::new(
                    store,
                    vault_adapter,
                    task_assistance,
                ),
            task_service: crate::application::task_service::TaskApplicationService::new(&db_path),
        })
    }

    pub fn execute_work_tracking(&self, operation: NativeOperation) -> NativeResponse {
        if let Some(response) = self.recovery_block() {
            return response;
        }
        // finish_advance owns publication after an accepted completion. Ordinary
        // checkpoints and reads do not need another database/outbox round trip.
        work_tracking::execute(&self.work_tracking, operation)
    }

    pub fn work_tracking_service(
        &self,
    ) -> crate::application::work_tracking_service::WorkTrackingApplicationService {
        self.work_tracking.clone()
    }

    pub fn isolated(vault: &Path, db_path: &Path) -> Result<Self, String> {
        let settings_path = db_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("settings.json");
        Self::build(
            vault.to_owned(),
            db_path.to_owned(),
            settings_path,
            None,
            false,
        )
    }

    #[cfg(test)]
    pub fn isolated_with_model(
        vault: &Path,
        db_path: &Path,
        embedding_model_dir: &Path,
    ) -> Result<Self, String> {
        let settings_path = db_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("settings.json");
        Self::build(
            vault.to_owned(),
            db_path.to_owned(),
            settings_path,
            Some(embedding_model_dir.to_owned()),
            false,
        )
    }

    pub fn db_path(&self) -> PathBuf {
        self.db_path.clone()
    }

    pub fn vault_path(&self) -> PathBuf {
        self.vault.clone()
    }

    pub fn settings_path(&self) -> PathBuf {
        self.settings_path.clone()
    }

    pub fn vault_setup_status(&self) -> Result<Value, String> {
        Ok(json!({
            "required": self.vault_setup_required,
            "introRequired": settings::intro_required(&self.settings_path)?,
            "path": if self.vault_setup_required { Value::Null } else { Value::String(self.vault.to_string_lossy().into_owned()) }
        }))
    }

    pub fn migration_recovery_status(&self) -> Result<Value, String> {
        Ok(json!({"recovery": database::migration_failure(&self.db_path)?}))
    }

    fn recovery_block(&self) -> Option<NativeResponse> {
        match database::migration_failure(&self.db_path) {
            Ok(Some(recovery)) => Some(NativeResponse {
                status: 503,
                body: json!({"detail":"migration_recovery_required","recovery":recovery}),
            }),
            Ok(None) => None,
            Err(error) => Some(NativeResponse {
                status: 503,
                body: json!({"detail":"migration_recovery_unavailable","error":error}),
            }),
        }
    }

    pub fn restore_migration_backup(&self, manifest_file: &str) -> Result<Value, String> {
        let marker = database::migration_failure(&self.db_path)?
            .ok_or("No migration recovery is pending")?;
        let expected = marker
            .get("manifestFile")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or("No verified migration backup is available")?;
        let requested = Path::new(manifest_file);
        if requested.components().count() != 1
            || requested.file_name().and_then(|name| name.to_str()) != Some(expected)
        {
            return Err("Invalid migration recovery manifest".into());
        }
        let parent = self.db_path.parent().unwrap_or_else(|| Path::new("."));
        database::restore_verified_backup(&self.db_path, &parent.join(expected))?;
        database::initialize(&self.db_path)?;
        Ok(json!({"recovered":true,"action":"restore"}))
    }

    pub fn retry_migration(&self) -> Result<Value, String> {
        if database::migration_failure(&self.db_path)?.is_none() {
            return Err("No migration recovery is pending".into());
        }
        database::initialize(&self.db_path)?;
        Ok(json!({"recovered":true,"action":"retry"}))
    }

    pub fn complete_first_run_intro(&self) -> Result<(), String> {
        settings::complete_intro(&self.settings_path)
    }

    pub fn save_vault_selection(&self, path: &Path) -> Result<(), String> {
        if !path.is_absolute() || !path.is_dir() {
            return Err("Choose an existing Vault folder".into());
        }
        let selected = path.canonicalize().map_err(|error| error.to_string())?;
        settings::save_vault_path(&self.settings_path, &selected)
    }

    fn desktop_e2e_failure(&self, operation_name: &str) -> Option<NativeResponse> {
        if std::env::var_os("LLM_WIKI_E2E_RESULT").is_some()
            && crate::native::database::open(&self.db_path)
                .and_then(|connection| {
                    connection
                        .execute(
                            "DELETE FROM desktop_e2e_failures WHERE operation=?",
                            [operation_name],
                        )
                        .map_err(|error| error.to_string())
                })
                .unwrap_or(0)
                > 0
        {
            return Some(NativeResponse {
                status: 503,
                body: json!({"detail":"deterministic one-shot desktop E2E failure"}),
            });
        }
        None
    }

    pub fn execute(&self, operation: NativeOperation) -> NativeResponse {
        if let Some(response) = self
            .recovery_block()
            .or_else(|| self.desktop_e2e_failure(&operation.name))
        {
            return response;
        }
        // Completion enqueues parent publications; opening Task detail retries failures.
        // Unrelated capture/workbench requests must not open and scan the outbox twice.
        let publish_subtasks = !self.vault_setup_required
            && matches!(
                operation.name.as_str(),
                "task.completion.create" | "task.get"
            );
        if publish_subtasks {
            let _ = task_hierarchy::publish_pending(&self.db_path, &self.vault);
        }
        let result = self.dispatch(&operation.name, &operation.input);
        if publish_subtasks {
            let _ = task_hierarchy::publish_pending(&self.db_path, &self.vault);
        }
        match result {
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
                    | "task"
                    | "work-log"
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
        if let Some(response) = self.recovery_block().or_else(|| self.desktop_e2e_failure("jobs.enqueue")) {
            return response;
        }
        match jobs::enqueue(
            self.db_path.clone(),
            self.settings_path.clone(),
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
        if let Some(response) = self
            .recovery_block()
            .or_else(|| self.desktop_e2e_failure(&operation.name))
        {
            return response;
        }
        let name = operation.name.clone();
        let input = operation.input.clone();
        if name.starts_with("task-refinement.")
            || name.starts_with("task-review.")
            || name.starts_with("task-knowledge.")
            || name == "task.lineage"
        {
            return match task_assistance::execute_with_registry(
                &self.db_path,
                &self.settings_path,
                &self.vault,
                self.semantic.clone(),
                self.jobs.clone(),
                &name,
                &input,
            )
            .await
            {
                Ok(body) => NativeResponse { status: 200, body },
                Err(error) => NativeResponse {
                    status: error_status(&error),
                    body: json!({"detail":error}),
                },
            };
        }
        let mut response = self.execute_domain("workflow", operation);
        if !(200..300).contains(&response.status) {
            return response;
        }
        if name == "task.work-log.create" && input["attachment"]["mediaType"].as_str().is_some_and(|media| media.starts_with("image/")) && input["attachment"]["data"].as_str().is_some_and(|data| !data.is_empty()) {
            let entry_id = response.body["id"].as_str().unwrap_or("");
            let queued = self.enqueue_job(json!({"taskKind":"image_summary","entityType":"task_work_log_entries","entityId":entry_id,"automatic":true,"locale":input.get("locale").and_then(Value::as_str).unwrap_or("en")})).await;
            if queued.status < 300 { response.body["imageSummaryJob"] = queued.body; }
            else { response.body["imageSummaryQueueError"] = queued.body["detail"].clone(); }
        }

        let derived = match name.as_str() {
            "capture.create" => Some(("captures", "text")),
            "solution.progress.add" => Some(("solution_progress_entries", "body")),
            "solution.comment.add" => Some(("solution_progress_comments", "body")),
            "solution.checklist.add" => Some(("solution_checklist_items", "body")),
            _ => None,
        };
        if let Some((entity_type, field)) = derived {
            let source = input
                .get(if name == "capture.create" {
                    "text"
                } else {
                    "body"
                })
                .and_then(Value::as_str)
                .unwrap_or("");
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
        response
    }

    fn dispatch(&self, name: &str, input: &Value) -> Result<(u16, Value), String> {
        if matches!(
            name,
            "board.get"
                | "capture.promote"
                | "problem.approve"
                | "solution.create"
                | "solution.approve"
                | "solution.stage.save"
                | "solution.progress.get"
                | "solution.progress.add"
                | "solution.comment.add"
                | "solution.checklist.add"
                | "solution.checklist.update"
                | "solution.follow_up"
                | "problem.complete"
                | "solution.completion.create"
                | "solution.completion.verify"
                | "solution.conflict.resolve"
        ) {
            return Err(format!("Native operation is not implemented: {name}"));
        }
        let id = |key: &str| {
            input
                .get(key)
                .and_then(Value::as_str)
                .ok_or_else(|| format!("{key} is required"))
        };
        let result = match name {
            "workbench.get"
            | "capture.create"
            | "task.create"
            | "task.get"
            | "task.delete"
            | "task.revision"
            | "task.transition"
            | "task.problem-link.create"
            | "task.problem-link.delete"
            | "task.relationship.create"
            | "task.relationship.delete"
            | "task.readiness.get"
            | "task.readiness.decision"
            | "task.work-log.get"
            | "task.work-log.create"
            | "work-log.comment.create"
            | "task.checklist.create"
            | "task.checklist.update"
            | "task.decision.create"
            | "task.completion.create"
            | "problem.create"
            | "problem.revision"
            | "problem.resolution.create" => self.task_service.execute(name, input)?,
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
                &self.settings_path,
                input
                    .get("browserLocale")
                    .and_then(Value::as_str)
                    .unwrap_or("en"),
            )?,
            "locale.save" => settings::save_locale(&self.settings_path, input)?,
            "i18n.get" => settings::resources(id("locale")?)?,
            "provider.get" => settings::provider(&self.settings_path)?,
            "provider.save" => settings::save_provider(&self.settings_path, input)?,
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
            "problem.complete" => completion::complete(&self.db_path, id("problemId")?, input)?,
            "problem.playbook.delete" => completion::remove(
                &self.db_path,
                &self.vault,
                id("problemId")?,
                input.get("force").and_then(Value::as_bool).unwrap_or(false),
            )?,
            "problem.playbook.publish" => {
                completion::publish_playbook(&self.db_path, &self.vault, id("problemId")?, input)?
            }
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
                &self.settings_path,
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
    } else if normalized.contains("conflict")
        || normalized.contains("changed")
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

#[cfg(test)]
mod recovery_tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn failed_migration_starts_read_only_until_an_explicit_retry() {
        let root = tempdir().unwrap();
        let db = root.path().join("state.sqlite3");
        std::fs::write(
            root.path().join("state.sqlite3.migration-failure.json"),
            json!({"stage":"schema_migration","safeError":"migration_failed","manifestFile":null,"retryAvailable":true,"restoreAvailable":false,"createdAt":"2026-09-05T00:00:00Z"}).to_string(),
        ).unwrap();
        let app = NativeApplication::isolated(&root.path().join("vault"), &db).unwrap();
        assert!(app.migration_recovery_status().unwrap()["recovery"].is_object());
        assert_eq!(
            app.execute(NativeOperation {
                name: "task.create".into(),
                input: json!({"operationId":"blocked","inputText":"blocked","title":"blocked"})
            })
            .status,
            503
        );
        assert_eq!(app.retry_migration().unwrap()["recovered"], true);
        let ready = app.execute(NativeOperation {
            name: "task.create".into(),
            input: json!({"operationId":"ready","inputText":"ready","title":"ready"}),
        });
        assert_eq!(ready.status, 200);
        assert_eq!(ready.body["state"], "task");
    }
}
