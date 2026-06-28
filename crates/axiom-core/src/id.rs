//! Strongly-typed identifiers to prevent ID confusion at compile time.

use serde::{Deserialize, Serialize};

macro_rules! define_id {
    ($name:ident, $doc:expr) => {
        #[doc = $doc]
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub struct $name(pub String);

        impl $name {
            pub fn new(id: impl Into<String>) -> Self {
                Self(id.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl From<String> for $name {
            fn from(s: String) -> Self {
                Self(s)
            }
        }

        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                Self(s.to_string())
            }
        }
    };
}

define_id!(CellId, "Unique identifier for a Cell.");
define_id!(MsgId, "Unique message identifier for idempotency deduplication.");
define_id!(CorrelationId, "Correlation ID for distributed tracing across the entire call chain.");
define_id!(WitnessId, "Unique witness identifier for audit chain records.");
define_id!(LensId, "Unique lens identifier used for permission boundaries.");
define_id!(AxiomId, "Unique axiom identifier for invariant rules.");
define_id!(TraceId, "Top-level trace identifier grouping an entire workflow.");

#[cfg(feature = "uuid")]
impl CorrelationId {
    pub fn generate() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

#[cfg(feature = "uuid")]
impl MsgId {
    pub fn generate() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

#[cfg(feature = "uuid")]
impl WitnessId {
    pub fn generate() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

#[cfg(feature = "uuid")]
impl TraceId {
    pub fn generate() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}
