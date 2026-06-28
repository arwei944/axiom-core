# P12-P17 开发任务书：记忆系统 + 规划器 + 提示词/RAG + 测试评估 + CLI完善 + 示例

> 本文档覆盖P12（记忆系统）、P13（规划器）、P14（提示词+RAG）、P15（测试+评估）、P16（CLI完善）、P17（示例+文档）共六个阶段，T136-T199。
>
> **前置依赖**：P9（axiom-llm、axiom-tool）、P10（axiom-mcp）、P11（CLI脚手架）必须全部验收通过。
>
> **约束**：注释中文，rustdoc英文；MSRV 1.75；禁止async-trait（使用原生`async fn in traits`+`#![allow(async_fn_in_trait)]`）；
>          所有public API需有`///` rustdoc和`#[derive(Debug)]`；cargo build/clippy/test 均使用`-D warnings`；
>          每个crate的lib.rs顶部加`#![allow(async_fn_in_trait)]`。

---

## 任务依赖总览

```
P12: axiom-memory
  T136: 创建axiom-memory骨架
    ↓
  T137: MemoryEntry结构体
    ↓
  T138: WorkingMemory (L1)
    ↓
  T139: EpisodicMemory (L2)
    ↓
  T140: MemoryRetrieval trait
    ↓
  T141: SemanticMemory (L3, SQLite FTS5)
    ↓
  T142: MemorySummarizer
    ↓
  T143: TokenBudget
    ↓
  T144: MemoryLens (impl Lens)
    ↓
  T145: 测试
    ↓
P13: axiom-planner
  T146: 创建axiom-planner骨架
    ↓
  T147-T148: Plan/PlanStep类型
    ↓
  T149: ReAct planner
    ↓
  T150: Plan-Execute planner
    ↓
  T151: PlanExecutor
    ↓
  T152: Replanning trigger
    ↓
  T153: Plan可视化 (Timeline导出)
    ↓
  T154: 测试
    ↓
P14: axiom-prompt + axiom-rag
  T155: 创建axiom-prompt骨架
    ↓ T156-T161: PromptTemplate/PromptComposer/Token预算/测试
  T162: 创建axiom-rag骨架
    ↓ T163-T169: Document/BM25/DocumentStore/RagPipeline/Chunking/测试
    ↓
P15: axiom-test + axiom-eval
  T170: 创建axiom-test骨架
    ↓ T171-T176: MockLlm/MockTool/FaultInjector/TestRuntime/RecordReplay/测试
  T177: 创建axiom-eval骨架
    ↓ T178-T184: EvalCase/GoldenSet/EvalRunner/Metrics/RegressionDetector/CLI/测试
    ↓
P16: axiom-cli增强
  T185-T191: shell/replay/test/cell/config/completion/man
    ↓
P17: examples/ + 文档
  T192-T199: 五个示例+README+集成测试+最终文档
    ↓
最终验收: 全workspace验证
```

---

## P12阶段：记忆系统 (axiom-memory)

### T136：创建axiom-memory crate骨架

**文件修改**：
- `Cargo.toml`（workspace根）
- 新建：`crates/axiom-memory/Cargo.toml`
- 新建：`crates/axiom-memory/src/lib.rs`

**具体操作**：

1. 在根`Cargo.toml`的`[workspace.members]`中添加：
   ```toml
   "crates/axiom-memory",
   ```

2. 在根`Cargo.toml`的`[workspace.dependencies]`中添加（如尚未添加）：
   ```toml
   rusqlite = { version = "0.31", features = ["bundled"] }
   ```

3. 创建`crates/axiom-memory/Cargo.toml`：
   ```toml
   [package]
   name = "axiom-memory"
   version.workspace = true
   edition.workspace = true
   license.workspace = true
   rust-version.workspace = true
   description = "Four-layer memory system with token budgets and auto-summarization"

   [features]
   default = ["sqlite"]
   sqlite = ["dep:rusqlite"]

   [dependencies]
   axiom-core = { workspace = true }
   serde = { workspace = true }
   serde_json = { workspace = true }
   thiserror = { workspace = true }
   tracing = { workspace = true }
   tokio = { workspace = true }
   futures = { workspace = true }
   rusqlite = { workspace = true, optional = true }

   [dev-dependencies]
   tokio = { workspace = true, features = ["rt-multi-thread", "macros", "time"] }
   tracing-subscriber = { workspace = true }
   tempfile = "3"
   ```

4. 创建`crates/axiom-memory/src/lib.rs`：
   ```rust
   //! Axiom Memory - Four-layer memory system for agent context management.
   //!
   //! Layers:
   //! - L1 Working Memory: Current context window, token-budgeted with LRU eviction
   //! - L2 Episodic Memory: Recent interactions stored in a ring buffer
   //! - L3 Semantic Memory: Long-term knowledge with FTS5 search (SQLite)
   //! - L4 Procedural Memory: Learned skills and rules (from evolution engine)
   //!
   //! MemoryLens implements the Lens trait from axiom-core, projecting relevant
   //! memories into the agent's context window based on the current query.

   #![allow(async_fn_in_trait)]

   pub mod entry;
   pub mod error;
   pub mod working;
   pub mod episodic;
   pub mod retrieval;
   pub mod semantic;
   pub mod summarizer;
   pub mod budget;
   pub mod lens;
   pub mod procedural;

   pub use entry::{MemoryEntry, MemoryLayer};
   pub use error::{MemoryError, MemoryResult};
   pub use working::WorkingMemory;
   pub use episodic::EpisodicMemory;
   pub use retrieval::{MemoryRetrieval, RetrievedMemory, RelevanceScore};
   pub use semantic::SemanticMemory;
   pub use summarizer::MemorySummarizer;
   pub use budget::{TokenBudget, TokenCount, BudgetAllocation};
   pub use lens::MemoryLens;
   pub use procedural::ProceduralMemory;
   ```

5. 为所有模块文件创建空骨架（`pub struct`占位或空文件均可，lib.rs中的`pub mod`声明需能编译）：
   - `entry.rs`、`error.rs`、`working.rs`、`episodic.rs`、`retrieval.rs`
   - `semantic.rs`、`summarizer.rs`、`budget.rs`、`lens.rs`、`procedural.rs`

**验收标准**：
- [ ] `cargo build -p axiom-memory` 编译通过零警告
- [ ] `cargo clippy -p axiom-memory -- -D warnings` 零警告
- [ ] lib.rs模块声明完整，re-export正确
- [ ] rusqlite通过feature gate（default feature启用，不强制依赖）

**Commit message**：
```
feat(memory): create axiom-memory crate skeleton (T136)
```

---

### T137：定义MemoryEntry结构体 + MemoryLayer枚举

**新建/修改文件**：
- `crates/axiom-memory/src/entry.rs`
- `crates/axiom-memory/src/error.rs`

**具体操作**：

1. 创建`error.rs`：
   ```rust
   //! Memory error types.

   use axiom_core::AxiomError;
   use thiserror::Error;

   /// Errors from memory operations.
   #[derive(Error, Debug)]
   pub enum MemoryError {
       /// Token budget exceeded.
       #[error("Token budget exceeded: used {used}, budget {budget}")]
       TokenBudgetExceeded { used: usize, budget: usize },

       /// Entry not found in memory store.
       #[error("Memory entry not found: {0}")]
       NotFound(String),

       /// Storage backend error.
       #[error("Storage error: {0}")]
       Storage(String),

       /// Serialization/deserialization error.
       #[error("Serialization error: {0}")]
       Serde(#[from] serde_json::Error),

       /// Database error (SQLite).
       #[cfg(feature = "sqlite")]
       #[error("Database error: {0}")]
       Database(#[from] rusqlite::Error),

       /// IO error.
       #[error("IO error: {0}")]
       Io(#[from] std::io::Error),

       /// Summarization error.
       #[error("Summarization failed: {0}")]
       Summarization(String),

       /// Internal error.
       #[error("Internal error: {0}")]
       Internal(String),
   }

   impl From<MemoryError> for AxiomError {
       fn from(e: MemoryError) -> Self {
           AxiomError::Internal(format!("memory: {}", e))
       }
   }

   /// Result type for memory operations.
   pub type MemoryResult<T> = std::result::Result<T, MemoryError>;
   ```

2. 创建`entry.rs`：
   ```rust
   //! Memory entry types.

   use axiom_core::id::CorrelationId;
   use serde::{Deserialize, Serialize};
   use std::time::{SystemTime, UNIX_EPOCH};

   /// Memory layer identifier.
   #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
   pub enum MemoryLayer {
       /// L1: Working memory (in-context, token-budgeted).
       Working,
       /// L2: Episodic memory (recent turns, ring buffer).
       Episodic,
       /// L3: Semantic memory (long-term knowledge, searchable).
       Semantic,
       /// L4: Procedural memory (learned skills/rules).
       Procedural,
   }

   impl std::fmt::Display for MemoryLayer {
       fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
             match self {
                 MemoryLayer::Working => write!(f, "working"),
                 MemoryLayer::Episodic => write!(f, "episodic"),
                 MemoryLayer::Semantic => write!(f, "semantic"),
                 MemoryLayer::Procedural => write!(f, "procedural"),
             }
        }
   }

   /// Importance score for memory entries (0.0 = least important, 1.0 = most important).
   #[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
   pub struct Importance(pub f32);

   impl Importance {
       /// Lowest importance.
       pub const LOW: Importance = Importance(0.1);
       /// Medium importance.
       pub const MEDIUM: Importance = Importance(0.5);
       /// Highest importance.
       pub const HIGH: Importance = Importance(1.0);

       /// Create a new importance score, clamped to [0.0, 1.0].
       pub fn new(score: f32) -> Self {
             Self(score.clamp(0.0, 1.0))
        }
   }

   impl Default for Importance {
       fn default() -> Self {
             Self::MEDIUM
        }
   }

   /// A single memory entry across all layers.
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct MemoryEntry {
       /// Unique identifier for this memory entry.
       pub id: String,
       /// The textual content of this memory.
       pub content: String,
       /// Optional embedding vector for similarity search (L3).
       pub embedding: Option<Vec<f32>>,
       /// Timestamp in nanoseconds since UNIX epoch.
       pub timestamp_ns: u64,
       /// Importance score used for eviction prioritization.
       pub importance: Importance,
       /// Number of times this entry has been accessed (for recency/frequency scoring).
       pub access_count: u64,
       /// Last access timestamp in nanoseconds.
       pub last_accessed_ns: u64,
       /// Which memory layer this entry belongs to.
       pub layer: MemoryLayer,
       /// Associated correlation ID for traceability.
       pub correlation_id: Option<CorrelationId>,
       /// Source tag (e.g., "user", "assistant", "tool", "summary").
       pub source: String,
       /// Estimated token count for budget calculations.
       pub token_count: usize,
       /// Arbitrary metadata.
       pub metadata: serde_json::Value,
   }

   impl MemoryEntry {
       /// Create a new memory entry with default metadata.
       pub fn new(id: impl Into<String>, content: impl Into<String>, layer: MemoryLayer) -> Self {
             let now = now_ns();
             Self {
                 id: id.into(),
                 content: content.into(),
                 embedding: None,
                 timestamp_ns: now,
                 importance: Importance::default(),
                 access_count: 0,
                 last_accessed_ns: now,
                 layer,
                 correlation_id: None,
                 source: "unknown".to_string(),
                 token_count: 0,
                 metadata: serde_json::Value::Null,
             }
        }

       /// Set importance score.
       pub fn with_importance(mut self, importance: Importance) -> Self {
             self.importance = importance;
             self
        }

       /// Set source tag.
       pub fn with_source(mut self, source: impl Into<String>) -> Self {
             self.source = source.into();
             self
        }

       /// Set correlation ID.
       pub fn with_correlation_id(mut self, cid: CorrelationId) -> Self {
             self.correlation_id = Some(cid);
             self
        }

       /// Set embedding vector.
       pub fn with_embedding(mut self, embedding: Vec<f32>) -> Self {
             self.embedding = Some(embedding);
             self
        }

       /// Set estimated token count.
       pub fn with_token_count(mut self, tokens: usize) -> Self {
             self.token_count = tokens;
             self
        }

       /// Record an access to this entry (updates access_count and last_accessed_ns).
       pub fn record_access(&mut self) {
             self.access_count += 1;
             self.last_accessed_ns = now_ns();
        }

       /// Compute a recency score (1.0 = just now, decays over time).
       pub fn recency_score(&self, now_ns: u64, half_life_ns: u64) -> f32 {
             let age_ns = now_ns.saturating_sub(self.last_accessed_ns);
             let half_lives = age_ns as f64 / half_life_ns as f64;
             (0.5_f64.powf(half_lives)) as f32
        }

       /// Compute a combined retrieval score (importance + recency + frequency).
       pub fn retrieval_score(&self, now_ns: u64) -> f32 {
             let recency = self.recency_score(now_ns, 3_600_000_000_000); // 1 hour half-life
             let frequency = (self.access_count as f32).ln_1p() / 10.0;
             self.importance.0 * 0.5 + recency * 0.3 + frequency.min(1.0) * 0.2
        }
   }

   fn now_ns() -> u64 {
       SystemTime::now()
           .duration_since(UNIX_EPOCH)
           .unwrap_or_default()
           .as_nanos() as u64
   }

   #[cfg(test)]
   mod tests {
       use super::*;

       #[test]
       fn test_importance_clamping() {
             assert_eq!(Importance::new(1.5).0, 1.0);
             assert_eq!(Importance::new(-0.5).0, 0.0);
             assert_eq!(Importance::new(0.5).0, 0.5);
        }

       #[test]
       fn test_memory_entry_creation() {
             let entry = MemoryEntry::new("m1", "hello world", MemoryLayer::Working)
                 .with_importance(Importance::HIGH)
                 .with_source("user");
             assert_eq!(entry.id, "m1");
             assert_eq!(entry.content, "hello world");
             assert_eq!(entry.layer, MemoryLayer::Working);
             assert_eq!(entry.importance, Importance::HIGH);
             assert_eq!(entry.source, "user");
             assert_eq!(entry.access_count, 0);
        }

       #[test]
       fn test_record_access() {
             let mut entry = MemoryEntry::new("m1", "test", MemoryLayer::Working);
             entry.record_access();
             entry.record_access();
             assert_eq!(entry.access_count, 2);
        }
   }
   ```

