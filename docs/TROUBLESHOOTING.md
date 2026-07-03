# Axiom Core 故障排查指南

> **版本:** v0.1.0
> **最后更新:** 2026-07-04

---

## 1. 编译期问题

### 1.1 ARCHITECTURE VIOLATION: REVERSE DEPENDENCY

**现象**：
```
error[E???]: ARCHITECTURE VIOLATION: REVERSE DEPENDENCY
xxx-crate (level N) depends on yyy-crate (level M) which is a HIGHER layer
```

**原因**：Layer N 的 crate 依赖了 Layer < N 的 crate，违反了分层规则。

**解决方案**：
1. 检查 `.axiom/architecture.toml` 中的 `[crate-layers]`，确认两个 crate 的层
2. 如果依赖是设计需要的，添加 `[reverse-dependency-exemptions]`
3. 否则，移除该依赖或调整 crate 分层

```bash
# 查看 crate 层分配
cargo run -p archcheck -- --list-crates
```

### 1.2 FORBIDDEN DEPENDENCY

**现象**：
```
error[E???]: FORBIDDEN DEPENDENCY
'async-trait' is FORBIDDEN in axiom crates.
```

**原因**：引入了禁止依赖 `async-trait`。

**解决方案**：
1. 移除 `async-trait` 依赖
2. 使用 Rust 1.75+ 原生 `async fn in traits`

```rust
// 替代 async-trait
trait MyTrait {
    async fn my_method(&self) -> Result<()>;
}
```

### 1.3 UNAUDITED DEPENDENCY

**现象**：
```
error[E???]: UNAUDITED DEPENDENCY
'xxx-crate' has not been audited (R-022).
```

**原因**：引入了未在 `[audited-deps]` 中的第三方依赖。

**解决方案**：
1. 确认依赖是否必要
2. 如果是，添加到 `.axiom/architecture.toml` 的 `[audited-deps]`
3. 如果否，寻找替代方案或移除

```bash
# 查看已审计依赖列表
grep -A 30 '\[audited-deps\]' .axiom/architecture.toml
```

### 1.4 build.rs 执行失败

**现象**：
```
error: could not compile `xxx-crate` (lib) due to 1 previous error
```

**原因**：`build.rs` 中的 `archcheck::build_hook::check_current_crate()` 检测到违规。

**解决方案**：
1. 查看完整的 panic 错误信息
2. 根据错误类型修复（反向依赖 / 禁止依赖 / 未审计依赖）
3. 重新运行 `cargo check` 验证

### 1.5 TOML 解析错误

**现象**：
```
TOML parse error in ../../../.axiom/architecture.toml
```

**原因**：`.axiom/architecture.toml` 语法错误。

**解决方案**：
1. 验证 TOML 语法：
   ```bash
   cargo run -p archcheck -- --validate-architecture
   ```
2. 检查 TOML 文件格式，确保所有 section 正确闭合
3. 修复后重新编译

---

## 2. 依赖问题

### 2.1 循环依赖

**现象**：
```
error[E???]: cyclic non-exhaustive crate
```

**原因**：两个或多个 crate 之间存在循环依赖。

**解决方案**：
1. 使用 `cargo tree` 查看依赖树
2. 找到循环依赖的路径
3. 通过引入新的抽象 crate 或使用 trait 打破循环

```bash
# 查看依赖树
cargo tree -i xxx-crate

# 查看反向依赖
cargo tree --invert xxx-crate
```

### 2.2 版本冲突

**现象**：
```
error[E???]: trait bounds not satisfied
```

**原因**：依赖的不同版本之间存在冲突。

**解决方案**：
1. 检查 `Cargo.toml` 中的版本要求
2. 统一依赖版本
3. 使用 `cargo update -p <package>` 更新特定依赖

```bash
# 查看依赖版本
cargo tree

# 更新特定依赖
cargo update -p serde
```

---

## 3. 运行时问题

### 3.1 Cell 借用冲突

**现象**：
```
error[E0499]: cannot borrow `xxx` as mutable more than once at a time
```

**原因**：在 `Cell::handle` 中错误地多次访问 `ctx`。

**解决方案**：
1. 使用 "Drain Inside" 模式
2. 在 `handle` 内部调用 `ctx.end_processing()` 排空所有数据
3. 返回三元组，不要在 `handle().await` 后访问 `ctx`

### 3.2 循环中 handle 借用

**现象**：
```
error[E0499]: cannot borrow `xxx` as mutable more than once at a time
```

**原因**：循环中多次调用 `handle`，借用扩展到所有迭代。

**解决方案**：
1. 使用 `Arc<Mutex<Cell>>` 包装
2. 每次循环获取本地 guard

```rust
for i in 0..5 {
    let mut guard = cell.lock().await;
    let mut ctx = CellContext::new(&cell_id, Layer::Exec);
    let (r, outgoing, witnesses) = guard.handle(signal, &mut ctx).await;
}
```

### 3.3 async block 中 `?` 运算符

**现象**：
```
error[E???]: `?` operator has incompatible types
```

**原因**：async block 返回 tuple 时不能直接用 `?`。

**解决方案**：
使用闭包模式：
```rust
async move {
    let result: Result<()> = (|| {
        ctx.emit_event(event, Layer::Exec)?;
        Ok(())
    })();
    let (outgoing, witnesses) = ctx.end_processing();
    (result, outgoing, witnesses)
}
```

---

