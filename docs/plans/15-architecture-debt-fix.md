# 架构债务修复开发计划

> **目标:** 修复全面审查发现的所有 P0/P1 级架构债务，使架构约束从"设计原则"升级为"编译期/运行时强制"，确保增加功能时熵不增长、问题可一秒定位、功能可方便增删。

> **基线:** P1 核心原语已实现，179 个测试通过，build/clippy/fmt 零警告。但存在 6 个 P0 级正确性 Bug、3 个 P1 级架构违规、大量死代码和测试缺口。

---

## 全局约束

- Rust edition 2021，MSRV 1.75，禁止 async-trait
- `cargo build/clippy/test --workspace` 零警告，`cargo fmt --all --check` 通过
- 每个 Task 完成后必须通过 `cargo test --workspace`
- 错误类型使用 thiserror，应用边界用 anyhow
- 所有 public API 必须有 `///` rustdoc 注释
- 不允许 `unwrap()` / `expect()` 出现在非测试代码中（`unwrap_or_else(|e| e.into_inner())` 处理 poisoned lock 除外）
- 序列化失败必须返回 `Err`，禁止 `unwrap_or(Null)` / `unwrap_or(0)` 静默吞错

---

## Phase 0: 基础设施（错误类型 + API 签名）

> 一切修复的前提。新增错误变体、修正 API 签名，后续所有 Task 依赖这些变更。

### Task 0.1: 新增 AxiomError 变体

**文件:** `crates/axiom-core/src/error.rs`

**问题:** 当前 29 个错误变体中 20 个是死代码，同时缺少关键变体：类型不匹配只能用 `Internal(String)` 表示，Witness 序列化失败无处可报，Signal 序列化失败被静默吞掉。

**步骤:**

- [ ] **Step 1: 新增 `TypeMismatch` 变体**
  ```rust
  #[error("type mismatch: expected {expected}, got {actual}")]
  TypeMismatch { expected: &'static str, actual: &'static str },
  ```
  用于 `DynAxiom::check_dyn` 的 downcast 失败场景，替代 `Internal(String)`。

- [ ] **Step 2: 新增 `WitnessSerialization` 变体**
  ```rust
  #[error("witness serialization failed: {0}")]
  WitnessSerialization(String),
  ```
  用于 Witness 哈希计算中 `serde_json::to_vec` 失败的场景。

- [ ] **Step 3: 新增 `SignalSerialization` 变体**
  ```rust
  #[error("signal serialization failed: {0}")]
  SignalSerialization(String),
  ```
  用于 `Signal::serialize_to_json` 失败的场景。

- [ ] **Step 4: 确认 `Serde(#[from] serde_json::Error)` 仍然保留**
  - `WitnessSerialization` / `SignalSerialization` 携带上下文信息（哪个组件、哪个信号）
  - `Serde` 作为底层序列化错误的兜底
  - 两者不冲突：上层用 `WitnessSerialization` / `SignalSerialization` 包装，底层仍可用 `?` 传播 `serde_json::Error`

**验收标准:**
- [ ] `cargo build -p axiom-core` 通过
- [ ] `cargo clippy -p axiom-core -- -D warnings` 零警告
- [ ] 新增的 3 个变体都有 `#[error(...)]` 属性
- [ ] `AxiomError` 仍然实现 `std::error::Error`（thiserror 自动）

---

### Task 0.2: 修改 Signal::serialize_to_json 返回 Result

**文件:** `crates/axiom-core/src/signal.rs`

**问题:** `serialize_to_json(&self) -> serde_json::Value` 无法表达错误，宏实现用 `unwrap_or(Value::Null)` 静默吞掉序列化失败。

**步骤:**

- [ ] **Step 1: 修改 trait 签名**
  ```rust
  // 旧
  fn serialize_to_json(&self) -> serde_json::Value;
  // 新
  fn serialize_to_json(&self) -> crate::Result<serde_json::Value>;
  ```

- [ ] **Step 2: 更新 SignalEnvelope::new 中的调用方**
  - `signal.rs` 中 `SignalEnvelope::new` 调用 `serialize_to_json` 的地方
  - 用 `?` 传播错误而非 `unwrap_or(Value::Null)`

- [ ] **Step 3: 更新 signal.rs 中测试实现**
  - 测试 mock 的 `serialize_to_json` 也要改签名
  - `serde_json::to_value(self).map_err(|e| AxiomError::SignalSerialization(e.to_string()))`

**验收标准:**
- [ ] `Signal::serialize_to_json` 返回 `Result<Value>`
- [ ] 编译通过（允许暂时有多处 `unwrap()` 需要在后续 Task 中修复）
- [ ] `cargo test -p axiom-core` 通过

---

### Task 0.3: 更新 SignalPayload 宏 — serialize_to_json + validate

**文件:** `crates/axiom-macros/src/lib.rs` (SignalPayload derive, 约 156-194 行)

**问题:**
1. `validate()` 恒返回 `ValidationResult::ok()`，用户定义的 `Schema::validate` 永远不被调用
2. `serialize_to_json()` 用 `unwrap_or(Value::Null)` 吞掉错误