**验收标准**：
- [ ] `cargo build -p axiom-memory` 编译通过
- [ ] `cargo test -p axiom-memory` 通过
- [ ] MemoryEntry所有字段有合理默认值
- [ ] Importance clamp到[0,1]
- [ ] retrieval_score公式正确
- [ ] MemoryError可转换为AxiomError

**Commit message**：
```
feat(memory): define MemoryEntry and MemoryLayer types (T137)
```

---

### T138：实现WorkingMemory (L1: token-budgeted LRU eviction)

**新建文件**：
- `crates/axiom-memory/src/working.rs`

**具体操作**：

1. 创建`working.rs`：
   ```rust
   //! Working Memory (L1) - token-budgeted in-context memory with LRU eviction.

   use crate::entry::{MemoryEntry, MemoryLayer};
   use crate::error::MemoryResult;
   use std::collections::HashMap;

   /// Working memory maintains a bounded set of entries within a token budget.
   /// Eviction uses LRU (least recently accessed) when the budget is exceeded.
   #[derive(Debug)]
   pub struct WorkingMemory {
       /// Maximum tokens allowed in working memory.
       max_tokens: usize,
       /// Current total token count.
       current_tokens: usize,
       /// Stored entries keyed by ID.
       entries: HashMap<String, MemoryEntry>,
       /// Access order for LRU (most recent at the end).
       access_order: Vec<String>,
   }

   impl WorkingMemory {
       /// Create a new working memory with the given token budget.
       pub fn new(max_tokens: usize) -> Self {
             Self {
                 max_tokens,
                 current_tokens: 0,
                 entries: HashMap::new(),
                 access_order: Vec::new(),
             }
        }

       /// Current token usage.
       pub fn token_count(&self) -> usize {
             self.current_tokens
        }

       /// Maximum token budget.
       pub fn max_tokens(&self) -> usize {
             self.max_tokens
        }

       /// Number of entries in working memory.
       pub fn len(&self) -> usize {
             self.entries.len()
        }

       /// Whether working memory is empty.
       pub fn is_empty(&self) -> bool {
             self.entries.is_empty()
        }

       /// Insert an entry into working memory. Evicts LRU entries if over budget.
       pub fn insert(&mut self, mut entry: MemoryEntry) -> MemoryResult<Vec<MemoryEntry>> {
             entry.layer = MemoryLayer::Working;
             let entry_tokens = entry.token_count.max(1);
             entry.token_count = entry_tokens;

             // 如果已存在同ID条目，先移除
             if let Some(existing) = self.entries.remove(&entry.id) {
                 self.current_tokens = self.current_tokens.saturating_sub(existing.token_count);
                 self.access_order.retain(|id| id != &entry.id);
             }

             // 容量不足时按LRU驱逐
             let mut evicted = Vec::new();
             while self.current_tokens + entry_tokens > self.max_tokens && !self.access_order.is_empty() {
                 let victim_id = self.access_order.remove(0);
                 if let Some(victim) = self.entries.remove(&victim_id) {
                     self.current_tokens = self.current_tokens.saturating_sub(victim.token_count);
                     evicted.push(victim);
                 }
             }

             // 如果单条entry就超过max_tokens，直接拒绝
             if entry_tokens > self.max_tokens {
                 return Err(crate::error::MemoryError::TokenBudgetExceeded {
                     used: entry_tokens,
                     budget: self.max_tokens,
                 });
             }

             self.current_tokens += entry_tokens;
             self.entries.insert(entry.id.clone(), entry);
             self.access_order.push(entry.id);

             Ok(evicted)
        }

       /// Retrieve an entry by ID, marking it as recently used.
       pub fn get(&mut self, id: &str) -> Option<&MemoryEntry> {
             if let Some(entry) = self.entries.get_mut(id) {
                 entry.record_access();
                 // 更新LRU顺序
                 self.access_order.retain(|x| x != id);
                 self.access_order.push(id.to_string());
             }
             self.entries.get(id)
        }

       /// Remove an entry by ID.
       pub fn remove(&mut self, id: &str) -> Option<MemoryEntry> {
             if let Some(entry) = self.entries.remove(id) {
                 self.current_tokens = self.current_tokens.saturating_sub(entry.token_count);
                 self.access_order.retain(|x| x != id);
                 Some(entry)
             } else {
                 None
             }
        }

       /// Get all entries in LRU order (oldest first).
       pub fn entries(&self) -> Vec<&MemoryEntry> {
             self.access_order
                 .iter()
                 .filter_map(|id| self.entries.get(id))
                 .collect()
        }

       /// Get all entries in recency order (most recent first).
       pub fn entries_recent(&self) -> Vec<&MemoryEntry> {
             let mut entries: Vec<&MemoryEntry> = self.entries.values().collect();
             entries.sort_by(|a, b| b.last_accessed_ns.cmp(&a.last_accessed_ns));
             entries
        }

       /// Clear all entries.
       pub fn clear(&mut self) {
             self.entries.clear();
             self.access_order.clear();
             self.current_tokens = 0;
        }
   }

   #[cfg(test)]
   mod tests {
       use super::*;

       #[test]
       fn test_insert_within_budget() {
             let mut wm = WorkingMemory::new(100);
             let entry = MemoryEntry::new("m1", "hello", MemoryLayer::Working).with_token_count(10);
             let evicted = wm.insert(entry).unwrap();
             assert!(evicted.is_empty());
             assert_eq!(wm.len(), 1);
             assert_eq!(wm.token_count(), 10);
        }

       #[test]
       fn test_lru_eviction() {
             let mut wm = WorkingMemory::new(30);
             wm.insert(MemoryEntry::new("m1", "a", MemoryLayer::Working).with_token_count(10)).unwrap();
             wm.insert(MemoryEntry::new("m2", "b", MemoryLayer::Working).with_token_count(10)).unwrap();
             wm.insert(MemoryEntry::new("m3", "c", MemoryLayer::Working).with_token_count(10)).unwrap();
             assert_eq!(wm.len(), 3);
             assert_eq!(wm.token_count(), 30);

             // 访问m1使其变新
             wm.get("m1");
             // 插入m4，应该驱逐m2（最老且m1刚被访问过）
             wm.insert(MemoryEntry::new("m4", "d", MemoryLayer::Working).with_token_count(10)).unwrap();
             assert!(wm.entries.contains_key("m1"));
             assert!(!wm.entries.contains_key("m2"));
             assert!(wm.entries.contains_key("m3"));
             assert!(wm.entries.contains_key("m4"));
        }

       #[test]
       fn test_entry_exceeds_budget() {
             let mut wm = WorkingMemory::new(5);
             let result = wm.insert(MemoryEntry::new("m1", "big", MemoryLayer::Working).with_token_count(10));
             assert!(result.is_err());
        }

       #[test]
       fn test_remove_entry() {
             let mut wm = WorkingMemory::new(100);
             wm.insert(MemoryEntry::new("m1", "a", MemoryLayer::Working).with_token_count(10)).unwrap();
             assert_eq!(wm.len(), 1);
             wm.remove("m1");
             assert_eq!(wm.len(), 0);
             assert_eq!(wm.token_count(), 0);
        }
   }
   ```

**验收标准**：
- [ ] `cargo test -p axiom-memory` 通过（含working模块测试）
- [ ] LRU驱逐顺序正确（访问后刷新顺序）
- [ ] token计数准确，不超过max_tokens
- [ ] 单条超预算entry返回错误
- [ ] 同ID插入替换旧entry

**Commit message**：
```
feat(memory): implement WorkingMemory with LRU eviction (T138)
```

---

### T139：实现EpisodicMemory (L2: ring buffer + auto-compact)

**新建文件**：
- `crates/axiom-memory/src/episodic.rs`

**具体操作**：

1. 创建`episodic.rs`：
   ```rust
   //! Episodic Memory (L2) - ring buffer of recent interaction turns with auto-compaction.

   use crate::entry::{MemoryEntry, MemoryLayer};
   use crate::error::MemoryResult;
   use std::collections::VecDeque;

   /// Episodic memory stores a bounded number of recent interaction turns.
   /// When full, old entries are evicted (returned for summarization).
   #[derive(Debug)]
   pub struct EpisodicMemory {
       /// Maximum number of turns to keep.
       capacity: usize,
       /// Ring buffer of entries (oldest at front).
       entries: VecDeque<MemoryEntry>,
       /// Total tokens in episodic memory.
       total_tokens: usize,
   }

   /// Summary of a compacted batch of episodic entries.
   #[derive(Debug, Clone)]
   pub struct EpisodicSummary {
       /// Entries that were compacted out.
       pub compacted_entries: Vec<MemoryEntry>,
       /// Number of entries before compaction.
       pub before_count: usize,
       /// Number of entries after compaction.
       pub after_count: usize,
   }

   impl EpisodicMemory {
       /// Create a new episodic memory with the given turn capacity.
       pub fn new(capacity: usize) -> Self {
             assert!(capacity > 0, "capacity must be > 0");
             Self {
                 capacity,
                 entries: VecDeque::with_capacity(capacity),
                 total_tokens: 0,
             }
        }

       /// Maximum number of turns.
       pub fn capacity(&self) -> usize {
             self.capacity
        }

       /// Current number of entries.
       pub fn len(&self) -> usize {
             self.entries.len()
        }

       /// Whether episodic memory is empty.
       pub fn is_empty(&self) -> bool {
             self.entries.is_empty()
        }

       /// Total tokens across all entries.
       pub fn total_tokens(&self) -> usize {
             self.total_tokens
        }

       /// Add a new turn to episodic memory.
       /// Returns entries that were evicted (oldest first) if buffer is full.
       pub fn push(&mut self, mut entry: MemoryEntry) -> MemoryResult<Vec<MemoryEntry>> {
             entry.layer = MemoryLayer::Episodic;
             self.total_tokens += entry.token_count;
             self.entries.push_back(entry);

             let mut evicted = Vec::new();
             while self.entries.len() > self.capacity {
                 if let Some(old) = self.entries.pop_front() {
                     self.total_tokens = self.total_tokens.saturating_sub(old.token_count);
                     evicted.push(old);
                 }
             }
             Ok(evicted)
        }

       /// Get entries in chronological order (oldest first).
       pub fn entries(&self) -> Vec<&MemoryEntry> {
             self.entries.iter().collect()
        }

       /// Get the most recent N entries.
       pub fn recent(&self, n: usize) -> Vec<&MemoryEntry> {
             let start = self.entries.len().saturating_sub(n);
             self.entries.iter().skip(start).collect()
        }

       /// Compact the oldest batch of entries into a summary trigger.
       /// Returns the compacted entries for the summarizer to process.
       pub fn compact_oldest(&mut self, count: usize) -> EpisodicSummary {
             let before_count = self.entries.len();
             let to_compact = count.min(self.entries.len() / 2);
             let mut compacted = Vec::new();
             for _ in 0..to_compact {
                 if let Some(entry) = self.entries.pop_front() {
                     self.total_tokens = self.total_tokens.saturating_sub(entry.token_count);
                     compacted.push(entry);
                 }
             }
             EpisodicSummary {
                 compacted_entries: compacted,
                 before_count,
                 after_count: self.entries.len(),
             }
        }

       /// Clear all entries.
       pub fn clear(&mut self) {
             self.entries.clear();
             self.total_tokens = 0;
        }
   }

   #[cfg(test)]
   mod tests {
       use super::*;

       fn make_entry(id: &str, tokens: usize) -> MemoryEntry {
             MemoryEntry::new(id, format!("content-{}", id), MemoryLayer::Episodic).with_token_count(tokens)
       }

       #[test]
       fn test_ring_buffer_eviction() {
             let mut em = EpisodicMemory::new(3);
             em.push(make_entry("m1", 10)).unwrap();
             em.push(make_entry("m2", 10)).unwrap();
             em.push(make_entry("m3", 10)).unwrap();
             assert_eq!(em.len(), 3);
             assert_eq!(em.total_tokens(), 30);

             let evicted = em.push(make_entry("m4", 10)).unwrap();
             assert_eq!(evicted.len(), 1);
             assert_eq!(evicted[0].id, "m1");
             assert_eq!(em.len(), 3);
             assert_eq!(em.total_tokens(), 30);
        }

       #[test]
       fn test_recent() {
             let mut em = EpisodicMemory::new(5);
             for i in 0..5 {
                 em.push(make_entry(&format!("m{}", i), 10)).unwrap();
             }
             let recent = em.recent(2);
             assert_eq!(recent.len(), 2);
             assert_eq!(recent[0].id, "m3");
             assert_eq!(recent[1].id, "m4");
        }

       #[test]
       fn test_compact_oldest() {
             let mut em = EpisodicMemory::new(10);
             for i in 0..6 {
                 em.push(make_entry(&format!("m{}", i), 10)).unwrap();
             }
             assert_eq!(em.len(), 6);
             let summary = em.compact_oldest(3);
             assert_eq!(summary.compacted_entries.len(), 3);
             assert_eq!(summary.before_count, 6);
             assert_eq!(summary.after_count, 3);
             assert_eq!(em.len(), 3);
        }
   }
   ```

**验收标准**：
- [ ] `cargo test -p axiom-memory` 通过
- [ ] Ring buffer容量限制正确
- [ ] token计数准确
- [ ] recent()返回最近N条
- [ ] compact_oldest()返回被压缩条目

**Commit message**：
```
feat(memory): implement EpisodicMemory ring buffer with compaction (T139)
```

---

### T140：定义MemoryRetrieval trait + RetrievedMemory/RelevanceScore

**新建文件**：
- `crates/axiom-memory/src/retrieval.rs`

**具体操作**：

