use crate::domain::work_tracking_state::AppError;
use serde_json::Value;

pub trait EventLog: Send + Sync {
    fn session(&self, connection_id: &str, session_id: &str) -> Result<Value, AppError>;
    fn drain_projection_jobs(&self, limit: usize) -> Result<usize, AppError>;
}
