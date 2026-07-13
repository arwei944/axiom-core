# Axiom Core 插件开发指南

本指南面向插件开发者，帮助你理解 Axiom Core 插件系统、开发 WASM 和 Native 插件、打包分发插件。

---

## 目录

- [插件系统概述](#插件系统概述)
- [WASM 插件开发](#wasm-插件开发)
- [Native 插件开发](#native-插件开发)
- [插件清单文件](#插件清单文件manifest)
- [插件打包和分发](#插件打包和分发)
- [安全沙箱限制](#安全沙箱限制)

---

## 插件系统概述

Axiom Core v0.4.0 插件系统支持运行时动态加载两种类型的插件：**WASM 插件**和 **Native 插件**。

### 两种插件类型对比

| 特性 | WASM 插件 | Native 插件 |
|------|----------|------------|
| 文件格式 | `.wasm` | `.so` / `.dll` / `.dylib` |
| 运行环境 | wasmtime 沙箱 | 操作系统原生进程 |
| 隔离性 | 完全沙箱隔离 | 共享进程地址空间 |
| 性能 | 接近原生（有少量开销） | 原生性能 |
| 安全性 | 高（沙箱限制） | 需信任来源 |
| 多语言支持 | Rust/TS/Go 等可编译为 WASM | Rust/C/C++ 等 |
| 适用场景 | 不受信任的第三方插件 | 受信任的高性能插件 |

### 架构

```
┌──────────────────────────────────────────────────┐
│                  PluginRegistry                   │
│  ┌────────────────────────────────────────────┐  │
│  │              PluginLoader                  │  │
│  │  ┌───────────────┐  ┌──────────────────┐  │  │
│  │  │  WASM Loader   │  │  Native Loader   │  │  │
│  │  │ (wasmtime)     │  │ (libloading)     │  │  │
│  │  └───────┬───────┘  └────────┬─────────┘  │  │
│  └──────────┼────────────────────┼────────────┘  │
│             │                    │               │
│             ▼                    ▼               │
│  ┌──────────────────┐  ┌────────────────────┐   │
│  │  WASM Plugin     │  │  Native Plugin     │   │
│  │  (.wasm)         │  │  (.so/.dll)        │   │
│  │  sandboxed        │  │  full access       │   │
│  └──────────────────┘  └────────────────────┘   │
└──────────────────────────────────────────────────┘
```

### 核心 Trait

所有插件都实现 `AxiomPlugin` trait（定义在 `crates/axiom-kernel/src/plugin/abi.rs`）：

```rust
pub trait AxiomPlugin: Send + Sync {
    fn id(&self) -> &'static str;
    fn version(&self) -> &'static str;
    fn dependencies(&self) -> &[&'static str];
    fn capabilities(&self) -> &[CapabilityDescriptor];
    fn init(&mut self, ctx: PluginContext) -> PluginResult<()>;
    fn start(&mut self) -> PluginResult<()> { Ok(()) }
    fn stop(&mut self) -> PluginResult<()> { Ok(()) }
    fn handle_message(&mut self, msg: PluginMessage) -> PluginResult<PluginReply>;
    fn clone_box(&self) -> Box<dyn AxiomPlugin>;
}
```

### 插件消息类型

插件通过 `PluginMessage` 接收请求，通过 `PluginReply` 返回结果：

```rust
pub enum PluginMessage {
    CallTool { tool: String, input: Vec<u8> },
    QueryMemory { key: String },
    SendSignal { signal: String, payload: Vec<u8> },
    CheckAxiom { axiom: String, state: Vec<u8> },
    QueryLens { lens: String, state: Vec<u8> },
    Custom { kind: String, payload: Vec<u8> },
}

pub enum PluginReply {
    Ok(Vec<u8>),
    Err(String),
}
```

---

## WASM 插件开发

### SDK 使用

WASM 插件使用 `axiom-plugin-wasm-sdk` 进行开发，该 SDK 位于 `crates/axiom-plugin-wasm-sdk/`。

### 最小示例代码

以下是一个完整的 Echo 插件示例（参考 `crates/axiom-plugin-example-wasm/src/echo.rs`）：

**第一步：创建 Cargo.toml**

```toml
[package]
name = "my-echo-plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["rlib", "cdylib"]    # 必须包含 cdylib 以生成 .wasm 文件

[dependencies]
axiom-plugin-wasm-sdk = { path = "../../crates/axiom-plugin-wasm-sdk" }
axiom-kernel = { path = "../../crates/axiom-kernel" }
parking_lot = { workspace = true }
```

> **注意**：`crate-type` 必须包含 `"cdylib"`，否则无法编译为 `.wasm` 文件。

**第二步：实现插件**

```rust
// src/lib.rs
use axiom_kernel::plugin::abi::{AxiomPlugin, PluginContext, PluginMessage, PluginReply};

#[derive(Default)]
pub struct EchoPlugin;

impl AxiomPlugin for EchoPlugin {
    fn id(&self) -> &'static str {
        "echo"
    }

    fn version(&self) -> &'static str {
        "0.1.0"
    }

    fn dependencies(&self) -> &[&'static str] {
        &[]  // 无依赖
    }

    fn capabilities(&self) -> &[axiom_kernel::plugin::abi::CapabilityDescriptor] {
        &[]  // 无特殊能力声明
    }

    fn init(&mut self, _ctx: PluginContext) -> axiom_kernel::plugin::abi::PluginResult<()> {
        // 初始化逻辑（如果需要）
        Ok(())
    }

    fn handle_message(
        &mut self,
        msg: PluginMessage,
    ) -> axiom_kernel::plugin::abi::PluginResult<PluginReply> {
        // 提取 payload
        let payload = match msg {
            PluginMessage::Custom { payload, .. } => payload,
            _ => Vec::new(),
        };
        // 原样返回
        Ok(PluginReply::Ok(payload))
    }

    fn clone_box(&self) -> Box<dyn AxiomPlugin> {
        Box::new(EchoPlugin)
    }
}
```

**第三步：导出 C ABI 入口**

使用 `axiom_wasm_plugin!` 宏自动生成 WASM 导出函数：

```rust
// src/lib.rs（续）
axiom_plugin_wasm_sdk::axiom_wasm_plugin!(EchoPlugin);
```

该宏会生成以下导出函数（见 `crates/axiom-plugin-wasm-sdk/src/lib.rs`）：

- `axiom_plugin_create()` — 创建插件实例，返回指针
- `axiom_plugin_destroy(ptr)` — 销毁插件实例
- `axiom_plugin_handle_message(ptr, msg_ptr, msg_len)` — 处理消息

### 带状态的插件示例

以下是一个带计数器状态的插件示例（参考 `crates/axiom-plugin-example-wasm/src/counter.rs`）：

```rust
use axiom_kernel::plugin::abi::{AxiomPlugin, PluginContext, PluginMessage, PluginReply};
use parking_lot::Mutex;

#[derive(Default)]
pub struct CounterPlugin {
    count: Mutex<u64>,
}

impl AxiomPlugin for CounterPlugin {
    fn id(&self) -> &'static str {
        "counter"
    }

    fn version(&self) -> &'static str {
        "0.1.0"
    }

    fn dependencies(&self) -> &[&'static str] {
        &[]
    }

    fn capabilities(&self) -> &[axiom_kernel::plugin::abi::CapabilityDescriptor] {
        &[]
    }

    fn init(&mut self, _ctx: PluginContext) -> axiom_kernel::plugin::abi::PluginResult<()> {
        Ok(())
    }

    fn handle_message(
        &mut self,
        _msg: PluginMessage,
    ) -> axiom_kernel::plugin::abi::PluginResult<PluginReply> {
        let mut count = self.count.lock();
        *count += 1;
        Ok(PluginReply::Ok(count.to_string().into_bytes()))
    }

    fn clone_box(&self) -> Box<dyn AxiomPlugin> {
        // 注意：克隆时需要复制状态
        Box::new(CounterPlugin {
            count: Mutex::new(*self.count.lock()),
        })
    }
}

axiom_plugin_wasm_sdk::axiom_wasm_plugin!(CounterPlugin);
```

### 编译和部署

**编译为 WASM**：

```bash
# 添加 wasm32 target
rustup target add wasm32-unknown-unknown

# 编译
cargo build --target wasm32-unknown-unknown --release

# 产物位于
# target/wasm32-unknown-unknown/release/my_echo_plugin.wasm
```

**部署到 Runtime**：

```rust
use axiom_kernel::plugin::{PluginRegistry, WasmPluginLoader};

let registry = PluginRegistry::new();
let loader = WasmPluginLoader::new();

// 加载 WASM 插件
let plugin = loader.load(std::path::Path::new("my_echo_plugin.wasm"))?;
registry.register(plugin).await;

// 使用插件
let mut instance = registry.get("echo").await.unwrap();
let reply = instance.handle_message(PluginMessage::Custom {
    kind: "echo".to_string(),
    payload: b"hello".to_vec(),
})?;
```

> **注意**：WASM 加载器需要启用 `wasm-loader` feature：`axiom-kernel = { features = ["wasm-loader"] }`

---

## Native 插件开发

Native 插件以动态库形式加载，提供完整系统访问能力和原生性能。

### ABI 接口说明

Native 插件必须导出以下 C ABI 函数（见 `crates/axiom-kernel/src/plugin/loader/native.rs`）：

| 函数签名 | 说明 |
|---------|------|
| `axiom_plugin_create() -> *mut dyn AxiomPlugin` | 创建插件实例 |
| `axiom_plugin_destroy(ptr: *mut dyn AxiomPlugin)` | 销毁插件实例（可选） |

Native 加载器使用 `libloading` 库在运行时加载动态库并查找符号。

### 动态库加载机制

```
NativePluginLoader::load(path)
    │
    ├── Library::new(path)                    # 加载 .so/.dll/.dylib
    ├── lib.get(b"axiom_plugin_create")       # 查找 create 符号
    ├── create()                              # 调用 create 获取插件指针
    └── Box::from_raw(ptr)                   # 将裸指针转为 Box<dyn AxiomPlugin>
```

### 示例代码

**第一步：创建 Cargo.toml**

```toml
[package]
name = "my-native-plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]    # 必须为 cdylib

[dependencies]
axiom-kernel = { path = "../../crates/axiom-kernel" }
```

**第二步：实现插件并导出入口**

```rust
// src/lib.rs
use axiom_kernel::plugin::abi::{
    AxiomPlugin, CapabilityDescriptor, PluginContext, PluginError, PluginMessage, PluginReply,
};

pub struct MyNativePlugin {
    name: String,
}

impl Default for MyNativePlugin {
    fn default() -> Self {
        Self { name: "native-greeter".to_string() }
    }
}

impl AxiomPlugin for MyNativePlugin {
    fn id(&self) -> &'static str {
        "native-greeter"
    }

    fn version(&self) -> &'static str {
        "0.1.0"
    }

    fn dependencies(&self) -> &[&'static str] {
        &[]
    }

    fn capabilities(&self) -> &[CapabilityDescriptor] {
        &[]
    }

    fn init(&mut self, _ctx: PluginContext) -> Result<(), PluginError> {
        println!("Native plugin {} initialized", self.name);
        Ok(())
    }

    fn handle_message(&mut self, msg: PluginMessage) -> Result<PluginReply, PluginError> {
        match msg {
            PluginMessage::SendSignal { signal, payload } => {
                let text = String::from_utf8_lossy(&payload);
                Ok(PluginReply::Ok(format!("{}: {}", signal, text).into_bytes()))
            }
            _ => Ok(PluginReply::Ok(Vec::new())),
        }
    }

    fn clone_box(&self) -> Box<dyn AxiomPlugin> {
        Box::new(MyNativePlugin { name: self.name.clone() })
    }
}

// 导出 C ABI 入口函数
#[no_mangle]
pub extern "C" fn axiom_plugin_create() -> *mut dyn AxiomPlugin {
    let plugin = MyNativePlugin::default();
    Box::into_raw(Box::new(plugin)) as *mut dyn AxiomPlugin
}

#[no_mangle]
pub extern "C" fn axiom_plugin_destroy(ptr: *mut dyn AxiomPlugin) {
    if !ptr.is_null() {
        // SAFETY: ptr was allocated by axiom_plugin_create using Box::into_raw
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}
```

**第三步：编译和加载**

```bash
# 编译
cargo build --release

# 产物：
# Linux:   target/release/libmy_native_plugin.so
# macOS:   target/release/libmy_native_plugin.dylib
# Windows: target/release/my_native_plugin.dll
```

**加载到 Runtime**：

```rust
use axiom_kernel::plugin::loader::NativePluginLoader;

let loader = NativePluginLoader::new();
let plugin = loader.load(std::path::Path::new("libmy_native_plugin.so"))?;
registry.register(plugin).await;
```

> **注意**：Native 加载器需要启用 `native-loader` feature（默认启用）。

---

## 插件清单文件（manifest）

每个插件可以附带一个清单文件，描述插件的元信息。清单格式定义在 `crates/axiom-kernel/src/plugin/package.rs`：

```rust
pub struct PluginManifest {
    pub id: String,
    pub version: String,
    pub description: Option<String>,
    pub kind: PluginKind,
    pub entry: String,
    pub dependencies: Vec<String>,
}
```

### manifest 示例（JSON）

```json
{
    "id": "echo-plugin",
    "version": "0.1.0",
    "description": "A simple echo plugin that returns received payloads",
    "kind": "Tool",
    "entry": "echo_plugin.wasm",
    "dependencies": []
}
```

### PluginKind 枚举

插件类型用于自动分类，定义在 `crates/axiom-kernel/src/plugin/abi.rs`：

| 类型 | 说明 | 自动检测关键词 |
|------|------|---------------|
| `Llm` | LLM 相关 | llm, chat, gpt, claude |
| `Memory` | 记忆存储 | memory, storage |
| `Tool` | 工具调用 | tool, function |
| `Mcp` | MCP 协议 | mcp, model |
| `Planner` | 计划执行 | plan, task |
| `Alert` | 告警通知 | alert, notify |
| `Viz` | 可视化 | viz, visual, graph |
| `Governance` | 治理策略 | govern, entropy, policy |

### 依赖声明

插件可以声明对其他插件的依赖：

```json
{
    "id": "agent-plugin",
    "version": "0.2.0",
    "dependencies": ["llm-provider", "memory-store"]
}
```

依赖解析在注册时自动进行，如果依赖缺失会返回 `PluginError::DependencyMissing`，如果存在循环依赖会返回 `PluginError::DependencyCycle`。

### 版本管理

插件版本使用语义化版本（semver），定义在 `crates/axiom-kernel/src/plugin/version.rs`：

```rust
pub struct PluginVersion {
    pub id: String,
    pub version: semver::Version,
    pub dependencies: Vec<Dependency>,
}

pub struct Dependency {
    pub id: String,
    pub version_req: semver::VersionReq,  // 如 "^0.1.0" 或 ">=1.0.0"
    pub optional: bool,
}
```

---

## 插件打包和分发

### 打包格式

Axiom Core 使用自定义的 `.axmp` 打包格式，将 manifest 和 WASM 字节码打包为单个文件。

格式定义在 `crates/axiom-kernel/src/plugin/package.rs`：

```
┌─────────────────────────────────────────────┐
│  Magic: "AXMP" (4 bytes)                   │
├─────────────────────────────────────────────┤
│  Version: u32 LE (4 bytes)                 │
├─────────────────────────────────────────────┤
│  JSON Length: u32 LE (4 bytes)             │
├─────────────────────────────────────────────┤
│  JSON Payload (manifest + wasm + signature)│
└─────────────────────────────────────────────┘
```

### 打包 API

```rust
use axiom_kernel::plugin::{
    pack, pack_to_file, unpack, unpack_from_file,
    PluginManifest, PluginPackage,
};

// 创建 manifest
let manifest = PluginManifest {
    id: "echo-plugin".to_string(),
    version: "0.1.0".to_string(),
    description: Some("Echo plugin".to_string()),
    kind: axiom_kernel::plugin::abi::PluginKind::Tool,
    entry: "echo_plugin.wasm".to_string(),
    dependencies: vec![],
};

// 读取 WASM 字节码
let wasm_bytes = std::fs::read("echo_plugin.wasm")?;

// 打包到内存
let packed = pack(manifest.clone(), wasm_bytes.clone(), None)?;

// 打包到文件
pack_to_file(manifest, wasm_bytes, None, std::path::Path::new("echo_plugin.axmp"))?;
```

### 解包 API

```rust
use axiom_kernel::plugin::{unpack, unpack_from_file};

// 从内存解包
let package: PluginPackage = unpack(&packed_bytes)?;

// 从文件解包
let package = unpack_from_file(std::path::Path::new("echo_plugin.axmp"))?;

// 使用解包后的插件
println!("Plugin ID: {}", package.manifest.id);
println!("WASM size: {} bytes", package.wasm_bytes.len());
```

### 签名支持

打包格式支持可选的签名字段，用于验证插件来源：

```rust
// 打包时签名
let signature = sign_with_private_key(&wasm_bytes);
let packed = pack(manifest, wasm_bytes, Some(signature))?;

// 解包时验证
let package = unpack(&packed)?;
if let Some(sig) = &package.signature {
    verify_signature(&package.wasm_bytes, sig)?;
}
```

### 插件仓库索引

分发插件时，可以生成仓库索引文件供消费者查询：

```rust
use axiom_kernel::plugin::{RepositoryIndex, PluginVersion, Dependency};
use semver::{Version, VersionReq};

let mut index = RepositoryIndex::new();
index.add(PluginVersion {
    id: "echo-plugin".to_string(),
    version: Version::new(0, 1, 0),
    dependencies: vec![],
});

// 保存索引
let json = serde_json::to_string_pretty(&index)?;
std::fs::write("plugin-index.json", json)?;

// 加载索引
let index = axiom_kernel::plugin::load_index(std::path::Path::new("plugin-index.json"))?;

// 查询
let req = VersionReq::parse("^0.1.0")?;
if let Some(plugin) = index.resolve("echo-plugin", &req) {
    println!("Found: {} v{}", plugin.id, plugin.version);
}
```

---

## 安全沙箱限制

### 沙箱概述

Axiom Core 为插件提供沙箱限制，定义在 `crates/axiom-kernel/src/plugin/sandbox.rs`。沙箱分为两种实现：

- `WasmPluginSandbox` — 用于 WASM 插件（wasmtime 沙箱）
- `NativePluginSandbox` — 用于 Native 插件（线程池 + 超时）

### SandboxLimits 配置

```rust
use axiom_kernel::plugin::sandbox::SandboxLimits;

let limits = SandboxLimits::new()
    .with_memory(64)                              // 最大 64MB 内存
    .with_cpu(5000)                               // 最大 5 秒 CPU 时间
    .with_read_signals(&["memory", "axiom"])      // 允许读取的信号类型
    .with_write_signals(&["response"])            // 允许写入的信号类型
    .allow_network();                             // 允许网络访问（默认禁止）
```

### 限制项说明

| 限制项 | 说明 | 默认值 |
|--------|------|--------|
| `memory_limit_mb` | 最大内存使用（MB） | 无限制 |
| `cpu_time_limit_ms` | 最大 CPU 时间（毫秒） | 无限制 |
| `read_signals` | 允许读取的信号类型白名单 | 空列表（拒绝所有） |
| `write_signals` | 允许写入的信号类型白名单 | 空列表（拒绝所有） |
| `network` | 是否允许网络访问 | `false`（禁止） |

### 信号白名单

信号白名单支持通配符 `*`（允许所有）：

```rust
// 允许读取所有信号
let limits = SandboxLimits::new()
    .with_read_signals(&["*"]);

// 允许写入特定信号
let limits = SandboxLimits::new()
    .with_write_signals(&["response", "event", "log"]);
```

### 沙箱强制执行

使用 `PluginSandbox` trait 包装插件的消息处理，在调用 `handle_message` 前自动检查权限：

```rust
use axiom_kernel::plugin::sandbox::{WasmPluginSandbox, PluginSandbox};

let sandbox = WasmPluginSandbox::new(limits);

// sandbox 会在调用 handle_message 前检查权限
let reply = sandbox.handle_message(&mut *plugin, msg)?;
```

检查逻辑：

| 消息类型 | 检查项 |
|---------|--------|
| `SendSignal` | 检查 `write_signals` 是否包含该信号 |
| `QueryMemory` | 检查 `read_signals` 是否包含 `"memory"` |
| `CheckAxiom` | 检查 `read_signals` 是否包含 `"axiom"` |
| `CallTool` | 检查 `read_signals` 是否包含 `"tool:{tool_name}"` |
| `QueryLens` | 检查 `read_signals` 是否包含 `"lens"` |
| `Custom` | 检查 `write_signals` 是否包含该 kind |

如果权限检查失败，返回 `PluginError::PermissionDenied`。

### 最佳实践

1. **不信任的插件使用 WASM 沙箱**：第三方插件应编译为 WASM，通过沙箱限制资源访问
2. **受信任的插件可以使用 Native**：内部插件可以使用 Native 插件获得原生性能
3. **最小权限原则**：只授予插件所需的最小权限，避免使用 `*` 通配符
4. **设置资源上限**：为所有插件设置内存和 CPU 时间上限，防止资源耗尽
5. **禁止网络访问**：除非插件明确需要（如 LLM 调用），否则禁止网络访问
6. **验证签名**：对从外部获取的插件包验证签名，确保来源可信

---

## 更多资源

- [插件系统设计文档](PLUGIN_SYSTEM.md) — 插件系统架构设计
- [架构设计文档](ARCHITECTURE.md) — 完整架构说明
- [开发指南](development.md) — 项目开发指南
- [用户指南](user-guide.md) — 面向使用者的指南
- [WASM 插件示例](../crates/axiom-plugin-example-wasm/src/) — Echo/Counter/Transformer 示例