**步骤:**

- [ ] **Step 1: 修改 serialize_to_json 生成代码**
  ```rust
  // 旧 (line 191-193)
  fn serialize_to_json(&self) -> serde_json::Value {
      serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
  }
  // 新
  fn serialize_to_json(&self) -> ::axiom_core::Result<serde_json::Value> {
      serde_json::to_value(self)
          .map_err(|e| ::axiom_core::AxiomError::SignalSerialization(e.to_string()))
  }
  ```

- [ ] **Step 2: 修改 validate() 生成代码，调用 Schema::validate**
  ```rust
  // 旧 (line 188-190)
  fn validate(&self) -> ::axiom_core::ValidationResult {
      ::axiom_core::ValidationResult::ok()
  }
  // 新
  fn validate(&self) -> ::axiom_core::ValidationResult {
      <Self as ::axiom_core::Schema>::validate(self)
  }
  ```

- [ ] **Step 3: 宏生成默认 impl Schema**
  ```rust
  // 在 Signal impl 之后追加
  impl ::axiom_core::Schema for #name #impl_generics #where_clause {
      fn validate(&self) -> ::axiom_core::ValidationResult {
          ::axiom_core::ValidationResult::ok()
      }
  }
  ```
  - 如果用户手动写了 `impl Schema for MySignal`，会编译冲突
  - 这是**预期行为**：编译冲突时用户应添加 `#[schema(skip)]` 属性
  - 本 Step 只生成默认 impl，`#[schema(skip)]` 解析在 Task 0.4 中实现

- [ ] **Step 4: 更新所有使用 SignalPayload 的测试和示例**
  - `signal.rs` 测试中如果有手动 impl Signal 的 mock，更新 `serialize_to_json` 签名
  - `hello_cell.rs` 示例如果手动 impl Schema，确认无冲突

**验收标准:**
- [ ] `cargo build --workspace` 通过
- [ ] `cargo test --workspace` 通过
- [ ] `SignalPayload` 宏生成的 `validate()` 调用 `Schema::validate(self)`
- [ ] `SignalPayload` 宏生成的 `serialize_to_json()` 返回 `Result`
- [ ] 用户手动实现 `impl Schema` 时会编译冲突（预期行为，Task 0.4 解决）

---

### Task 0.4: SignalPayload 宏支持 #[schema(skip)] 属性

**文件:** `crates/axiom-macros/src/lib.rs`

**问题:** Task 0.3 生成了默认 `impl Schema`，用户想自定义校验时会编译冲突。需要 `#[schema(skip)]` 属性来跳过默认生成。

**步骤:**

- [ ] **Step 1: 解析 #[schema(skip)] 属性**
  - 在 SignalPayload derive 的 parse 阶段，检查 struct 上的 `#[schema(skip)]`
  - 设置 `skip_schema_impl: bool` 标志

- [ ] **Step 2: 条件生成 impl Schema**
  ```rust
  if !skip_schema_impl {
      // 生成默认 impl Schema { validate() -> ok() }
  }
  // 如果 skip_schema_impl == true，不生成 impl Schema
  // 用户自己写 impl Schema for MySignal { ... }
  ```

- [ ] **Step 3: 更新文档注释**
  - 在宏的 doc comment 中说明 `#[schema(skip)]` 的用法

**验收标准:**
- [ ] 不加 `#[schema(skip)]` 时，宏生成默认 `impl Schema`（validate 返回 ok）
- [ ] 加 `#[schema(skip)]` 时，宏不生成 `impl Schema`，用户可自行实现
- [ ] `cargo test -p axiom-macros` 通过
- [ ] trybuild 测试中新增 `schema_skip_pass.rs` 和 `schema_skip_fail.rs`

---

## Phase 1: 核心正确性修复（Bug 修复）

> 修复导致错误行为的生产级 Bug。每个 Task 独立，但依赖 Phase 0 的错误类型。

### Task 1.1: 修复 DynAxiomChain::check_all 类型误报

**文件:** `crates/axiom-core/src/axiom.rs` (约 134-152 行)

**问题:** 全局注册表包含所有类型的 axiom。`check_all` 传入任意 state/message 时，类型不匹配的 axiom 返回 `AxiomError::Internal`，被 `filter_map` 误当作违反收集。后果：每次状态变更都报 N 个假违规。

**步骤:**

- [ ] **Step 1: 修改 check_all 过滤逻辑**
  ```rust
  // 旧
  .filter_map(|a| {
      a.check_dyn(current, new, msg)
          .err()
          .map(|e| AxiomViolation { axiom_name: a.name(), error: e, action: a.violation_action() })
  })
  // 新
  .filter_map(|a| {
      match a.check_dyn(current, new, msg) {
          Ok(()) => None,
          Err(AxiomError::TypeMismatch { .. }) => None, // 类型不匹配 = 不适用，跳过
          Err(e) => Some(AxiomViolation {
              axiom_name: a.name(),
              error: e,
              action: a.violation_action(),
          }),
      }
  })
  ```

- [ ] **Step 2: 添加单元测试**
  - 注册两个不同类型的 axiom
  - 用类型 A 的 state 调用 check_all
  - 断言：类型 B 的 axiom 不产生 violation

