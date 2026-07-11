use super::RuntimeConfig;

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            mailbox_capacity: 1024,
            entropy_threshold: 100.0,
            entropy_cooldown_ms: 60_000,
            dispatch_poll_interval_ms: 10,
            metrics_endpoint: None,
            telemetry_enabled: false,
            dlq_capacity: 1000,
            api_endpoint: None,
        }
    }
}
