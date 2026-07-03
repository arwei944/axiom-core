use crate::axiom::DynAxiom;
use crate::witness::Witness;
use parking_lot::Mutex;

#[linkme::distributed_slice]
pub static MIGRATION_REGISTRY: [fn() -> (u16, u16, &'static str, &'static str)] = [..];

#[linkme::distributed_slice]
pub static AXIOM_REGISTRY: [&'static dyn DynAxiom] = [..];

pub struct WitnessRegistry {
    witnesses: Mutex<Vec<Witness>>,
}

impl WitnessRegistry {
    pub const fn new() -> Self {
        Self {
            witnesses: Mutex::new(Vec::new()),
        }
    }

    pub fn record(&self, witness: Witness) {
        self.witnesses.lock().push(witness);
    }

    pub fn get_recent(&self, limit: usize) -> Vec<Witness> {
        let guard = self.witnesses.lock();
        guard.iter().rev().take(limit).cloned().rev().collect()
    }

    pub fn len(&self) -> usize {
        self.witnesses.lock().len()
    }
}

pub static WITNESS_REGISTRY: WitnessRegistry = WitnessRegistry::new();

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
