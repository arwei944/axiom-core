# P0.5 + P0.6 开发任务书：三层自动化门禁

> 本文档将P0.5（L0开发门禁）和P0.6（L1编译期门禁）拆分为不可再分的最小任务单元，每个任务有严格验收标准。
> 任务按依赖顺序排列，必须逐个完成（前一个通过验收才能开始下一个）。
>
> Spec参考：[03-automation-gates.md](../architecture/03-automation-gates.md)

---

## 任务依赖总览

```
T01: 添加clap/linkme/syn等依赖到workspace
  ↓
T02: 创建axiom-cli crate骨架 + lib.rs + 两个bin入口
  ↓
T03: 实现Check trait + checks/mod.rs框架
  ↓
T04-T13: 实现10个独立check模块（每个可并行但建议串行）
  ↓
T14: 实现commands/preflight（聚合所有checks）
  ↓
T15: 实现commands/check（fmt→build→clippy→test→verify流水线）
  ↓
T16: 实现commands/verify（组合deps_audit+unsafe_audit+constraints_hash+todo_scan）
  ↓
T17: 实现commands/version
  ↓
T18: constraints.lock机制（生成+校验+--update-constraints）
  ↓
T19: CI workflow文件
  ↓
T20: hook脚本模板（版本受控到hooks/目录）
  ↓
T21: axm init命令骨架（安装hooks+生成constraints.lock）
  ↓
T22: axiom-cli集成测试 + 自检通过
  ↓
T23: axiom-macros proc-macro crate骨架
  ↓
T24: Layer类型重构：marker types + Sealed + CanSendTo方向矩阵
  ↓
T25: axiom-core/build.rs（rustc版本+feature检查）
  ↓
T26: 实现#[derive(SignalPayload)]宏 + #[signal(...)]属性
  ↓
T27: 实现#[cell(...)]属性宏
  ↓
T28: 实现#[axiom(...)]属性宏 + AxiomRegistry + linkme收集
  ↓
T29: 实现#[schema_version(N)]属性宏
  ↓
T30: 实现#[migration(from=N)]属性宏 + MigrationRegistry::auto_collect
  ↓
T31: trybuild编译失败测试（覆盖所有compile_error场景）
  ↓
T32: proc macro集成测试（Windows平台验证linkme）
  ↓
T33: 全workspace编译+clippy+测试通过，提交推送
```

---

## P0.5 阶段：L0开发门禁（T01-T22）

### T01：新增workspace依赖

**文件修改**：
- `Cargo.toml`（workspace根）

**具体操作**：
1. 在`[workspace.dependencies]`中添加：
   - `clap = { version = "4", features = ["derive"] }`
   - `sha2`（已有）
   - `serde_json`（已有传递依赖，显式添加）
2. 在workspace members中添加`"crates/axiom-cli"`

**验收标准**：
- [ ] `cargo build --workspace` 零警告编译通过
- [ ] `cargo tree -p axiom-core` 不显示async-trait（已有约束，验证未被引入）
- [ ] 根Cargo.toml中clap版本确定且features正确

---

### T02：创建axiom-cli crate骨架

**新建文件**：
- `crates/axiom-cli/Cargo.toml`
- `crates/axiom-cli/src/lib.rs`
- `crates/axiom-cli/src/bin/axm.rs`
- `crates/axiom-cli/src/bin/cargo-axiom.rs`
- `crates/axiom-cli/src/commands/mod.rs`
- `crates/axiom-cli/src/checks/mod.rs`

**具体操作**：
1. Cargo.toml配置：
   ```toml
   [package]
   name = "axiom-cli"
   version.workspace = true
   edition.workspace = true
   [[bin]]
   name = "axm"
   path = "src/bin/axm.rs"
   [[bin]]
   name = "cargo-axiom"
   path = "src/bin/cargo-axiom.rs"
   [dependencies]
   clap = { workspace = true }
   serde = { workspace = true }
   serde_json = { workspace = true }
   sha2 = { workspace = true }
   ```
2. lib.rs：空（预留pub mod commands; pub mod checks;）
3. commands/mod.rs：定义`Command` trait（`fn name(&self) -> &str; fn run(&self, args: &Cli) -> Result<ExitCode>;`）
4. checks/mod.rs：定义`Check` trait（`fn name(&self) -> &str; fn run(&self) -> CheckResult;`）和`CheckResult`结构体（passed: bool, is_blocking: bool, message: String）
5. bin/axm.rs：clap derive CLI结构体，包含`Preflight`/`Check`/`Verify`/`Version`/`Help`子命令枚举，main函数调用commands::run()
6. bin/cargo-axiom.rs：同axm.rs但clap命令名为"cargo-axiom"，内部调用同样的commands::run()

