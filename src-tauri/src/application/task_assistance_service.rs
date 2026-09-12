use crate::native::{database, semantic::SemanticEngine, task_assistance};
use serde_json::Value;
use std::path::PathBuf;

/// Closed assistance operations available to an authorized application-service caller.
/// MCP conflict synthesis stays in the current Chat, so this enum cannot start a review model.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TaskAssistanceAction {
    RefinementOpen,
    RefinementGet,
    RefinementMessage,
    RefinementWorkspace,
    RefinementProposals,
    RefinementDecision,
    ReviewGet,
    ReviewHistory,
    ReviewCancel,
    ReviewDecision,
    Lineage,
    TaskLineage,
    KnowledgeDraft,
    KnowledgeCorrection,
    KnowledgeRegenerate,
    KnowledgePublish,
    KnowledgeWithdraw,
}

impl TaskAssistanceAction {
    fn operation(self) -> &'static str {
        match self {
            Self::RefinementOpen => "task-refinement.open",
            Self::RefinementGet => "task-refinement.get",
            Self::RefinementMessage => "task-refinement.message",
            Self::RefinementWorkspace => "task-refinement.workspace",
            Self::RefinementProposals => "task-refinement.proposals",
            Self::RefinementDecision => "task-refinement.decision",
            Self::ReviewGet => "task-review.get",
            Self::ReviewHistory => "task-review.history",
            Self::ReviewCancel => "task-review.cancel",
            Self::ReviewDecision => "task-review.decision",
            Self::Lineage | Self::TaskLineage => "task.lineage",
            Self::KnowledgeDraft => "task-knowledge.draft",
            Self::KnowledgeCorrection => "task-knowledge.correction",
            Self::KnowledgeRegenerate => "task-knowledge.regenerate",
            Self::KnowledgePublish => "task-knowledge.publish",
            Self::KnowledgeWithdraw => "task-knowledge.withdraw",
        }
    }
}

#[derive(Clone)]
pub(crate) struct TaskAssistanceApplicationService {
    db_path: PathBuf,
    settings_path: PathBuf,
    vault_root: PathBuf,
    semantic: SemanticEngine,
}

impl TaskAssistanceApplicationService {
    pub(crate) fn new(
        db_path: PathBuf,
        settings_path: PathBuf,
        vault_root: PathBuf,
        semantic: SemanticEngine,
    ) -> Self {
        Self {
            db_path,
            settings_path,
            vault_root,
            semantic,
        }
    }

    /// Authorization and exact review belong to the calling application service, before dispatch.
    /// Dependencies stay in the GUI owner; a stdio child cannot construct a persistence adapter.
    pub(crate) async fn execute(
        &self,
        action: TaskAssistanceAction,
        input: &Value,
    ) -> Result<Value, String> {
        // These operations share the same SQLite transaction as their immutable Task
        // lineage/advisory rows. They deliberately never dispatch a provider job.
        match action {
            TaskAssistanceAction::TaskLineage => {
                return self.transaction(|tx| {
                    task_assistance::task_lineage_tx(tx, required(input, "taskId")?)
                });
            }
            TaskAssistanceAction::Lineage => {
                return self.transaction(|tx| {
                    task_assistance::knowledge_lineage_tx(
                        tx,
                        required(input, "taskId")?,
                        input.get("expectedTaskRevision").and_then(Value::as_i64),
                    )
                });
            }
            TaskAssistanceAction::KnowledgeDraft if input.get("bodyMarkdown").is_some() => {
                let body = required(input, "bodyMarkdown")?;
                return self.transaction(|tx| {
                    task_assistance::save_supplied_knowledge_draft_tx(tx, input, body, "supplied")
                });
            }
            TaskAssistanceAction::ReviewGet => {
                return self.transaction(|tx| {
                    task_assistance::current_chat_advisory_get_tx(tx, required(input, "runId")?)
                });
            }
            TaskAssistanceAction::ReviewHistory => {
                return self.transaction(|tx| {
                    task_assistance::current_chat_advisory_history_tx(
                        tx,
                        required(input, "taskId")?,
                    )
                });
            }
            TaskAssistanceAction::ReviewCancel => {
                return self
                    .transaction(|tx| task_assistance::cancel_current_chat_advisory_tx(tx, input));
            }
            TaskAssistanceAction::ReviewDecision => {
                return self
                    .transaction(|tx| task_assistance::decide_current_chat_advisory_tx(tx, input));
            }
            _ => {}
        }
        task_assistance::execute(
            &self.db_path,
            &self.settings_path,
            &self.vault_root,
            self.semantic.clone(),
            action.operation(),
            input,
        )
        .await
    }

    /// Governed persistence uses the review owner's transaction, so failure cannot consume
    /// consent and no child mutation can race between freshness validation and application.
    pub(crate) fn execute_reviewed_tx(
        &self,
        tx: &rusqlite::Transaction<'_>,
        action: TaskAssistanceAction,
        input: &Value,
    ) -> Result<Value, String> {
        match action {
            TaskAssistanceAction::RefinementDecision => {
                task_assistance::refinement_decision_tx(tx, input)
            }
            TaskAssistanceAction::ReviewDecision => {
                task_assistance::decide_current_chat_advisory_tx(tx, input)
            }
            TaskAssistanceAction::KnowledgeCorrection => {
                task_assistance::knowledge_correction_tx(tx, input)
            }
            TaskAssistanceAction::KnowledgeDraft => {
                task_assistance::save_supplied_knowledge_draft_tx(
                    tx,
                    input,
                    required(input, "bodyMarkdown")?,
                    "supplied",
                )
            }
            _ => Err("invalid_input: unsupported reviewed persistence action".into()),
        }
    }

    pub(crate) fn create_current_chat_advisory(&self, input: &Value) -> Result<Value, String> {
        self.transaction(|tx| task_assistance::create_current_chat_advisory_tx(tx, input))
    }

    pub(crate) fn complete_current_chat_advisory(
        &self,
        input: &Value,
        findings: &Value,
        evidence_refs: &Value,
    ) -> Result<Value, String> {
        self.transaction(|tx| {
            task_assistance::complete_current_chat_advisory_tx(tx, input, findings, evidence_refs)
        })
    }

    pub(crate) async fn prepare_knowledge_draft(&self, input: &Value) -> Result<Value, String> {
        task_assistance::prepare_knowledge_draft(&self.db_path, &self.settings_path, input).await
    }

    fn transaction<T>(
        &self,
        operation: impl FnOnce(&rusqlite::Transaction<'_>) -> Result<T, String>,
    ) -> Result<T, String> {
        let mut connection = database::open(&self.db_path)?;
        let tx = connection
            .transaction()
            .map_err(|error| error.to_string())?;
        let value = operation(&tx)?;
        tx.commit().map_err(|error| error.to_string())?;
        Ok(value)
    }
}

fn required<'a>(input: &'a Value, key: &str) -> Result<&'a str, String> {
    input
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("invalid_input: {key} is required"))
}
