//! Centralized macro error formatting (P3-1).

use proc_macro2::Span;
use syn::Error;

/// Build a readable compile error for axiom macros.
pub fn macro_err(span: Span, kind: &str, detail: impl std::fmt::Display) -> Error {
    Error::new(
        span,
        format!(
            "[axiom-macros::{kind}] {detail}\n  hint: see docs/plugin-development.md and crate docs"
        ),
    )
}

pub fn missing_attr(span: Span, attr: &str, example: &str) -> Error {
    macro_err(
        span,
        "attr",
        format!("missing required attribute `{attr}` (example: {example})"),
    )
}