**验收标准:**
- [ ] 类型不匹配的 axiom 不产生 violation
- [ ] 类型匹配且 check 失败的 axiom 仍产生 violation
- [ ] 类型匹配且 check 通过的 axiom 不产生 violation
- [ ] 新增至少 2 个单元测试

---

### Task 1.2: 修复 axiom 宏 check_dyn 返回 TypeMismatch

**文件:** `crates/axiom-macros/src/lib.rs` (约 293-312 行)

**问题:** `check_dyn` 在 downcast 失败时返回 `AxiomError::Internal`，无法与真正的内部错误区分。Task 1.1 依赖 `TypeMismatch` 变体来过滤。

**步骤:**

- [ ] **Step 1: 修改 check_dyn 生成的代码**
  ```rust
  // 旧
  .ok_or_else(|| ::axiom_core::AxiomError::Internal(
      format!("DynAxiom type mismatch for state in {}", #name_str)
  ))?;
  // 新
  .ok_or_else(|| ::axiom_core::AxiomError::TypeMismatch {
      expected: stringify!(<Self as ::axiom_core::axiom::Axiom>::State),
      actual: std::any::type_name_of_val(current),
  })?;
  ```
  - 对 state、new、msg 三个 downcast 都做同样的修改
  - 注意 `type_name_of_val` 是 nightly 特性，稳定版用 `(*current).type_id()` 配合 `std::any::type_name`

- [ ] **Step 2: 更新 trybuild 测试**
  - 确保宏展开后的代码编译通过

**验收标准:**
- [ ] `check_dyn` downcast 失败时返回 `AxiomError::TypeMismatch`
- [ ] `cargo test -p axiom-macros` 通过
- [ ] Task 1.1 的测试能正确匹配 `TypeMismatch` 变体

---

### Task 1.3: 修复 EntropyEvent::Custom 丢弃 weight

**文件:** `crates/axiom-oversight/src/entropy_governor.rs` (约 108, 131 行)

**问题:** `EntropyEvent::Custom { cell_id, weight }` 的 `weight` 字段被完全忽略，事件被错误计入 `axiom_violations`（权重 3.0），污染审计指标。

**步骤:**

- [ ] **Step 1: 在 EntropyScore 中新增 record_custom 方法**
  - 文件: `crates/axiom-core/src/entropy.rs`
  ```rust
  pub fn record_custom(&mut self, weight: f64) {
      self.value += weight;
  }
  ```

- [ ] **Step 2: 修改 EntropyGovernorCell::record 中 Custom 分支**
  ```rust
  // 旧 (line 108)
  EntropyEvent::Custom { .. } => global.record_axiom_violation(),
  // 新
  EntropyEvent::Custom { weight, .. } => global.record_custom(*weight),
  ```

- [ ] **Step 3: 同步修改 per_cell 的 Custom 分支**
  ```rust
  // 旧 (line 131)
  EntropyEvent::Custom { .. } => entry.record_axiom_violation(),
  // 新
  EntropyEvent::Custom { weight, .. } => entry.record_custom(*weight),
  ```

- [ ] **Step 4: 添加单元测试**
  - 发送 `EntropyEvent::Custom { weight: 0.5 }` 
  - 断言全局熵值增加 0.5（而非 3.0）

**验收标准:**
- [ ] `Custom` 事件使用自身 weight 而非 axiom_violation 权重
- [ ] 全局和 per_cell 的 Custom 分支一致
- [ ] 新增至少 1 个单元测试

---

### Task 1.4: 修复 Witness 序列化错误被静默吞掉

**文件:** `crates/axiom-core/src/witness.rs` (约 87-89, 136-138, 311-313 行)

**问题:** 三处 `serde_json::to_vec(...)` 失败时分别返回 0 或跳过哈希更新。Witness 是审计证据载体，载荷大小为 0 会导致指纹失真。

**步骤:**

- [ ] **Step 1: 修复 compute_signal_fingerprint (约 87-89 行)**
  ```rust
  // 旧
  if let Ok(bytes) = serde_json::to_vec(payload) {
      hasher.update(&bytes);
  }
  // 新
  let bytes = serde_json::to_vec(payload)
      .map_err(|e| crate::AxiomError::WitnessSerialization(
          format!("signal fingerprint payload: {e}")
      ))?;
  hasher.update(&bytes);
  ```
  - 函数签名从 `[u8; 32]` 改为 `crate::Result<[u8; 32]>`
  - 调用方用 `?` 传播

- [ ] **Step 2: 修复 compute_hash (约 136-138 行)**
  ```rust
  // 旧
  if let Ok(vi_bytes) = serde_json::to_vec(&self.version_info) {
      hasher.update(&vi_bytes);
  }
  // 新
  let vi_bytes = serde_json::to_vec(&self.version_info)
      .map_err(|e| crate::AxiomError::WitnessSerialization(
          format!("version_info: {e}")
      ))?;
  hasher.update(&vi_bytes);
  ```
  - `compute_hash` 签名改为 `crate::Result<[u8; 32]>`
  - 调用方用 `?` 传播