**验收标准**：
- [ ] `cargo build -p axiom-cli` 编译通过
- [ ] `cargo run --bin axm -- help` 输出帮助信息，列出所有子命令
- [ ] `cargo run --bin cargo-axiom -- help` 输出帮助信息（命令名显示cargo-axiom）
- [ ] 两个bin入口共享lib.rs中的逻辑，无代码重复
- [ ] clap derive使用正确，子命令解析正常

---

### T03：Check trait框架 + checks/mod.rs聚合

**文件修改**：
- `crates/axiom-cli/src/checks/mod.rs`

**具体操作**：
1. 定义`CheckResult`：
   ```rust
   pub struct CheckResult {
       pub name: &'static str,
       pub passed: bool,
       pub blocking: bool,
       pub message: String,
   }
   ```
2. 定义`Check` trait：
   ```rust
   pub trait Check {
       fn name(&self) -> &'static str;
       fn blocking(&self) -> bool;
       fn run(&self) -> CheckResult;
   }
   ```
3. 实现`pub fn run_all_checks(checks: &[&dyn Check]) -> (Vec<CheckResult>, bool)` 运行所有check并聚合结果，返回(结果列表, 是否有blocking failure)
4. 实现结果打印函数：绿色✓/红色✗/黄色⚠，按passed/blocking分类输出

**验收标准**：
- [ ] 一个MockCheck实现Check trait，调用run_all_checks能正确聚合
- [ ] 单元测试覆盖：全部通过/部分失败/blocking failure/non-blocking failure场景
- [ ] 输出格式清晰：passed项绿色✓，blocking failed项红色✗ BLOCKING，non-blocking failed项黄色⚠ WARNING

---

### T04：cargo_fmt检查

**新建文件**：`crates/axiom-cli/src/checks/cargo_fmt.rs`

**具体操作**：
1. 实现`CargoFmtCheck`结构体，impl Check trait
2. run()中调用`cargo fmt --all -- --check`（使用std::process::Command）
3. 检查退出码：0=passed，非0=blocking failure
4. 捕获stdout/stderr，失败时输出diff摘要

**验收标准**：
- [ ] 代码格式正确时，check passed=true
- [ ] 故意格式化错误（如乱缩进）后，check passed=false, blocking=true
- [ ] 单元测试mock std::Command（或通过insta snapshot测试输出格式）

---

### T05：cargo_build检查

**新建文件**：`crates/axiom-cli/src/checks/cargo_build.rs`

**具体操作**：
1. 实现`CargoBuildCheck`，调用`cargo build --workspace`，设置`RUSTFLAGS="-D warnings"`
2. 检查退出码，解析stderr中的warning/error信息
3. blocking=true（编译失败或有warning都阻断）

**验收标准**：
- [ ] 干净代码build passed=true
- [ ] 有warning时代码passed=false, blocking=true（通过注入一个#[allow(dead_code)]的无用函数测试）
- [ ] 有编译错误时代码passed=false, blocking=true

---

### T06：cargo_clippy检查

**新建文件**：`crates/axiom-cli/src/checks/cargo_clippy.rs`

**具体操作**：
1. 实现`CargoClippyCheck`，调用`cargo clippy --workspace -- -D warnings`
2. 解析输出，blocking=true

**验收标准**：
- [ ] 无clippy警告时passed=true
- [ ] 有clippy警告时passed=false, blocking=true

---

### T07：cargo_test检查

**新建文件**：`crates/axiom-cli/src/checks/cargo_test.rs`

**具体操作**：
1. 实现`CargoTestCheck`，调用`cargo test --workspace`
2. 解析输出中的test result信息
3. blocking=true

**验收标准**：
- [ ] 所有测试通过时passed=true
- [ ] 有测试失败时passed=false, blocking=true

---

### T08：constraints_hash检查

**新建文件**：`crates/axiom-cli/src/checks/constraints_hash.rs`

**具体操作**：
1. 定义约束文件列表：
   ```rust
   const CONSTRAINT_FILES: &[&str] = &[
       ".axiom/AGENTS.md",
       ".axiom/identity.md",
       ".axiom/skills.md",
       ".axiom/mcp.md",
       ".axiom/rules/axiom-builder-rules.md",
       ".axiom/preflight.md",
   ];
   ```
2. run()读取每个文件，计算SHA-256 hash，与`.axiom/.constraints.lock`中记录的hash比对
3. 如果.lock文件不存在：passed=true但message提示"运行axm preflight --update-constraints生成锁文件"，non-blocking
4. 如果hash不匹配：blocking failure，显示哪个文件被篡改
5. 如果文件不存在：blocking failure

**验收标准**：
- [ ] 所有文件存在且hash匹配时passed=true
- [ ] 修改一个约束文件内容后，check检测到hash不匹配，passed=false, blocking=true
- [ ] 删除一个约束文件后，passed=false, blocking=true
- [ ] 单元测试验证hash计算正确性

---

### T09：todo_scan检查

**新建文件**：`crates/axiom-cli/src/checks/todo_scan.rs`

