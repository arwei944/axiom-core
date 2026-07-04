# 开发门禁：未加载约束禁止开发

> **本文件是开发的唯一入口。不完成预检，禁止修改任何代码。**
> This is a HARD GATE. No code changes without completing preflight.

---

## 给开发者（包括AI Agent）

**在修改任何代码之前，你必须：**

### 第一步：加载约束（强制）

读取以下文件，按顺序：

1. **[.axiom/AGENTS.md](.axiom/AGENTS.md)** — 约束入口和身份定义
2. **[.axiom/rules/axiom-builder-rules.md](.axiom/rules/axiom-builder-rules.md)** — 20条开发铁律
3. **[.axiom/identity/axiom-builder.md](.axiom/identity/axiom-builder.md)** — 开发者身份和权限边界
4. **[.axiom/skills/axiom-builder-skills.md](.axiom/skills/axiom-builder-skills.md)** — 可用技能包
5. **[.axiom/tools.md](.axiom/tools.md)** — 工具权限边界

### 第二步：完成预检清单

读取 **[.axiom/preflight.md](.axiom/preflight.md)**，逐项确认所有检查项通过。

### 第三步：确认当前任务

读取当前阶段的plan文件（`docs/plans/` 目录下），确认：
- 当前是哪个Phase、哪个Task、哪个Step
- 验收标准是什么

---

## 约束体系说明

```
.axiom/
├── AGENTS.md              ← 你在这里。约束入口。
├── preflight.md           ← 🔴 预检清单（不通过=不能开发）
├── identity/              ← 你是谁
├── rules/                 ← 你必须遵守什么
├── skills/                ← 你会什么
└── tools.md               ← 你能用什么
```

**规则优先级**：R-000（强制加载约束）> 其他Critical规则 > Strict > Warning > Info

**违反R-000的后果**：任何在未加载约束情况下产生的代码提交一律回滚。

---

## R-000: 强制约束加载（最高优先级）

> **R-000 是 R-001 之上的最高规则。**

- 🔴 **每次新会话开始**，必须执行上述"第一步"和"第二步"
- 🔴 **每次切换Task**，必须重新确认预检清单
- 🔴 **未完成预检，不得执行任何文件修改操作**（Edit/Write/Delete/RunCommand修改类命令）
- 🔴 **只读操作**（Read/Grep/LS/Search）不受限制——用来加载约束本身

---

## 当前项目状态

- **当前阶段**: v0.3.0（生产就绪、可观测性、工程债偿还）
- **当前Task**: 文档更新与清理
- **Plan**: [docs/plans/v0.3-task-breakdown.md](docs/plans/v0.3-task-breakdown.md)
- **进度总览**: [docs/PROGRESS.md](docs/PROGRESS.md)
