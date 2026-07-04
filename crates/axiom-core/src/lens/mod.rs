//! Lens projection subsystem.
//!
//! This module provides typed projections over event streams,
//! with caching and dependency validation.

pub mod accessor;
pub mod cache;
pub mod error;
pub mod events;
pub mod registry;
pub mod traits;

pub use accessor::LensAccessor;
pub use cache::{CacheMetrics, InMemoryProjectionCache, IncrementalProjectionCache};
pub use error::{LensAccessError, LensError};
pub use events::{LensEvent, Projection, ProjectionDowncastError};
pub use registry::{DependencyCycleError, LensRegistry, LENS_REGISTRY};
pub use traits::{Lens, Projectable, ProjectionCache};