**具体操作**：
1. 扫描crates/目录下所有.rs文件
2. grep查找`TODO!`、`FIXME!`、`unimplemented!()`、`todo!()`宏调用
3. 排除`#[cfg(test)]`模块和`tests/`目录、`test/`目录下的文件
4. 排除注释中的匹配（简单处理：行首或`//`后的匹配）
5. 发现任何匹配→blocking failure，输出文件:行号:内容

**验收标准**：
- [ ] 干净代码中passed=true
- [ ] 在非测试代码中加入`todo!()`→passed=false, blocking=true，输出准确位置
- [ ] 在#[test]函数中加入`unimplemented!()`→passed=true（豁免）
- [ ] 在tests/目录文件中加入`unimplemented!()`→passed=true（豁免）

---

### T10：unsafe_audit检查

**新建文件**：`crates/axiom-cli/src/checks/unsafe_audit.rs`

**具体操作**：
1. 扫描crates/目录下所有.rs文件
2. 查找`unsafe`关键字（unsafe fn/unsafe trait/unsafe impl/unsafe块）
3. 每个unsafe出现必须在前3行内有`// SAFETY:`注释（行首，允许空白缩进）
4. 没有SAFETY注释的unsafe→blocking failure，输出文件:行号
5. 注意：unsafe关键字在字符串/注释中不计算（简单行级解析，不做完整AST）

**验收标准**：
- [ ] 无unsafe代码时passed=true
- [ ] unsafe块有SAFETY注释时passed=true
- [ ] unsafe块无SAFETY注释时passed=false, blocking=true
- [ ] 单元测试覆盖各种unsafe场景

---

### T11：deps_audit检查

**新建文件**：`crates/axiom-cli/src/checks/deps_audit.rs`

**具体操作**：
1. 解析根Cargo.toml和所有crate的Cargo.toml
2. 提取所有第三方依赖（非path依赖、非workspace成员）
3. 维护白名单：当前已知安全依赖（tokio/serde/serde_json/sha2/uuid/clap/linkme/proc-macro2/quote/syn/trybuild/tracing等）
4. 发现不在白名单中的依赖→blocking failure，提示需经R-022审计
5. 特别检查`async-trait`依赖：存在→blocking failure

**验收标准**：
- [ ] 当前已知依赖全部在白名单中，passed=true
- [ ] 添加async-trait到某个Cargo.toml→passed=false, blocking=true
- [ ] 添加一个未知crate到Cargo.toml→passed=false, blocking=true

---

### T12：git_status检查（non-blocking）

**新建文件**：`crates/axiom-cli/src/checks/git_status.rs`

**具体操作**：
1. 调用`git status --porcelain`
2. 如果有输出→non-blocking warning（提醒有未提交变更）
3. 如果git命令失败（非git仓库）→non-blocking warning
4. blocking=false（仅警告）

**验收标准**：
- [ ] 干净工作区passed=true
- [ ] 有未暂存变更时passed=false, blocking=false（WARNING）

---

### T13：branch检查（non-blocking）

**新建文件**：`crates/axiom-cli/src/checks/branch.rs`

**具体操作**：
1. 调用`git rev-parse --abbrev-ref HEAD`获取当前分支
2. 如果分支是master或匹配`phase/*`pattern→passed=true
3. 否则→non-blocking warning（建议在phase分支上开发）

**验收标准**：
- [ ] master分支passed=true
- [ ] phase/p0-5分支passed=true
- [ ] feature/random分支passed=false, blocking=false（WARNING）

---

### T14：commands/preflight 实现

**新建/修改文件**：
- `crates/axiom-cli/src/commands/preflight.rs`
- `crates/axiom-cli/src/commands/mod.rs`（注册Preflight子命令）

**具体操作**：
1. 定义Preflight子命令clap结构体：支持`--update-constraints` flag
2. run()逻辑：
   a. 如果`--update-constraints`：计算所有约束文件hash，写入`.axiom/.constraints.lock`
   b. 按顺序运行所有checks：constraints_hash → cargo_build → cargo_clippy → cargo_test → cargo_fmt → unsafe_audit → deps_audit → todo_scan → git_status → branch
   c. 调用run_all_checks聚合结果
   d. 如果有blocking failure→退出码1；如果只有warning→退出码2；全部通过→退出码0
3. 输出汇总：X passed, Y blocking failures, Z warnings

**验收标准**：
- [ ] `axm preflight`在当前干净代码上运行，所有check通过，退出码0
- [ ] `axm preflight --update-constraints`生成.constraints.lock文件，包含6个文件的hash
- [ ] 注入clippy警告后axm preflight退出码1
- [ ] 有未提交文件时axm preflight退出码2（如果无blocking failure）
- [ ] 输出格式清晰，有passed/failed/warning统计

---

### T15：commands/check 实现

**新建/修改文件**：
- `crates/axiom-cli/src/commands/check.rs`
- `crates/axiom-cli/src/commands/mod.rs`（注册Check子命令）