- [ ] **Step 3: 修复 emit 中 payload_size (约 311-313 行)**
  ```rust
  // 旧
  let payload_size = serde_json::to_vec(&self.summary)
      .map(|v| v.len())
      .unwrap_or(0);
  // 新
  let payload_size = serde_json::to_vec(&self.summary)
      .map_err(|e| crate::AxiomError::WitnessSerialization(
          format!("summary payload_size: {e}")
      ))?
      .len();
  ```

- [ ] **Step 4: 更新所有调用方**
  - `emit` 方法中调用 `compute_signal_fingerprint` 和 `compute_hash` 的地方
  - 返回类型变为 `crate::Result<Witness>`，用 `?` 传播
  - `WitnessBuilder::emit` 签名变更：`pub fn emit(self, ctx: &mut CellContext<'_>) -> crate::Result<()>`

**验收标准:**
- [ ] 三处序列化失败都返回 `Err(AxiomError::WitnessSerialization(...))`
- [ ] 不存在任何 `unwrap_or(0)` / `unwrap_or(Value::Null)` / `if let Ok(...)` 静默吞错
- [ ] `compute_signal_fingerprint` 和 `compute_hash` 返回 `Result`
- [ ] `WitnessBuilder::emit` 返回 `Result<()>`
- [ ] 所有调用方用 `?` 传播错误
- [ ] `cargo test -p axiom-core` 通过

---

### Task 1.5: 修复 Witness 哈希链断裂

**文件:** `crates/axiom-core/src/witness.rs` (约 299-313 行) + `crates/axiom-core/src/context.rs` (约 217-223 行)

**问题:**
1. `emit` 中 `prev_hash = None` 硬编码，每个 witness 独立，哈希链断裂
2. `signal_fingerprint` 使用硬编码 `SchemaVersion::new(1)` 和 `Value::Null`，而非实际信号的版本和载荷

**步骤:**

- [ ] **Step 1: 修复 signal_fingerprint 使用真实信号数据**
  - `WitnessBuilder` 新增字段：`signal_type: Option<String>`, `schema_version: SchemaVersion`, `payload: serde_json::Value`
  - `triggering_signal::<S: Signal>(signal: &S)` 方法设置这三个字段
  - `emit` 中使用这些字段计算 fingerprint，而非硬编码

- [ ] **Step 2: 修复 prev_hash 从 CellContext 获取**
  - `CellContext` 新增字段：`last_witness_hash: Option<[u8; 32]>`
  - `WitnessBuilder::emit` 从 `ctx.last_witness_hash` 读取 prev_hash
  - emit 成功后更新 `ctx.last_witness_hash = Some(new_hash)`

- [ ] **Step 3: 移除 context.rs 中 add_witness 的覆盖逻辑**
  - 当前 `add_witness` (约 217-223 行) 尝试修复 prev_hash，但应该在 emit 中直接设置正确
  - 移除 `add_witness` 中的 prev_hash 覆盖，只保留 push

- [ ] **Step 4: 添加哈希链完整性测试**
  - 产出 3 个连续 witness
  - 验证 witness[1].prev_hash == hash(witness[0])
  - 验证 witness[2].prev_hash == hash(witness[1])
  - 篡改 witness[1].summary 后验证 `verify_chain_integrity` 返回 Err

**验收标准:**
- [ ] `prev_hash` 正确链接到前一个 witness 的哈希
- [ ] `signal_fingerprint` 使用真实信号类型、版本和载荷
- [ ] 第一个 witness 的 `prev_hash` 为 `None`
- [ ] `verify_chain_integrity` 能检测篡改
- [ ] 新增至少 2 个测试

---

### Task 1.6: 修复 context.rs hop_count 不继承

**文件:** `crates/axiom-core/src/context.rs` (约 128 行)

**问题:** `emit_internal` 中 `env.hop_count = 0` 硬编码，而非继承入站消息的 `hop_count + 1`。导致 `HopLimitInterceptor` 无法检测跨层循环。

**步骤:**

- [ ] **Step 1: 修改 hop_count 继承逻辑**
  ```rust
  // 旧 (line 128)
  env.hop_count = 0;
  // 新
  env.hop_count = self.current_hop_count.map(|h| h + 1).unwrap_or(0);
  ```

- [ ] **Step 2: CellContext 新增 current_hop_count 字段**
  ```rust
  pub(crate) struct CellContext<'a> {
      // ... 已有字段 ...
      current_hop_count: Option<u8>,  // 入站消息的 hop_count
  }
  ```

- [ ] **Step 3: 在 handle 入口设置 current_hop_count**
  - `CellContext::for_handle` 或构造函数中接收入站 envelope 的 hop_count
  - 如果是顶层消息（无入站），则为 None

- [ ] **Step 4: 添加测试**
  - 模拟 Oversight → Agent → Validate → Exec 四层调用链
  - 验证每层 hop_count 递增
  - 验证超过 hop_limit 时 `HopLimitInterceptor` 拒绝

**验收标准:**
- [ ] hop_count 在跨层调用时正确递增
- [ ] 顶层消息的 hop_count 为 0
- [ ] 子消息的 hop_count = 父消息 hop_count + 1
- [ ] 新增至少 1 个测试

