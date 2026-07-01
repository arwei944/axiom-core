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

/// Semantic version (MAJOR.MINOR.PATCH) following https://semver.org/
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Version {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl Version {
    pub const fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    pub const CURRENT: Self = Self::new(0, 1, 0);

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

    pub fn can_read(&self, writer_version: SchemaVersion) -> bool {
        self.0 >= writer_version.0
    }

    pub fn next(&self) -> Self {
        Self(self.0 + 1)
    }
}

impl fmt::Display for SchemaVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "v{}", self.0)
    }
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
// Compatibility
// ============================================================

/// Compatibility level between two versions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compatibility {
    Exact,
    Patch,
    NewerMinor,
    OlderMinor,
    Breaking,
}

impl Compatibility {
    pub fn is_safe(&self) -> bool {
        matches!(self, Compatibility::Exact | Compatibility::Patch)
    }
}

// ============================================================
// Versioned trait
// ============================================================

/// Trait for types that carry a schema version.
///
/// Every serializable data structure (Signal, Event, Witness) must implement
/// this trait to declare its current schema version and minimum supported version.
pub trait Versioned {
    fn schema_version() -> SchemaVersion;

    fn min_supported_version() -> SchemaVersion {
        Self::schema_version()
    }
}

// ============================================================
// Migration trait
// ============================================================

/// Data migration from one schema version to the next.
///
/// Migrations form a chain: v1→v2→v3... Each migration is a deterministic
/// pure function (no IO, no randomness) that transforms a JSON value from
/// schema FROM to schema TO.
pub trait Migration: Send + Sync {
    fn source_version(&self) -> SchemaVersion;
    fn target_version(&self) -> SchemaVersion;

    fn migrate(&self, input: serde_json::Value) -> crate::Result<serde_json::Value>;

    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}

/// Schema migration manager built from the distributed migration registry.
///
/// Groups migrations by signal type and provides convenience methods for
/// migrating JSON payloads to the latest version or a specific version.
pub struct SchemaMigrator {
    migrations: std::collections::HashMap<&'static str, Vec<Box<dyn Migration>>>,
    max_versions: std::collections::HashMap<&'static str, SchemaVersion>,
}

impl SchemaMigrator {
    /// Build a SchemaMigrator from the linkme distributed migration registry.
    ///
    /// Populates `max_versions` from registry metadata. Actual `Migration` trait
    /// objects must be added via `register()` since the distributed registry only
    /// stores version metadata, not trait objects.
    pub fn from_registry() -> Self {
        let migrations: std::collections::HashMap<&'static str, Vec<Box<dyn Migration>>> =
            std::collections::HashMap::new();
        let mut max_versions: std::collections::HashMap<&'static str, SchemaVersion> =
            std::collections::HashMap::new();

        let chains = crate::registry::registered_migration_chains();
        for (_from, to, for_type, _name) in &chains {
            if !for_type.is_empty() {
                let entry = max_versions
                    .entry(for_type)
                    .or_insert_with(|| SchemaVersion::new(0));
                if *to > entry.0 {
                    *entry = SchemaVersion::new(*to);
                }
            }
        }

        Self {
            migrations,
            max_versions,
        }
    }