1. 创建`retrieval.rs`：
   ```rust
   //! Memory retrieval trait and related types.

   use crate::entry::MemoryEntry;
   use crate::error::MemoryResult;

   /// Relevance score for retrieved memories (0.0 = irrelevant, 1.0 = perfect match).
   #[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
   pub struct RelevanceScore(pub f32);

   impl RelevanceScore {
       /// Create a new relevance score, clamped to [0.0, 1.0].
       pub fn new(score: f32) -> Self {
             Self(score.clamp(0.0, 1.0))
        }
   }

   /// A retrieved memory entry with its relevance score.
   #[derive(Debug, Clone)]
   pub struct RetrievedMemory {
       /// The memory entry.
       pub entry: MemoryEntry,
       /// Relevance score for the query.
       pub score: RelevanceScore,
       /// Which layer this was retrieved from.
       pub matched_layer: String,
   }

   /// Query parameters for memory retrieval.
   #[derive(Debug, Clone)]
   pub struct MemoryQuery {
       /// The query text.
       pub text: String,
       /// Maximum number of results to return.
       pub limit: usize,
       /// Minimum relevance score threshold.
       pub min_score: RelevanceScore,
       /// Which layers to search (empty = all).
       pub layers: Vec<crate::entry::MemoryLayer>,
       /// Maximum tokens in returned results.
       pub max_tokens: Option<usize>,
   }

   impl MemoryQuery {
       /// Create a simple text query with default parameters.
       pub fn new(text: impl Into<String>) -> Self {
             Self {
                 text: text.into(),
                 limit: 10,
                 min_score: RelevanceScore::new(0.1),
                 layers: Vec::new(),
                 max_tokens: None,
             }
        }

       /// Set result limit.
       pub fn with_limit(mut self, limit: usize) -> Self {
             self.limit = limit;
             self
        }

       /// Set minimum relevance score.
       pub fn with_min_score(mut self, score: f32) -> Self {
             self.min_score = RelevanceScore::new(score);
             self
        }

       /// Set target layers.
       pub fn with_layers(mut self, layers: Vec<crate::entry::MemoryLayer>) -> Self {
             self.layers = layers;
             self
        }

       /// Set max tokens for results.
       pub fn with_max_tokens(mut self, max: usize) -> Self {
             self.max_tokens = Some(max);
             self
        }
   }

   /// Trait for memory retrieval backends.
   pub trait MemoryRetrieval: Send + Sync {
       /// Retrieve memories relevant to the given query.
       async fn retrieve(&self, query: &MemoryQuery) -> MemoryResult<Vec<RetrievedMemory>>;

       /// Retrieve memories relevant to the query from a specific layer only.
       async fn retrieve_from_layer(
             &self,
             query: &MemoryQuery,
             layer: crate::entry::MemoryLayer,
       ) -> MemoryResult<Vec<RetrievedMemory>> {
             let mut q = query.clone();
             q.layers = vec![layer];
             self.retrieve(&q).await
        }
   }
   ```

   注意：需要在文件顶部添加`use serde::{Serialize, Deserialize};`

**验收标准**：
- [ ] `cargo build -p axiom-memory` 编译通过
- [ ] MemoryRetrieval trait是object-safe的（可以用`dyn MemoryRetrieval`）
- [ ] RelevanceScore clamp到[0,1]
- [ ] MemoryQuery builder模式易用
- [ ] 默认trait方法`retrieve_from_layer`有合理实现

**Commit message**：
```
feat(memory): define MemoryRetrieval trait and retrieval types (T140)
```

---

### T141：实现SemanticMemory (L3: SQLite FTS5存储)

**新建文件**：
- `crates/axiom-memory/src/semantic.rs`

**具体操作**：

1. 创建`semantic.rs`：
   ```rust
   //! Semantic Memory (L3) - long-term knowledge storage with SQLite FTS5 full-text search.
   //!
   //! V1 uses SQLite FTS5 for keyword-based retrieval (no heavy vector DB dependency).
   //! Embeddings are stored but not yet indexed for similarity search.

   use crate::entry::{MemoryEntry, MemoryLayer};
   use crate::error::MemoryResult;
   use crate::retrieval::{MemoryQuery, MemoryRetrieval, RelevanceScore, RetrievedMemory};

   #[cfg(feature = "sqlite")]
   use rusqlite::Connection;
   use std::path::{Path, PathBuf};

   /// Configuration for semantic memory.
   #[derive(Debug, Clone)]
   pub struct SemanticMemoryConfig {
       /// Path to SQLite database file (None = in-memory).
       pub db_path: Option<PathBuf>,
       /// Maximum number of results per query.
       pub default_limit: usize,
   }

   impl Default for SemanticMemoryConfig {
       fn default() -> Self {
             Self {
                 db_path: None,
                 default_limit: 20,
             }
        }
   }

   /// Semantic memory backed by SQLite with FTS5 full-text search.
   pub struct SemanticMemory {
       #[cfg(feature = "sqlite")]
       conn: Connection,
       config: SemanticMemoryConfig,
   }

   impl std::fmt::Debug for SemanticMemory {
       fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
             f.debug_struct("SemanticMemory")
                 .field("config", &self.config)
                 .finish()
        }
   }

   impl SemanticMemory {
       /// Open or create a semantic memory store.
       #[cfg(feature = "sqlite")]
       pub fn open(config: SemanticMemoryConfig) -> MemoryResult<Self> {
             let conn = match &config.db_path {
                 Some(path) => Connection::open(path)?,
                 None => Connection::open_in_memory()?,
             };
             Self::init_schema(&conn)?;
             Ok(Self { conn, config })
        }

       /// Create an in-memory semantic memory store for testing.
       #[cfg(feature = "sqlite")]
       pub fn in_memory() -> MemoryResult<Self> {
             Self::open(SemanticMemoryConfig::default())
        }

       #[cfg(feature = "sqlite")]
       fn init_schema(conn: &Connection) -> MemoryResult<()> {
             conn.execute_batch(
                 "CREATE TABLE IF NOT EXISTS memories (
                     id TEXT PRIMARY KEY,
                     content TEXT NOT NULL,
                     embedding_json TEXT,
                     timestamp_ns INTEGER NOT NULL,
                     importance REAL NOT NULL,
                     access_count INTEGER NOT NULL DEFAULT 0,
                     last_accessed_ns INTEGER NOT NULL,
                     source TEXT NOT NULL DEFAULT '',
                     token_count INTEGER NOT NULL DEFAULT 0,
                     metadata_json TEXT NOT NULL DEFAULT 'null'
                 );
                 CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts USING fts5(
                     content,
                     content='memories',
                     content_rowid='rowid'
                 );
                 CREATE TRIGGER IF NOT EXISTS memories_ai AFTER INSERT ON memories BEGIN
                     INSERT INTO memories_fts(rowid, content) VALUES (new.rowid, new.content);
                 END;
                 CREATE TRIGGER IF NOT EXISTS memories_ad AFTER DELETE ON memories BEGIN
                     INSERT INTO memories_fts(memories_fts, rowid, content) VALUES('delete', old.rowid, old.content);
                 END;
                 CREATE TRIGGER IF NOT EXISTS memories_au AFTER UPDATE ON memories BEGIN
                     INSERT INTO memories_fts(memories_fts, rowid, content) VALUES('delete', old.rowid, old.content);
                     INSERT INTO memories_fts(rowid, content) VALUES (new.rowid, new.content);
                 END;
                 ",
             )?;
             Ok(())
        }

       /// Store a memory entry in semantic memory.
       #[cfg(feature = "sqlite")]
       pub fn store(&self, entry: &MemoryEntry) -> MemoryResult<()> {
             let embedding_json = match &entry.embedding {
                 Some(e) => serde_json::to_string(e)?,
                 None => String::new(),
             };
             let metadata_json = serde_json::to_string(&entry.metadata)?;
             self.conn.execute(
                 "INSERT OR REPLACE INTO memories (id, content, embedding_json, timestamp_ns, importance, access_count, last_accessed_ns, source, token_count, metadata_json)
                  VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                 rusqlite::params![
                     entry.id,
                     entry.content,
                     embedding_json,
                     entry.timestamp_ns as i64,
                     entry.importance.0,
                     entry.access_count as i64,
                     entry.last_accessed_ns as i64,
                     entry.source,
                     entry.token_count as i64,
                     metadata_json,
                 ],
             )?;
             Ok(())
        }

       /// Search semantic memory using FTS5.
       #[cfg(feature = "sqlite")]
       pub fn search(&self, query: &str, limit: usize) -> MemoryResult<Vec<(MemoryEntry, f32)>> {
             let fts_query = build_fts_query(query);
             let mut stmt = self.conn.prepare(
                 "SELECT m.id, m.content, m.embedding_json, m.timestamp_ns, m.importance,
                         m.access_count, m.last_accessed_ns, m.source, m.token_count, m.metadata_json,
                         bm25(memories_fts) as rank
                  FROM memories_fts
                  JOIN memories m ON m.rowid = memories_fts.rowid
                  WHERE memories_fts MATCH ?1
                  ORDER BY rank ASC
                  LIMIT ?2",
             )?;

             let rows = stmt.query_map(rusqlite::params![fts_query, limit as i64], |row| {
                 let id: String = row.get(0)?;
                 let content: String = row.get(1)?;
                 let embedding_json: String = row.get(2)?;
                 let timestamp_ns: i64 = row.get(3)?;
                 let importance: f64 = row.get(4)?;
                 let access_count: i64 = row.get(5)?;
                 let last_accessed_ns: i64 = row.get(6)?;
                 let source: String = row.get(7)?;
                 let token_count: i64 = row.get(8)?;
                 let metadata_json: String = row.get(9)?;
                 let rank: f64 = row.get(10)?;

                 let embedding = if embedding_json.is_empty() {
                     None
                 } else {
                     serde_json::from_str(&embedding_json).ok()
                 };
                 let metadata = serde_json::from_str(&metadata_json).unwrap_or(serde_json::Value::Null);

                 let mut entry = MemoryEntry::new(&id, &content, MemoryLayer::Semantic);
                 entry.embedding = embedding;
                 entry.timestamp_ns = timestamp_ns as u64;
                 entry.importance = crate::entry::Importance::new(importance as f32);
                 entry.access_count = access_count as u64;
                 entry.last_accessed_ns = last_accessed_ns as u64;
                 entry.source = source;
                 entry.token_count = token_count as usize;
                 entry.metadata = metadata;

                 // BM25 scores are negative (lower is better); convert to positive relevance
                 let score = (1.0 / (1.0 + rank.abs())) as f32;
                 Ok((entry, score))
             })?;

             let mut results = Vec::new();
             for row in rows {
                 results.push(row?);
             }
             Ok(results)
        }

       /// Get total number of entries.
       #[cfg(feature = "sqlite")]
       pub fn count(&self) -> MemoryResult<usize> {
             let count: i64 = self.conn.query_row("SELECT COUNT(*) FROM memories", [], |row| row.get(0))?;
             Ok(count as usize)
        }
   }

   #[cfg(feature = "sqlite")]
   impl MemoryRetrieval for SemanticMemory {
       async fn retrieve(&self, query: &MemoryQuery) -> MemoryResult<Vec<RetrievedMemory>> {
             let results = self.search(&query.text, query.limit)?;
             let limit = query.max_tokens;
             let mut total_tokens = 0usize;
             let mut retrieved = Vec::new();

             for (entry, raw_score) in results {
                 let score = RelevanceScore::new(raw_score);
                 if score < query.min_score {
                     break;
                 }
                 if let Some(max) = limit {
                     if total_tokens + entry.token_count > max {
                         continue;
                     }
                     total_tokens += entry.token_count;
                 }
                 retrieved.push(RetrievedMemory {
                     entry,
                     score,
                     matched_layer: "semantic".to_string(),
                 });
             }
             Ok(retrieved)
        }
   }

   /// Convert a natural language query into an FTS5 MATCH expression.
   fn build_fts_query(text: &str) -> String {
       text.split_whitespace()
           .filter(|w| w.len() > 1)
           .map(|w| format!("\"{}\"", w.replace('"', "")))
           .collect::<Vec<_>>()
           .join(" ")
   }

   // 无sqlite feature时提供空实现
   #[cfg(not(feature = "sqlite"))]
   pub struct SemanticMemory { config: SemanticMemoryConfig }
   #[cfg(not(feature = "sqlite"))]
   impl std::fmt::Debug for SemanticMemory {
       fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.debug_struct("SemanticMemory").finish() }
   }

   #[cfg(test)]
   #[cfg(feature = "sqlite")]
   mod tests {
       use super::*;

       #[tokio::test]
       async fn test_store_and_search() {
             let mem = SemanticMemory::in_memory().unwrap();
             let mut e1 = MemoryEntry::new("s1", "Rust is a systems programming language", MemoryLayer::Semantic);
             e1.token_count = 5;
             mem.store(&e1).unwrap();
             let mut e2 = MemoryEntry::new("s2", "Python is a scripting language", MemoryLayer::Semantic);
             e2.token_count = 5;
             mem.store(&e2).unwrap();
             assert_eq!(mem.count().unwrap(), 2);

             let results = mem.search("Rust systems", 10).unwrap();
             assert!(!results.is_empty());
             assert_eq!(results[0].0.id, "s1");
        }

       #[tokio::test]
       async fn test_retrieval_trait() {
             let mem = SemanticMemory::in_memory().unwrap();
             let mut e = MemoryEntry::new("s1", "machine learning and artificial intelligence", MemoryLayer::Semantic);
             e.token_count = 5;
             mem.store(&e).unwrap();

             let query = MemoryQuery::new("machine learning").with_limit(5);
             let results = mem.retrieve(&query).await.unwrap();
             assert!(!results.is_empty());
             assert!(results[0].score.0 > 0.0);
        }
   }
   ```

**验收标准**：
- [ ] `cargo build -p axiom-memory` 编译通过（sqlite feature）
- [ ] `cargo test -p axiom-memory` 通过
- [ ] FTS5 schema创建正确（trigger自动同步FTS索引）
- [ ] BM25分数转换为正相关度
- [ ] MemoryRetrieval trait已实现
- [ ] token预算限制在retrieve中生效

**Commit message**：
```
feat(memory): implement SemanticMemory with SQLite FTS5 (T141)
```

---

### T142：实现MemorySummarizer（自动摘要，episodic→semantic）

**新建文件**：
- `crates/axiom-memory/src/summarizer.rs`

**具体操作**：