---

## Phase 2: 架构强制约束（移除违规 API）

> 移除违反层级调用方向的 API，使架构约束在编译期生效。

### Task 2.1: 移除 ExecCellContext::send_to_validate

**文件:** `crates/axiom-core/src/context.rs` (约 255-257 行)

**问题:** Exec (L1) 只能发送到 Exec (L1)，`send_to_validate` (L2) 违反层级规则。运行时调用会返回 `LayerViolation`，但编译期无法阻止。

**层级规则 (layer.rs:31-37):**
```
Layer::Exec => matches!(target, Layer::Exec),  // L1 只能 → L1
```

**步骤:**

- [ ] **Step 1: 删除 send_to_validate 方法**
  ```rust
  // 删除 (line 255-257)
  pub fn send_to_validate<S: Signal>(&mut self, signal: S, target_cell: &str) -> crate::Result<()> {
      self.0.send(signal, target_cell, Layer::Validate)
  }
  ```

- [ ] **Step 2: 全 workspace 搜索并修复调用方**
  - 搜索 `send_to_validate` 在 Exec 上下文中的使用
  - 如果有调用方，改为通过 Validate 层的 Cell 转发

- [ ] **Step 3: 添加编译期测试**
  - trybuild: 验证在 Exec Cell 中调用 `send_to_validate` 会编译失败

**验收标准:**
- [ ] `ExecCellContext` 不存在 `send_to_validate` 方法
- [ ] `cargo build --workspace` 通过（无调用方残留）
- [ ] trybuild 测试验证编译失败

---

### Task 2.2: 移除 ValidateCellContext::send_to_agent

**文件:** `crates/axiom-core/src/context.rs` (约 315-321 行)

**问题:** Validate (L2) 只能发送到 Validate (L2) 或 Exec (L1)，`send_to_agent` (L3) 违反层级规则。

**层级规则:**
```
Layer::Validate => matches!(target, Layer::Validate | Layer::Exec),  // L2 → L2, L1
```

**步骤:**

- [ ] **Step 1: 删除 send_to_agent 方法**
  ```rust
  // 删除 (line 315-321)
  pub fn send_to_agent<S: Signal>(&mut self, signal: S, target_cell: &str) -> crate::Result<()> {
      self.0.send(signal, target_cell, Layer::Agent)
  }
  ```

- [ ] **Step 2: 全 workspace 搜索并修复调用方**

- [ ] **Step 3: 添加 trybuild 编译期测试**

**验收标准:**
- [ ] `ValidateCellContext` 不存在 `send_to_agent` 方法
- [ ] `cargo build --workspace` 通过
- [ ] trybuild 测试验证编译失败

---

### Task 2.3: 修复 emit_internal 忽略 warnings

**文件:** `crates/axiom-core/src/context.rs` (约 107-112 行)

**问题:** `emit_internal` 调用 `signal.validate()` 后只检查 `has_errors()`，`has_warnings()` 被完全忽略。校验警告既不记录也不传递给 Witness。

**步骤:**

- [ ] **Step 1: 在 emit_internal 中记录 warnings**
  ```rust
  let validation = signal.validate();
  if validation.has_errors() {
      return Err(crate::AxiomError::SignalValidation { ... });
  }
  if validation.has_warnings() {
      // 记录到 Witness 或 tracing，但不阻止发送
      tracing::warn!(
          signal_type = signal.signal_type(),
          warnings = %validation,
          "signal validation produced warnings"
      );
  }
  ```

- [ ] **Step 2: 添加测试**
  - 构造一个 Schema::validate 返回 warnings 但无 errors 的 Signal
  - 验证 emit_internal 不返回 Err
  - 验证 warnings 被记录（可通过 tracing test 或返回值检查）

**验收标准:**
- [ ] warnings 不阻止 Signal 发送
- [ ] warnings 被记录到 tracing
- [ ] errors 仍然阻止发送
- [ ] 新增至少 1 个测试

---

## Phase 3: Runtime 集成（接线 + 派发修复）

> 让架构的监控能力真正生效。

### Task 3.1: 统一 EntropyGovernor — 删除 runtime 副本，复用 oversight 版本

**文件:**
- 删除: `crates/axiom-runtime/src/entropy_gov.rs`
- 修改: `crates/axiom-runtime/src/runtime.rs`
- 修改: `crates/axiom-runtime/Cargo.toml`

**问题:** runtime 有自己的 `EntropyGovernor`（简单原子计数器），oversight 有 `EntropyGovernorCell`（完整实现）。两者权重不一致，runtime 版本未接线到派发路径。

**步骤:**

- [ ] **Step 1: axiom-runtime 依赖 axiom-oversight**
  - 在 `Cargo.toml` 中添加 `axiom-oversight = { path = "../axiom-oversight" }`
  - 确认不违反 crate 依赖方向（runtime 层可以依赖 oversight 层）

