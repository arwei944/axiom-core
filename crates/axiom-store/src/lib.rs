//! Axiom Store - Immutable event log, the single source of truth.
#![allow(async_fn_in_trait)]

pub mod store;
pub mod memory;
pub mod event;

pub use store::{EventStore, StoreError};
pub use event::Event;
