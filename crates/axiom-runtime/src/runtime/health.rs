use super::RuntimeHealth;

impl Default for RuntimeHealth {
    fn default() -> Self {
        Self {
            started: false,
            uptime_ms: 0,
            cells_running: 0,
            cells_stopped: 0,
            total_restarts: 0,
            messages_delivered: 0,
            messages_rejected: 0,
            entropy_score: 0.0,
            preflight_passed: false,
            metrics_endpoint: None,
            telemetry_enabled: false,
            store_connected: false,
            snapshot_store_connected: false,
        }
    }
}