1. 创建`summarizer.rs`：
   ```rust
   //! Memory Summarizer - auto-summarizes episodic entries into semantic memory.
   //!
   //! When episodic memory compacts old entries, the summarizer produces a condensed
   //! summary that is moved to semantic memory for long-term retention.

   use crate::entry::{Importance, MemoryEntry, MemoryLayer};
   use crate::error::MemoryResult;
   use std::time::{SystemTime, UNIX_EPOCH};

   /// Trait for generating summaries from batches of memory entries.
   /// Implementations can use LLMs or extractive methods.
   pub trait SummaryGenerator: Send + Sync {
       /// Generate a summary string from a batch of entries.
       async fn summarize(&self, entries: &[MemoryEntry]) -> MemoryResult<String>;
   }

   /// A simple extractive summarizer that concatenates key points.
   /// This is the default (no LLM dependency); LLM-based summarization is a future enhancement.
   #[derive(Debug, Default)]
   pub struct ExtractiveSummarizer {
       /// Maximum length of the summary in characters.
       max_chars: usize,
   }

   impl ExtractiveSummarizer {
       /// Create a new extractive summarizer.
       pub fn new(max_chars: usize) -> Self {
             Self { max_chars }
        }
   }

   impl Default for SummaryGenerator for ExtractiveSummarizer {
       async fn summarize(&self, entries: &[MemoryEntry]) -> MemoryResult<String> {
             if entries.is_empty() {
                 return Ok(String::new());
             }
             let mut texts: Vec<&str> = entries.iter().map(|e| e.content.as_str()).collect();
             texts.sort_by_key(|t| std::cmp::Reverse(t.len()));

             let mut summary = String::from("[Summary of previous conversation]\n");
             for text in texts {
                 if summary.len() + text.len() + 2 > self.max_chars {
                     break;
                 }
                 summary.push_str("- ");
                 summary.push_str(text);
                 summary.push('\n');
             }
             Ok(summary)
        }
   }

   /// MemorySummarizer orchestrates summarization of compacted episodic entries
   /// and promotes them to semantic memory.
   #[derive(Debug)]
   pub struct MemorySummarizer<G: SummaryGenerator = ExtractiveSummarizer> {
       generator: G,
       summary_id_counter: u64,
   }

   impl MemorySummarizer<ExtractiveSummarizer> {
       /// Create a summarizer with the default extractive generator.
       pub fn new() -> Self {
             Self {
                 generator: ExtractiveSummarizer::new(2000),
                 summary_id_counter: 0,
             }
        }
   }

   impl Default for MemorySummarizer<ExtractiveSummarizer> {
       fn default() -> Self {
             Self::new()
        }
   }

   impl<G: SummaryGenerator> MemorySummarizer<G> {
       /// Create a summarizer with a custom summary generator.
       pub fn with_generator(generator: G) -> Self {
             Self {
                 generator,
                 summary_id_counter: 0,
             }
        }

       /// Summarize a batch of entries and return a new MemoryEntry for semantic memory.
       pub async fn summarize_batch(&mut self, entries: &[MemoryEntry]) -> MemoryResult<MemoryEntry> {
             let summary_text = self.generator.summarize(entries).await?;
             self.summary_id_counter += 1;
             let id = format!("summary-{}", self.summary_id_counter);
             let token_estimate = summary_text.len() / 4;
             let avg_importance = if entries.is_empty() {
                 Importance::MEDIUM
             } else {
                 let sum: f32 = entries.iter().map(|e| e.importance.0).sum();
                 Importance::new(sum / entries.len() as f32)
             };

             let mut entry = MemoryEntry::new(&id, &summary_text, MemoryLayer::Semantic)
                 .with_importance(avg_importance)
                 .with_source("summarizer")
                 .with_token_count(token_estimate);
             // 收集所有correlation id到metadata
             let cids: Vec<String> = entries.iter()
                 .filter_map(|e| e.correlation_id.as_ref().map(|c| c.as_str().to_string()))
                 .collect();
             if !cids.is_empty() {
                 entry.metadata = serde_json::json!({ "summarized_from": cids, "entry_count": entries.len() });
             }
             Ok(entry)
        }
   }

   fn now_ns() -> u64 {
       SystemTime::now()
           .duration_since(UNIX_EPOCH)
           .unwrap_or_default()
           .as_nanos() as u64
   }

   #[cfg(test)]
   mod tests {
       use super::*;

       #[tokio::test]
       async fn test_extractive_summarizer() {
             let summarizer = ExtractiveSummarizer::new(500);
             let entries = vec![
                 MemoryEntry::new("m1", "The user asked about Rust memory management", MemoryLayer::Episodic),
                 MemoryEntry::new("m2", "The assistant explained ownership and borrowing", MemoryLayer::Episodic),
                 MemoryEntry::new("m3", "The user asked about lifetimes", MemoryLayer::Episodic),
             ];
             let summary = summarizer.summarize(&entries).await.unwrap();
             assert!(summary.contains("[Summary of previous conversation]"));
             assert!(summary.contains("ownership"));
        }

       #[tokio::test]
       async fn test_memory_summarizer_batch() {
             let mut summarizer = MemorySummarizer::new();
             let entries = vec![
                 MemoryEntry::new("m1", "discussed topic A", MemoryLayer::Episodic).with_importance(Importance::HIGH),
                 MemoryEntry::new("m2", "discussed topic B", MemoryLayer::Episodic).with_importance(Importance::MEDIUM),
             ];
             let summary_entry = summarizer.summarize_batch(&entries).await.unwrap();
             assert_eq!(summary_entry.layer, MemoryLayer::Semantic);
             assert!(summary_entry.importance.0 < Importance::HIGH.0);
             assert!(summary_entry.importance.0 >= Importance::MEDIUM.0);
        }

       #[tokio::test]
       async fn test_empty_batch() {
             let summarizer = ExtractiveSummarizer::new(500);
             let summary = summarizer.summarize(&[]).await.unwrap();
             assert!(summary.is_empty());
        }
   }
   ```

   注意：文件顶部需要`use serde::{Serialize, Deserialize};`和正确的use导入。

**验收标准**：
- [ ] `cargo build -p axiom-memory` 编译通过
- [ ] `cargo test -p axiom-memory` 通过
- [ ] SummaryGenerator trait可扩展（可注入LLM实现）
- [ ] ExtractiveSummarizer默认工作，不依赖LLM
- [ ] 摘要条目metadata记录来源
- [ ] 平均importance计算正确

**Commit message**：
```
feat(memory): implement MemorySummarizer with extractive fallback (T142)
```

---

### T143：实现TokenBudget（跨层token预算管理）

**新建文件**：
- `crates/axiom-memory/src/budget.rs`

**具体操作**：

1. 创建`budget.rs`：
   ```rust
   //! Token Budget - enforces max tokens across all memory layers with priority-based allocation.

   use crate::entry::{Importance, MemoryEntry, MemoryLayer};
   use serde::{Deserialize, Serialize};

   /// Token count type.
   pub type TokenCount = usize;

   /// Budget allocation per layer.
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct BudgetAllocation {
       /// Total token budget.
       pub total: TokenCount,
       /// Working memory allocation (L1).
       pub working: TokenCount,
       /// Episodic memory allocation (L2).
       pub episodic: TokenCount,
       /// Semantic retrieval allocation (L3).
       pub semantic: TokenCount,
       /// Reserved for system prompt / overhead.
       pub reserved: TokenCount,
   }

   impl Default for BudgetAllocation {
       fn default() -> Self {
             Self {
                 total: 8192,
                 working: 2048,
                 episodic: 2048,
                 semantic: 2048,
                 reserved: 2048,
             }
        }
   }

   impl BudgetAllocation {
       /// Create a budget allocation with specified total, distributed proportionally.
       pub fn new(total: TokenCount) -> Self {
             let working = total / 4;
             let episodic = total / 4;
             let semantic = total / 4;
             let reserved = total - working - episodic - semantic;
             Self {
                 total,
                 working,
                 episodic,
                 semantic,
                 reserved,
             }
        }

       /// Tokens available for memory content (total - reserved).
       pub fn available(&self) -> TokenCount {
             self.total.saturating_sub(self.reserved)
        }
   }

   /// TokenBudget enforces token limits and selects entries for context injection.
   #[derive(Debug)]
   pub struct TokenBudget {
       allocation: BudgetAllocation,
   }

   impl TokenBudget {
       /// Create a new token budget with the given allocation.
       pub fn new(allocation: BudgetAllocation) -> Self {
             Self { allocation }
        }

       /// Create a token budget from a total token count.
       pub fn from_total(total: TokenCount) -> Self {
             Self::new(BudgetAllocation::new(total))
        }

       /// Get the current allocation.
       pub fn allocation(&self) -> &BudgetAllocation {
             &self.allocation
        }

       /// Select entries from working memory within the working budget, prioritized by importance/recency.
       pub fn select_working<'a>(&self, entries: &'a [MemoryEntry]) -> Vec<&'a MemoryEntry> {
             self.select_within_budget(entries, self.allocation.working)
        }

       /// Select entries from episodic memory within the episodic budget.
       pub fn select_episodic<'a>(&self, entries: &'a [MemoryEntry]) -> Vec<&'a MemoryEntry> {
             self.select_within_budget(entries, self.allocation.episodic)
        }

       /// Select entries from semantic retrieval results within the semantic budget.
       pub fn select_semantic<'a>(&self, entries: &'a [crate::retrieval::RetrievedMemory]) -> Vec<&'a MemoryEntry> {
             // 按score排序，token budget内选入
             let mut scored: Vec<&MemoryEntry> = entries.iter().map(|r| &r.entry).collect();
             let now = crate::entry::now_ns();
             scored.sort_by(|a, b| {
                 b.retrieval_score(now).partial_cmp(&a.retrieval_score(now)).unwrap_or(std::cmp::Ordering::Equal)
             });
             self.pick_within_token_budget(&scored, self.allocation.semantic)
        }

       fn select_within_budget<'a>(&self, entries: &'a [MemoryEntry], budget: TokenCount) -> Vec<&'a MemoryEntry> {
             let now = crate::entry::now_ns();
             let mut sorted: Vec<&MemoryEntry> = entries.iter().collect();
             sorted.sort_by(|a, b| {
                 b.retrieval_score(now).partial_cmp(&a.retrieval_score(now)).unwrap_or(std::cmp::Ordering::Equal)
             });
             self.pick_within_token_budget(&sorted, budget)
        }

       fn pick_within_token_budget<'a>(&self, sorted_entries: &[&'a MemoryEntry], budget: TokenCount) -> Vec<&'a MemoryEntry> {
             let mut selected = Vec::new();
             let mut used = 0usize;
             for entry in sorted_entries {
                 let tokens = entry.token_count.max(1);
                 if used + tokens > budget {
                     continue;
                 }
                 used += tokens;
                 selected.push(*entry);
             }
             selected
        }
   }

   #[cfg(test)]
   mod tests {
       use super::*;

       fn make_entry(id: &str, tokens: usize, importance: f32) -> MemoryEntry {
             MemoryEntry::new(id, format!("content-{}", id), MemoryLayer::Working)
                 .with_token_count(tokens)
                 .with_importance(Importance::new(importance))
        }

       #[test]
       fn test_default_allocation() {
             let alloc = BudgetAllocation::default();
             assert_eq!(alloc.working + alloc.episodic + alloc.semantic + alloc.reserved, alloc.total);
        }

       #[test]
       fn test_select_within_budget() {
             let budget = TokenBudget::from_total(100);
             let entries = vec![
                 make_entry("high", 10, 1.0),
                 make_entry("low", 10, 0.1),
                 make_entry("mid", 10, 0.5),
             ];
             let selected = budget.select_working(&entries);
             assert_eq!(selected.len(), 3);
        }

       #[test]
       fn test_budget_enforced() {
             let budget = TokenBudget::new(BudgetAllocation {
                 total: 100,
                 working: 15,
                 episodic: 0,
                 semantic: 0,
                 reserved: 85,
             });
             let entries = vec![
                 make_entry("a", 10, 1.0),
                 make_entry("b", 10, 0.8),
                 make_entry("c", 10, 0.5),
             ];
             let selected = budget.select_working(&entries);
             let total_tokens: usize = selected.iter().map(|e| e.token_count).sum();
             assert!(total_tokens <= 15);
        }
   }
   ```

**验收标准**：
- [ ] `cargo build -p axiom-memory` 编译通过
- [ ] `cargo test -p axiom-memory` 通过
- [ ] 默认分配比例合理（total = working + episodic + semantic + reserved）
- [ ] 条目按retrieval_score排序选入
- [ ] Token预算强制执行不超支
- [ ] BudgetAllocation可序列化

**Commit message**：
```
feat(memory): implement TokenBudget with priority-based allocation (T143)
```

---

### T144：实现MemoryLens（impl Lens trait）

**新建文件**：
- `crates/axiom-memory/src/lens.rs`
- `crates/axiom-memory/src/procedural.rs`（空骨架）

**具体操作**：

1. 先创建空`procedural.rs`：
   ```rust
   //! Procedural Memory (L4) - learned skills and rules from the evolution engine.
   //! Placeholder for future integration with axiom-evolution.

   use crate::entry::MemoryEntry;
   use crate::error::MemoryResult;

   /// Procedural memory stores learned skills and rules.
   /// This is a minimal placeholder; full implementation integrates with axiom-evolution.
   #[derive(Debug, Default)]
   pub struct ProceduralMemory {
       skills: Vec<MemoryEntry>,
   }

   impl ProceduralMemory {
       pub fn new() -> Self { Self::default() }
       pub fn add_skill(&mut self, entry: MemoryEntry) { self.skills.push(entry); }
       pub fn skills(&self) -> &[MemoryEntry] { &self.skills }
       pub fn is_empty(&self) -> bool { self.skills.is_empty() }
   }
   ```

