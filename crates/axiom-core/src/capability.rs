use crate::layer::Layer;
use crate::version::{Compatibility, Version};
use crate::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapabilityDimension {
    Witness,
    Schema,
    Layer,
    Tool,
    Guard,
}

impl CapabilityDimension {
    pub fn as_str(&self) -> &'static str {
        match self {
            CapabilityDimension::Witness => "witness",
            CapabilityDimension::Schema => "schema",
            CapabilityDimension::Layer => "layer",
            CapabilityDimension::Tool => "tool",
            CapabilityDimension::Guard => "guard",
        }
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
            Compatibility::SemVer => self.version.major == other.version.major,
        }
    }
}

#[linkme::distributed_slice]
pub static CAPABILITY_REGISTRY: [&'static CapabilityDescriptor] = [..];

pub struct CapabilityVersionRegistry;

impl CapabilityVersionRegistry {
    pub fn registered_capabilities() -> Vec<&'static CapabilityDescriptor> {
        CAPABILITY_REGISTRY.iter().copied().collect()
    }

    pub fn capabilities_by_dimension(dim: CapabilityDimension) -> Vec<&'static CapabilityDescriptor> {
        CAPABILITY_REGISTRY
            .iter()
            .filter(|c| c.dimension == dim)
            .copied()
            .collect()
    }

    pub fn latest_version_for_dimension(dim: CapabilityDimension) -> Option<Version> {
        CAPABILITY_REGISTRY
            .iter()
            .filter(|c| c.dimension == dim)
            .map(|c| c.version.clone())
            .max()
    }

    pub fn auto_check_compatibility() -> Result<()> {
        let capabilities = Self::registered_capabilities();
        
        for dim in [
            CapabilityDimension::Witness,
            CapabilityDimension::Schema,
            CapabilityDimension::Layer,
            CapabilityDimension::Tool,
            CapabilityDimension::Guard,
        ] {
            let dim_caps = Self::capabilities_by_dimension(dim);
            if dim_caps.is_empty() {
                continue;
            }

            let latest = dim_caps
                .iter()
                .max_by_key(|c| c.version.clone())
                .unwrap();

            for cap in &dim_caps {
                if !cap.is_compatible_with(latest) {
                    return Err(crate::AxiomError::VersionMismatch {
                        compatibility: cap.compatibility.clone(),
                        required: latest.version.clone(),
                        found: cap.version.clone(),
                    });
                }
            }
        }

        Ok(())
    }

    pub fn verify_all() -> Result<()> {
        Self::auto_check_compatibility()?;
        Ok(())
    }

    pub fn count_by_dimension(dim: CapabilityDimension) -> usize {
        Self::capabilities_by_dimension(dim).len()
    }
}

pub static CAPABILITY_VERSION_REGISTRY: CapabilityVersionRegistry = CapabilityVersionRegistry;
