//! T12 — product metrics registry (unified observation counters).
//!
//! Complements RuntimeHealth; exposed on surface `/metrics` and `/api/v1/metrics`.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Process-local commercial path counters (no second observability stack).
#[derive(Debug, Default)]
pub struct ProductMetrics {
    pub tasks_submitted: AtomicU64,
    pub tasks_ok: AtomicU64,
    pub tasks_fail: AtomicU64,
    pub handoffs_submitted: AtomicU64,
    pub handoffs_ok: AtomicU64,
    pub handoffs_rejected: AtomicU64,
    pub governor_allows: AtomicU64,
    pub governor_rejects: AtomicU64,
    pub witnesses_emitted: AtomicU64,
    pub workbench_executions: AtomicU64,
    pub lens_queries: AtomicU64,
    pub plugin_invocations: AtomicU64,
}

pub type SharedMetrics = Arc<ProductMetrics>;

pub fn new_metrics() -> SharedMetrics {
    Arc::new(ProductMetrics::default())
}

impl ProductMetrics {
    pub fn inc_task_ok(&self, witnesses: u64) {
        self.tasks_submitted.fetch_add(1, Ordering::Relaxed);
        self.tasks_ok.fetch_add(1, Ordering::Relaxed);
        self.witnesses_emitted.fetch_add(witnesses, Ordering::Relaxed);
    }

    pub fn inc_task_fail(&self, witnesses: u64) {
        self.tasks_submitted.fetch_add(1, Ordering::Relaxed);
        self.tasks_fail.fetch_add(1, Ordering::Relaxed);
        self.witnesses_emitted.fetch_add(witnesses, Ordering::Relaxed);
    }

    pub fn inc_handoff_ok(&self, witnesses: u64) {
        self.handoffs_submitted.fetch_add(1, Ordering::Relaxed);
        self.handoffs_ok.fetch_add(1, Ordering::Relaxed);
        self.workbench_executions.fetch_add(1, Ordering::Relaxed);
        self.governor_allows.fetch_add(1, Ordering::Relaxed);
        self.witnesses_emitted.fetch_add(witnesses, Ordering::Relaxed);
    }

    pub fn inc_handoff_reject(&self, witnesses: u64) {
        self.handoffs_submitted.fetch_add(1, Ordering::Relaxed);
        self.handoffs_rejected.fetch_add(1, Ordering::Relaxed);
        self.governor_rejects.fetch_add(1, Ordering::Relaxed);
        self.witnesses_emitted.fetch_add(witnesses, Ordering::Relaxed);
    }

    pub fn inc_lens(&self) {
        self.lens_queries.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_plugin(&self) {
        self.plugin_invocations.fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            tasks_submitted: self.tasks_submitted.load(Ordering::Relaxed),
            tasks_ok: self.tasks_ok.load(Ordering::Relaxed),
            tasks_fail: self.tasks_fail.load(Ordering::Relaxed),
            handoffs_submitted: self.handoffs_submitted.load(Ordering::Relaxed),
            handoffs_ok: self.handoffs_ok.load(Ordering::Relaxed),
            handoffs_rejected: self.handoffs_rejected.load(Ordering::Relaxed),
            governor_allows: self.governor_allows.load(Ordering::Relaxed),
            governor_rejects: self.governor_rejects.load(Ordering::Relaxed),
            witnesses_emitted: self.witnesses_emitted.load(Ordering::Relaxed),
            workbench_executions: self.workbench_executions.load(Ordering::Relaxed),
            lens_queries: self.lens_queries.load(Ordering::Relaxed),
            plugin_invocations: self.plugin_invocations.load(Ordering::Relaxed),
        }
    }

    /// Prometheus text exposition (T12).
    pub fn prometheus_text(&self) -> String {
        let s = self.snapshot();
        format!(
            concat!(
                "# HELP ule_tasks_submitted Total task submits\n",
                "# TYPE ule_tasks_submitted counter\n",
                "ule_tasks_submitted {}\n",
                "# HELP ule_tasks_ok Successful tasks\n",
                "# TYPE ule_tasks_ok counter\n",
                "ule_tasks_ok {}\n",
                "# HELP ule_tasks_fail Failed tasks\n",
                "# TYPE ule_tasks_fail counter\n",
                "ule_tasks_fail {}\n",
                "# HELP ule_handoffs_submitted Handoff submits\n",
                "# TYPE ule_handoffs_submitted counter\n",
                "ule_handoffs_submitted {}\n",
                "# HELP ule_handoffs_ok Successful handoffs\n",
                "# TYPE ule_handoffs_ok counter\n",
                "ule_handoffs_ok {}\n",
                "# HELP ule_handoffs_rejected Rejected handoffs\n",
                "# TYPE ule_handoffs_rejected counter\n",
                "ule_handoffs_rejected {}\n",
                "# HELP ule_governor_allows Governor allow decisions\n",
                "# TYPE ule_governor_allows counter\n",
                "ule_governor_allows {}\n",
                "# HELP ule_governor_rejects Governor reject decisions\n",
                "# TYPE ule_governor_rejects counter\n",
                "ule_governor_rejects {}\n",
                "# HELP ule_witnesses_emitted Witness records emitted\n",
                "# TYPE ule_witnesses_emitted counter\n",
                "ule_witnesses_emitted {}\n",
                "# HELP ule_workbench_executions Workbench closed loops\n",
                "# TYPE ule_workbench_executions counter\n",
                "ule_workbench_executions {}\n",
                "# HELP ule_lens_queries Lens projections\n",
                "# TYPE ule_lens_queries counter\n",
                "ule_lens_queries {}\n",
                "# HELP ule_plugin_invocations Plugin product-path invocations\n",
                "# TYPE ule_plugin_invocations counter\n",
                "ule_plugin_invocations {}\n",
            ),
            s.tasks_submitted,
            s.tasks_ok,
            s.tasks_fail,
            s.handoffs_submitted,
            s.handoffs_ok,
            s.handoffs_rejected,
            s.governor_allows,
            s.governor_rejects,
            s.witnesses_emitted,
            s.workbench_executions,
            s.lens_queries,
            s.plugin_invocations,
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MetricsSnapshot {
    pub tasks_submitted: u64,
    pub tasks_ok: u64,
    pub tasks_fail: u64,
    pub handoffs_submitted: u64,
    pub handoffs_ok: u64,
    pub handoffs_rejected: u64,
    pub governor_allows: u64,
    pub governor_rejects: u64,
    pub witnesses_emitted: u64,
    pub workbench_executions: u64,
    pub lens_queries: u64,
    pub plugin_invocations: u64,
}