- [ ] **Step 2: runtime.rs 中用 EntropyGovernorCell 替换 EntropyGovernor**
  - `governor: Arc<EntropyGovernorCell>` 替代 `Arc<EntropyGovernor>`
  - `EntropyGovernorCell::new(thresholds)` 替代 `EntropyGovernor::new(threshold)`
  - 更新 `governor()` 访问器返回类型

- [ ] **Step 3: 删除 entropy_gov.rs**
  - 删除整个文件
  - 从 `mod.rs` / `lib.rs` 中移除 `pub mod entropy_gov;`

- [ ] **Step 4: 更新所有引用**
  - `runtime.rs` 中所有 `governor.record_*` 调用改为 `governor.record(EntropyEvent::*)`
  - `snapshot()` 调用适配新 API

**验收标准:**
- [ ] `entropy_gov.rs` 被删除
- [ ] runtime 使用 `EntropyGovernorCell`
- [ ] 权重与 core 的 `EntropyWeights` 一致
- [ ] `cargo build --workspace` 通过
- [ ] `cargo test --workspace` 通过

---

### Task 3.2: 修复 dispatch loop — 实际调用 Cell::handle

**文件:** `crates/axiom-runtime/src/runtime.rs` (约 308-314 行)

**问题:** 派发循环从 mailbox pop 消息后，不调用 `Cell::handle`，直接 `record_success`。消息被消费但从未执行。

**步骤:**

- [ ] **Step 1: 保存 CellHandle 引用**
  - dispatch loop 的 `cells_data` 中除了 mailbox 和 cell_id，还需要保存 `CellHandle` 引用
  - 或通过 `Arc<Mutex<HashMap<CellId, CellHandle>>>` 查找

- [ ] **Step 2: pop 后调用 Cell::handle**
  ```rust
  while let Some(env) = mb.pop().await {
      let cell = cells.get(&cid);
      if let Some(handle) = cell {
          let ctx = CellContext::new(/* ... */);
          match handle.handle(env.signal, &mut ctx).await {
              Ok(()) => {
                  supervisor.record_success(&cid).await;
                  // 将 ctx.outgoing 推入 bus
              }
              Err(e) => {
                  supervisor.record_failure(&cid, &e.to_string()).await;
                  governor.record(EntropyEvent::Custom {
                      cell_id: cid.clone(),
                      weight: 1.0,
                  });
              }
          }
      }
  }
  ```

- [ ] **Step 3: 处理 ctx.outgoing**
  - handle 执行后，将 CellContext 中的 outgoing envelopes 推回 bus
  - 注意层级检查（emit_internal 已做，但 bus dispatch 时再做一次）

- [ ] **Step 4: 添加集成测试**
  - 注册一个简单的 EchoCell
  - 发送消息，验证 Cell::handle 被调用
  - 验证返回的消息被正确路由

**验收标准:**
- [ ] 消息 pop 后实际调用 `Cell::handle`
- [ ] handle 成功时 `record_success`
- [ ] handle 失败时 `record_failure` + 记录熵事件
- [ ] outgoing envelopes 被推回 bus
- [ ] 新增至少 1 个集成测试

---

### Task 3.3: 接线 EntropyGovernor 到派发路径

**文件:** `crates/axiom-runtime/src/runtime.rs` (约 317-323 行)

**问题:** `should_reduce` 触发后只 log + reset，不发出治理动作。且 `record_*` 方法在派发路径中从未被调用。

**步骤:**

- [ ] **Step 1: 在 dispatch loop 中记录熵事件**
  - 消息被 mailbox 拒绝时 → `record(EntropyEvent::DroppedMessage { cell_id })`
  - 拦截器拒绝时 → `record(EntropyEvent::RejectedByGuardian { cell_id })`
  - handle 失败时 → `record(EntropyEvent::Custom { cell_id, weight: 1.0 })`
  - supervisor 重启 Cell 时 → `record(EntropyEvent::CellRestart { cell_id })`
  - circuit breaker 触发时 → `record(EntropyEvent::CircuitBreak { cell_id })`

- [ ] **Step 2: should_reduce 触发时执行治理动作**
  ```rust
  if governor.should_reduce(cooldown) {
      let snapshot = governor.snapshot();
      let action = governor.take_action();
      match action {
          GovernanceAction::Warn => tracing::warn!(score = snapshot.global.value, "entropy high"),
          GovernanceAction::Throttle => { /* 限制最热 Cell 的 mailbox 推送速率 */ }
          GovernanceAction::Emergency => { /* 停止接受新消息，只处理存量 */ }
          GovernanceAction::None => {}
      }
      governor.reset();
  }
  ```

- [ ] **Step 3: 健康检查中报告真实熵值**
  - `h.entropy_score = governor.snapshot().global.value`
  - 移除硬编码的 `h.total_restarts = 0`

**验收标准:**
- [ ] 5 种熵事件在对应场景被记录
- [ ] `should_reduce` 触发时执行 `take_action()` 返回的治理动作
- [ ] 健康检查报告真实熵值
- [ ] `total_restarts` 从 supervisor 读取真实数据

---

## Phase 4: 死代码清理

> 移除未使用的代码，降低维护成本和认知负担。

### Task 4.1: 删除 MigrationRegistry

