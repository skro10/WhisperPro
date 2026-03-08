#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub audio_ready: bool,
    pub inference_ready: bool,
}

impl Default for HealthStatus {
    fn default() -> Self {
        Self {
            audio_ready: false,
            inference_ready: false,
        }
    }
}

pub fn current_health() -> HealthStatus {
    HealthStatus::default()
}
