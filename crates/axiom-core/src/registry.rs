//! Distributed registries using linkme for automatic discovery.
//!
//! Instead of manually pushing every Cell/Axiom/Migration into registries,
//! use #[distributed_slice] to collect them at link time. The proc macros
//! #[cell], #[axiom], #[migration] emit the necessary attributes to register
//! into these slices automatically.

use crate::axiom::DynAxiom;

#[linkme::distributed_slice]
pub static MIGRATION_REGISTRY: [fn() -> (u16, u16, &'static str, &'static str)] = [..];

#[linkme::distributed_slice]
pub static AXIOM_REGISTRY: [&'static dyn DynAxiom] = [..];

pub fn registered_migration_chains() -> Vec<(u16, u16, &'static str, &'static str)> {
    MIGRATION_REGISTRY.iter().map(|f| f()).collect()
}

pub fn registered_axioms() -> Vec<&'static dyn DynAxiom> {
    AXIOM_REGISTRY.iter().copied().collect()
}

pub fn count_registered_axioms() -> usize {
    AXIOM_REGISTRY.len()
}

pub fn verify_migration_chain_completeness(up_to: u16) -> Result<(), Vec<String>> {
    let chains = registered_migration_chains();
    let mut gaps = Vec::new();
    for v in 1..up_to {
        let found = chains
            .iter()
            .any(|(from, to, _for_type, _name)| *from == v && *to == v + 1);
        if !found {
            gaps.push(format!("missing migration {v}→{}", v + 1));
        }
    }
    if gaps.is_empty() {
        Ok(())
    } else {
        Err(gaps)
    }
}

/// Verify migration chains for a specific signal type by name.
pub fn verify_migration_chain_for_type(signal_type: &str, current_version: u16) -> Result<(), Vec<String>> {
    let chains = registered_migration_chains();
    let relevant: Vec<_> = chains
        .iter()
        .filter(|(_, _, for_type, _)| for_type == &signal_type)
        .collect();
    let mut gaps = Vec::new();
    for v in 1..current_version {
        let found = relevant
            .iter()
            .any(|(from, to, _, _)| *from == v && *to == v + 1);
        if !found {
            gaps.push(format!(
                "missing migration {}→{} for type {}",
                v,
                v + 1,
                signal_type
            ));
        }
    }
    if gaps.is_empty() {
        Ok(())
    } else {
        Err(gaps)
    }
}