**具体操作**：
1. 定义Check子命令：无必需参数，支持`--staged` flag（P0.5阶段--staged可以只做fmt+check+test不跑verify）
2. run()逻辑：按顺序执行 fmt→build→clippy→test→verify，前一步失败则停止（不执行后续步骤）
3. fmt和build/clippy/test复用checks模块中的逻辑
4. verify调用verify命令的run()
5. 任何步骤失败→退出码1；全部通过→退出码0

**验收标准**：
- [ ] `axm check`按fmt→build→clippy→test→verify顺序执行
- [ ] fmt失败时不执行后续步骤（fail-fast）
- [ ] 全部通过时退出码0
- [ ] 任何步骤失败时退出码1，并输出哪个步骤失败

---

### T16：commands/verify 实现

**新建/修改文件**：
- `crates/axiom-cli/src/commands/verify.rs`
- `crates/axiom-cli/src/commands/mod.rs`（注册Verify子命令）

**具体操作**：
1. 定义Verify子命令，支持flags：
   - `--unsafe-audit`：仅运行unsafe审计
   - `--deps-audit`：仅运行依赖审计
   - 无flags时运行所有静态检查（unsafe+deps+constraints_hash+todo_scan）
2. run()根据flags选择checks运行
3. blocking failure→退出码1；通过→退出码0

**验收标准**：
- [ ] `axm verify`（无flags）运行所有静态checks，退出码0
- [ ] `axm verify --unsafe-audit`仅运行unsafe_audit
- [ ] `axm verify --deps-audit`仅运行deps_audit
- [ ] CI中调用`cargo axiom verify --unsafe-audit`等单flag命令正常工作

---

### T17：commands/version 实现

**新建/修改文件**：
- `crates/axiom-cli/src/commands/version.rs`
- `crates/axiom-cli/src/commands/mod.rs`（注册Version子命令）

**具体操作**：
1. 定义Version子命令，支持`--check` flag（检查版本兼容性，P1后有运行时才真正检查，P0.5只输出信息）
2. run()读取workspace Cargo.toml，输出：
   - axiom-cli版本
   - axiom-core版本（从crates/axiom-core/Cargo.toml读取）
   - 列出所有workspace member crate的版本
   - Schema版本范围（从axiom-core的version模块读取常量，需要先pub use）
3. 输出格式：机器可读+人类可读（默认文本格式，--json flag可输出JSON）

**验收标准**：
- [ ] `axm version`输出格式清晰，包含所有crate版本
- [ ] 版本号与Cargo.toml一致
- [ ] `axm version --json`输出有效JSON（供CI解析）

---

### T18：constraints.lock机制集成

**修改文件**：
- `crates/axiom-cli/src/checks/constraints_hash.rs`
- `crates/axiom-cli/src/commands/preflight.rs`

**具体操作**：
1. 定义.lock文件格式（简单文本，每行`路径 = sha256:hexhash`）
2. 实现`--update-constraints`：计算hash写入文件
3. 首次运行preflight如果.lock不存在：提示运行`--update-constraints`，但不阻断（non-blocking）
4. .lock文件存在后，hash不匹配则blocking
5. .lock文件本身加入.gitignore？——不，应该版本受控（约束文件的hash快照应提交到git，防止篡改）

**验收标准**：
- [ ] .constraints.lock文件格式正确，每行一个文件hash
- [ ] `--update-constraints`正确生成/更新.lock
- [ ] lock文件不存在时给出明确提示
- [ ] hash不匹配时blocking，指出具体哪个文件
- [ ] .constraints.lock被git跟踪（不在.gitignore中）

---

### T19：CI/CD GitHub Actions workflow

**新建文件**：`.github/workflows/ci.yml`

**具体操作**：
1. 按spec中定义创建ci.yml
2. 触发条件：push到master + pull_request
3. 步骤：checkout→rust-toolchain(clippy+rustfmt)→build(-D warnings)→fmt check→clippy(-D warnings)→test→verify→unsafe-audit→deps-audit→version check
4. 使用dtolnay/rust-toolchain@stable action
5. 注意：CI中axiom-cli需要先install（`cargo install --path crates/axiom-cli`或直接用`cargo run --bin cargo-axiom`）

**验收标准**：
- [ ] 文件存在于`.github/workflows/ci.yml`
- [ ] YAML语法正确（可通过yamllint或GitHub parser验证）
- [ ] 包含所有8个检查步骤
- [ ] push到master和PR都触发
- [ ] workflow在GitHub上可运行（push一次验证CI实际触发）

---

### T20：Git hook脚本模板

**新建文件**：
- `hooks/pre-commit`
- `hooks/pre-push`
- `hooks/install-hooks.sh`

**具体操作**：
1. pre-commit脚本：
   ```sh
   #!/bin/sh
   set -e
   axm check
   axm preflight
   ```
2. pre-push脚本：
   ```sh
   #!/bin/sh
   set -e
   axm check
   axm verify
   ```
