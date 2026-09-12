use crate::domain::work_tracking_state::AppError;
use serde_json::Value;

pub trait WorkflowRepository: Send + Sync {
    fn current_workbench(&self, connection_id: &str, limit: usize) -> Result<Value, AppError>;
    fn overview(
        &self,
        connection_id: &str,
        limit: usize,
        offset: usize,
        attention_offset: usize,
    ) -> Result<Value, AppError>;
    fn topic(&self, topic_id: &str, limit: usize) -> Result<Value, AppError>;
}