2. 创建`lens.rs`：
   ```rust
   //! MemoryLens - implements the Lens trait to project relevant memories into context.

   use crate::budget::TokenBudget;
   use crate::entry::{MemoryEntry, MemoryLayer};
   use crate::episodic::EpisodicMemory;
   use crate::error::MemoryResult;
   use crate::retrieval::{MemoryQuery, MemoryRetrieval};
   use crate::semantic::SemanticMemory;
   use crate::working::WorkingMemory;
   use axiom_core::id::LensId;
   use axiom_core::lens::Lens;
   use axiom_core::signal::VectorClock;

   /// View projected by MemoryLens: formatted context from all memory layers.
   #[derive(Debug, Clone)]
   pub struct MemoryView {
       /// Formatted working memory content (most recent context).
       pub working_context: String,
       /// Formatted episodic memory (recent turns summary).
       pub episodic_context: String,
       /// Formatted semantic memory (retrieved knowledge).
       pub semantic_context: String,
       /// Total tokens used.
       pub total_tokens: usize,
   }

   impl MemoryView {
       /// Compose all contexts into a single string for LLM injection.
       pub fn to_prompt_string(&self) -> String {
             let mut parts = Vec::new();
             if !self.working_context.is_empty() {
                 parts.push(format!("## Recent Context\n{}", self.working_context));
             }
             if !self.episodic_context.is_empty() {
                 parts.push(format!("## Recent History\n{}", self.episodic_context));
             }
             if !self.semantic_context.is_empty() {
                 parts.push(format!("## Relevant Knowledge\n{}", self.semantic_context));
             }
             parts.join("\n\n")
        }
   }

   /// MemoryLens combines all memory layers and projects relevant context
   /// within token budgets, implementing the Lens trait.
   pub struct MemoryLens {
       lens_id: LensId,
       working: WorkingMemory,
       episodic: EpisodicMemory,
       semantic: SemanticMemory,
       budget: TokenBudget,
   }

   impl std::fmt::Debug for MemoryLens {
       fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
             f.debug_struct("MemoryLens")
                 .field("lens_id", &self.lens_id)
                 .field("budget", &self.budget.allocation())
                 .finish()
        }
   }

   impl MemoryLens {
       /// Create a new MemoryLens with the given components.
       #[cfg(feature = "sqlite")]
       pub fn new(
             working: WorkingMemory,
             episodic: EpisodicMemory,
             semantic: SemanticMemory,
             budget: TokenBudget,
       ) -> Self {
             Self {
                 lens_id: LensId::new("MemoryLens"),
                 working,
                 episodic,
                 semantic,
                 budget,
             }
        }

       /// Get a mutable reference to working memory.
       pub fn working_mut(&mut self) -> &mut WorkingMemory {
             &mut self.working
        }

       /// Get a mutable reference to episodic memory.
       pub fn episodic_mut(&mut self) -> &mut EpisodicMemory {
             &mut self.episodic
        }

       /// Get a reference to semantic memory.
       #[cfg(feature = "sqlite")]
       pub fn semantic(&self) -> &SemanticMemory {
             &self.semantic
        }

       /// Add a new entry to working memory, handling eviction to episodic.
       pub fn add_to_working(&mut self, entry: MemoryEntry) -> MemoryResult<()> {
             let evicted = self.working.insert(entry)?;
             for evicted_entry in evicted {
                 self.episodic.push(evicted_entry)?;
             }
             Ok(())
        }

       /// Format entries into a context string.
       fn format_entries(entries: &[&MemoryEntry]) -> String {
             entries.iter()
                 .map(|e| format!("[{}] {}", e.source, e.content))
                 .collect::<Vec<_>>()
                 .join("\n")
        }
   }

   #[cfg(feature = "sqlite")]
   impl Lens for MemoryLens {
       type View = MemoryView;

       fn lens_id(&self) -> LensId {
             self.lens_id.clone()
        }

       async fn project(&self) -> axiom_core::Result<Self::View> {
             let working_entries: Vec<&MemoryEntry> = self.working.entries_recent();
             let working_selected = self.budget.select_working(
                 &working_entries.into_iter().cloned().collect::<Vec<_>>()
             );
             let working_text = Self::format_entries(&working_selected);
             let working_tokens: usize = working_selected.iter().map(|e| e.token_count).sum();

             let episodic_entries = self.episodic.recent(10);
             let episodic_selected = self.budget.select_episodic(
                 &episodic_entries.into_iter().cloned().collect::<Vec<_>>()
             );
             let episodic_text = Self::format_entries(&episodic_selected);
             let episodic_tokens: usize = episodic_selected.iter().map(|e| e.token_count).sum();

             // 默认query为空，仅用最近上下文；有query时检索semantic
             let query = MemoryQuery::new("").with_limit(5);
             let semantic_results = self.semantic.retrieve(&query).await
                 .map_err(|e| axiom_core::AxiomError::Internal(format!("memory retrieve: {}", e)))?;
             let semantic_selected = self.budget.select_semantic(&semantic_results);
             let semantic_text = Self::format_entries(&semantic_selected);
             let semantic_tokens: usize = semantic_selected.iter().map(|e| e.token_count).sum();

             Ok(MemoryView {
                 working_context: working_text,
                 episodic_context: episodic_text,
                 semantic_context: semantic_text,
                 total_tokens: working_tokens + episodic_tokens + semantic_tokens,
             })
        }

       async fn project_at(&self, _clock: &VectorClock) -> axiom_core::Result<Self::View> {
             // MemoryLens不支持时间点投影（简化实现）
             self.project().await
        }

       fn token_estimate(&self) -> usize {
             self.budget.allocation().available()
        }
   }

   #[cfg(test)]
   #[cfg(feature = "sqlite")]
   mod tests {
       use super::*;

       #[tokio::test]
       async fn test_memory_lens_project() {
             let working = WorkingMemory::new(1000);
             let episodic = EpisodicMemory::new(20);
             let semantic = SemanticMemory::in_memory().unwrap();
             let budget = TokenBudget::from_total(4096);
             let lens = MemoryLens::new(working, episodic, semantic, budget);
             let view = lens.project().await.unwrap();
             assert!(view.total_tokens >= 0);
        }
   }
   ```

**验收标准**：
- [ ] `cargo build -p axiom-memory` 编译通过
- [ ] `cargo test -p axiom-memory` 通过
- [ ] MemoryLens正确impl Lens trait（lens_id/project/project_at/token_estimate）
- [ ] Working内存驱逐自动转入Episodic
- [ ] MemoryView.to_prompt_string()格式化正确
- [ ] token budget在所有层生效

**Commit message**：
```
feat(memory): implement MemoryLens as Lens trait impl (T144)
```

---

### T145：Memory系统测试

**新建/修改文件**：
- `crates/axiom-memory/tests/memory_integration.rs`

**具体操作**：

1. 创建集成测试文件：
   ```rust
   //! Integration tests for axiom-memory.

   use axiom_memory::*;

   #[tokio::test]
   async fn test_working_memory_lru_eviction() {
       let mut wm = WorkingMemory::new(30);
       wm.insert(MemoryEntry::new("m1", "a", MemoryLayer::Working).with_token_count(10)).unwrap();
       wm.insert(MemoryEntry::new("m2", "b", MemoryLayer::Working).with_token_count(10)).unwrap();
       wm.insert(MemoryEntry::new("m3", "c", MemoryLayer::Working).with_token_count(10)).unwrap();
       assert_eq!(wm.len(), 3);
       wm.get("m1");
       wm.insert(MemoryEntry::new("m4", "d", MemoryLayer::Working).with_token_count(10)).unwrap();
       assert!(wm.entries.contains_key("m1"));
       assert!(!wm.entries.contains_key("m2"));
   }

   #[tokio::test]
   async fn test_episodic_compaction() {
       let mut em = EpisodicMemory::new(10);
       for i in 0..12 {
           let entry = MemoryEntry::new(
                 &format!("m{}", i),
                 &format!("turn {}", i),
                 MemoryLayer::Episodic,
           ).with_token_count(5);
           em.push(entry).unwrap();
       }
       assert_eq!(em.len(), 10);
       let summary = em.compact_oldest(5);
       assert_eq!(summary.compacted_entries.len(), 5);
       assert_eq!(em.len(), 5);
   }

   #[cfg(feature = "sqlite")]
   #[tokio::test]
   async fn test_token_budget_enforcement() {
       let budget = TokenBudget::from_total(100);
       let entries: Vec<MemoryEntry> = (0..20)
           .map(|i| {
                 MemoryEntry::new(
                     &format!("e{}", i),
                     &format!("content {}", i),
                     MemoryLayer::Working,
                 )
                 .with_token_count(10)
                 .with_importance(Importance::new(0.5 + (i as f32) * 0.02))
           })
           .collect();
       let selected = budget.select_working(&entries);
       let total: usize = selected.iter().map(|e| e.token_count).sum();
       assert!(total <= budget.allocation().working);
       assert!(total > 0);
   }

   #[cfg(feature = "sqlite")]
   #[tokio::test]
   async fn test_memory_lens_end_to_end() {
       let working = WorkingMemory::new(500);
       let episodic = EpisodicMemory::new(20);
       let semantic = SemanticMemory::in_memory().unwrap();
       let budget = TokenBudget::from_total(2048);
       let mut lens = MemoryLens::new(working, episodic, semantic, budget);

       lens.add_to_working(
           MemoryEntry::new("w1", "User: Hello, what is Rust?", MemoryLayer::Working)
               .with_source("user")
               .with_token_count(10),
       ).unwrap();
       lens.add_to_working(
           MemoryEntry::new("w2", "Assistant: Rust is a systems programming language.", MemoryLayer::Working)
               .with_source("assistant")
               .with_token_count(12),
       ).unwrap();

       let view = lens.project().await.unwrap();
       assert!(view.to_prompt_string().contains("Rust"));
   }
   ```

2. 运行完整验证：
   ```
   cargo build -p axiom-memory
   cargo clippy -p axiom-memory -- -D warnings
   cargo test -p axiom-memory
   ```

**验收标准**：
- [ ] `cargo test -p axiom-memory` 全部通过
- [ ] `cargo clippy -p axiom-memory -- -D warnings` 零警告
- [ ] 集成测试覆盖：working LRU驱逐、episodic压缩、token预算、MemoryLens端到端
- [ ] `cargo doc -p axiom-memory --no-deps` 无警告

**Commit message**：
```
feat(memory): add integration tests for memory system (T145)
```

---

## P13阶段：规划器 (axiom-planner)

### T146：创建axiom-planner crate骨架

**文件修改**：
- `Cargo.toml`（workspace根）
- 新建：`crates/axiom-planner/Cargo.toml`
- 新建：`crates/axiom-planner/src/lib.rs`

**具体操作**：

1. 在根`Cargo.toml`的`[workspace.members]`中添加：
   ```toml
   "crates/axiom-planner",
   ```

2. 创建`crates/axiom-planner/Cargo.toml`：
   ```toml
   [package]
   name = "axiom-planner"
   version.workspace = true
   edition.workspace = true
   license.workspace = true
   rust-version.workspace = true
   description = "Planning patterns: ReAct and Plan-Execute for multi-step agent tasks"

   [dependencies]
   axiom-core = { workspace = true }
   axiom-memory = { workspace = true }
   serde = { workspace = true }
   serde_json = { workspace = true }
   thiserror = { workspace = true }
   tracing = { workspace = true }
   tokio = { workspace = true }
   futures = { workspace = true }

   [dev-dependencies]
   tokio = { workspace = true, features = ["rt-multi-thread", "macros", "time"] }
   tracing-subscriber = { workspace = true }
   ```

   注意：axiom-llm和axiom-tool的依赖在P9完成后添加；P13骨架阶段可先用trait抽象或用`#[cfg]`预留。本计划假设P9已完成并在workspace.dependencies中，若尚未完成则暂时使用泛型参数。

   **补充操作**：在workspace根的`[workspace.dependencies]`中添加（如尚未存在）：
   ```toml
   axiom-memory = { path = "crates/axiom-memory" }
   axiom-planner = { path = "crates/axiom-planner" }
   ```

3. 创建`crates/axiom-planner/src/lib.rs`：
   ```rust
   //! Axiom Planner - ReAct and Plan-Execute patterns for agent reasoning.
   //!
   //! This crate provides:
   //! - [`ReActPlanner`]: Think-Act-Observe loop for dynamic planning
   //! - [`PlanExecutePlanner`]: Create full plan first, then execute step by step
   //! - [`PlanExecutor`]: Executes plan steps using tools and LLM
   //! - Replanning triggers for failure recovery
   //! - Plan visualization via Timeline export

   #![allow(async_fn_in_trait)]

   pub mod error;
   pub mod plan;
   pub mod react;
   pub mod plan_execute;
   pub mod executor;
   pub mod replan;
   pub mod viz;

   pub use error::{PlannerError, PlannerResult};
   pub use plan::{Plan, PlanStep, PlanStatus, StepStatus, ToolCall, Observation};
   pub use react::ReActPlanner;
   pub use plan_execute::PlanExecutePlanner;
   pub use executor::PlanExecutor;
   pub use replan::{Replanner, ReplanTrigger};
   pub use viz::PlanTimeline;
   ```

4. 为各模块创建空骨架文件。

**验收标准**：
- [ ] `cargo build -p axiom-planner` 编译通过
- [ ] `cargo clippy -p axiom-planner -- -D warnings` 零警告
- [ ] 模块声明完整

**Commit message**：
```
feat(planner): create axiom-planner crate skeleton (T146)
```

---

### T147-T148：定义Plan/PlanStep/PlanStatus/ToolCall/Observation类型

**新建文件**：
- `crates/axiom-planner/src/error.rs`
- `crates/axiom-planner/src/plan.rs`

**具体操作**：

1. 创建`error.rs`：
   ```rust
   //! Planner error types.

   use axiom_core::AxiomError;
   use thiserror::Error;

   /// Errors from planning operations.
   #[derive(Error, Debug)]
   pub enum PlannerError {
       /// Maximum planning iterations exceeded.
       #[error("Max iterations exceeded: {max_iters}")]
       MaxIterationsExceeded { max_iters: usize },

       /// Plan execution failed.
       #[error("Plan execution failed at step {step_id}: {reason}")]
       ExecutionFailed { step_id: String, reason: String },

       /// Replanning threshold exceeded.
       #[error("Replanning threshold exceeded: {attempts} attempts")]
       ReplanningExceeded { attempts: usize },

       /// Tool call failed.
       #[error("Tool call failed: {tool} - {message}")]
       ToolCallFailed { tool: String, message: String },

       /// LLM error.
       #[error("LLM error: {0}")]
       Llm(String),

       /// Internal error.
       #[error("Internal error: {0}")]
       Internal(String),

       /// Serialization error.
       #[error("Serialization error: {0}")]
       Serde(#[from] serde_json::Error),
   }

   impl From<PlannerError> for AxiomError {
       fn from(e: PlannerError) -> Self {
             AxiomError::Internal(format!("planner: {}", e))
        }
   }

   pub type PlannerResult<T> = std::result::Result<T, PlannerError>;
   ```