3. install-hooks.sh：复制hooks/pre-commit和hooks/pre-push到`.git/hooks/`，设置可执行权限
4. 所有脚本使用POSIX sh（不依赖bash特性），Windows兼容（Git Bash可执行）

**验收标准**：
- [ ] hooks/pre-commit和hooks/pre-push存在，有执行权限（chmod +x后）
- [ ] install-hooks.sh能正确复制到.git/hooks/
- [ ] 脚本内容简洁，调用axm命令
- [ ] 在Git Bash on Windows上可执行

---

### T21：axm init命令

**修改文件**：`crates/axiom-cli/src/commands/`（添加init.rs）

**具体操作**：
1. 定义Init子命令
2. run()逻辑：
   a. 检查当前目录是否已有Cargo.toml和.axiom/目录
   b. 运行hooks/install-hooks.sh（或直接复制hook文件）
   c. 运行`axm preflight --update-constraints`生成.constraints.lock
   d. 输出"Initialized axiom project"
3. 幂等：重复运行不报错，只更新hooks和lock文件

**验收标准**：
- [ ] `axm init`在项目根目录运行成功
- [ ] 运行后.git/hooks/pre-commit存在且包含axm check调用
- [ ] .constraints.lock被生成
- [ ] 重复运行axm init不报错
- [ ] 注：在已初始化项目中运行axm init是幂等操作

---

### T22：axiom-cli集成测试 + 自验证

**新建目录/文件**：
- `crates/axiom-cli/tests/`
- `crates/axiom-cli/tests/cli_tests.rs`

**具体操作**：
1. 集成测试：
   a. 测试`axm version`退出码0
   b. 测试`axm help`列出所有子命令
   c. 测试`axm preflight`在当前workspace通过
   d. 测试`axm check`在当前workspace通过
   e. 测试`axm verify`在当前workspace通过
2. 运行完整自验证：
   ```
   cargo build -p axiom-cli
   cargo test -p axiom-cli
   cargo clippy -p axiom-cli -- -D warnings
   ```
3. 确保axiom-cli自身也通过自己定义的门禁检查（吃自己的狗粮）

**验收标准**：
- [ ] `cargo test -p axiom-cli` 全部通过
- [ ] `cargo clippy -p axiom-cli -- -D warnings` 零警告
- [ ] `axm check`在整个workspace上运行通过（包含axiom-cli自己）
- [ ] `axm preflight`在整个workspace上运行通过，退出码0
- [ ] 集成测试覆盖主要命令路径

---

## P0.6阶段：L1编译期门禁（T23-T32）

### T23：axiom-macros proc-macro crate骨架

**新建/修改文件**：
- `crates/axiom-macros/Cargo.toml`
- `crates/axiom-macros/src/lib.rs`

**具体操作**：
1. Cargo.toml配置proc-macro crate：
   ```toml
   [package]
   name = "axiom-macros"
   version.workspace = true
   edition.workspace = true
   [lib]
   proc-macro = true
   [dependencies]
   proc-macro2 = "1"
   quote = "1"
   syn = { version = "2", features = ["full", "extra-traits", "derive"] }
   [dev-dependencies]
   trybuild = "1"
   axiom-core = { path = "../axiom-core" }
   ```
2. lib.rs导出所有宏入口（先空实现，后续任务逐个填充）：
   ```rust
   extern crate proc_macro;
   // 宏在后续任务中实现
   ```
3. 将axiom-macros添加到workspace members
4. axiom-core的Cargo.toml添加axiom-macros依赖：
   ```toml
   axiom-macros = { path = "../axiom-macros", optional = true }
   ```
   添加feature `"macros" = ["dep:axiom-macros", "dep:linkme"]`

**验收标准**：
- [ ] `cargo build -p axiom-macros` 编译通过
- [ ] `cargo build -p axiom-core --features macros` 编译通过
- [ ] proc-macro crate类型正确（cargo check确认）

---

### T24：Layer类型重构：marker types + Sealed + CanSendTo

**修改文件**：
- `crates/axiom-core/src/layer.rs`（或新建）
- `crates/axiom-core/src/lib.rs`

**具体操作**：
1. 定义零大小marker types：
   ```rust
   pub struct OversightLayer;
   pub struct ExecLayer;
   pub struct ValidateLayer;
   pub struct AgentLayer;
   ```
2. 定义Sealed trait（私有mod）：
   ```rust
   mod sealed {
       pub trait Sealed {}
       impl Sealed for super::OversightLayer {}
       impl Sealed for super::ExecLayer {}
       impl Sealed for super::ValidateLayer {}
       impl Sealed for super::AgentLayer {}
   }
   pub trait LayerMarker: sealed::Sealed {
       fn layer() -> Layer;
   }
   ```
3. 为每个marker type impl LayerMarker
4. 定义CanSendTo trait和合法方向impl（严格按spec方向矩阵）
5. 保留原有的Layer enum（用于runtime），提供从marker type到Layer enum的转换