    /// Register a migration for a specific signal type.
    pub fn register<M: Migration + 'static>(&mut self, signal_type: &'static str, m: M) {
        let entry = self.migrations.entry(signal_type).or_default();
        let target = m.target_version();
        entry.push(Box::new(m));
        entry.sort_by_key(|m| m.source_version().0);
        let current_max = self
            .max_versions
            .entry(signal_type)
            .or_insert_with(|| SchemaVersion::new(1));
        if target > *current_max {
            *current_max = target;
        }
    }

    /// Get the latest known schema version for a signal type.
    pub fn latest_version(&self, signal_type: &str) -> Option<SchemaVersion> {
        self.max_versions.get(signal_type).copied()
    }

    /// Migrate a JSON payload from its current version to the latest known version.
    pub fn migrate_to_latest(
        &self,
        signal_type: &str,
        from_version: SchemaVersion,
        json: serde_json::Value,
    ) -> crate::Result<(serde_json::Value, SchemaVersion)> {
        let latest =
            self.latest_version(signal_type)
                .ok_or(crate::AxiomError::MigrationPathNotFound {
                    found: from_version.0,
                    current: 0,
                })?;
        let result = self.migrate_to(signal_type, from_version, latest, json)?;
        Ok((result, latest))
    }

    /// Migrate a JSON payload from one version to another.
    pub fn migrate_to(
        &self,
        signal_type: &str,
        from: SchemaVersion,
        to: SchemaVersion,
        mut json: serde_json::Value,
    ) -> crate::Result<serde_json::Value> {
        if from == to {
            return Ok(json);
        }
        if from.0 > to.0 {
            return Err(crate::AxiomError::MigrationFailed {
                from: from.0,
                to: to.0,
                reason: "Cannot migrate backwards".into(),
            });
        }
        let chain =
            self.migrations
                .get(signal_type)
                .ok_or(crate::AxiomError::MigrationPathNotFound {
                    found: from.0,
                    current: to.0,
                })?;
        let mut current = from.0;
        while current < to.0 {
            let next = current + 1;
            let migration = chain
                .iter()
                .find(|m| m.source_version().0 == current && m.target_version().0 == next)
                .ok_or(crate::AxiomError::MigrationChainGap {
                    from: current,
                    to: next,
                })?;
            json = migration
                .migrate(json)
                .map_err(|e| crate::AxiomError::MigrationFailed {
                    from: current,
                    to: next,
                    reason: e.to_string(),
                })?;
            current = next;
        }
        Ok(json)
    }

    /// Verify that the migration chain is complete for a signal type (no gaps).
    pub fn verify_chain(&self, signal_type: &str, up_to: SchemaVersion) -> crate::Result<()> {
        let chain =
            self.migrations
                .get(signal_type)
                .ok_or(crate::AxiomError::MigrationPathNotFound {
                    found: 0,
                    current: 0,
                })?;
        for v in 1..up_to.0 {
            let found = chain
                .iter()
                .any(|m| m.source_version().0 == v && m.target_version().0 == v + 1);
            if !found {
                return Err(crate::AxiomError::MigrationChainGap { from: v, to: v + 1 });
            }
        }
        Ok(())
    }

    /// List all registered signal types and their latest versions.
    pub fn registered_types(&self) -> Vec<(&str, SchemaVersion)> {
        self.max_versions.iter().map(|(k, v)| (*k, *v)).collect()
    }
}

