use crate::application::work_tracking_service::WorkTrackingApplicationService;
use std::time::Duration;

pub async fn run(service: WorkTrackingApplicationService) {
    loop {
        match service.drain(100) {
            Ok(processed) if processed > 0 => continue,
            Ok(_) => tokio::time::sleep(Duration::from_millis(250)).await,
            Err(error) => {
                eprintln!("work-tracking projector error: {}", error.code);
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }
    }
}