**验收标准**：
- [ ] 四个marker types存在
- [ ] Sealed trait阻止外部crate impl LayerMarker
- [ ] CanSendTo impl严格匹配spec中的方向矩阵（11个合法方向）
- [ ] 非法方向没有CanSendTo impl（如Exec→Oversight没有impl）
- [ ] `cargo build -p axiom-core` 编译通过
- [ ] 单元测试验证方向矩阵（使用static assertions或trait bound测试）

---

### T25：axiom-core/build.rs

**新建文件**：`crates/axiom-core/build.rs`

**具体操作**：
1. 检查rustc版本：解析rustc --version输出，确认>=1.75.0
2. 如果版本不够，panic with明确错误信息："axiom-core requires Rust >=1.75 for async fn in traits support"
3. 可选：检查互斥feature组合（暂时不做，等有feature组合再添加）

**验收标准**：
- [ ] build.rs存在
- [ ] 使用当前rustc（>=1.75）编译正常通过
- [ ] 可通过手动修改版本号模拟低版本场景验证错误输出

---

### T26：#[derive(SignalPayload)] + #[signal(...)]宏

**新建/修改文件**：
- `crates/axiom-macros/src/lib.rs`
- `crates/axiom-macros/src/signal_payload.rs`（新建，derive宏实现）
- `crates/axiom-core/src/signal.rs`（重构：添加SignalPayload trait，SignalEnvelope）

**具体操作**：
1. 在axiom-core/src/signal.rs中定义：
   ```rust
   pub trait SignalPayload: Serialize + DeserializeOwned + Send + Sync + Clone + Debug + 'static {
       fn source_layer() -> Layer;
       fn target_layer() -> Layer;
       fn schema_version() -> SchemaVersion;
       fn signal_type() -> &'static str { std::any::type_name::<Self>() }
   }

   #[derive(Debug, Clone)]
   pub struct SignalEnvelope<T: SignalPayload> {
       pub msg_id: MsgId,
       pub correlation_id: CorrelationId,
       pub reply_to: Option<MsgId>,
       pub source_layer: Layer,
       pub target_layer: Layer,
       pub hop_count: u8,
       pub schema_version: SchemaVersion,
       pub payload: T,
   }
   ```
2. SignalEnvelope实现自动wrap/unwrap方法
3. proc macro解析`#[signal(source = "exec", target = "validate")]`属性
4. 解析`#[schema_version(N)]`属性（如果在struct上也存在）
5. 生成SignalPayload impl
6. 编译期校验：
   - source/target值必须是"oversight"/"exec"/"validate"/"agent"之一
   - 方向必须合法（通过CanSendTo矩阵在编译期检查，宏中直接匹配字符串）
   - 非法方向输出compile_error!，信息包含合法方向提示

**验收标准**：
- [ ] 使用`#[derive(SignalPayload)]`+`#[signal(source="exec", target="validate")]`标记的struct编译通过
- [ ] 生成的impl正确返回source_layer/target_layer/schema_version
- [ ] `#[signal(source="invalid")]`→编译错误，提示合法值
- [ ] `#[signal(source="exec", target="oversight")]`→编译错误，信息包含"use event emission instead"
- [ ] SignalEnvelope能正确包装/解包payload
- [ ] 单元测试验证宏展开正确（使用trybuild测试compile-fail场景）
- [ ] `cargo test -p axiom-core`通过

---

### T27：#[cell(...)]属性宏

**新建文件**：`crates/axiom-macros/src/cell.rs`
**修改文件**：`crates/axiom-core/src/cell.rs`

**具体操作**：
1. 在axiom-core中确保Cell trait和各层marker trait（ExecCell/ValidateCell/AgentCell/OversightCell）定义正确
2. proc macro解析`#[cell(layer = "exec")]`属性
3. 为struct生成Cell trait impl（state_hash默认返回None，其他方法要求用户手动impl handle_message等）
4. 根据layer值生成对应的marker trait impl
5. 编译期校验：layer值必须是四个合法值之一，否则compile_error!
6. 注意：Cell trait的handle_message方法仍然需要用户手动impl，宏只生成trait的marker部分和默认方法

**验收标准**：
- [ ] `#[cell(layer = "exec")]`标记的struct自动impl Cell和ExecCell
- [ ] 该struct可在ExecCellContext中使用（通过编译）
- [ ] `#[cell(layer = "invalid")]`→编译错误
- [ ] `#[cell(layer = "validate")]`→impl ValidateCell
- [ ] 用户仍需手动impl handle_message等业务方法
- [ ] trybuild测试覆盖非法layer值

---

### T28：#[axiom(...)]属性宏 + AxiomRegistry + linkme自动收集

**新建/修改文件**：
- `crates/axiom-macros/src/axiom.rs`（宏实现）
- `crates/axiom-core/src/axiom.rs`（添加AxiomRegistry、AxiomRegistrar trait）
- `Cargo.toml`（workspace根和axiom-core添加linkme依赖）