**文件:** `crates/axiom-core/src/version.rs` (约 208-315 行)

**问题:** `MigrationRegistry` 被 `SchemaMigrator` 取代，仅测试使用。两套迁移逻辑造成混淆。

**步骤:**

- [ ] **Step 1: 确认 MigrationRegistry 无生产调用方**
  - 全 workspace 搜索 `MigrationRegistry`
  - 确认只在 tests 和自身 impl 中使用

- [ ] **Step 2: 将 MigrationRegistry 的测试迁移到 SchemaMigrator**
  - 等价测试用 `SchemaMigrator` 重新实现

- [ ] **Step 3: 删除 MigrationRegistry 及相关代码**
  - 删除 struct、impl、registry slice
  - 删除 `verify_complete`、`list_types` 等方法

- [ ] **Step 4: 更新 lib.rs 导出**

**验收标准:**
- [ ] `MigrationRegistry` 完全删除
- [ ] 等价测试覆盖在 `SchemaMigrator` 中
- [ ] `cargo test --workspace` 通过

---

### Task 4.2: 删除其他死代码

**文件:** 多个

**清单:**

- [ ] **Step 1: 删除 DynamicSchema**
  - 文件: `crates/axiom-core/src/schema.rs` (约 288-315 行)
  - 零调用者，无法运行时校验

- [ ] **Step 2: 删除 typed AxiomChain<T,M>**
  - 文件: `crates/axiom-core/src/axiom.rs` (约 61-110 行)
  - 仅测试使用，DynAxiomChain 已覆盖

- [ ] **Step 3: 删除 Lens 模块**
  - 文件: `crates/axiom-core/src/lens.rs` 全文
  - 骨架已建，无人使用
  - 从 `lib.rs` 移除 `pub mod lens;`

- [ ] **Step 4: 清理 AxiomError 死变体**
  - 统计 29 个变体中哪些从未被构造
  - 删除确认无调用方的变体
  - 保留有明确未来用途的变体（如 `Store`, `Io`）

- [ ] **Step 5: 删除其他零调用项**
  - `verify_migration_chain_for_type` (registry.rs:47)
  - `VersionInfo::with_identity` (version.rs:499)
  - `Compatibility::can_read_witness` (version.rs:167)
  - `CrateVersion` 别名 (version.rs:535)

**验收标准:**
- [ ] 每个删除项确认零生产调用方
- [ ] `cargo build --workspace` 通过
- [ ] `cargo test --workspace` 通过
- [ ] `cargo clippy --workspace -- -D warnings` 零警告

---

## Phase 5: 代码重复消除

### Task 5.1: 统一 EntropyLevel

**文件:**
- `crates/axiom-core/src/entropy.rs`
- `crates/axiom-oversight/src/entropy_governor.rs`

**问题:** `EntropyLevel` 在 core 和 oversight 中各有一份定义，逐字复制 + 手工映射。

**步骤:**

- [ ] **Step 1: oversight 中删除 EntropyLevel 定义**
- [ ] **Step 2: oversight 改用 `axiom_core::entropy::EntropyLevel`**
- [ ] **Step 3: 删除手工映射函数**

**验收标准:**
- [ ] `EntropyLevel` 只在 core 中定义
- [ ] oversight re-export core 的版本
- [ ] 无编译警告

---

### Task 5.2: 统一 now_ns

**文件:** 5 处定义

**问题:** `now_ns()` 在 5 个文件中分别定义，2 种实现变体。

**步骤:**

- [ ] **Step 1: 在 core 中标记 `pub fn now_ns()` 为唯一来源**
- [ ] **Step 2: 其他 crate 改用 `axiom_core::signal::now_ns()`**
- [ ] **Step 3: 删除重复定义**

**验收标准:**
- [ ] `now_ns()` 只在 core 中定义一次
- [ ] 全 workspace 无重复定义

---

## Phase 6: 测试补齐

### Task 6.1: 错误路径测试

**文件:** `crates/axiom-core/tests/integration_tests.rs`

**步骤:**

- [ ] **Step 1: LayerViolation 测试**
  - Exec Cell 尝试发送到 Validate → 断言返回 `LayerViolation`

- [ ] **Step 2: Witness 链断裂检测测试**
  - 篡改 witness 后 `verify_chain_integrity` 返回 Err

- [ ] **Step 3: 序列化失败测试**
  - 构造不可序列化的 Signal → 断言返回 `SignalSerialization`

- [ ] **Step 4: Axiom 类型不匹配测试**
  - 注册类型 A 的 axiom，用类型 B 调用 check_all → 断言不产生 violation

- [ ] **Step 5: hop_count 溢出测试**
  - 构造超过 hop_limit 的调用链 → 断言被拦截

**验收标准:**
- [ ] 5 个错误路径各至少 1 个测试
- [ ] 所有测试通过

---

### Task 6.2: 并发测试

**文件:** `crates/axiom-core/tests/integration_tests.rs` 或新建 `concurrency_tests.rs`

**步骤:**

- [ ] **Step 1: tokio::spawn 并发 Cell 测试**
  - 多个 Cell 并发处理消息
  - 验证 Cell 状态不互相干扰

