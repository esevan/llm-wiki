use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionStatus {
    PendingReview,
    Queued,
    Claimed,
    WaitingSolution,
    Applied,
    IgnoredLate,
    Conflict,
    Failed,
}

impl ProjectionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PendingReview => "pending_review",
            Self::Queued => "pending",
            Self::Claimed => "claimed",
            Self::WaitingSolution => "waiting_solution",
            Self::Applied => "applied",
            Self::IgnoredLate => "ignored_late",
            Self::Conflict => "conflict",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    Capture,
    ProblemDraft,
    SolutionDraft,
    WorkLogCheckpoint,
    ConflictProposal,
    CompletionProposal,
    WorkCompleted,
    KnowledgeDraftSaved,
    KnowledgePublishRequested,
    KnowledgePublished,
    WorkflowLink,
}

impl EventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Capture => "capture",
            Self::ProblemDraft => "problem_draft",
            Self::SolutionDraft => "solution_draft",
            Self::WorkLogCheckpoint => "work_log_checkpoint",
            Self::ConflictProposal => "conflict_proposal",
            Self::CompletionProposal => "completion_proposal",
            Self::WorkCompleted => "work_completed",
            Self::KnowledgeDraftSaved => "knowledge_draft_saved",
            Self::KnowledgePublishRequested => "knowledge_publish_requested",
            Self::KnowledgePublished => "knowledge_published",
            Self::WorkflowLink => "workflow_link",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub struct EventOrderKey {
    pub occurred_at: String,
    pub source_sequence: i64,
    pub event_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_revision: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conflicting_fields: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revisions: Option<Box<ConflictRevisions>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictRevisions {
    pub session: i64,
    pub entity: Option<i64>,
    pub watermark: Option<i64>,
    pub projection: Option<i64>,
}

impl AppError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            retryable: false,
            current_revision: None,
            conflicting_fields: None,
            revisions: None,
        }
    }

    pub fn conflict(message: impl Into<String>, revision: i64) -> Self {
        Self {
            code: "head_conflict".into(),
            message: message.into(),
            retryable: false,
            current_revision: Some(revision),
            conflicting_fields: Some(vec!["headRevision".into()]),
            revisions: Some(Box::new(ConflictRevisions {
                session: revision,
                entity: None,
                watermark: None,
                projection: None,
            })),
        }
    }
}
