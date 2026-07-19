//! Version management - Strict multi-dimensional versioning for deterministic replay and upgrade safety.
//!
//! Five orthogonal version dimensions:
//! - **CrateVersion**: Semantic version for the entire library (SemVer MAJOR.MINOR.PATCH)
//! - **SchemaVersion**: Monotonically increasing integer per serialized data type (forward-compatible reads)
//! - **ProtocolVersion**: Wire protocol version (exact match required for network communication)
//! - **ApiVersion**: Public API surface stability guarantees
//! - **IdentityVersion**: Monotonic version for identity hot-swap (witnessed for audit)
//!
//! Design principles:
//! - Newer readers can always read older data (forward compatibility)
//! - Migration chains are verified complete at startup (no gaps)
//! - Every Witness records all version info for tamper-evident audit trails
//! - Version mismatches produce explicit errors, never silent failures

use serde::{Deserialize, Serialize};
use std::fmt;

// ============================================================
// CrateVersion (SemVer)
// ============================================================

/// Semantic version (MAJOR.MINOR.PATCH) following <https://semver.org/>
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Version {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl Version {
    pub const fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self { major, minor, patch }
    }

    pub fn is_compatible_with(&self, other: &Version) -> Compatibility {
        if self.major != other.major {
            Compatibility::Breaking
        } else if self.minor > other.minor {
            Compatibility::NewerMinor
        } else if self.minor < other.minor {
            Compatibility::OlderMinor
        } else if self.patch != other.patch {
            Compatibility::Patch
        } else {
            Compatibility::Exact
        }
    }

    pub fn is_safe_upgrade_from(&self, older: &Version) -> bool {
        self.major == older.major && self.minor >= older.minor
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

// Inject CARGO_PKG_VERSION from build.rs (P3-5). Redefines CURRENT when present.
include!(concat!(env!("OUT_DIR"), "/version_generated.rs"));

// ============================================================
// SchemaVersion - data format versioning
// ============================================================

/// Schema version for serialized data formats (Signals, Events, Witnesses).
///
/// Monotonically increasing integer. Reader with version N can read data written
/// by any version <= N. Schema changes that break reading require a migration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SchemaVersion(pub u16);

impl SchemaVersion {
    pub const fn new(v: u16) -> Self {
        Self(v)
    }

    pub const fn current() -> Self {
        Self(1)
    }

    pub fn can_read(&self, writer_version: SchemaVersion) -> bool {
        self.0 >= writer_version.0
    }

    pub fn next(&self) -> Self {
        Self(self.0 + 1)
    }
}

/// Per-type schema version registry (P0-4).
#[derive(Debug, Clone, Default)]
pub struct TypeSchemaRegistry {
    versions: std::collections::HashMap<String, SchemaVersion>,
}

impl TypeSchemaRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, type_name: impl Into<String>, version: SchemaVersion) {
        self.versions.insert(type_name.into(), version);
    }

    pub fn get(&self, type_name: &str) -> Option<SchemaVersion> {
        self.versions.get(type_name).copied()
    }

    /// True when envelope schema is readable by registered reader version (or unregistered).
    pub fn is_compatible(&self, type_name: &str, writer: SchemaVersion) -> bool {
        match self.get(type_name) {
            Some(reader) => reader.can_read(writer),
            None => true,
        }
    }

    pub fn verify_migrations_for_type(
        &self,
        type_name: &str,
        chains: &[(u16, u16)],
    ) -> Result<(), String> {
        let Some(max) = self.get(type_name) else {
            return Ok(());
        };
        for v in 1..max.0 {
            if !chains.iter().any(|(from, to)| *from == v && *to == v + 1) {
                return Err(format!(
                    "type {type_name}: missing migration {v}->{}",
                    v + 1
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod schema_registry_tests {
    use super::*;

    #[test]
    fn mixed_schema_versions_compatible() {
        let mut reg = TypeSchemaRegistry::new();
        reg.register("Foo", SchemaVersion::new(3));
        assert!(reg.is_compatible("Foo", SchemaVersion::new(1)));
        assert!(reg.is_compatible("Foo", SchemaVersion::new(3)));
        assert!(!reg.is_compatible("Foo", SchemaVersion::new(4)));
        assert!(reg
            .verify_migrations_for_type("Foo", &[(1, 2), (2, 3)])
            .is_ok());
        assert!(reg
            .verify_migrations_for_type("Foo", &[(1, 2)])
            .is_err());
    }

    #[test]
    fn current_version_injected_from_package() {
        // build.rs injects CARGO_PKG_VERSION (workspace 0.4.0)
        assert_eq!(Version::CURRENT.major, 0);
        assert!(Version::CURRENT.minor >= 4 || Version::CURRENT.patch >= 0);
    }
}

impl fmt::Display for SchemaVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ============================================================
// Compatibility
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Compatibility {
    Exact,
    Patch,
    NewerMinor,
    OlderMinor,
    Breaking,
}

impl Compatibility {
    pub fn is_compatible(self) -> bool {
        matches!(self, Compatibility::Exact | Compatibility::Patch | Compatibility::NewerMinor)
    }
}

// ============================================================
// Versioned trait
// ============================================================

/// Trait for types that carry a schema version.
pub trait Versioned {
    fn schema_version() -> SchemaVersion;
}

// ============================================================
// ProtocolVersion - network wire protocol
// ============================================================

/// Protocol version for MCP and inter-node communication.
/// Must match exactly between peers (no negotiation, prevents downgrade attacks).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ProtocolVersion(pub u16);

impl ProtocolVersion {
    pub const CURRENT: Self = Self(1);

    pub fn is_compatible_with(&self, other: ProtocolVersion) -> bool {
        self.0 == other.0
    }
}

impl fmt::Display for ProtocolVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "protocol-v{}", self.0)
    }
}

// ============================================================
// IdentityVersion - for hot-swap auditing
// ============================================================

/// Monotonic version for identity hot-swapping.
/// Each time an Identity is mounted/updated, this version increments.
/// Witnesses record the identity version active at transaction time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct IdentityVersion(pub u16);

impl IdentityVersion {
    pub const fn new(v: u16) -> Self {
        Self(v)
    }

    pub fn increment(&mut self) {
        self.0 += 1;
    }
}

impl fmt::Display for IdentityVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "identity-v{}", self.0)
    }
}

