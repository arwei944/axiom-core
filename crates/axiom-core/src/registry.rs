//! Distributed registries using linkme for automatic discovery.
//!
//! Instead of manually pushing every Cell/Axiom/Migration into registries,
//! use #[distributed_slice] to collect them at link time. The proc macros
//! #[cell], #[axiom], #[migration] emit the necessary attributes to register
//! into these slices automatically.

#[linkme::distributed_slice]
pub static MIGRATION_REGISTRY: [fn() -> (u16, u16, &'static str)] = [..];

#[linkme::distributed_slice]
pub static AXIOM_REGISTRY: [fn() -> &'static str] = [..];

pub fn registered_migration_chains() -> Vec<(u16, u16, &'static str)> {
    MIGRATION_REGISTRY.iter().map(|f| f()).collect()
}

pub fn count_registered_axioms() -> usize {
    AXIOM_REGISTRY.len()
}

pub fn verify_migration_chain_completeness(up_to: u16) -> Result<(), Vec<String>> {
    let chains = registered_migration_chains();
    let mut gaps = Vec::new();
    for v in 1..up_to {
        let found = chains.iter().any(|(from, to, _)| *from == v && *to == v + 1);
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