2. 创建`plan.rs`：
   ```rust
   //! Plan data structures: goal, steps, status, observations.

   use axiom_core::id::CorrelationId;
   use serde::{Deserialize, Serialize};
   use std::time::{SystemTime, UNIX_EPOCH};

   /// Unique plan identifier.
   #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
   pub struct PlanId(pub String);

   impl PlanId {
       pub fn new(id: impl Into<String>) -> Self { Self(id.into()) }
       pub fn generate() -> Self {
             use std::sync::atomic::{AtomicU64, Ordering};
             static COUNTER: AtomicU64 = AtomicU64::new(0);
             let n = COUNTER.fetch_add(1, Ordering::SeqCst);
             Self(format!("plan-{}", n))
        }
   }

   impl std::fmt::Display for PlanId {
       fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
             write!(f, "{}", self.0)
        }
   }

   /// Status of the entire plan.
   #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
   pub enum PlanStatus {
       /// Plan is being created.
       Drafting,
       /// Plan is ready for execution.
       Ready,
       /// Plan is currently executing.
       Executing,
       /// Plan completed successfully.
       Completed,
       /// Plan failed.
       Failed,
       /// Plan was replanned (superseded by a new plan).
       Replanned,
       /// Plan was cancelled.
       Cancelled,
   }

   /// Status of an individual plan step.
   #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
   pub enum StepStatus {
       /// Step is waiting to be executed.
       Pending,
       /// Step is currently executing.
       InProgress,
       /// Step completed successfully.
       Completed,
       /// Step failed.
       Failed,
       /// Step was skipped.
       Skipped,
   }

   /// A tool call to be executed as part of a plan step.
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct ToolCall {
       /// Unique call ID.
       pub call_id: String,
       /// Name of the tool to call.
       pub tool_name: String,
       /// Arguments as JSON.
       pub arguments: serde_json::Value,
   }

   /// Observation from a tool call or LLM response.
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct Observation {
       /// Which tool call produced this observation.
       pub call_id: Option<String>,
       /// The observation content.
       pub content: String,
       /// Whether the observation indicates an error.
       pub is_error: bool,
       /// Timestamp in nanoseconds.
       pub timestamp_ns: u64,
   }

   impl Observation {
       /// Create a successful observation.
       pub fn success(call_id: Option<&str>, content: impl Into<String>) -> Self {
             Self {
                 call_id: call_id.map(|s| s.to_string()),
                 content: content.into(),
                 is_error: false,
                 timestamp_ns: now_ns(),
             }
        }

       /// Create an error observation.
       pub fn error(call_id: Option<&str>, content: impl Into<String>) -> Self {
             Self {
                 call_id: call_id.map(|s| s.to_string()),
                 content: content.into(),
                 is_error: true,
                 timestamp_ns: now_ns(),
             }
        }
   }

   /// A single step in a plan.
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct PlanStep {
       /// Unique step identifier.
       pub id: String,
       /// Human-readable description of what this step does.
       pub description: String,
       /// Current status of this step.
       pub status: StepStatus,
       /// Tool calls to execute for this step.
       pub tool_calls: Vec<ToolCall>,
       /// Expected output description (for validation).
       pub expected_output: Option<String>,
       /// Observations collected during execution.
       pub observations: Vec<Observation>,
       /// The reasoning/thought that produced this step.
       pub reasoning: Option<String>,
       /// Step number (1-based).
       pub step_number: u32,
   }

   impl PlanStep {
       /// Create a new pending step.
       pub fn new(id: impl Into<String>, description: impl Into<String>, step_number: u32) -> Self {
             Self {
                 id: id.into(),
                 description: description.into(),
                 status: StepStatus::Pending,
                 tool_calls: Vec::new(),
                 expected_output: None,
                 observations: Vec::new(),
                 reasoning: None,
                 step_number,
             }
        }

       /// Add a tool call to this step.
       pub fn with_tool_call(mut self, call: ToolCall) -> Self {
             self.tool_calls.push(call);
             self
        }

       /// Set expected output.
       pub fn with_expected_output(mut self, output: impl Into<String>) -> Self {
             self.expected_output = Some(output.into());
             self
        }

       /// Add an observation.
       pub fn add_observation(&mut self, obs: Observation) {
             self.observations.push(obs);
        }

       /// Mark step as in progress.
       pub fn mark_in_progress(&mut self) {
             self.status = StepStatus::InProgress;
        }

       /// Mark step as completed.
       pub fn mark_completed(&mut self) {
             self.status = StepStatus::Completed;
        }

       /// Mark step as failed.
       pub fn mark_failed(&mut self, reason: impl Into<String>) {
             self.status = StepStatus::Failed;
             self.add_observation(Observation::error(None, reason));
        }
   }

   /// A complete plan with goal, steps, and execution state.
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct Plan {
       /// Unique plan identifier.
       pub id: PlanId,
       /// The goal this plan aims to achieve.
       pub goal: String,
       /// Ordered list of steps.
       pub steps: Vec<PlanStep>,
       /// Current plan status.
       pub status: PlanStatus,
       /// Correlation ID for tracing.
       pub correlation_id: Option<CorrelationId>,
       /// Timestamp when plan was created.
       pub created_at_ns: u64,
       /// Timestamp when plan was last updated.
       pub updated_at_ns: u64,
       /// Number of times this plan was replanned.
       pub replan_count: u32,
       /// Final result/answer when plan completes.
       pub final_answer: Option<String>,
   }

   impl Plan {
       /// Create a new draft plan.
       pub fn new(goal: impl Into<String>) -> Self {
             let now = now_ns();
             Self {
                 id: PlanId::generate(),
                 goal: goal.into(),
                 steps: Vec::new(),
                 status: PlanStatus::Drafting,
                 correlation_id: None,
                 created_at_ns: now,
                 updated_at_ns: now,
                 replan_count: 0,
                 final_answer: None,
             }
        }

       /// Add a step to the plan.
       pub fn add_step(&mut self, step: PlanStep) {
             self.steps.push(step);
             self.updated_at_ns = now_ns();
        }

       /// Mark plan as ready for execution.
       pub fn mark_ready(&mut self) {
             self.status = PlanStatus::Ready;
             self.updated_at_ns = now_ns();
        }

       /// Mark plan as executing.
       pub fn mark_executing(&mut self) {
             self.status = PlanStatus::Executing;
             self.updated_at_ns = now_ns();
        }

       /// Mark plan as completed with a final answer.
       pub fn mark_completed(&mut self, answer: impl Into<String>) {
             self.status = PlanStatus::Completed;
             self.final_answer = Some(answer.into());
             self.updated_at_ns = now_ns();
        }

       /// Mark plan as failed.
       pub fn mark_failed(&mut self, reason: impl Into<String>) {
             self.status = PlanStatus::Failed;
             self.final_answer = Some(reason.into());
             self.updated_at_ns = now_ns();
        }

       /// Get the next pending step.
       pub fn next_pending_step(&self) -> Option<&PlanStep> {
             self.steps.iter().find(|s| s.status == StepStatus::Pending)
        }

       /// Get a mutable reference to the next pending step.
       pub fn next_pending_step_mut(&mut self) -> Option<&mut PlanStep> {
             self.steps.iter_mut().find(|s| s.status == StepStatus::Pending)
        }

       /// Check if all steps are completed.
       pub fn all_steps_completed(&self) -> bool {
             self.steps.iter().all(|s| s.status == StepStatus::Completed || s.status == StepStatus::Skipped)
        }

       /// Check if any step has failed.
       pub fn has_failed_steps(&self) -> bool {
             self.steps.iter().any(|s| s.status == StepStatus::Failed)
        }
   }

   fn now_ns() -> u64 {
       SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos() as u64
   }

   #[cfg(test)]
   mod tests {
       use super::*;

       #[test]
       fn test_plan_lifecycle() {
             let mut plan = Plan::new("book a flight");
             assert_eq!(plan.status, PlanStatus::Drafting);
             plan.add_step(PlanStep::new("s1", "search flights", 1));
             plan.add_step(PlanStep::new("s2", "select flight", 2));
             plan.mark_ready();
             assert_eq!(plan.status, PlanStatus::Ready);
             assert_eq!(plan.steps.len(), 2);
        }

       #[test]
       fn test_step_status() {
             let mut step = PlanStep::new("s1", "do something", 1);
             assert_eq!(step.status, StepStatus::Pending);
             step.mark_in_progress();
             assert_eq!(step.status, StepStatus::InProgress);
             step.mark_completed();
             assert_eq!(step.status, StepStatus::Completed);
        }

       #[test]
       fn test_all_steps_completed() {
             let mut plan = Plan::new("test");
             let mut s1 = PlanStep::new("s1", "a", 1);
             s1.mark_completed();
             plan.add_step(s1);
             let mut s2 = PlanStep::new("s2", "b", 2);
             s2.mark_completed();
             plan.add_step(s2);
             assert!(plan.all_steps_completed());
        }
   }
   ```

**验收标准**：
- [ ] `cargo build -p axiom-planner` 编译通过
- [ ] `cargo test -p axiom-planner` 通过
- [ ] Plan/PlanStep状态机正确
- [ ] ToolCall和Observation可序列化
- [ ] PlanId生成唯一ID
- [ ] 所有public类型有#[derive(Debug)]和rustdoc

**Commit message**：
```
feat(planner): define Plan, PlanStep, and plan status types (T147-T148)
```

---

### T149：实现ReAct planner (Think-Act-Observe loop)

**新建文件**：
- `crates/axiom-planner/src/react.rs`

**具体操作**：

1. 创建`react.rs`：
   ```rust
   //! ReAct Planner - Think-Act-Observe loop for dynamic reasoning.
   //!
   //! ReAct interleaves reasoning traces (Thought) with actions (Tool calls)
   //! and observations, allowing the agent to dynamically adjust its approach.

   use crate::error::PlannerResult;
   use crate::plan::{Observation, Plan, PlanStatus, StepStatus, ToolCall};

   /// Trait for LLM reasoning in the ReAct loop.
   pub trait ReasoningEngine: Send + Sync {
       /// Given the current context and goal, produce the next thought and optional tool calls.
       async fn think(
             &self,
             goal: &str,
             context: &str,
             previous_observations: &[Observation],
       ) -> PlannerResult<ReActThought>;
   }

   /// The result of a thinking step: a thought and optional tool calls.
   #[derive(Debug, Clone)]
   pub struct ReActThought {
       /// The reasoning thought text.
       pub thought: String,
       /// Tool calls to execute (empty = task is complete).
       pub tool_calls: Vec<ToolCall>,
       /// Final answer if the task is done.
       pub final_answer: Option<String>,
   }

   /// Trait for executing tool calls.
   pub trait ToolExecutor: Send + Sync {
       /// Execute a tool call and return an observation.
       async fn execute(&self, call: &ToolCall) -> PlannerResult<Observation>;
   }

   /// ReAct planner implementing the Think-Act-Observe loop.
   #[derive(Debug)]
   pub struct ReActPlanner<R: ReasoningEngine, T: ToolExecutor> {
       reasoner: R,
       executor: T,
       max_iterations: usize,
   }

   impl<R: ReasoningEngine, T: ToolExecutor> ReActPlanner<R, T> {
       /// Create a new ReAct planner.
       pub fn new(reasoner: R, executor: T, max_iterations: usize) -> Self {
             Self { reasoner, executor, max_iterations }
        }

       /// Run the ReAct loop to achieve a goal.
       pub async fn run(&self, goal: &str, initial_context: &str) -> PlannerResult<Plan> {
             let mut plan = Plan::new(goal);
             plan.mark_executing();
             let mut observations: Vec<Observation> = Vec::new();
             let mut context = initial_context.to_string();

             for iteration in 0..self.max_iterations {
                 // Think
                 let thought = self.reasoner.think(goal, &context, &observations).await?;
                 tracing::debug!(iteration, thought = %thought.thought, "ReAct think");

                 if let Some(answer) = thought.final_answer {
                     plan.mark_completed(answer);
                     return Ok(plan);
                 }

                 if thought.tool_calls.is_empty() {
                     // 无tool call也无final answer，视为完成
                     plan.mark_completed(thought.thought);
                     return Ok(plan);
                 }

                 // Act - execute each tool call
                 for call in &thought.tool_calls {
                     let step_id = format!("react-step-{}", iteration);
                     let mut step = crate::plan::PlanStep::new(&step_id, &thought.thought, (iteration + 1) as u32);
                     step.tool_calls = vec![call.clone()];
                     step.reasoning = Some(thought.thought.clone());
                     step.mark_in_progress();

                     // Observe
                     let observation = match self.executor.execute(call).await {
                         Ok(obs) => obs,
                         Err(e) => Observation::error(Some(&call.call_id), e.to_string()),
                     };

                     if observation.is_error {
                         step.add_observation(observation.clone());
                         step.mark_failed("tool call returned error");
                         plan.add_step(step);
                         plan.mark_failed(format!("Tool call failed at iteration {}: {}", iteration, observation.content));
                         return Ok(plan);
                     }

                     step.add_observation(observation.clone());
                     step.mark_completed();
                     plan.add_step(step);
                     observations.push(observation);
                 }

                 // Update context with new observations
                 context = format!(
                     "{}\nThought: {}\nObservation: {}",
                     context,
                     thought.thought,
                     observations.last().map(|o| o.content.as_str()).unwrap_or("")
                 );
             }

             // 超出迭代次数
             Err(crate::error::PlannerError::MaxIterationsExceeded {
                 max_iters: self.max_iterations,
             })
        }
   }

   #[cfg(test)]
   mod tests {
       use super::*;

       struct MockReasoner {
             responses: std::sync::Mutex<Vec<ReActThought>>,
       }

       impl MockReasoner {
             fn new(responses: Vec<ReActThought>) -> Self {
                 Self { responses: std::sync::Mutex::new(responses) }
             }
       }

       impl ReasoningEngine for MockReasoner {
             async fn think(&self, _goal: &str, _context: &str, _obs: &[Observation]) -> PlannerResult<ReActThought> {
                 let mut responses = self.responses.lock().unwrap();
                 if responses.is_empty() {
                     Ok(ReActThought {
                         thought: "I'm done".to_string(),
                         tool_calls: vec![],
                         final_answer: Some("done".to_string()),
                     })
                 } else {
                     Ok(responses.remove(0))
                 }
             }
        }

       struct MockExecutor;
       impl ToolExecutor for MockExecutor {
             async fn execute(&self, call: &ToolCall) -> PlannerResult<Observation> {
                 Ok(Observation::success(Some(&call.call_id), format!("result of {}", call.tool_name)))
             }
        }

       #[tokio::test]
       async fn test_react_single_step() {
             let thought = ReActThought {
                 thought: "I need to search".to_string(),
                 tool_calls: vec![ToolCall { call_id: "c1".into(), tool_name: "search".into(), arguments: serde_json::json!({"q": "test"}) }],
                 final_answer: None,
             };
             let final_thought = ReActThought {
                 thought: "Found the answer".to_string(),
                 tool_calls: vec![],
                 final_answer: Some("42".to_string()),
             };
             let reasoner = MockReasoner::new(vec![thought, final_thought]);
             let planner = ReActPlanner::new(reasoner, MockExecutor, 10);
             let plan = planner.run("what is the answer?", "").await.unwrap();
             assert_eq!(plan.status, PlanStatus::Completed);
             assert_eq!(plan.final_answer, Some("42".to_string()));
        }

       #[tokio::test]
       async fn test_react_max_iterations() {
             let thought = ReActThought {
                 thought: "looping".to_string(),
                 tool_calls: vec![ToolCall { call_id: "c1".into(), tool_name: "search".into(), arguments: serde_json::json!({}) }],
                 final_answer: None,
             };
             let reasoner = MockReasoner::new(vec![thought.clone(); 5]);
             let planner = ReActPlanner::new(reasoner, MockExecutor, 3);
             let result = planner.run("loop forever", "").await;
             assert!(result.is_err());
        }
   }
   ```

