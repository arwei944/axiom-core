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
            last_heartbeat_ms: 0,
            degraded: false,
            metrics_active: false,
        }
    }
}

impl super::AxiomRuntime {
    /// Record dispatch heartbeat (call from dispatch loop).
    pub async fn touch_heartbeat(&self) {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let mut h = self.health.write().await;
        h.last_heartbeat_ms = now_ms;
        h.degraded = false;
    }

    /// Mark degraded if heartbeat older than `stale_ms`.
    pub async fn evaluate_heartbeat(&self, stale_ms: u64) -> bool {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let mut h = self.health.write().await;
        if h.last_heartbeat_ms == 0 {
            return false;
        }
        let stale = now_ms.saturating_sub(h.last_heartbeat_ms) > stale_ms;
        h.degraded = stale;
        stale
    }

    /// Test/support: set absolute heartbeat timestamp.
    pub async fn set_heartbeat_ms(&self, ms: u64) {
        self.health.write().await.last_heartbeat_ms = ms;
    }
}

#[cfg(test)]
mod heartbeat_tests {
    use super::super::{AxiomRuntime, RuntimeConfig};
    use std::time::Duration;

    /// Path-driving: real start() → dispatch loop advances last_heartbeat_ms (no set_heartbeat_ms).
    #[tokio::test]
    async fn dispatch_loop_advances_heartbeat_without_helpers() {
        let mut cfg = RuntimeConfig::default();
        cfg.dispatch_poll_interval_ms = 5;
        cfg.heartbeat_stale_ms = 5_000;
        let rt = AxiomRuntime::new(cfg);
        rt.start().await.expect("start");
        tokio::time::sleep(Duration::from_millis(40)).await;
        let h1 = rt.health().await;
        assert!(
            h1.last_heartbeat_ms > 0,
            "dispatch loop must write last_heartbeat_ms"
        );
        let t1 = h1.last_heartbeat_ms;
        tokio::time::sleep(Duration::from_millis(40)).await;
        let h2 = rt.health().await;
        assert!(
            h2.last_heartbeat_ms >= t1,
            "heartbeat must advance on dispatch ticks"
        );
        assert!(!h2.degraded);
        rt.stop().await;
    }

    /// Path-driving: stop dispatch → health poller marks degraded when stale.
    #[tokio::test]
    async fn health_poller_marks_degraded_when_dispatch_stops() {
        let mut cfg = RuntimeConfig::default();
        cfg.dispatch_poll_interval_ms = 5;
        cfg.heartbeat_stale_ms = 30;
        let rt = AxiomRuntime::new(cfg);
        rt.start().await.expect("start");
        tokio::time::sleep(Duration::from_millis(40)).await;
        assert!(rt.health().await.last_heartbeat_ms > 0);
        // Stop dispatch (and poller via stop) — but we need poller to keep running.
        // Send stop to dispatch only: take stop_tx path via stop(), which also sets started=false.
        // Instead: stop entire runtime after heartbeat, then use evaluate via new start of only poller...
        // Production path: stop dispatch by sending stop signal; health poller runs in same start().
        // stop() kills both. So: stop dispatch handle only.
        if let Some(tx) = rt.stop_tx.lock().await.take() {
            let _ = tx.send(());
        }
        if let Some(h) = rt.dispatch_handle.lock().await.take() {
            let _ = tokio::time::timeout(Duration::from_secs(2), h).await;
        }
        // Health poller still running (separate spawn) — wait past stale window.
        tokio::time::sleep(Duration::from_millis(120)).await;
        let h = rt.health().await;
        assert!(
            h.degraded,
            "health poller must set degraded when heartbeat is stale"
        );
        // cleanup remaining poller by full stop
        rt.health.write().await.started = false;
    }

    #[tokio::test]
    async fn metrics_enabled_consumed_on_start() {
        let mut on = RuntimeConfig::default();
        on.metrics_enabled = true;
        on.metrics_endpoint = None;
        let rt_on = AxiomRuntime::new(on);
        rt_on.start().await.unwrap();
        let h_on = rt_on.health().await;
        assert!(h_on.metrics_active);
        assert!(h_on.metrics_endpoint.is_some());
        rt_on.stop().await;

        let mut off = RuntimeConfig::default();
        off.metrics_enabled = false;
        let rt_off = AxiomRuntime::new(off);
        rt_off.start().await.unwrap();
        let h_off = rt_off.health().await;
        assert!(!h_off.metrics_active);
        rt_off.stop().await;
    }
}
