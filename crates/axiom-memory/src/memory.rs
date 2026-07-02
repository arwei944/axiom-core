//! Working memory implementation.

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;
use serde_json::Value;

use crate::item::{estimate_tokens, MemoryItem, MemoryItemType};

#[derive(Debug, thiserror::Error)]
pub enum MemoryError {
    #[error("memory not found: {0}")]
    NotFound(String),
    #[error("out of token budget")]
    OutOfBudget,
    #[error("serialization error: {0}")]
    Serialization(String),
}

pub struct WorkingMemory {
    items: Arc<RwLock<Vec<MemoryItem>>>,
    token_budget: usize,
    auto_summarize: bool,
    summary_threshold: f64,
    tags_index: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl WorkingMemory {
    pub fn new(token_budget: usize) -> Self {
        Self {
            items: Arc::new(RwLock::new(Vec::new())),
            token_budget,
            auto_summarize: true,
            summary_threshold: 0.8,
            tags_index: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_auto_summarize(mut self, enabled: bool) -> Self {
        self.auto_summarize = enabled;
        self
    }

    pub fn with_summary_threshold(mut self, threshold: f64) -> Self {
        self.summary_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    pub fn add(&self, item: MemoryItem) {
        for tag in &item.tags {
            self.tags_index
                .write()
                .entry(tag.clone())
                .or_default()
                .push(item.id.clone());
        }

        self.items.write().push(item);

        if self.auto_summarize {
            self.check_and_summarize();
        }
    }

    pub fn add_thought(&self, content: impl Into<String>) -> String {
        let item = MemoryItem::thought(content);
        let id = item.id.clone();
        self.add(item);
        id
    }

    pub fn add_observation(&self, content: impl Into<String>) -> String {
        let item = MemoryItem::observation(content);
        let id = item.id.clone();
        self.add(item);
        id
    }

    pub fn add_action(&self, content: impl Into<String>) -> String {
        let item = MemoryItem::action(content);
        let id = item.id.clone();
        self.add(item);
        id
    }

    pub fn add_result(&self, content: impl Into<String>) -> String {
        let item = MemoryItem::result(content);
        let id = item.id.clone();
        self.add(item);
        id
    }

    pub fn get(&self, id: &str) -> Option<MemoryItem> {
        self.items.read().iter().find(|i| i.id == id).cloned()
    }

    pub fn all(&self) -> Vec<MemoryItem> {
        self.items.read().clone()
    }

    pub fn filter_by_type(&self, item_type: MemoryItemType) -> Vec<MemoryItem> {
        self.items
            .read()
            .iter()
            .filter(|i| i.item_type == item_type)
            .cloned()
            .collect()
    }

    pub fn filter_by_tag(&self, tag: &str) -> Vec<MemoryItem> {
        let ids = self
            .tags_index
            .read()
            .get(tag)
            .cloned()
            .unwrap_or_default();

        let items = self.items.read();
        ids.iter()
            .filter_map(|id| items.iter().find(|i| i.id == *id).cloned())
            .collect()
    }

    pub fn search(&self, query: &str) -> Vec<(MemoryItem, f64)> {
        let items = self.items.read();
        let mut results: Vec<(MemoryItem, f64)> = items
            .iter()
            .map(|item| {
                let score = relevance_score(&item.content, query) * item.importance;
                (item.clone(), score)
            })
            .filter(|(_, score)| *score > 0.0)
            .collect();

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results
    }

    pub fn token_usage(&self) -> usize {
        self.items.read().iter().map(|i| i.token_estimate).sum()
    }

    pub fn remaining_budget(&self) -> usize {
        self.token_budget.saturating_sub(self.token_usage())
    }

    pub fn item_count(&self) -> usize {
        self.items.read().len()
    }

    pub fn clear(&self) {
        self.items.write().clear();
        self.tags_index.write().clear();
    }

    pub fn remove(&self, id: &str) -> bool {
        let mut items = self.items.write();
        if let Some(pos) = items.iter().position(|i| i.id == id) {
            let item = items.remove(pos);
            for tag in &item.tags {
                if let Some(tag_ids) = self.tags_index.write().get_mut(tag) {
                    tag_ids.retain(|i| i != &item.id);
                }
            }
            true
        } else {
            false
        }
    }

    fn check_and_summarize(&self) {
        let usage = self.token_usage();
        let threshold = (self.token_budget as f64 * self.summary_threshold) as usize;

        if usage > threshold {
            self.summarize_low_importance();
        }
    }

    fn summarize_low_importance(&self) {
        let mut items = self.items.write();
        let mut indices_to_remove: Vec<usize> = Vec::new();
        let mut summary_text = String::new();

        let mut sorted: Vec<(usize, f64)> = items
            .iter()
            .enumerate()
            .map(|(idx, item)| (idx, item.importance))
            .collect();
        sorted.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut removed_tokens = 0;
        let target_tokens = (self.token_budget as f64 * 0.5) as usize;
        let current_tokens: usize = items.iter().map(|i| i.token_estimate).sum();

        for (idx, _) in sorted {
            if current_tokens - removed_tokens <= target_tokens {
                break;
            }

            if let Some(item) = items.get(idx) {
                if item.item_type != MemoryItemType::Goal
                    && item.item_type != MemoryItemType::Plan
                {
                    indices_to_remove.push(idx);
                    removed_tokens += item.token_estimate;
                    if !summary_text.is_empty() {
                        summary_text.push('\n');
                    }
                    summary_text.push_str(&format!(
                        "[{}] {}",
                        item.item_type.as_str(),
                        &item.content[..item.content.len().min(100)]
                    ));
                }
            }
        }

        indices_to_remove.sort_unstable_by(|a, b| b.cmp(a));

        for idx in indices_to_remove {
            items.remove(idx);
        }

        if !summary_text.is_empty() {
            let summary = MemoryItem::new(
                MemoryItemType::Reflection,
                format!("Summary of {} older memory items:\n{}", items.len(), summary_text),
            )
            .with_importance(0.3);
            items.push(summary);
        }
    }

    pub fn render_as_prompt(&self) -> String {
        let items = self.items.read();
        let mut result = String::new();

        for item in items.iter() {
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str(&format!(
                "[{}] {}",
                item.item_type.as_str().to_uppercase(),
                item.content
            ));
        }

        result
    }

    pub fn render_with_limit(&self, max_tokens: usize) -> String {
        let items = self.items.read();
        let mut result = String::new();
        let mut tokens_used = 0;

        for item in items.iter().rev() {
            let item_tokens = item.token_estimate;
            if tokens_used + item_tokens > max_tokens {
                break;
            }
            if !result.is_empty() {
                result = format!("[{}] {}\n{}", item.item_type.as_str().to_uppercase(), item.content, result);
            } else {
                result = format!("[{}] {}", item.item_type.as_str().to_uppercase(), item.content);
            }
            tokens_used += item_tokens;
        }

        result
    }
}

fn relevance_score(content: &str, query: &str) -> f64 {
    if query.is_empty() || content.is_empty() {
        return 0.0;
    }

    let content_lower = content.to_lowercase();
    let query_lower = query.to_lowercase();

    let query_words: Vec<&str> = query_lower.split_whitespace().collect();
    if query_words.is_empty() {
        return 0.0;
    }

    let mut matches = 0;
    for word in &query_words {
        if content_lower.contains(word) {
            matches += 1;
        }
    }

    matches as f64 / query_words.len() as f64
}

impl Default for WorkingMemory {
    fn default() -> Self {
        Self::new(4000)
    }
}