**具体操作**：
1. 在axiom-core中添加linkme依赖（feature-gated by "macros" feature）
2. 定义分布式slice：
   ```rust
   #[linkme::distributed_slice]
   pub static AXIOM_REGISTRY: [fn() -> Box<dyn AxiomRule>] = [..];
   ```
3. 定义AxiomRegistry::collect()方法，遍历AXIOM_REGISTRY收集所有Axiom
4. proc macro解析`#[axiom(layer = "validate", action = "reject")]`
5. 为标注类型生成一个注册函数，通过linkme::distributed_slice提交
6. 编译期校验：layer值合法，action值合法（"reject"/"warn"/"correct"）

**验收标准**：
- [ ] `#[axiom(layer="validate", action="reject")]`标注的struct被AXIOM_REGISTRY收集
- [ ] AxiomRegistry::collect()返回包含该axiom的Vec
- [ ] 无需手动push到AxiomChain（或AxiomChain内部使用collect结果）
- [ ] `#[axiom(layer="invalid")]`→编译错误
- [ ] 在Windows上linkme收集正常工作（cargo test验证）
- [ ] 单元测试：定义两个axiom，collect返回2个

---

### T29：#[schema_version(N)]属性宏

**新建/修改文件**：
- `crates/axiom-macros/src/schema_version.rs`
- `crates/axiom-core/src/version.rs`（确保Versioned trait定义正确）

**具体操作**：
1. proc macro解析`#[schema_version(N)]`属性
2. 为struct生成Versioned trait impl
3. 支持可选`min = M`参数：`#[schema_version(2, min = 1)]`
4. 编译期校验：N和M必须是u16字面量，N > 0，M <= N，否则compile_error!
5. 注意：此宏可与#[derive(SignalPayload)]组合使用（derive宏内部也能读取schema_version属性）

**验收标准**：
- [ ] `#[schema_version(2)]`生成Versioned impl，schema_version()返回2
- [ ] `#[schema_version(2, min = 1)]`设置min_supported_version=1
- [ ] `#[schema_version(0)]`→编译错误（版本号从1开始）
- [ ] `#[schema_version(2, min = 3)]`→编译错误（min不能大于version）
- [ ] 与#[derive(SignalPayload)]组合使用正常
- [ ] trybuild测试覆盖非法参数

---

### T30：#[migration(from=N)]属性宏 + MigrationRegistry::auto_collect

**新建/修改文件**：
- `crates/axiom-macros/src/migration.rs`
- `crates/axiom-core/src/version.rs`（MigrationRegistry添加auto_collect和verify_all_chains）

**具体操作**：
1. 在version.rs中添加MIGRATION_REGISTRY分布式slice
2. 添加MigrationRegistrar trait
3. proc macro解析`#[migration(from = N)]`属性
4. 计算to = N+1，生成注册函数
5. 编译期校验：
   - N必须是u16字面量
   - 用户的Migration impl中，source_version()必须返回SchemaVersion(N)，target_version()必须返回SchemaVersion(N+1)
   - 注意：proc macro无法直接检查impl方法的返回值，所以这里通过以下方式：
     a. 宏为struct生成一个const验证（通过const fn或类型层面trick）
     b. 或者：宏生成的注册函数创建Migration实例，调用source_version()/target_version()在运行时验证（但这是runtime check不是compile check）
     c. 最佳方案：宏要求用户不要手动impl source_version/target_version，而是由宏生成这两个方法（通过宏在impl块中追加方法）。但attribute macro on struct无法直接修改impl块。
     d. 修正方案：`#[migration(from = N)]`放在impl Migration块上而非struct上，宏替换source_version/target_version的返回值。
6. 简化方案：`#[migration(from = N)]`放在struct定义上，宏为struct自动impl Migration的source_version()和target_version()，用户只需实现migrate()方法。

最终设计：
```rust
#[migration(from = 1)]
struct MigrateV1toV2;
// 用户只写：
impl MigrateV1toV2 {
    fn migrate(&self, data: Value) -> Result<Value> { ... }
}
// 宏自动生成：
// impl Migration for MigrateV1toV2 {
//     fn source_version(&self) -> SchemaVersion { SchemaVersion(1) }
//     fn target_version(&self) -> SchemaVersion { SchemaVersion(2) }
//     fn migrate(&self, data: Value) -> Result<Value> { self.migrate(data) }  // 委托给用户的inherent impl
// }
```
Wait——这会有冲突。更好的方案：宏放在impl块上。

最终最终方案：使用struct级attribute，要求struct有一个`fn do_migrate(&self, data: Value) -> Result<Value>`方法（特定名称），宏生成Migration trait impl并委托到该方法。

7. MigrationRegistry添加auto_collect()和verify_all_chains()方法：
   - auto_collect()：从MIGRATION_REGISTRY收集所有迁移，注册
   - verify_all_chains()：验证所有类型的迁移链无gap

