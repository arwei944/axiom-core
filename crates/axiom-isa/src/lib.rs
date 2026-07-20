//! ULE business ISA — Atom / Port / Adapter / Composer.
//!
//! Constitution (UNIFIED_MODEL):
//! - Business logic is expressed only with these four primitives.
//! - Composer runs **inside** a Cell (Composer-in-Cell).
//! - Execution history is **Witness-only** via [`WitnessJournal`].
//! - Entropy decisions go through a single [`Governor`] — the **sole** product admit API.
//! - Agent transfer uses [`HandoffRequest`] as Signal payload (not a second bus).

pub mod discipline;
pub mod error;
pub mod governor;
pub mod handoff;
pub mod journal;
pub mod primitives;

pub use discipline::{
    assert_witness_step_kinds, discover_composer_sources, is_composer_bearing_source, policy_text,
    scan_source, scan_workspace_commercial, DisciplineReport, DisciplineViolation,
    BANNED_SIDE_EFFECT_TOKENS, COMMERCIAL_CRATE_SRC_ROOTS, COMMERCIAL_ISA_SOURCES,
    REQUIRED_ISA_HELPERS,
};
pub use error::{IsaError, IsaResult};
pub use governor::{Decision, Governor, GovernorConfig};
pub use handoff::{
    is_allowed_intent, HandoffRequest, HandoffResult, WorkbenchLimits, WorkbenchProposal,
};
pub use journal::WitnessJournal;
pub use primitives::{
    run_adapter, run_atom, run_port, Adapter, AdapterFn, Atom, AtomFn, Composer, Port, PortFn,
    SeqComposer, StepKind,
};

/// Product-facing **sole** admit entry (constitutional freeze).
///
/// **Commercial business Cells MUST call this function by name** before Composer work.
/// Do not call [`Governor::admit`] directly on product write paths (keeps a single
/// greppable / reviewable product admit surface). Never use runtime/oversight
/// entropy cells as product authorization.
pub fn product_admit(governor: &Governor, journal: &mut WitnessJournal<'_>) -> IsaResult<()> {
    governor.admit(journal)
}

/// Product-facing sole decide entry without Witness side effects.
pub fn product_decide(governor: &Governor) -> Decision {
    governor.decide()
}