**验收标准**：
- [ ] `cargo build -p axiom-planner` 编译通过
- [ ] `cargo test -p axiom-planner` 通过
- [ ] ReAct loop：think→act→observe闭环正确
- [ ] Tool错误时plan标记失败
- [ ] 超出max_iterations返回错误
- [ ] 最终answer正确完成plan

**Commit message**：
```
feat(planner): implement ReAct planner with Think-Act-Observe loop (T149)
```

---

### T150：实现Plan-Execute planner

**新建文件**：
- `crates/axiom-planner/src/plan_execute.rs`

**具体操作**：

1. 创建`plan_execute.rs`：
   ```rust
   //! Plan-Execute Planner - creates a full plan first, then executes step by step.
   //!
   //! Unlike ReAct which interleaves planning and acting, Plan-Execute:
   //! 1. Creates a complete plan upfront
   //! 2. Executes steps sequentially
   //! 3. Triggers replanning if a step fails or observations invalidate the plan

   use crate::error::{PlannerError, PlannerResult};
   use crate::executor::StepExecutor;
   use crate::plan::{Observation, Plan, PlanStatus, PlanStep, StepStatus};
   use crate::replan::Replanner;

   /// Trait for generating an initial plan from a goal.
   pub trait PlanGenerator: Send + Sync {
       /// Generate a complete plan for the given goal.
       async fn generate_plan(&self, goal: &str, context: &str) -> PlannerResult<Plan>;
   }

   /// Configuration for the Plan-Execute planner.
   #[derive(Debug, Clone)]
   pub struct PlanExecuteConfig {
       /// Maximum number of replanning attempts.
       pub max_replans: u32,
       /// Whether to continue after a failed step if replan is possible.
       pub continue_on_replan: bool,
   }

   impl Default for PlanExecuteConfig {
       fn default() -> Self {
             Self {
                 max_replans: 3,
                 continue_on_replan: true,
             }
        }
   }

   /// Plan-Execute planner.
   #[derive(Debug)]
   pub struct PlanExecutePlanner<G: PlanGenerator, E: StepExecutor, R: Replanner> {
       generator: G,
       executor: E,
       replanner: R,
       config: PlanExecuteConfig,
   }

   impl<G: PlanGenerator, E: StepExecutor, R: Replanner> PlanExecutePlanner<G, E, R> {
       /// Create a new Plan-Execute planner.
       pub fn new(generator: G, executor: E, replanner: R, config: PlanExecuteConfig) -> Self {
             Self { generator, executor, replanner, config }
        }

       /// Run the full plan-execute-replan cycle.
       pub async fn run(&self, goal: &str, initial_context: &str) -> PlannerResult<Plan> {
             let mut plan = self.generator.generate_plan(goal, initial_context).await?;
             plan.mark_ready();
             plan.mark_executing();

             let mut replan_count = 0u32;
             let mut context = initial_context.to_string();

             loop {
                 match self.execute_plan(&mut plan, &context).await {
                     Ok(()) => {
                         plan.mark_completed(
                             plan.final_answer.clone().unwrap_or_else(|| "Plan completed".to_string())
                         );
                         return Ok(plan);
                     }
                     Err(PlannerError::ExecutionFailed { step_id, reason }) => {
                         if !self.config.continue_on_replan || replan_count >= self.config.max_replans {
                             plan.mark_failed(format!("Step {} failed: {} (replans exhausted)", step_id, reason));
                             return Ok(plan);
                         }

                         tracing::warn!(step_id, %reason, replan_count, "Step failed, attempting replan");
                         let observations = self.collect_observations(&plan);
                         match self.replanner.replan(&plan, &observations, &reason).await? {
                             Some(new_plan) => {
                                 plan = new_plan;
                                 plan.replan_count = replan_count + 1;
                                 plan.mark_executing();
                                 replan_count += 1;
                                 context = format!("{}\n[Replanned after step {} failed: {}]", context, step_id, reason);
                             }
                             None => {
                                 plan.mark_failed(format!("Step {} failed: {} (replan returned None)", step_id, reason));
                                 return Ok(plan);
                             }
                         }
                     }
                     Err(e) => return Err(e),
                 }
             }
        }

       async fn execute_plan(&self, plan: &mut Plan, context: &str) -> PlannerResult<()> {
             while let Some(_step) = plan.next_pending_step() {
                 let step_id = plan.next_pending_step().unwrap().id.clone();
                 plan.next_pending_step_mut().unwrap().mark_in_progress();
                 let idx = plan.steps.iter().position(|s| s.id == step_id).unwrap();
                 let step = &plan.steps[idx];
                 let desc = step.description.clone();

                 match self.executor.execute_step(step, context).await {
                     Ok(observations) => {
                         plan.steps[idx].observations.extend(observations);
                         plan.steps[idx].mark_completed();
                     }
                     Err(e) => {
                         plan.steps[idx].mark_failed(e.to_string());
                         return Err(PlannerError::ExecutionFailed {
                             step_id,
                             reason: e.to_string(),
                         });
                     }
                 }
             }
             Ok(())
        }

       fn collect_observations(&self, plan: &Plan) -> Vec<Observation> {
             plan.steps.iter()
                 .flat_map(|s| s.observations.iter().cloned())
                 .collect()
        }
   }

   #[cfg(test)]
   mod tests {
       use super::*;

       struct MockGenerator {
             plan: Plan,
       }
       impl PlanGenerator for MockGenerator {
             async fn generate_plan(&self, _goal: &str, _context: &str) -> PlannerResult<Plan> {
                 Ok(self.plan.clone())
             }
        }

       struct MockExecutor {
             fail_step: Option<String>,
       }
       impl StepExecutor for MockExecutor {
             async fn execute_step(&self, step: &PlanStep, _context: &str) -> PlannerResult<Vec<Observation>> {
                 if let Some(fail_id) = &self.fail_step {
                     if step.id == *fail_id {
                         return Err(PlannerError::Internal("injected failure".into()));
                     }
                 }
                 Ok(vec![Observation::success(None, format!("executed {}", step.description))])
             }
        }

       struct MockReplanner {
             new_plan: Option<Plan>,
       }
       impl Replanner for MockReplanner {
             async fn replan(&self, _old: &Plan, _obs: &[Observation], _failure: &str) -> PlannerResult<Option<Plan>> {
                 Ok(self.new_plan.clone())
             }
        }

       #[tokio::test]
       async fn test_plan_execute_success() {
             let mut plan = Plan::new("test goal");
             plan.add_step(PlanStep::new("s1", "step 1", 1));
             plan.add_step(PlanStep::new("s2", "step 2", 2));
             let planner = PlanExecutePlanner::new(
                 MockGenerator { plan },
                 MockExecutor { fail_step: None },
                 MockReplanner { new_plan: None },
                 PlanExecuteConfig::default(),
             );
             let result = planner.run("test", "").await.unwrap();
             assert_eq!(result.status, PlanStatus::Completed);
             assert!(result.all_steps_completed());
        }
   }
   ```

**验收标准**：
- [ ] `cargo build -p axiom-planner` 编译通过
- [ ] `cargo test -p axiom-planner` 通过
- [ ] Plan-Execute流程：先生成完整plan再逐步执行
- [ ] 步骤失败时触发replanning
- [ ] replan次数超限后标记失败
- [ ] 所有步骤完成后标记plan completed

**Commit message**：
```
feat(planner): implement Plan-Execute planner with replanning (T150)
```

---

### T151：实现PlanExecutor

**新建文件**：
- `crates/axiom-planner/src/executor.rs`

**具体操作**：

1. 创建`executor.rs`：
   ```rust
   //! Plan Executor - executes plan steps using tools and LLM.

   use crate::error::PlannerResult;
   use crate::plan::{Observation, PlanStep, ToolCall};

   /// Trait for executing a single plan step (may involve multiple tool calls).
   pub trait StepExecutor: Send + Sync {
       /// Execute a plan step and return observations from all tool calls.
       async fn execute_step(
             &self,
             step: &PlanStep,
             context: &str,
       ) -> PlannerResult<Vec<Observation>>;
   }

   /// A simple executor that runs tool calls sequentially using a ToolExecutor.
   #[derive(Debug)]
   pub struct SimpleExecutor<T: super::react::ToolExecutor> {
       tool_executor: T,
   }

   impl<T: super::react::ToolExecutor> SimpleExecutor<T> {
       /// Create a new simple executor.
       pub fn new(tool_executor: T) -> Self {
             Self { tool_executor }
       }
   }

   impl<T: super::react::ToolExecutor> StepExecutor for SimpleExecutor<T> {
       async fn execute_step(
             &self,
             step: &PlanStep,
             _context: &str,
       ) -> PlannerResult<Vec<Observation>> {
             let mut observations = Vec::new();
             for call in &step.tool_calls {
                 let obs = self.tool_executor.execute(call).await?;
                 if obs.is_error {
                     observations.push(obs);
                     return Err(crate::error::PlannerError::ToolCallFailed {
                         tool: call.tool_name.clone(),
                         message: observations.last().unwrap().content.clone(),
                     });
                 }
                 observations.push(obs);
             }
             // 如果没有tool calls，创建一个基于step描述的synthetic observation
             if observations.is_empty() {
                 observations.push(Observation::success(None, format!("Completed: {}", step.description)));
             }
             Ok(observations)
        }
   }

   /// Configuration for retries in the executor.
   #[derive(Debug, Clone)]
   pub struct RetryConfig {
       /// Maximum retries per tool call.
       pub max_retries: u32,
       /// Base delay in milliseconds for backoff.
       pub base_delay_ms: u64,
   }

   impl Default for RetryConfig {
       fn default() -> Self {
             Self {
                 max_retries: 2,
                 base_delay_ms: 100,
             }
        }
   }

   /// An executor with retry logic for transient tool failures.
   pub struct RetryExecutor<T: super::react::ToolExecutor> {
       inner: T,
       config: RetryConfig,
   }

   impl<T: super::react::ToolExecutor> std::fmt::Debug for RetryExecutor<T> {
       fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
             f.debug_struct("RetryExecutor").field("config", &self.config).finish()
        }
   }

   impl<T: super::react::ToolExecutor> RetryExecutor<T> {
       /// Create a new retry executor.
       pub fn new(inner: T, config: RetryConfig) -> Self {
             Self { inner, config }
       }
   }

   impl<T: super::react::ToolExecutor> super::react::ToolExecutor for RetryExecutor<T> {
       async fn execute(&self, call: &ToolCall) -> PlannerResult<Observation> {
             let mut last_error = None;
             for attempt in 0..=self.config.max_retries {
                 match self.inner.execute(call).await {
                     Ok(obs) if !obs.is_error => return Ok(obs),
                     Ok(obs) => {
                         last_error = Some(obs.content);
                     }
                     Err(e) => {
                         last_error = Some(e.to_string());
                     }
                 }
                 if attempt < self.config.max_retries {
                     let delay = self.config.base_delay_ms * (1 << attempt);
                     tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                 }
             }
             Ok(Observation::error(
                 Some(&call.call_id),
                 format!("Failed after {} retries: {}", self.config.max_retries, last_error.unwrap_or_default()),
             ))
        }
   }

   #[cfg(test)]
   mod tests {
       use super::*;
       use crate::react::ToolExecutor;

       struct MockTool {
             calls: std::sync::atomic::AtomicU32,
             fail_until: u32,
       }

       impl ToolExecutor for MockTool {
             async fn execute(&self, call: &ToolCall) -> PlannerResult<Observation> {
                 let n = self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                 if n < self.fail_until {
                     Ok(Observation::error(Some(&call.call_id), "transient error"))
                 } else {
                     Ok(Observation::success(Some(&call.call_id), "ok"))
                 }
             }
       }

       #[tokio::test]
       async fn test_retry_eventually_succeeds() {
             let tool = MockTool { calls: std::sync::atomic::AtomicU32::new(0), fail_until: 2 };
             let executor = RetryExecutor::new(tool, RetryConfig { max_retries: 3, base_delay_ms: 1 });
             let call = ToolCall { call_id: "c1".into(), tool_name: "test".into(), arguments: serde_json::json!({}) };
             let obs = executor.execute(&call).await.unwrap();
             assert!(!obs.is_error);
       }
   }
   ```

