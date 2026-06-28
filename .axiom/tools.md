# 工具边界 MCP/Tools: axiom-builder 可用工具

> 定义我（axiom-builder）可以使用的工具及其权限边界。
> 类比MCP（Model Context Protocol）——这是我的"工具注册表"。

## 工具注册表

### Tool: file-read

**能力**: 读取文件内容
**权限**: 只读
**允许路径**: `d:\work\trae\axiom-core\` 下所有文件
**禁止路径**: 无（项目内文件都可读）
**使用约束**:
- 读取文件前确认路径正确
- 修改文件前必须先Read获取最新内容
- 大文件使用offset/limit分段读取

### Tool: file-write

**能力**: 写入/创建文件
**权限**: 读写
**允许路径**: `d:\work\trae\axiom-core\` 下所有文件
**禁止操作**:
- ❌ 写入项目路径外的文件
- ❌ 覆盖文件前不先读取确认最新内容
- ❌ 写入包含TODO/FIXME占位符的代码
- ❌ 写入编译不通过的代码（写完后必须验证）
**使用约束**:
- Write操作前确认文件存在或需要新建
- 新建.rs文件必须在lib.rs/mod.rs中声明模块
- 每次Write后必须运行编译验证

### Tool: file-edit

**能力**: 精确替换文件中的文本
**权限**: 读写
**约束**:
- old_string必须是文件中唯一存在的精确文本
- Edit前必须Read最新文件内容
- 不使用replace_all除非确实需要全局替换
- 每次Edit不超过50行变更

### Tool: file-delete

**权限**: 仅在plan明确要求时使用
**默认**: ❌ 禁止删除文件
**例外**: plan中明确列出"Delete: path"时才允许

### Tool: command-run

**能力**: 执行终端命令
**权限**: 受限执行
**允许命令**:
- ✅ `cargo build --workspace`
- ✅ `cargo build -p <crate>`
- ✅ `cargo test --workspace`
- ✅ `cargo test -p <crate>`
- ✅ `cargo test -p <crate> <test_name>`
- ✅ `cargo run --example <name> -p <crate>`
- ✅ `cargo fmt`
- ✅ `cargo clippy --workspace -- -D warnings`
- ✅ `cargo doc -p <crate> --no-deps`
- ✅ `cargo tree -p <crate>`
- ✅ `git add <files>`
- ✅ `git commit -m "<message>"`
- ✅ `git push`
- ✅ `git status`
- ✅ `git log -n 3`
- ✅ `New-Item -ItemType Directory -Force -Path <path>`
- ❌ 禁止: `cargo publish`
- ❌ 禁止: `git push --force`
- ❌ 禁止: `rm -rf` / `Remove-Item -Recurse -Force`
- ❌ 禁止: 任何网络请求命令（除gh和cargo）
- ❌ 禁止: 修改系统配置/安装全局软件

### Tool: cargo-check

**能力**: 编译检查
**触发时机**:
- 每次Write/Edit后
- 每次Task完成后
**期望结果**: zero errors, zero warnings

### Tool: cargo-test

**能力**: 运行测试
**触发时机**:
- 每个Task完成后
- 每次功能实现后
**期望结果**: all tests pass

### Tool: gh-cli

**能力**: GitHub CLI操作
**允许操作**:
- ✅ `gh repo create` (仅初始化时)
- ✅ `gh repo view`
- ❌ 禁止: `gh repo delete`
- ❌ 禁止: `gh secret set/remove`
- 环境变量: `GH_TOKEN` 已配置在会话中

### Tool: search-grep

**能力**: 搜索代码
**允许**: 在项目目录内搜索
**约束**: 使用Grep/SearchCodebase工具，不用ripgrep命令行（避免依赖外部工具）

### Tool: web-search/fetch

**能力**: 网络搜索和网页获取
**权限**: 仅用于调研，不用于复制粘贴代码
**约束**:
- 调研文档/API时使用
- 不复制Stack Overflow等外部代码直接使用
- 所有代码自己编写

### Tool: todo-write

**能力**: 任务追踪
**使用**: 每个阶段开始前创建TodoList，追踪进度
**约束**: 不跳过pending任务，顺序执行

## 工具调用铁律

1. **先读后写**: 任何修改文件操作前，必须先Read获取最新内容
2. **写完即验**: Write/Edit后立即运行cargo build/test验证
3. **不碰外部**: 不写项目外文件，不执行未授权命令
4. **小步前进**: 每次只改少量代码，立即验证，避免大规模改动
5. **失败即停**: 编译错误或测试失败时，不继续写新代码，先修复当前问题

## 禁止操作清单（绝对不能做）

- ❌ 不得提交 `cargo build` 有警告的代码
- ❌ 不得提交 `cargo test` 不通过的代码
- ❌ 不得使用 `unwrap()` 在非测试代码中
- ❌ 不得使用 `unsafe` 代码（无SAFETY注释）
- ❌ 不得引入 `async-trait` 依赖
- ❌ 不得添加 `Cargo.lock` 到git（library项目）
- ❌ 不得修改 `docs/architecture/` 下的设计文档（除非用户要求）
- ❌ 不得创建docs/plans外的额外md文件（除非用户要求）
- ❌ 不得在代码中添加注释（除非用户要求或SAFETY注释）
- ❌ 不得使用 `.expect()` 代替错误处理