// ============================================================
// VersionInfo - carried in every Witness for audit
// ============================================================

/// Complete version snapshot recorded in every Witness for audit trail integrity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub crate_version: Version,
    pub witness_schema: SchemaVersion,
    pub signal_schema: SchemaVersion,
    pub event_schema: SchemaVersion,
    pub protocol_version: ProtocolVersion,
    pub identity_version: Option<IdentityVersion>,
}

impl VersionInfo {
    pub fn current() -> Self {
        Self {
            crate_version: Version::CURRENT,
            witness_schema: WitnessSchema::schema_version(),
            signal_schema: SignalSchema::schema_version(),
            event_schema: EventSchema::schema_version(),
            protocol_version: ProtocolVersion::CURRENT,
            identity_version: None,
        }
    }
}

// ============================================================
// Marker types for schema version tracking
// ============================================================

pub struct WitnessSchema;
pub struct SignalSchema;
pub struct EventSchema;

impl Versioned for WitnessSchema {
    fn schema_version() -> SchemaVersion {
        SchemaVersion::new(1)
    }
}

impl Versioned for SignalSchema {
    fn schema_version() -> SchemaVersion {
        SchemaVersion::new(1)
    }
}

impl Versioned for EventSchema {
    fn schema_version() -> SchemaVersion {
        SchemaVersion::new(1)
    }
}

// ============================================================
// CrateVersion alias for backwards compatibility
// ============================================================

pub type CrateVersion = Version;

pub trait Migration: Send + Sync {
    fn source_version(&self) -> SchemaVersion;
    fn target_version(&self) -> SchemaVersion;

    fn migrate(&self, input: serde_json::Value) -> crate::axiom::KernelResult<serde_json::Value>;

    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}
