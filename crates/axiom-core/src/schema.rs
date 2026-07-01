//! Schema - Compile-time and runtime message validation.
//!
//! Every Signal must implement the Schema trait to define:
//! - Field-level validation rules
//! - Size limits
//! - Required vs optional fields
//! - Semantic constraints
//!
//! Schema validation runs at Layer 2 (Validate) before signals reach
//! Layer 1 (Exec) or Layer 3 (Agent).

use crate::error::AxiomError;
use crate::Result;
use serde::{Deserialize, Serialize};
use std::ops::AddAssign;

/// Validation severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationSeverity {
    Info,
    Warning,
    Error,
}

/// A single validation error.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
    pub severity: ValidationSeverity,
}

impl ValidationError {
    pub fn error(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            severity: ValidationSeverity::Error,
        }
    }

    pub fn warning(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            severity: ValidationSeverity::Warning,
        }
    }
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{:?}] {}: {}", self.severity, self.field, self.message)
    }
}

/// Result of schema validation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ValidationResult {
    pub errors: Vec<ValidationError>,
}

impl ValidationResult {
    pub fn ok() -> Self {
        Self { errors: Vec::new() }
    }

    pub fn from_errors(errors: Vec<ValidationError>) -> Self {
        Self { errors }
    }

    pub fn is_valid(&self) -> bool {
        !self
            .errors
            .iter()
            .any(|e| e.severity == ValidationSeverity::Error)
    }

    pub fn is_ok(&self) -> bool {
        self.is_valid()
    }

    pub fn has_errors(&self) -> bool {
        self.errors
            .iter()
            .any(|e| e.severity == ValidationSeverity::Error)
    }

    pub fn has_warnings(&self) -> bool {
        self.errors
            .iter()
            .any(|e| e.severity == ValidationSeverity::Warning)
    }

    pub fn add_error(&mut self, field: impl Into<String>, message: impl Into<String>) {
        self.errors.push(ValidationError::error(field, message));
    }

    pub fn add_warning(&mut self, field: impl Into<String>, message: impl Into<String>) {
        self.errors.push(ValidationError::warning(field, message));
    }

    pub fn merge(&mut self, other: ValidationResult) {
        self.errors.extend(other.errors);
    }

    pub fn into_result(self) -> Result<()> {
        if self.has_errors() {
            Err(AxiomError::SignalValidation {
                message: self.to_string(),
            })
        } else {
            Ok(())
        }
    }
}

impl AddAssign for ValidationResult {
    fn add_assign(&mut self, other: Self) {
        self.merge(other);
    }
}

impl std::fmt::Display for ValidationResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_valid() && !self.has_warnings() {
            write!(f, "valid")
        } else {
            for e in &self.errors {
                writeln!(f, "{}", e)?;
            }
            Ok(())
        }
    }
}

/// Schema trait for validating message structure and content.
///
/// Implement this on your Signal types to define validation rules.
/// Validation is deterministic and runs synchronously (no async).
pub trait Schema {
    /// Validate this message against its schema.
    fn validate(&self) -> ValidationResult;

    /// Maximum serialized size in bytes (0 = unlimited).
    fn max_size_bytes() -> usize
    where
        Self: Sized,
    {
        0
    }

    /// Estimated token count for LLM context budgeting.
    fn estimate_tokens(&self) -> usize {
        0
    }
}

/// Built-in schema validators.
pub mod validators {
    use super::*;

    pub fn require_non_empty(field: &str, value: &str) -> ValidationResult {
        let mut result = ValidationResult::default();
        if value.is_empty() {
            result.add_error(field, "must not be empty");
        }
        result
    }

    pub fn require_max_length(field: &str, value: &str, max: usize) -> ValidationResult {
        let mut result = ValidationResult::default();
        if value.len() > max {
            result.add_error(field, format!("exceeds max length of {}", max));
        }
        result
    }

    pub fn require_min_length(field: &str, value: &str, min: usize) -> ValidationResult {
        let mut result = ValidationResult::default();
        if value.len() < min {
            result.add_error(field, format!("below min length of {}", min));
        }
        result
    }

    pub fn require_max_size<T: Serialize>(
        field: &str,
        value: &T,
        max_bytes: usize,
    ) -> ValidationResult {
        let mut result = ValidationResult::default();
        if let Ok(json) = serde_json::to_vec(value) {
            if json.len() > max_bytes {
                result.add_error(
                    field,
                    format!("serialized size {} exceeds max {}", json.len(), max_bytes),
                );
            }
        }
        result
    }

    pub fn require_in_range<T: PartialOrd + std::fmt::Display>(
        field: &str,
        value: T,
        min: T,
        max: T,
    ) -> ValidationResult {
        let mut result = ValidationResult::default();
        if value < min || value > max {
            result.add_error(
                field,
                format!("value {} outside range [{}, {}]", value, min, max),
            );
        }
        result
    }

    pub fn require_id_format(field: &str, value: &str) -> ValidationResult {
        let mut result = ValidationResult::default();
        if value.is_empty() {
            result.add_error(field, "id must not be empty");
        } else if !value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
        {
            result.add_error(
                field,
                "id must contain only alphanumerics, '-', '_', or '.'",
            );
        }
        result
    }

    pub fn require_url_like(field: &str, value: &str) -> ValidationResult {
        let mut result = ValidationResult::default();
        if value.is_empty() {
            result.add_error(field, "URL must not be empty");
        } else if !(value.starts_with("http://")
            || value.starts_with("https://")
            || value.starts_with("cell://")
            || value.starts_with("axiom://"))
        {
            result.add_warning(
                field,
                "URL should start with http://, https://, cell://, or axiom://",
            );
        }
        result
    }

    pub fn require_none_if<T>(
        field: &str,
        value: &Option<T>,
        condition: bool,
        msg: &str,
    ) -> ValidationResult {
        let mut result = ValidationResult::default();
        if condition && value.is_some() {
            result.add_error(field, msg);
        }
        result
    }

    pub fn require_some<T>(field: &str, value: &Option<T>, msg: &str) -> ValidationResult {
        let mut result = ValidationResult::default();
        if value.is_none() {
            result.add_error(field, msg);
        }
        result
    }

    pub fn require_contains_only(field: &str, value: &str, allowed: &[char]) -> ValidationResult {
        let mut result = ValidationResult::default();
        for c in value.chars() {
            if !allowed.contains(&c) {
                result.add_error(field, format!("contains disallowed character '{}'", c));
                break;
            }
        }
        result
    }
}
