use crate::domain::work_tracking_state::AppError;
use serde_json::Value;

pub trait WorkflowRepository: Send + Sync {
    fn current_workbench(&self, limit: usize) -> Result<Value, AppError>;
    fn overview(&self, limit: usize, offset: usize) -> Result<Value, AppError>;
    fn topic(&self, topic_id: &str, limit: usize) -> Result<Value, AppError>;
}
