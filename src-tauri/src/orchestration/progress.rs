use serde::Serialize;
use tauri::{AppHandle, Emitter};

/// Payload emitted on the `job:progress` channel.
#[derive(Debug, Clone, Serialize)]
pub struct ProgressEvent {
    pub job_id: String,
    pub phase: String,
    pub progress: f64,
    pub message: String,
}

/// Helper that wraps an `AppHandle` and a job ID for emitting progress events.
pub struct ProgressReporter {
    app_handle: AppHandle,
    pub job_id: String,
}

impl ProgressReporter {
    pub fn new(app_handle: AppHandle, job_id: String) -> Self {
        Self { app_handle, job_id }
    }

    pub fn report(&self, phase: &str, progress: f64, message: &str) {
        let event = ProgressEvent {
            job_id: self.job_id.clone(),
            phase: phase.to_string(),
            progress,
            message: message.to_string(),
        };
        let _ = self.app_handle.emit("job:progress", &event);
    }
}
