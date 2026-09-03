mod database;
pub mod jobs;
mod semantic;
pub mod settings;
mod vault;
pub mod workflow;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct NativeApplication {
    db_path: PathBuf,
    vault: PathBuf,
    semantic: semantic::SemanticEngine,
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
        std::fs::create_dir_all(&vault).map_err(|error| error.to_string())?;
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        database::open(&db_path)?;
        Ok(Self {
            db_path,
            vault,
            semantic: semantic::SemanticEngine::new(embedding_model_dir),
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

    pub fn execute(&self, operation: NativeOperation) -> NativeResponse {
        match self.dispatch(&operation.name, &operation.input) {
            Ok((status, body)) => NativeResponse { status, body },
            Err(error) => NativeResponse {
                status: if error.contains("not found") {
                    404
                } else {
                    400
                },
                body: json!({"detail":error}),
            },
        }
    }

    pub fn execute_domain(&self, domain: &str, operation: NativeOperation) -> NativeResponse {
        let belongs_to_domain = match domain {
            "system" => matches!(operation.name.as_str(), "health.get" | "events.list"),
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

    fn dispatch(&self, name: &str, input: &Value) -> Result<(u16, Value), String> {
        let id = |key: &str| {
            input
                .get(key)
                .and_then(Value::as_str)
                .ok_or_else(|| format!("{key} is required"))
        };
        let result = match name {
            "health.get" => vault::health(&self.db_path, &self.semantic)?,
            "vault.index" => vault::index(&self.db_path, &self.vault, &self.semantic)?,
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
            "knowledge.read" => vault::read(&self.vault, id("path")?)?,
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
            "board.get" => workflow::board(&self.db_path)?,
            "capture.promote" => workflow::promote_capture(&self.db_path, id("captureId")?, input)?,
            "problem.approve" => workflow::approve_problem(&self.db_path, id("problemId")?)?,
            "solution.create" => workflow::create_feature(&self.db_path, id("problemId")?, input)?,
            "solution.conflict.save" => {
                workflow::set_conflict(&self.db_path, id("solutionId")?, input)?
            }
            "solution.approve" => workflow::approve_feature(&self.db_path, id("solutionId")?)?,
            "solution.stage.save" => workflow::set_stage(&self.db_path, id("solutionId")?, input)?,
            "solution.progress.get" => workflow::progress(&self.db_path, id("solutionId")?)?,
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
                workflow::complete_problem(&self.db_path, &self.vault, id("problemId")?, input)?
            }
            "solution.lineage" => workflow::lineage(&self.db_path, id("solutionId")?)?,
            "solution.handoff" => workflow::handoff(&self.db_path, id("solutionId")?)?,
            "compass.goal.create" => workflow::create_goal(&self.db_path, input)?,
            "compass.dashboard" => workflow::dashboard(&self.db_path)?,
            "item.delete" => workflow::delete(&self.db_path, id("entityType")?, id("entityId")?)?,
            "item.restore" => workflow::restore(&self.db_path, id("entityType")?, id("entityId")?)?,
            "item.update" => {
                workflow::update_item(&self.db_path, id("entityType")?, id("entityId")?, input)?
            }
            "item.get" => workflow::item(&self.db_path, id("entityType")?, id("entityId")?)?,
            "workbench.category.save" => workflow::set_category(&self.db_path, input)?,
            "workbench.importance.save" => workflow::set_importance(&self.db_path, input)?,
            "refinement.context" => {
                workflow::refinement_context(&self.db_path, id("entityType")?, id("entityId")?)?
            }
            "events.list" => Value::String("event: ready\ndata: indexed\n\n".into()),
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
            "workbench.recent" => json!({"documents":[]}),
            "workbench.completed" => json!({"solutions":[]}),
            "jobs.list" => jobs::list(&self.db_path)?,
            "jobs.events" => Value::String("id: 1\nevent: jobs\ndata: {\"sequence\":1}\n\n".into()),
            "jobs.get" => jobs::get(&self.db_path, id("jobId")?)?,
            "jobs.result" => jobs::result(&self.db_path, id("jobId")?)?,
            "jobs.cancel" => jobs::cancel(&self.db_path, id("jobId")?)?,
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
            | "compass.goal.create" => 201,
            "problem.approve"
            | "solution.approve"
            | "solution.stage.save"
            | "solution.checklist.update"
            | "item.delete"
            | "item.restore"
            | "item.update"
            | "workbench.category.save"
            | "workbench.importance.save" => 204,
            _ => 200,
        };
        Ok((status, result))
    }
}
