use super::AxiomRuntime;
use super::CellRegistration;
use super::RegisteredCell;
use crate::constraint_validator::{ConstraintValidator, ValidationContext};
use crate::entropy_interceptors::{EmergencyInterceptor, ThrottleInterceptor};
use crate::guardian::ArchitectureGuardian;
use crate::interceptors::{
    CapabilityVersionInterceptor, GuardInterceptor, HopLimitInterceptor, IdempotencyInterceptor,
    LoopDetectInterceptor, SchemaVersionInterceptor,
};
use axiom_kernel::id::CellId;
use axiom_kernel::KernelError;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{oneshot, Mutex as TokioMutex};

impl AxiomRuntime {
    pub async fn register_cell(
        &self,
        reg: CellRegistration,
    ) -> Result<Arc<crate::mailbox::Mailbox>, KernelError> {
        let mailbox = Arc::new(crate::mailbox::Mailbox::new(self.config.mailbox_capacity));
        self.bus.register_cell(&reg.id, mailbox.clone(), reg.layer).await;
        self.supervisor.register_cell(reg.id.as_str(), reg.supervision_strategy).await;

        let cell = reg.cell.map(|c| Arc::new(TokioMutex::new(c)));
        self.cells.write().await.push(RegisteredCell {
            id: reg.id.clone(),
            mailbox: mailbox.clone(),
            layer: reg.layer,
            version: reg.version,
            cell,
            factory: reg.factory,
        });
        Ok(mailbox)
    }

    pub async fn mailbox_for(&self, cell_id: &str) -> Option<Arc<crate::mailbox::Mailbox>> {
        let cells = self.cells.read().await;
        cells.iter().find(|c| c.id.as_str() == cell_id).map(|c| c.mailbox.clone())
    }

    async fn preflight(&self) -> Result<(), Vec<String>> {
        let mut issues = Vec::new();

        if let Err(gaps) = axiom_kernel::registry::verify_migration_chain_completeness(1) {
            issues.extend(gaps);
        }

        let cells = self.cells.read().await;
        for reg in cells.iter() {
            if reg.version.major != 0 {
                issues.push(format!(
                    "cell {} has non-zero major version {}: not 0.x pre-release",
                    reg.id.as_str(),
                    reg.version.major
                ));
            }
        }

        if issues.is_empty() {
            Ok(())
        } else {
            Err(issues)
        }
    }

    pub async fn start(&self) -> Result<(), KernelError> {
        match self.preflight().await {
            Ok(()) => {
                tracing::info!("preflight passed");
            }
            Err(issues) => {
                for i in &issues {
                    tracing::error!("preflight: {i}");
                }
                return Err(KernelError::InternalError(format!(
                    "preflight failed: {}",
                    issues.join("; ")
                )));
            }
        }

        let guardian = Arc::new(ArchitectureGuardian::new());
        self.bus.register_interceptor(guardian).await;

        let throttle = Arc::new(ThrottleInterceptor::new(self.throttle_state.clone()));
        self.bus.register_interceptor(throttle).await;

        let emergency = Arc::new(EmergencyInterceptor::new(self.emergency_mode.clone()));
        self.bus.register_interceptor(emergency).await;

        if self.auto_interceptors.load(std::sync::atomic::Ordering::Relaxed) {
            self.bus.register_interceptor(Arc::new(HopLimitInterceptor::default())).await;
            self.bus.register_interceptor(Arc::new(IdempotencyInterceptor::default())).await;
            self.bus.register_interceptor(Arc::new(SchemaVersionInterceptor)).await;
            self.bus.register_interceptor(Arc::new(LoopDetectInterceptor::default())).await;
            self.bus
                .register_interceptor(Arc::new(CapabilityVersionInterceptor::new(
                    ConstraintValidator::new(ValidationContext::default()),
                )))
                .await;
            self.bus.register_interceptor(Arc::new(GuardInterceptor)).await;
        }

        let (tx, rx) = oneshot::channel::<()>();
        *self.stop_tx.lock().await = Some(tx);

        let cells_count = self.cells.read().await.len();
        {
            let mut h = self.health.write().await;
            h.started = true;
            h.preflight_passed = true;
            h.cells_running = cells_count as u64;
        }

        let bus = self.bus.clone();
        let supervisor = self.supervisor.clone();
        let governor = self.governor.clone();
        let witness_store = self.witness_store.clone();
        let snapshot_store = self.snapshot_store.clone();
        let throttle_state = self.throttle_state.clone();
        let emergency_mode = self.emergency_mode.clone();
        let dlq = self.dlq.clone();
        let events_since_snapshot = self.events_since_snapshot.clone();
        let poll_interval = self.config.dispatch_poll_interval_ms;
        let cell_kernel = Some(self.kernel_bridge.cell_kernel.clone());

        let cells_data: Vec<_> = self
            .cells
            .read()
            .await
            .iter()
            .map(|r| (r.mailbox.clone(), r.id.clone(), r.layer, r.cell.clone(), r.factory.clone()))
            .collect();
        let cells_len = cells_data.len();
        let cell_ids: Vec<CellId> =
            cells_data.iter().map(|(_, cid, _, _, _)| cid.clone()).collect();

        let handle = tokio::spawn(crate::dispatch::dispatch_loop(
            rx,
            poll_interval,
            cells_data,
            bus,
            supervisor,
            governor,
            witness_store,
            snapshot_store,
            throttle_state,
            emergency_mode,
            dlq,
            events_since_snapshot,
            cell_kernel,
        ));

        *self.dispatch_handle.lock().await = Some(handle);

        let bus_h = self.bus.clone();
        let gov_h = self.governor.clone();
        let sup_h = self.supervisor.clone();
        let config_h = self.config.clone();
        let witness_store_h = self.witness_store.clone();
        let snapshot_store_h = self.snapshot_store.clone();
        let health = self.health.clone();
        let cell_ids_h = cell_ids.clone();
        tokio::spawn(async move {
            let start = Instant::now();
            let mut tick = tokio::time::interval(Duration::from_secs(1));
            loop {
                tick.tick().await;
                let mut h = health.write().await;
                h.uptime_ms = start.elapsed().as_millis() as u64;
                h.messages_delivered = bus_h.delivered_count();
                h.messages_rejected = bus_h.rejected_count();
                h.entropy_score = gov_h.snapshot().global.value;
                h.metrics_endpoint = config_h.metrics_endpoint.clone();
                h.telemetry_enabled = config_h.telemetry_enabled;
                h.store_connected = witness_store_h.read().await.is_some();
                h.snapshot_store_connected = snapshot_store_h.read().await.is_some();
                let mut total = 0u64;
                for cid in &cell_ids_h {
                    total += sup_h.restart_count(cid.as_str()).await;
                }
                h.total_restarts = total;
                h.cells_running = cells_len as u64;
            }
        });

        tracing::info!("runtime started with {cells_count} cells");
        Ok(())
    }

    pub async fn stop(&self) {
        if let Some(tx) = self.stop_tx.lock().await.take() {
            let _ = tx.send(());
        }
        if let Some(h) = self.dispatch_handle.lock().await.take() {
            let _ = tokio::time::timeout(Duration::from_secs(5), h).await;
        }
        self.health.write().await.started = false;
        tracing::info!("runtime stopped");
    }
}
