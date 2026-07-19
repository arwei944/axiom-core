//! Axiom Macros - Procedural macros for compile-time gates and ergonomic API.
//!
//! # Auto-Injection Macros
//! - `#[derive(SignalPayload)]`: Auto-implement Signal trait with required metadata + Witness generation
//! - `#[cell(layer = "exec")]`: Register a Cell with compile-time layer enforcement + AUTO-INJECTED Witness recording + permission checks + panic recovery
//! - `#[signal(kind = "command", layer = "exec")]`: Auto-implement Signal trait with required fields + serialization + validation
//! - `#[tool(permission = "read")]`: Auto-implement Tool trait with AUTO-INJECTED permission control + audit logging
//! - `#[axiom]`: Register an Axiom for automatic discovery + violation logging
//! - `#[schema_version(N)]`: Implement Versioned trait with given schema version
//! - `#[migration(from = N)]`: Register a Migration with compile-time gap detection
//! - `#[guard(layer = "all")]`: Auto-implement Guard trait with AUTO-INJECTED signal checking + Witness recording

extern crate proc_macro;

use proc_macro::TokenStream;

mod axiom;
mod capability;
mod cell;
mod error;
mod guard;
mod lens;
mod migration;
mod schema_version;
mod signal;
mod tool;
mod utils;

#[proc_macro_derive(SignalPayload, attributes(signal, schema))]
pub fn derive_signal_payload(input: TokenStream) -> TokenStream {
    signal::impl_derive_signal_payload(input)
}

#[proc_macro_attribute]
pub fn cell(attr: TokenStream, item: TokenStream) -> TokenStream {
    cell::impl_cell(attr, item)
}

#[proc_macro_attribute]
pub fn signal(attr: TokenStream, item: TokenStream) -> TokenStream {
    signal::impl_signal(attr, item)
}

#[proc_macro_attribute]
pub fn tool(attr: TokenStream, item: TokenStream) -> TokenStream {
    tool::impl_tool(attr, item)
}

#[proc_macro_attribute]
pub fn axiom(_attr: TokenStream, item: TokenStream) -> TokenStream {
    axiom::impl_axiom(_attr, item)
}

#[proc_macro_attribute]
pub fn schema_version(attr: TokenStream, item: TokenStream) -> TokenStream {
    schema_version::impl_schema_version(attr, item)
}

#[proc_macro_attribute]
pub fn migration(attr: TokenStream, item: TokenStream) -> TokenStream {
    migration::impl_migration(attr, item)
}

#[proc_macro_attribute]
pub fn guard(attr: TokenStream, item: TokenStream) -> TokenStream {
    guard::impl_guard(attr, item)
}

#[proc_macro_attribute]
pub fn lens(attr: TokenStream, item: TokenStream) -> TokenStream {
    lens::impl_lens(attr, item)
}

#[proc_macro_attribute]
pub fn capability(attr: TokenStream, item: TokenStream) -> TokenStream {
    capability::impl_capability(attr, item)
}
