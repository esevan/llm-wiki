use crate::domain::work_tracking_state::AppError;
use serde_json::Value;

pub trait VaultRepository: Send + Sync {
    fn lexical_search(
        &self,
        connection_id: &str,
        scope_kind: &str,
        scope_target: &str,
        query: &str,
        limit: usize,
    ) -> Result<Value, AppError>;
    fn semantic_search(
        &self,
        connection_id: &str,
        scope_kind: &str,
        scope_target: &str,
        query: &str,
        limit: usize,
    ) -> Result<Value, AppError>;
    fn evidence_read(
        &self,
        connection_id: &str,
        evidence_id: &str,
        expected_revision: Option<&str>,
    ) -> Result<Value, AppError>;
    fn publish(
        &self,
        relative_path: &str,
        expected_hash: Option<&str>,
        markdown: &str,
    ) -> Result<String, AppError>;
}