**验收标准**：
- [ ] `cargo build -p axiom-planner` 编译通过
- [ ] `cargo test -p axiom-planner` 通过
- [ ] SimpleExecutor顺序执行tool calls
- [ ] RetryExecutor带指数退避重试
- [ ] 无tool call的step生成synthetic observation

**Commit message**：
```
feat(planner): implement PlanExecutor with retry logic (T151)
```

---

### T152：实现Replanning trigger

**新建文件**：
- `crates/axiom-planner/src/replan.rs`

**具体操作**：

1. 创建`replan.rs`：
   ```rust
   //! Replanning - triggers and logic for plan revision.

   use crate::error::PlannerResult;
   use crate::plan::{Observation, Plan, PlanStep};

   /// Trigger conditions for replanning.
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub enum ReplanTrigger {
       /// A step failed with an error.
       StepFailed { step_id: String, error: String },
       /// Observation indicates the plan assumption is wrong.
       AssumptionInvalidated { reason: String },
       /// New information makes the current plan suboptimal.
       NewInformation { info: String },
       /// Max iterations/retries reached for a step.
       StepStuck { step_id: String, attempts: u32 },
   }

   /// Trait for replanning logic.
   pub trait Replanner: Send + Sync {
       /// Given a failed plan and observations, produce a new plan or None if replanning isn't possible.
       async fn replan(
             &self,
             current_plan: &Plan,
             observations: &[Observation],
             failure_reason: &str,
       ) -> PlannerResult<Option<Plan>>;

       /// Check whether a given observation should trigger replanning.
       fn should_replan(&self, plan: &Plan, observations: &[Observation]) -> Option<ReplanTrigger> {
             // 默认：检查是否有失败的step
             for step in &plan.steps {
                 if step.status == crate::plan::StepStatus::Failed {
                     return Some(ReplanTrigger::StepFailed {
                         step_id: step.id.clone(),
                         error: step.observations.last()
                             .map(|o| o.content.clone())
                             .unwrap_or_default(),
                     });
                 }
             }
             // 检查错误observations
             if let Some(err_obs) = observations.iter().find(|o| o.is_error) {
                 return Some(ReplanTrigger::AssumptionInvalidated {
                     reason: err_obs.content.clone(),
                 });
             }
             None
       }
   }

   /// A simple replanner that creates a new plan by adding a correction step.
   #[derive(Debug, Default)]
   pub struct SimpleReplanner;

   impl SimpleReplanner {
       pub fn new() -> Self { Self }
   }

   impl Replanner for SimpleReplanner {
       async fn replan(
             &self,
             current_plan: &Plan,
             observations: &[Observation],
             failure_reason: &str,
       ) -> PlannerResult<Option<Plan>> {
             let mut new_plan = Plan::new(&current_plan.goal);
             // 复制已完成的步骤
             for step in &current_plan.steps {
                 if step.status == crate::plan::StepStatus::Completed {
                     new_plan.add_step(step.clone());
                 }
             }
             // 添加纠错步骤
             let error_summary: String = observations.iter()
                 .filter(|o| o.is_error)
                 .map(|o| o.content.as_str())
                 .collect::<Vec<_>>()
                 .join("; ");
             let correction = PlanStep::new(
                 &format!("recover-{}", current_plan.replan_count + 1),
                 &format!("Recover from error: {}. {}", failure_reason, error_summary),
                 new_plan.steps.len() as u32 + 1,
             );
             new_plan.add_step(correction);

             // 添加剩余pending步骤
             for step in &current_plan.steps {
                 if step.status == crate::plan::StepStatus::Pending {
                     new_plan.add_step(step.clone());
                 }
             }
             new_plan.mark_ready();
             Ok(Some(new_plan))
        }
   }

   #[cfg(test)]
   mod tests {
       use super::*;

       #[tokio::test]
       async fn test_simple_replan() {
             let replanner = SimpleReplanner::new();
             let mut plan = Plan::new("test");
             plan.add_step(PlanStep::new("s1", "step1", 1));
             plan.steps[0].mark_completed();
             let mut s2 = PlanStep::new("s2", "step2", 2);
             s2.mark_failed("something went wrong");
             plan.add_step(s2);
             plan.add_step(PlanStep::new("s3", "step3", 3));

             let obs = vec![Observation::error(None, "error detail")];
             let new_plan = replanner.replan(&plan, &obs, "step2 failed").await.unwrap();
             assert!(new_plan.is_some());
             let np = new_plan.unwrap();
             assert!(np.steps.len() >= 3);
       }

       #[test]
       fn test_should_replan_on_failed_step() {
             let replanner = SimpleReplanner::new();
             let mut plan = Plan::new("test");
             let mut s1 = PlanStep::new("s1", "a", 1);
             s1.mark_failed("err");
             plan.add_step(s1);
             let trigger = replanner.should_replan(&plan, &[]);
             assert!(trigger.is_some());
       }
   }
   ```

**验收标准**：
- [ ] `cargo build -p axiom-planner` 编译通过
- [ ] `cargo test -p axiom-planner` 通过
- [ ] ReplanTrigger枚举覆盖主要触发场景
- [ ] should_replan默认实现检测失败step和error observation
- [ ] SimpleReplanner生成纠错步骤的新plan

**Commit message**：
```
feat(planner): implement replanning triggers and SimpleReplanner (T152)
```

---

### T153：Plan可视化（Timeline导出）

**新建文件**：
- `crates/axiom-planner/src/viz.rs`

**具体操作**：

1. 创建`viz.rs`：
   ```rust
   //! Plan visualization - export plan execution as Timeline data.

   use crate::plan::{Plan, PlanStatus, StepStatus};
   use serde::{Deserialize, Serialize};

   /// A timeline event for visualization.
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct TimelineEvent {
       /// Timestamp in nanoseconds.
       pub timestamp_ns: u64,
       /// Event type label.
       pub event_type: String,
       /// Description.
       pub description: String,
       /// Associated step ID (if any).
       pub step_id: Option<String>,
       /// Status/result.
       pub status: String,
   }

   /// Plan timeline for visualization (compatible with axiom-viz timeline format).
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct PlanTimeline {
       /// Plan ID.
       pub plan_id: String,
       /// Goal.
       pub goal: String,
       /// Overall plan status.
       pub status: PlanStatus,
       /// Timeline events.
       pub events: Vec<TimelineEvent>,
       /// Steps with their status.
       pub steps: Vec<StepTimelineEntry>,
       /// Final answer (if completed).
       pub final_answer: Option<String>,
       /// Total replan count.
       pub replan_count: u32,
   }

   /// Step entry in timeline.
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct StepTimelineEntry {
       pub id: String,
       pub description: String,
       pub status: StepStatus,
       pub tool_calls_count: usize,
       pub observations_count: usize,
   }

   impl PlanTimeline {
       /// Export a plan to a timeline for visualization.
       pub fn from_plan(plan: &Plan) -> Self {
             let mut events = Vec::new();
             events.push(TimelineEvent {
                 timestamp_ns: plan.created_at_ns,
                 event_type: "plan_created".to_string(),
                 description: plan.goal.clone(),
                 step_id: None,
                 status: format!("{:?}", plan.status),
             });

             for step in &plan.steps {
                 for obs in &step.observations {
                     events.push(TimelineEvent {
                         timestamp_ns: obs.timestamp_ns,
                         event_type: "observation".to_string(),
                         description: obs.content.clone(),
                         step_id: Some(step.id.clone()),
                         status: if obs.is_error { "error".to_string() } else { "success".to_string() },
                     });
                 }
             }

             let steps = plan.steps.iter().map(|s| StepTimelineEntry {
                 id: s.id.clone(),
                 description: s.description.clone(),
                 status: s.status,
                 tool_calls_count: s.tool_calls.len(),
                 observations_count: s.observations.len(),
             }).collect();

             Self {
                 plan_id: plan.id.to_string(),
                 goal: plan.goal.clone(),
                 status: plan.status,
                 events,
                 steps,
                 final_answer: plan.final_answer.clone(),
                 replan_count: plan.replan_count,
             }
        }

       /// Serialize timeline to JSON.
       pub fn to_json(&self) -> Result<String, serde_json::Error> {
             serde_json::to_string_pretty(self)
        }
   }

   #[cfg(test)]
   mod tests {
       use super::*;
       use crate::plan::{Observation, PlanStep};

       #[test]
       fn test_timeline_export() {
             let mut plan = Plan::new("test goal");
             let mut s1 = PlanStep::new("s1", "step one", 1);
             s1.add_observation(Observation::success(None, "done"));
             s1.mark_completed();
             plan.add_step(s1);
             plan.mark_completed("answer");

             let timeline = PlanTimeline::from_plan(&plan);
             assert_eq!(timeline.plan_id, plan.id.to_string());
             assert!(timeline.events.len() >= 2);
             assert_eq!(timeline.steps.len(), 1);
             let json = timeline.to_json().unwrap();
             assert!(json.contains("test goal"));
       }
   }
   ```

**验收标准**：
- [ ] `cargo build -p axiom-planner` 编译通过
- [ ] `cargo test -p axiom-planner` 通过
- [ ] PlanTimeline可序列化为JSON
- [ ] 导出的timeline包含plan创建、每个observation、step状态
- [ ] 与axiom-viz的timeline概念兼容

**Commit message**：
```
feat(planner): add plan visualization via Timeline export (T153)
```

---

### T154：Planner测试

**新建文件**：
- `crates/axiom-planner/tests/planner_integration.rs`

**具体操作**：

1. 创建集成测试（覆盖ReAct、Plan-Execute、replanning场景）。

2. 运行验证：
   ```
   cargo build -p axiom-planner
   cargo clippy -p axiom-planner -- -D warnings
   cargo test -p axiom-planner
   ```

**验收标准**：
- [ ] `cargo test -p axiom-planner` 全部通过
- [ ] `cargo clippy -p axiom-planner -- -D warnings` 零警告
- [ ] 集成测试覆盖：ReAct loop、plan execution、replanning触发
- [ ] `cargo doc -p axiom-planner --no-deps` 无警告

**Commit message**：
```
test(planner): add integration tests for planner (T154)
```

---

## P14阶段：提示词+RAG

### T155：创建axiom-prompt crate骨架

**文件修改**：
- `Cargo.toml`（workspace根）
- 新建：`crates/axiom-prompt/Cargo.toml`
- 新建：`crates/axiom-prompt/src/lib.rs`

**具体操作**：

1. 在根`Cargo.toml`添加members和deps：
   ```toml
   "crates/axiom-prompt",
   "crates/axiom-rag",
   ```
   ```toml
   axiom-prompt = { path = "crates/axiom-prompt" }
   axiom-rag = { path = "crates/axiom-rag" }
   ```

2. 创建`crates/axiom-prompt/Cargo.toml`：
   ```toml
   [package]
   name = "axiom-prompt"
   version.workspace = true
   edition.workspace = true
   license.workspace = true
   rust-version.workspace = true
   description = "Type-safe prompt templates with composable sections and token budgeting"

   [dependencies]
   axiom-core = { workspace = true }
   serde = { workspace = true }
   serde_json = { workspace = true }
   thiserror = { workspace = true }
   tracing = { workspace = true }

   [dev-dependencies]
   tokio = { workspace = true, features = ["rt-multi-thread", "macros"] }
   ```

3. 创建`crates/axiom-prompt/src/lib.rs`：
   ```rust
   //! Axiom Prompt - Type-safe prompt templates with composition and token budgeting.
   //!
   //! Features:
   //! - Typed [`PromptTemplate`] with variable placeholders validated at construction
   //! - [`SystemPrompt`], [`UserPrompt`], [`AssistantPrompt`] message types
   //! - [`PromptComposer`] for building prompts from sections (identity, rules, memory, task)
   //! - Token budgeting during composition to stay within context windows

   pub mod error;
   pub mod template;
   pub mod message;
   pub mod composer;
   pub mod tokens;

   pub use error::{PromptError, PromptResult};
   pub use template::PromptTemplate;
   pub use message::{AssistantPrompt, ChatMessage, MessageRole, SystemPrompt, UserPrompt};
   pub use composer::{PromptComposer, PromptSection};
   pub use tokens::{TokenBudget, TokenCount};
   ```

4. 空骨架文件。

**验收标准**：
- [ ] `cargo build -p axiom-prompt` 编译通过

**Commit message**：
```
feat(prompt): create axiom-prompt crate skeleton (T155)
```

---

### T156：定义PromptTemplate（类型安全模板+构造时验证）

**新建文件**：
- `crates/axiom-prompt/src/error.rs`
- `crates/axiom-prompt/src/template.rs`

**具体操作**：

1. `error.rs`：
   ```rust
   use thiserror::Error;

   #[derive(Error, Debug)]
   pub enum PromptError {
       #[error("Missing variable: {0}")]
       MissingVariable(String),
       #[error("Unknown variable: {0}")]
       UnknownVariable(String),
       #[error("Token budget exceeded: {used} > {budget}")]
       TokenBudgetExceeded { used: usize, budget: usize },
       #[error("Template error: {0}")]
       Template(String),
       #[error("Serialization error: {0}")]
       Serde(#[from] serde_json::Error),
   }

   pub type PromptResult<T> = std::result::Result<T, PromptError>;
   ```

2. `template.rs`：
   ```rust
   //! Typed prompt template with variable placeholder validation.

   use crate::error::{PromptError, PromptResult};
   use serde::{Deserialize, Serialize};
   use std::collections::HashSet;

   /// A validated prompt template with named variable placeholders.
   /// Variables use the `{{variable_name}}` syntax.
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct PromptTemplate {
       /// The template string with {{variable}} placeholders.
       template: String,
       /// Required variable names extracted from the template.
       variables: HashSet<String>,
       /// Estimated base token count (without variables filled).
       base_tokens: usize,
   }

   impl PromptTemplate {
       /// Create and validate a new prompt template.
       pub fn new(template: impl Into<String>) -> PromptResult<Self> {
             let template_str = template.into();
             let variables = extract_variables(&template_str);
             let base_tokens = estimate_tokens(&template_str);
             Ok(Self {
                 template: template_str,
                 variables,
                 base_tokens,
             })
        }

       /// Get required variable names.
       pub fn variables(&self) -> &HashSet<String> {
