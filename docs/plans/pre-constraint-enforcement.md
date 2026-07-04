# 事前约束体系开发计划

> **细分到最小任务单元的完整开发文档**
>
> 目标：在智能体写代码之前、生成代码之前、提交代码之前，实现多层事前约束，
> 将架构违规拦截从"编译期/事后"前移到"生成前/实时"。
>
> **状态**: ✅ 已完成

---

## 目录

1. [整体架构](#1-整体架构)
2. [Phase -1: 提示词工程](#2-phase--1-提示词工程)
3. [Phase 0: 脚手架增强](#3-phase-0-脚手架增强)
4. [Phase 1: IDE/LSP 实时反馈](#4-phase-1-idelsp-实时反馈)
5. [Phase 2: 预提交钩子](#5-phase-2-预提交钩子)
6. [Phase 3: 编译期增强](#6-phase-3-编译期增强)
7. [Phase 4: 运行时 API 扩展](#7-phase-4-运行时-api-扩展)
8. [验收标准总表](#8-验收标准总表)
9. [依赖关系图](#9-依赖关系图)
10. [风险与回滚](#10-风险与回滚)

---

## 1. 整体架构

### 1.1 约束时机全景

```
智能体意图
    │
    ▼
[Layer -1] 提示词约束 ─── 生成代码前，智能体已知晓规则
    │
    ▼
[Layer 0] 脚手架约束 ─── 创建 crate 时自动注册，无法忘记
    │
    ▼
[Layer 1] IDE/LSP 约束 ─── 编辑 Cargo.toml 时实时标红
    │
    ▼
[Layer 2] 预提交约束 ─── git commit 前自动检查
    │
    ▼
[Layer 3] 编译期约束 ─── build.rs 自动执行，违规 panic
    │
    ▼
[Layer 4] CI 约束 ─── push/PR 自动检查，非阻塞报告
    │
    ▼
代码合并
```

### 1.2 设计原则

| 原则 | 说明 | 实现方式 |
|------|------|----------|
| **零信任** | 不依赖智能体自觉 | 编译期/事前自动执行 |
| **单一数据源** | 规则只定义在一处 | `.axiom/architecture.toml` |
| **多层防御** | 多层约束，逐级拦截 | Layer -1 到 Layer 4 |

---

## 2. Phase -1: 提示词工程

**状态**: ✅ 已完成

- [x] `.axiom/AGENTS.md` — 约束入口
- [x] `.axiom/rules/axiom-builder-rules.md` — 20条开发铁律
- [x] `.axiom/identity/axiom-builder.md` — 开发者身份
- [x] `.axiom/skills/axiom-builder-skills.md` — 技能包
- [x] `.axiom/tools.md` — 工具权限

---

## 3. Phase 0: 脚手架增强

**状态**: ✅ 已完成

- [x] `xtask` 统一任务入口
- [x] `gatecheck` 命令
- [x] `state` 命令
- [x] `new_crate` 命令

---

## 4. Phase 1: IDE/LSP 实时反馈

**状态**: ✅ 已完成

- [x] `archcheck` 独立工具
- [x] `gate.rs` 运行时 API
- [x] `build.rs` 编译期门禁

---

## 5. Phase 2: 预提交钩子

**状态**: ✅ 已完成

- [x] pre-commit 配置
- [x] 自动格式化检查
- [x] 自动 clippy 检查

---

## 6. Phase 3: 编译期增强

**状态**: ✅ 已完成

- [x] 18 个 crate 的编译期门禁
- [x] 依赖方向检查
- [x] 禁止依赖检查
- [x] 审计依赖检查
- [x] dev-dependencies 检查

---

## 7. Phase 4: 运行时 API 扩展

**状态**: ✅ 已完成

- [x] `ArchitectureGuardian` interceptor
- [x] `ConstraintValidator`
- [x] 运行时架构检查

---

## 8. 验收标准总表

| 检查项 | 状态 | 说明 |
|--------|------|------|
| 提示词约束加载 | ✅ | 会话开始自动加载 |
| 脚手架约束 | ✅ | xtask 统一入口 |
| IDE/LSP 约束 | ✅ | archcheck 工具 |
| 预提交约束 | ✅ | git hook |
| 编译期约束 | ✅ | build.rs 门禁 |
| CI 约束 | ✅ | GitHub Actions |
| 运行时约束 | ✅ | ArchitectureGuardian |

---

## 9. 依赖关系图

```
Phase -1: 提示词工程
└── ✅ 已完成

Phase 0: 脚手架增强
└── ✅ 已完成

Phase 1: IDE/LSP 实时反馈
└── ✅ 已完成

Phase 2: 预提交钩子
└── ✅ 已完成

Phase 3: 编译期增强
└── ✅ 已完成

Phase 4: 运行时 API 扩展
└── ✅ 已完成
```

---

## 10. 风险与回滚

| 风险 | 概率 | 影响 | 状态 |
|------|------|------|------|
| 编译期门禁影响构建速度 | 低 | 低 | ✅ 已优化（OnceLock 缓存） |
| 规则过于严格影响开发效率 | 中 | 中 | ✅ 已平衡（豁免机制） |
| 智能体忽略约束 | 低 | 高 | ✅ 已解决（强制加载） |

---

## 完成总结

事前约束体系已全面完成：
- ✅ 提示词工程（Layer -1）
- ✅ 脚手架增强（Layer 0）
- ✅ IDE/LSP 实时反馈（Layer 1）
- ✅ 预提交钩子（Layer 2）
- ✅ 编译期增强（Layer 3）
- ✅ 运行时 API 扩展（Layer 4）

**核心成果**:
- `.axiom/` 约束体系完整建立
- `archcheck` 独立工具
- `xtask` 统一入口
- 18 个 crate 编译期门禁全覆盖
- 零信任架构治理体系
