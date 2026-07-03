//! Threshold and window evaluation.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ThresholdKind {
    Gt,
    Lt,
    Eq,
    Ne,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Threshold {
    pub kind: ThresholdKind,
    pub value: f64,
}

impl Default for Threshold {
    fn default() -> Self {
        Self {
            kind: ThresholdKind::Gt,
            value: 0.0,
        }
    }
}

const EPSILON: f64 = 1e-9;

impl Threshold {
    pub fn matches(&self, current: f64) -> bool {
        match self.kind {
            ThresholdKind::Gt => current > self.value,
            ThresholdKind::Lt => current < self.value,
            ThresholdKind::Eq => (current - self.value).abs() < EPSILON,
            ThresholdKind::Ne => (current - self.value).abs() >= EPSILON,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum WindowKind {
    TimeMs(u64),
    Count(u64),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Window {
    pub kind: WindowKind,
    pub min_hits: u64,
}

impl Default for Window {
    fn default() -> Self {
        Self {
            kind: WindowKind::Count(1),
            min_hits: 1,
        }
    }
}
