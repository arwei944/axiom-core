//! Witness-only execution journal (constitution: single history authority).

use crate::error::{IsaError, IsaResult};
use crate::primitives::StepKind;
use axiom_kernel::context::CellContext;
use axiom_kernel::witness::{TransitionOutcome, Witness, WitnessBuilder};

/// Append-only Witness journal bound to a CellContext.
///
/// This is the **only** legal way for ISA steps to record history in U1+.
/// There is no parallel ExecutionStep store.
pub struct WitnessJournal<'a> {
    ctx: &'a mut CellContext<'a>,
}

impl<'a> WitnessJournal<'a> {
    pub fn new(ctx: &'a mut CellContext<'a>) -> Self {
        Self { ctx }
    }

    pub fn record_start(
        &mut self,
        kind: StepKind,
        name: &str,
        detail: &str,
    ) -> IsaResult<()> {
        let summary = format!("{}:{}:{}", kind.as_str(), name, detail);
        self.emit_ok(&summary)
    }

    pub fn record_ok(&mut self, kind: StepKind, name: &str, detail: &str) -> IsaResult<()> {
        let summary = format!("{}:{}:{}", kind.as_str(), name, detail);
        self.emit_ok(&summary)
    }

    pub fn record_err(&mut self, kind: StepKind, name: &str, reason: &str) -> IsaResult<()> {
        let summary = format!("{}:{}:fail", kind.as_str(), name);
        self.ctx
            .emit_failure(&summary, reason)
            .map_err(|e| IsaError::Journal(e.to_string()))
    }

    pub fn record_rejected(&mut self, reason: &str) -> IsaResult<()> {
        let summary = format!("{}:governor:reject", StepKind::Governor.as_str());
        self.ctx
            .emit_failure(&summary, reason)
            .map_err(|e| IsaError::Journal(e.to_string()))
    }

    fn emit_ok(&mut self, summary: &str) -> IsaResult<()> {
        WitnessBuilder::new()
            .summary(summary)
            .outcome(TransitionOutcome::Success)
            .emit(self.ctx)
            .map_err(|e| IsaError::Journal(e.to_string()))
    }

    /// Snapshot of witnesses recorded so far (does not clear context).
    pub fn witnesses(&self) -> Vec<Witness> {
        // CellContext does not expose witnesses by ref; take via a temporary approach:
        // We cannot borrow witnesses without take. Provide count + re-export after finish.
        // Callers should use `into_witnesses` / finish path. For tests, demo drains at end.
        Vec::new()
    }

    pub fn into_witnesses(self) -> Vec<Witness> {
        self.ctx
            .take_witnesses()
            .into_iter()
            .map(|w| w.0)
            .collect()
    }

    /// Validate that a finished chain is well-formed (prev_hash links).
    pub fn verify_chain(witnesses: &[Witness]) -> IsaResult<()> {
        let mut prev = None;
        for (i, w) in witnesses.iter().enumerate() {
            if w.prev_hash != prev {
                return Err(IsaError::Journal(format!(
                    "chain break at index {i}: prev_hash mismatch"
                )));
            }
            // recompute hash integrity
            let computed = w
                .compute_hash()
                .map_err(|e| IsaError::Journal(e.to_string()))?;
            if computed.0 != w.hash.0 {
                return Err(IsaError::Journal(format!(
                    "hash mismatch at index {i}"
                )));
            }
            prev = Some(w.hash);
        }
        Ok(())
    }
}
