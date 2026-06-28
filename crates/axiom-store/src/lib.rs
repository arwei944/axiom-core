//! Axiom Store - Immutable event log, the single source of truth.
#![allow(async_fn_in_trait)]

pub mod event;
pub mod memory;
pub mod store;

pub use event::Event;
pub use store::{EventStore, StoreError};