impl Default for SchemaMigrator {
    fn default() -> Self {
        Self::from_registry()
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

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_ordering() {
        let v1 = Version::new(1, 0, 0);
        let v2 = Version::new(1, 1, 0);
        let v3 = Version::new(2, 0, 0);
        assert!(v1 < v2);
        assert!(v2 < v3);
    }

    #[test]
    fn test_compatibility() {
        let v100 = Version::new(1, 0, 0);
        let v101 = Version::new(1, 0, 1);
        let v110 = Version::new(1, 1, 0);
        let v200 = Version::new(2, 0, 0);

        assert!(v100.is_compatible_with(&v101).is_safe());
        assert!(!v100.is_compatible_with(&v110).is_safe());
        assert!(!v100.is_compatible_with(&v200).is_safe());
        assert_eq!(v100.is_compatible_with(&v200), Compatibility::Breaking);
        assert!(v110.is_safe_upgrade_from(&v100));
        assert!(!v200.is_safe_upgrade_from(&v100));
    }

    #[test]
    fn test_schema_version_can_read() {
        let v1 = SchemaVersion::new(1);
        let v2 = SchemaVersion::new(2);
        assert!(v2.can_read(v1));
        assert!(!v1.can_read(v2));
        assert_eq!(v1.next(), v2);
    }

    #[test]
    fn test_protocol_exact_match() {
        let v1 = ProtocolVersion(1);
        let v1b = ProtocolVersion(1);
        let v2 = ProtocolVersion(2);
        assert!(v1.is_compatible_with(v1b));
        assert!(!v1.is_compatible_with(v2));
    }

    #[test]
    fn test_schema_too_new_error() {
        // SchemaMigrator::migrate_to rejects backwards migration (from > to)
        let mig = SchemaMigrator::from_registry();
        let result = mig.migrate_to(
            "TestSignal",
            SchemaVersion(5),
            SchemaVersion(3),
            serde_json::json!({}),
        );
        assert!(result.is_err());
        match result.unwrap_err() {
            crate::AxiomError::MigrationFailed { from, to, .. } => {
                assert_eq!(from, 5);
                assert_eq!(to, 3);
            }
            e => panic!("Expected MigrationFailed, got {:?}", e),
        }
    }

    #[test]
    fn test_version_info_current() {
        let info = VersionInfo::current();
        assert_eq!(info.crate_version, Version::CURRENT);
        assert_eq!(info.witness_schema, WitnessSchema::schema_version());
        assert!(info.identity_version.is_none());
    }

    #[test]
    fn test_identity_version_increment() {
        let mut v = IdentityVersion::new(1);
        v.increment();
        assert_eq!(v, IdentityVersion(2));
    }

    #[test]
    fn test_schema_migrator_single_step() {
        struct M1to2;
        impl Migration for M1to2 {
            fn source_version(&self) -> SchemaVersion {
                SchemaVersion(1)
            }
            fn target_version(&self) -> SchemaVersion {
                SchemaVersion(2)
            }
            fn migrate(&self, mut input: serde_json::Value) -> crate::Result<serde_json::Value> {
                input["v2"] = serde_json::json!(true);
                Ok(input)
            }
        }

        let mut mig = SchemaMigrator::from_registry();
        mig.register("TestSignal", M1to2);

        let input = serde_json::json!({"data": "hello"});
        let result = mig
            .migrate_to("TestSignal", SchemaVersion(1), SchemaVersion(2), input)
            .unwrap();
        assert_eq!(result["v2"], serde_json::json!(true));
    }

    #[test]
    fn test_schema_migrator_multiple_steps() {
        struct M1to2;
        impl Migration for M1to2 {
            fn source_version(&self) -> SchemaVersion {
                SchemaVersion(1)
            }
            fn target_version(&self) -> SchemaVersion {
                SchemaVersion(2)
            }
            fn migrate(&self, mut v: serde_json::Value) -> crate::Result<serde_json::Value> {
                v["step"] = serde_json::json!(2);
                Ok(v)
            }
        }
        struct M2to3;
        impl Migration for M2to3 {
            fn source_version(&self) -> SchemaVersion {
                SchemaVersion(2)
            }
            fn target_version(&self) -> SchemaVersion {
                SchemaVersion(3)
            }
            fn migrate(&self, mut v: serde_json::Value) -> crate::Result<serde_json::Value> {
                v["step"] = serde_json::json!(3);
                Ok(v)
            }
        }

        let mut mig = SchemaMigrator::from_registry();
        mig.register("TestSignal", M1to2);
        mig.register("TestSignal", M2to3);

        let input = serde_json::json!({"start": true});
        let (result, latest) = mig
            .migrate_to_latest("TestSignal", SchemaVersion(1), input)
            .unwrap();
        assert_eq!(result["step"], serde_json::json!(3));
        assert_eq!(latest, SchemaVersion(3));
    }

    #[test]
    fn test_schema_migrator_gap_detection() {
        struct M1to2;
        impl Migration for M1to2 {
            fn source_version(&self) -> SchemaVersion {
                SchemaVersion(1)
            }
            fn target_version(&self) -> SchemaVersion {
                SchemaVersion(2)
            }
            fn migrate(&self, v: serde_json::Value) -> crate::Result<serde_json::Value> {
                Ok(v)
            }
        }

        let mut mig = SchemaMigrator::from_registry();
        mig.register("TestSignal", M1to2);

        let result = mig.migrate_to(
            "TestSignal",
            SchemaVersion(1),
            SchemaVersion(3),
            serde_json::json!({}),
        );
        assert!(result.is_err());
        match result.unwrap_err() {
            crate::AxiomError::MigrationChainGap { from, to } => {
                assert_eq!(from, 2);
                assert_eq!(to, 3);
            }
            e => panic!("Expected MigrationChainGap, got {:?}", e),
        }
    }

    #[test]
    fn test_schema_migrator_verify_chain() {
        struct M1to2;
        impl Migration for M1to2 {
            fn source_version(&self) -> SchemaVersion {
                SchemaVersion(1)
            }
            fn target_version(&self) -> SchemaVersion {
                SchemaVersion(2)
            }
            fn migrate(&self, v: serde_json::Value) -> crate::Result<serde_json::Value> {
                Ok(v)
            }
        }
        struct M2to3;
        impl Migration for M2to3 {
            fn source_version(&self) -> SchemaVersion {
                SchemaVersion(2)
            }
            fn target_version(&self) -> SchemaVersion {
                SchemaVersion(3)
            }
            fn migrate(&self, v: serde_json::Value) -> crate::Result<serde_json::Value> {
                Ok(v)
            }
        }

        let mut mig = SchemaMigrator::from_registry();
        mig.register("TestSignal", M1to2);
        mig.register("TestSignal", M2to3);

        assert!(mig.verify_chain("TestSignal", SchemaVersion(3)).is_ok());
    }

    #[test]
    fn test_schema_migrator_from_registry() {
        let mig = SchemaMigrator::from_registry();
        let _types = mig.registered_types();
    }
}