- [ ] **Step 2: Mailbox 并发 push/pop 测试**
  - 多生产者单消费者场景
  - 验证不丢消息、不重消息

- [ ] **Step 3: EntropyGovernor 并发 record 测试**
  - 多线程同时 record
  - 验证计数器正确

**验收标准:**
- [ ] 至少 3 个并发测试
- [ ] 测试通过且无 data race（cargo test 通过）
- [ ] 如可能，`cargo test -- --test-threads=1` 和 `--test-threads=8` 都通过

---

## Phase 7: 全量验证

### Task 7.1: 全量构建 + 质量门禁

**步骤:**

- [ ] **Step 1: cargo fmt --all -- --check**
- [ ] **Step 2: cargo clippy --workspace -- -D warnings**
- [ ] **Step 3: cargo build --workspace**
- [ ] **Step 4: cargo test --workspace**
- [ ] **Step 5: 验证测试数量 ≥ 179（不回退）**
- [ ] **Step 6: 验证零 `unwrap()` / `expect()` 在非测试代码中**

**验收标准:**
- [ ] 全部通过
- [ ] 测试数量不回退
- [ ] 零 warnings

---

## 任务依赖图

```
Phase 0 (基础)
  ├── Task 0.1 (Error 变体) ──────────────────────────┐
  ├── Task 0.2 (Signal::serialize_to_json 签名) ──────┤
  └── Task 0.3 (SignalPayload 宏) ── Task 0.4 (schema(skip))  │
                                                         │
Phase 1 (Bug 修复)                                       │
  ├── Task 1.1 (check_all) ←── 依赖 0.1, 1.2           │
  ├── Task 1.2 (check_dyn 宏) ←── 依赖 0.1              │
  ├── Task 1.3 (Custom weight) ←── 独立                 │
  ├── Task 1.4 (Witness 序列化) ←── 依赖 0.1            │
  ├── Task 1.5 (Witness 哈希链) ←── 依赖 1.4            │
  └── Task 1.6 (hop_count) ←── 独立                     │
                                                         │
Phase 2 (架构强制)                                       │
  ├── Task 2.1 (删 send_to_validate) ←── 独立           │
  ├── Task 2.2 (删 send_to_agent) ←── 独立             │
  └── Task 2.3 (warnings 记录) ←── 依赖 0.3             │
                                                         │
Phase 3 (Runtime 集成)                                   │
  ├── Task 3.1 (统一 EntropyGovernor) ←── 依赖 Phase 1  │
  ├── Task 3.2 (dispatch loop) ←── 依赖 3.1             │
  └── Task 3.3 (接线熵监控) ←── 依赖 3.1, 3.2          │
                                                         │
Phase 4 (死代码) ←── 依赖 Phase 0-3 全部完成             │
Phase 5 (去重复) ←── 依赖 Phase 4                        │
Phase 6 (测试)   ←── 可与 Phase 1-3 并行                │
Phase 7 (验证)   ←── 依赖全部                            │
```

---

## 执行顺序（推荐）

| 序号 | Task | 预估难度 | 依赖 |
|------|------|---------|------|
| 1 | 0.1 Error 变体 | ★☆☆ | 无 |
| 2 | 0.2 serialize_to_json 签名 | ★☆☆ | 无 |
| 3 | 0.3 SignalPayload 宏修复 | ★★☆ | 0.1, 0.2 |
| 4 | 0.4 schema(skip) 属性 | ★★☆ | 0.3 |
| 5 | 1.2 check_dyn 返回 TypeMismatch | ★☆☆ | 0.1 |
| 6 | 1.1 check_all 过滤 TypeMismatch | ★☆☆ | 0.1, 1.2 |
| 7 | 1.3 Custom weight 修复 | ★☆☆ | 无 |
| 8 | 1.4 Witness 序列化错误传播 | ★★☆ | 0.1 |
| 9 | 1.5 Witness 哈希链修复 | ★★★ | 1.4 |
| 10 | 1.6 hop_count 继承 | ★★☆ | 无 |
| 11 | 2.1 删 send_to_validate | ★☆☆ | 无 |
| 12 | 2.2 删 send_to_agent | ★☆☆ | 无 |
| 13 | 2.3 warnings 记录 | ★☆☆ | 0.3 |
| 14 | 3.1 统一 EntropyGovernor | ★★☆ | Phase 1 |
| 15 | 3.2 dispatch loop 修复 | ★★★ | 3.1 |
| 16 | 3.3 接线熵监控 | ★★☆ | 3.1, 3.2 |
| 17 | 4.1 删 MigrationRegistry | ★☆☆ | Phase 0-3 |
| 18 | 4.2 删其他死代码 | ★☆☆ | 4.1 |
| 19 | 5.1 统一 EntropyLevel | ★☆☆ | 4.2 |
| 20 | 5.2 统一 now_ns | ★☆☆ | 4.2 |
| 21 | 6.1 错误路径测试 | ★★☆ | Phase 1-2 |
| 22 | 6.2 并发测试 | ★★★ | Phase 3 |
| 23 | 7.1 全量验证 | ★☆☆ | 全部 |