## 4. 架构治理问题

### 4.1 预提交钩子失败

**现象**：
```
❌ 架构检查失败，请修复后再提交
```

**原因**：staging area 中的 Cargo.toml 变更触发了架构违规。

**解决方案**：
1. 查看详细错误信息
2. 修复架构违规（移除未审计依赖 / 调整层方向 / 添加豁免）
3. 重新提交

```bash
# 手动运行预提交检查
cargo run -p xtask -- precommit

# 紧急跳过（不推荐）
git commit --no-verify
```

### 4.2 未注册 Crate

**现象**：
```
warning: xxx-crate is not registered in architecture.toml
```

**原因**：新增的 crate 没有在 `.axiom/architecture.toml` 中注册。

**解决方案**：
1. 使用 `cargo run -p xtask -- new_crate` 创建 crate（自动注册）
2. 或手动编辑 `.axiom/architecture.toml` 添加 crate

```bash
# 自动注册
cargo run -p xtask -- new_crate --name myfeature --layer 4
```

### 4.3 架构规则不一致

**现象**：不同工具显示的架构规则不一致。

**原因**：`.axiom/architecture.toml` 被多处引用，可能存在缓存。

**解决方案**：
1. 清理构建缓存：
   ```bash
   cargo clean -p archcheck
   cargo clean -p xtask
   ```
2. 重新运行检查

---

## 5. 性能问题

### 5.1 编译缓慢

**现象**：`cargo check` 时间过长。

**解决方案**：
1. 使用 `cargo check -p <crate>` 只检查单个 crate
2. 启用增量编译（默认开启）
3. 清理增量编译缓存：
   ```bash
   cargo clean -p <crate>
   ```

### 5.2 测试缓慢

**现象**：`cargo test` 时间过长。

**解决方案**：
1. 使用 `cargo test -p <crate>` 只测试单个 crate
2. 使用 `cargo test --lib` 只测试库代码
3. 并行测试（默认开启）：
   ```bash
   cargo test --workspace -- --test-threads=4
   ```

---

## 6. 工具问题

### 6.1 archcheck 报错

**现象**：`archcheck` 无法解析 `Cargo.toml`。

**解决方案**：
1. 检查 `Cargo.toml` 语法是否正确
2. 确保 `Cargo.toml` 包含 `[package]` section
3. 检查文件编码是否为 UTF-8

### 6.2 xtask 命令找不到

**现象**：
```
error: no such subcommand: `precommit`
```

**原因**：`xtask` 未更新到最新版本。

**解决方案**：
1. 重新编译 `xtask`：
   ```bash
   cargo build -p xtask
   ```
2. 使用完整路径：
   ```bash
   cargo run -p xtask -- precommit
   ```

---

## 7. IDE 集成问题

### 7.1 rust-analyzer 报错

**现象**：IDE 显示大量错误，但 `cargo check` 通过。

**解决方案**：
1. 重启 rust-analyzer
2. 清理 IDE 缓存
3. 确保 `rust-toolchain.toml` 配置正确

### 7.2 宏展开错误

**现象**：IDE 无法理解宏生成的代码。

**解决方案**：
1. 使用 `cargo expand` 查看宏展开结果
2. 确保宏定义在依赖 crate 中正确导出

---

## 8. 紧急恢复

### 8.1 回滚架构变更

```bash
# 查看最近架构变更
git log --oneline -- .axiom/architecture.toml

# 回滚到某个版本
git checkout <commit-hash> -- .axiom/architecture.toml
```

### 8.2 禁用编译期检查（不推荐）

```bash
# 临时禁用 build.rs（紧急情况）
mv crates/axiom-xxx/build.rs crates/axiom-xxx/build.rs.bak
```

**注意**：这会导致架构违规无法被检测，仅用于紧急恢复。

### 8.3 重置状态

```bash
# 重置到最新 commit
git reset --hard HEAD

# 清理构建缓存
cargo clean

# 重新克隆（极端情况）
git clone https://github.com/arwei944/axiom-core.git
```

---

## 9. 获取帮助

### 9.1 文档资源

- [HANDOVER.md](HANDOVER.md) — 项目交接文档
- [PROGRESS.md](PROGRESS.md) — 进度总览
- [architecture-diagram.md](architecture-diagram.md) — 架构设计图
- [pre-constraint-enforcement.md](plans/pre-constraint-enforcement.md) — 事前约束计划
- [.axiom/bootstrap.md](../.axiom/bootstrap.md) — 会话引导协议
- [.axiom/prompts/architecture-constraints.md](../.axiom/prompts/architecture-constraints.md) — 提示词模板

### 9.2 常用诊断命令

```bash
# 完整检查清单
cargo check --workspace && \
cargo test --workspace && \
cargo run -p archcheck -- --validate-architecture && \
cargo run -p archcheck -- && \
cargo run -p xtask -- gatecheck --strict

# 查看依赖树
cargo tree

# 查看构建脚本输出
cargo check -p xxx --verbose

# 查看详细编译错误
cargo check -p xxx 2>&1 | less
```

### 9.3 联系支持

- **架构问题**：查看 [.axiom/architecture.toml](.axiom/architecture.toml)
- **开发问题**：查看 [docs/HANDOVER.md](HANDOVER.md)
- **约束问题**：查看 [docs/plans/pre-constraint-enforcement.md](plans/pre-constraint-enforcement.md)
- **Bug 报告**：提交 GitHub Issue
