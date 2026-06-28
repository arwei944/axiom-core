//! Axiom Viz - visualization data export layer.
//!
//! Exposes structured data from the runtime for visualization:
//! topology graphs, message flows, witness timelines, entropy dashboards,
//! trace data, and performance metrics.
//!
//! This crate provides JSON-serializable data structures;
//! actual rendering (TUI, Web UI, etc.) is handled by consumers.

pub mod entropy;
pub mod timeline;
pub mod topology;
