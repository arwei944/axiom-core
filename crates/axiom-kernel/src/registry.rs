//! Distributed registries for migrations, axioms, lenses, and capabilities.
//!
//! These registries use `linkme::distributed_slice` to allow crates across the
//! workspace to register items at compile time without a central crate owning
//! the registry.

use crate::axiom::DynAxiom;
use crate::layer::Layer;
use crate::version::{Compatibility, Version};
use crate::witness::Witness;
use linkme::distributed_slice;
use parking_lot::Mutex;

// ============================================================
// Migration registry
// ============================================================

#[distributed_slice]
pub static MIGRATION_REGISTRY: [fn() -> (u16, u16, &'static str, &'static str)] = [..];

pub fn registered_migration_chains() -> Vec<(u16, u16, &'static str, &'static str)> {
    MIGRATION_REGISTRY.iter().map(|f| f()).collect()
}

pub fn verify_migration_chain_completeness(up_to: u16) -> Result<(), Vec<String>> {
    let chains = registered_migration_chains();
    let mut gaps = Vec::new();
    for v in 1..up_to {
        let found = chains
            .iter()
            .any(|(from, to, _for_type, _name)| *from == v && *to == v + 1);
        if !found {
            gaps.push(format!("missing migration {v}->{}", v + 1));
        }
    }
    if gaps.is_empty() {
        Ok(())
    } else {
        Err(gaps)
    }
}

// ============================================================
// Axiom registry
// ============================================================

#[distributed_slice]
pub static AXIOM_REGISTRY: [&'static dyn DynAxiom] = [..];

pub fn registered_axioms() -> Vec<&'static dyn DynAxiom> {
    AXIOM_REGISTRY.iter().copied().collect()
}

pub fn count_registered_axioms() -> usize {
    AXIOM_REGISTRY.len()
}

// ============================================================
// Lens registry
// ============================================================

use crate::axiom::DynLens;

#[distributed_slice]
pub static LENS_REGISTRY: [fn() -> &'static dyn DynLens] = [..];

pub struct LensRegistry;

impl LensRegistry {
    pub fn registered_lenses() -> Vec<&'static dyn DynLens> {
        LENS_REGISTRY.iter().map(|f| f()).collect()
    }

    pub fn get_by_id(id: &str) -> Option<&'static dyn DynLens> {
        LENS_REGISTRY.iter().find(|f| f().id() == id).map(|f| f())
    }

    pub fn get_by_aggregate(aggregate_id: &str) -> Vec<&'static dyn DynLens> {
        LENS_REGISTRY
            .iter()
            .filter(|f| f().id().starts_with(aggregate_id))
            .map(|f| f())
            .collect()
    }
}

// ============================================================
// Capability registry
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapabilityDimension {
    Witness,
    Schema,
    Layer,
    Tool,
    Guard,
    Identity,
    Entropy,
    Runtime,
}

impl CapabilityDimension {
    pub fn as_str(&self) -> &'static str {
        match self {
            CapabilityDimension::Witness => "witness",
            CapabilityDimension::Schema => "schema",
            CapabilityDimension::Layer => "layer",
            CapabilityDimension::Tool => "tool",
            CapabilityDimension::Guard => "guard",
            CapabilityDimension::Identity => "identity",
            CapabilityDimension::Entropy => "entropy",
            CapabilityDimension::Runtime => "runtime",
        }
    }

    pub fn all() -> [Self; 8] {
        [
            CapabilityDimension::Witness,
            CapabilityDimension::Schema,
            CapabilityDimension::Layer,
            CapabilityDimension::Tool,
            CapabilityDimension::Guard,
            CapabilityDimension::Identity,
            CapabilityDimension::Entropy,
            CapabilityDimension::Runtime,
        ]
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CapabilityDescriptor {
    pub dimension: CapabilityDimension,
    pub name: &'static str,
    pub version: Version,
    pub compatibility: Compatibility,
    pub applies_to_layer: Option<Layer>,
    pub migration_chain_start: Option<u16>,
}

impl CapabilityDescriptor {
    pub fn new(
        dimension: CapabilityDimension,
        name: &'static str,
        version: Version,
        compatibility: Compatibility,
    ) -> Self {
        Self {
            dimension,
            name,
            version,
            compatibility,
            applies_to_layer: None,
            migration_chain_start: None,
        }
    }

    pub fn with_layer(mut self, layer: Layer) -> Self {
        self.applies_to_layer = Some(layer);
        self
    }

    pub fn with_migration_chain(mut self, start: u16) -> Self {
        self.migration_chain_start = Some(start);
        self
    }

    pub fn is_compatible_with(&self, other: &Self) -> bool {
        if self.dimension != other.dimension {
            return false;
        }
        match self.compatibility {
            Compatibility::Exact => self.version == other.version,
            Compatibility::Patch => {
                self.version.major == other.version.major
                    && self.version.minor == other.version.minor
            }
            Compatibility::NewerMinor => {
                self.version.major == other.version.major
                    && self.version.minor >= other.version.minor
            }
            Compatibility::OlderMinor => {
                self.version.major == other.version.major
                    && self.version.minor <= other.version.minor
            }
            Compatibility::Breaking => true,
        }
    }
}

#[distributed_slice]
pub static CAPABILITY_REGISTRY: [&'static CapabilityDescriptor] = [..];

pub struct CapabilityVersionRegistry;

impl CapabilityVersionRegistry {
    pub fn registered_capabilities() -> Vec<&'static CapabilityDescriptor> {
        CAPABILITY_REGISTRY.iter().copied().collect()
    }

    pub fn capabilities_by_dimension(
        dim: &CapabilityDimension,
    ) -> Vec<&'static CapabilityDescriptor> {
        CAPABILITY_REGISTRY
            .iter()
            .filter(|c| c.dimension == *dim)
            .copied()
            .collect()
    }

    pub fn latest_version_for_dimension(dim: &CapabilityDimension) -> Option<Version> {
        CAPABILITY_REGISTRY
            .iter()
            .filter(|c| c.dimension == *dim)
            .map(|c| c.version)
            .max()
    }

    pub fn verify_all() -> Result<(), String> {
        let _capabilities = Self::registered_capabilities();

        for dim in CapabilityDimension::all() {
            let dim_caps = Self::capabilities_by_dimension(&dim);
            if dim_caps.is_empty() {
                continue;
            }

            let Some(latest) = dim_caps.iter().max_by_key(|c| c.version) else {
                continue;
            };

            for cap in &dim_caps {
                if !cap.is_compatible_with(latest) {
                    return Err(format!(
                        "capability version mismatch for dimension {:?}: found {:?}, expected {:?}",
                        dim, cap.version, latest.version
                    ));
                }
            }
        }

        Ok(())
    }

    pub fn count_by_dimension(dim: &CapabilityDimension) -> usize {
        Self::capabilities_by_dimension(dim).len()
    }
}

pub static CAPABILITY_VERSION_REGISTRY: CapabilityVersionRegistry = CapabilityVersionRegistry;

// ============================================================
// Witness registry
// ============================================================

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

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for WitnessRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub static WITNESS_REGISTRY: WitnessRegistry = WitnessRegistry::new();
