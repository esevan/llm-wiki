use crate::domain::work_tracking_state::AppError;

pub trait WorkProjection: Send + Sync {
    fn drain(&self, limit: usize) -> Result<usize, AppError>;
}