**验收标准**：
- [ ] `#[migration(from=1)]`标注的struct被MIGRATION_REGISTRY收集
- [ ] auto_collect()后迁移被正确注册
- [ ] `#[migration(from=0)]`→编译错误（版本从1开始）
- [ ] verify_all_chains()在有gap时返回Err(MigrationChainGap)
- [ ] 链式迁移v1→v2→v3正确注册和验证
- [ ] 单元测试覆盖正常路径和gap检测

---

### T31：trybuild编译失败测试

**新建目录/文件**：
- `crates/axiom-macros/tests/`
- `crates/axiom-macros/tests/compile-fail/`（目录）
- `crates/axiom-macros/tests/trybuild.rs`

**具体操作**：
1. 添加trybuild dev-dependency
2. 创建tests/trybuild.rs：
   ```rust
   #[test]
   fn compile_fail_tests() {
       let t = trybuild::TestCases::new();
       t.compile_fail("tests/compile-fail/*.rs");
       t.pass("tests/pass/*.rs");
   }
   ```
3. 创建tests/pass/目录，放置合法用法测试用例
4. 创建tests/compile-fail/目录，放置非法用法，每个.stderr文件包含期望的编译错误信息
5. 覆盖的compile-fail场景：
   - `cf-invalid-layer.rs`：非法层名
   - `cf-invalid-direction.rs`：非法方向Exec→Oversight
   - `cf-invalid-cell-layer.rs`：cell宏非法layer值
   - `cf-invalid-axiom-layer.rs`：axiom宏非法layer值
   - `cf-invalid-schema-version.rs`：schema_version=0或min>version
   - `cf-invalid-migration-from.rs`：migration from=0

**验收标准**：
- [ ] `cargo test -p axiom-macros` 全部通过（包括trybuild测试）
- [ ] 每个compile-fail测试确实产生编译错误，错误信息包含指定关键词
- [ ] pass测试全部编译通过
- [ ] trybuild测试覆盖所有compile_error!分支

---

### T32：proc macro集成测试 + Windows验证

**新建文件**：`crates/axiom-macros/tests/integration.rs`

**具体操作**：
1. 端到端测试：
   - 定义一个使用所有宏的"迷你应用"（一个SignalPayload+一个Cell+一个Axiom+一个Migration）
   - 验证SignalPayload impl正确
   - 验证Cell impl正确
   - 验证Axiom被collect发现
   - 验证Migration被auto_collect发现并可执行
   - 验证CanSendTo方向检查在trait bound层面工作
2. 在Windows平台运行cargo test确认linkme工作正常
3. 确保axiom-core启用"macros" feature后所有功能正常

**验收标准**：
- [ ] 端到端集成测试通过
- [ ] linkme分布式收集在Windows上正常工作
- [ ] `cargo test --workspace` 全部通过
- [ ] `cargo clippy --workspace -- -D warnings` 零警告
- [ ] `axm check`在整个workspace通过（P0.5已完成，此时axiom-cli应已可用）

---

## P0.5+P0.6 最终验收（T33）

### T33：全workspace最终验证 + 提交

**具体操作**：
1. 运行完整门禁链：
   ```
   cargo build --workspace
   cargo fmt --all -- --check
   cargo clippy --workspace -- -D warnings
   cargo test --workspace
   axm check
   axm preflight
   axm verify
   ```
2. 确保CI workflow配置正确（push一次到分支验证CI触发）
3. 更新roadmap文档，标记P0.5和P0.6的实际完成情况
4. 更新AGENTS.md或相关文档说明新的宏和CLI使用方式
5. 提交所有变更

**验收标准**：
- [ ] 以上所有命令退出码0
- [ ] CI在实际push后绿灯
- [ ] .constraints.lock文件生成并提交
- [ ] hooks/目录版本受控
- [ ] 文档更新（roadmap+任何必要的使用说明）
- [ ] `axm version`正确显示所有crate版本
- [ ] P0.5专项验收标准（spec 6.2）13项全部通过
- [ ] P0.6专项验收标准（spec 6.3）12项全部通过

---

## 任务统计

| Phase | 任务数 | 预计主要工作 |
|-------|--------|-------------|
| P0.5 (L0开发门禁) | T01-T22 | 22个任务，CLI框架+10个check模块+4个command+CI+hooks |
| P0.6 (L1编译期门禁) | T23-T32 | 10个任务，proc macro crate+5个宏+trybuild+linkme+Sealed trait |
| 最终验收 | T33 | 1个任务，全链路验证+提交 |
| **合计** | **33个任务** | 每个任务不可再分，有明确验收标准 |

每个任务开始前必须：
1. 运行`axm preflight`确认当前工作区干净
2. 在正确的phase分支上工作（`phase/p0-5-gates`或`phase/p0-6-macros`）
3. 理解该任务的验收标准，实现后立即运行验收检查
4. 不破坏已有测试和clippy检查
