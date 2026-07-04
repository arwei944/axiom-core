//! Memory item types and structures.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryItemType {
    Thought,
    Observation,
    Action,
    Result,
    System,
    Goal,
    Plan,
    Reflection,
}

impl MemoryItemType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryItemType::Thought => "thought",
            MemoryItemType::Observation => "observation",
            MemoryItemType::Action => "action",
            MemoryItemType::Result => "result",
            MemoryItemType::System => "system",
            MemoryItemType::Goal => "goal",
            MemoryItemType::Plan => "plan",
            MemoryItemType::Reflection => "reflection",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryItem {
    pub id: String,
    pub item_type: MemoryItemType,
    pub content: String,
    pub token_estimate: usize,
    pub timestamp: u64,
    pub importance: f64,
    pub tags: Vec<String>,
    pub metadata: serde_json::Value,
}

impl MemoryItem {
    pub fn new(item_type: MemoryItemType, content: impl Into<String>) -> Self {
        let content = content.into();
        let token_estimate = estimate_tokens(&content);
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            item_type,
            content,
            token_estimate,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            importance: 0.5,
            tags: Vec::new(),
            metadata: serde_json::Value::Null,
        }
    }

    pub fn with_importance(mut self, importance: f64) -> Self {
        self.importance = importance.clamp(0.0, 1.0);
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn thought(content: impl Into<String>) -> Self {
        Self::new(MemoryItemType::Thought, content)
    }

    pub fn observation(content: impl Into<String>) -> Self {
        Self::new(MemoryItemType::Observation, content)
    }

    pub fn action(content: impl Into<String>) -> Self {
        Self::new(MemoryItemType::Action, content)
    }

    pub fn result(content: impl Into<String>) -> Self {
        Self::new(MemoryItemType::Result, content)
    }

    pub fn goal(content: impl Into<String>) -> Self {
        Self::new(MemoryItemType::Goal, content).with_importance(0.9)
    }

    pub fn plan(content: impl Into<String>) -> Self {
        Self::new(MemoryItemType::Plan, content).with_importance(0.7)
    }
}

pub fn estimate_tokens(text: &str) -> usize {
    let char_count = text.chars().count();
    let word_count = text.split_whitespace().count();
    ((char_count + word_count) / 2).max(1)
}
