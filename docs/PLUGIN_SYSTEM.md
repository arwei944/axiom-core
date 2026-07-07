# 插件系统

Axiom Core v0.4.0 新增插件系统，支持运行时动态加载 WASM 和 Native 插件，实现系统功能的灵活扩展。

---

## 目录

- [设计理念](#设计理念)
- [架构概览](#架构概览)
- [核心类型](#核心类型)
- [编写 WASM 插件](#编写-wasm-插件)
- [编写 Native 插件](#编写-native-插件)
- [使用插件](#使用插件)
- [插件生命周期](#插件生命周期)
- [安全考虑](#安全考虑)

---

## 设计理念

### 1. 运行时动态扩展
- 无需重新编译即可添加新功能
- 支持热插拔（部分场景）
- 插件可以独立版本管理

### 2. 多语言支持
- WASM 插件：支持 Rust、TypeScript、Go 等编译为 WASM 的语言
- Native 插件：支持 Rust、C、C++ 等编译为本地库的语言

### 3. 隔离性
- WASM 沙箱隔离
- 资源限制（内存、CPU）
- 权限控制

### 4. 类型安全
- 编译期类型检查
- ABI 兼容性保证
- 版本化接口

---

## 架构概览

```
┌────────────────────────────────────────────────────────────┐
│                    PluginRegistry                          │
│  ┌──────────────────────────────────────────────────────┐  │
│  │                   PluginLoader                       │  │
│  │  ┌─────────────────┐          ┌───────────────────┐  │  │
│  │  │   WASM Loader   │          │   Native Loader   │  │  │
│  │  │ (wasmtime)      │          │ (libloading)      │  │  │
│  │  └────────┬────────┘          └─────────┬─────────┘  │  │
│  └───────────┼─────────────────────────────┼────────────┘  │
│              │                             │                │
│              ▼                             ▼                │
│  ┌──────────────────┐         ┌──────────────────────┐      │
│  │   WASM Plugin    │         │   Native Plugin      │      │
│  │   (.wasm)        │         │   (.so/.dll/.dylib)  │      │
│  │  sandboxed       │         │  full system access  │      │
│  └──────────────────┘         └──────────────────────┘      │
└────────────────────────────────────────────────────────────┘
```

---

## 核心类型

### PluginRegistry

```rust
pub struct PluginRegistry {
    plugins: RwLock<HashMap<String, Box<dyn AxiomPlugin>>>,
}

impl PluginRegistry {
    pub fn new() -> Self;
    pub async fn load_wasm(&self, path: &str) -> Result<()>;
    pub async fn load_native(&self, path: &str) -> Result<()>;
    pub fn get(&self, name: &str) -> Option<Box<dyn AxiomPlugin>>;
    pub fn iter(&self) -> impl Iterator<Item = (&str, &dyn AxiomPlugin)>;
}
```

### AxiomPlugin Trait

```rust
pub trait AxiomPlugin: Send + Sync + 'static {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn kind(&self) -> PluginKind;
    fn start(&mut self) -> Result<()>;
    fn stop(&mut self) -> Result<()>;
    fn handle_signal(&mut self, signal: SignalEnvelope) -> Result<Option<SignalEnvelope>>;
}

pub enum PluginKind {
    Wasm,
    Native,
}
```

### PluginLoader

```rust
pub trait PluginLoader {
    async fn load(&self, path: &str) -> Result<Box<dyn AxiomPlugin>>;
}
```

---

## 编写 WASM 插件

### 步骤 1：创建插件项目

```bash
cargo new --lib my-plugin
cd my-plugin
```

### 步骤 2：添加依赖

```toml
[package]
name = "my-plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
axiom-plugin-wasm-sdk = "0.4"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

### 步骤 3：实现插件

```rust
use axiom_plugin_wasm_sdk::prelude::*;

struct EchoPlugin;

#[plugin]
impl AxiomPlugin for EchoPlugin {
    fn name(&self) -> &str { "echo" }
    fn version(&self) -> &str { "0.1.0" }
    
    fn handle_signal(&mut self, signal: SignalEnvelope) -> Result<Option<SignalEnvelope>> {
        let payload = signal.payload.clone();
        let mut response = SignalEnvelope::new();
        response.payload = serde_json::json!({
            "echo": payload
        });
        Ok(Some(response))
    }
}

plugin_export!(EchoPlugin);
```

### 步骤 4：编译为 WASM

```bash
cargo build --target wasm32-wasi --release
```

编译产物：`target/wasm32-wasi/release/my_plugin.wasm`

---

## 编写 Native 插件

### 步骤 1：创建插件项目

```bash
cargo new --lib my-native-plugin
cd my-native-plugin
```

### 步骤 2：添加依赖

```toml
[package]
name = "my-native-plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
axiom-kernel = "0.4"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

### 步骤 3：实现插件

```rust
use axiom_kernel::plugin::{AxiomPlugin, PluginKind, SignalEnvelope};

struct CounterPlugin {
    count: RwLock<u64>,
}

impl AxiomPlugin for CounterPlugin {
    fn name(&self) -> &str { "counter" }
    fn version(&self) -> &str { "0.1.0" }
    fn kind(&self) -> PluginKind { PluginKind::Native }
    
    fn start(&mut self) -> Result<()> {
        *self.count.write() = 0;
        Ok(())
    }
    
    fn handle_signal(&mut self, _signal: SignalEnvelope) -> Result<Option<SignalEnvelope>> {
        *self.count.write() += 1;
        Ok(None)
    }
}

#[no_mangle]
pub extern "C" fn axiom_plugin_new() -> *mut dyn AxiomPlugin {
    let plugin = Box::new(CounterPlugin {
        count: RwLock::new(0),
    });
    Box::into_raw(plugin)
}

#[no_mangle]
pub extern "C" fn axiom_plugin_delete(plugin: *mut dyn AxiomPlugin) {
    unsafe { Box::from_raw(plugin); }
}
```

### 步骤 4：编译

```bash
# Linux
cargo build --release
# 产物：target/release/libmy_native_plugin.so

# Windows
cargo build --release
# 产物：target/release/my_native_plugin.dll

# macOS
cargo build --release
# 产物：target/release/libmy_native_plugin.dylib
```

---

## 使用插件

### 加载插件

```rust
use axiom_kernel::plugin::{PluginRegistry, PluginKind};

let registry = PluginRegistry::new();

// 加载 WASM 插件
registry.load_wasm("plugins/echo.wasm").await?;

// 加载 Native 插件
registry.load_native("plugins/libcounter.so").await?;
```

### 获取插件

```rust
let plugin = registry.get("echo").unwrap();
println!("Plugin: {} v{}", plugin.name(), plugin.version());
```

### 执行插件

```rust
let signal = SignalEnvelope::new();
signal.payload = serde_json::json!({ "message": "hello" });

let result = plugin.handle_signal(signal)?;
if let Some(response) = result {
    println!("Response: {}", response.payload);
}
```

---

## 插件生命周期

```
加载 (load)
    │
    ▼
启动 (start) ──→ 运行中 (running) ──→ 停止 (stop)
                    │
                    │ handle_signal
                    ▼
              处理信号
```

### 生命周期回调

| 阶段 | 方法 | 说明 |
|------|------|------|
| 加载 | `load()` | 插件被加载到内存 |
| 启动 | `start()` | 插件初始化，分配资源 |
| 运行 | `handle_signal()` | 处理信号 |
| 停止 | `stop()` | 插件清理，释放资源 |

---

## 安全考虑

### WASM 插件

1. **沙箱隔离**：WASM 运行在沙箱中，无法直接访问系统资源
2. **内存限制**：可配置最大内存使用量
3. **CPU 限制**：可配置执行时间上限
4. **导入限制**：仅允许白名单内的函数导入

### Native 插件

1. **权限检查**：加载前检查文件权限
2. **签名验证**：可验证插件数字签名
3. **沙箱限制**：Native 插件拥有完整系统访问权限，需谨慎使用

### 通用安全

1. **版本验证**：检查插件版本兼容性
2. **依赖检查**：验证插件依赖的 ABI 版本
3. **日志审计**：记录所有插件操作

---

## 示例插件

### Echo 插件

返回接收到的消息的回声：

```rust
struct EchoPlugin;

#[plugin]
impl AxiomPlugin for EchoPlugin {
    fn name(&self) -> &str { "echo" }
    fn version(&self) -> &str { "0.1.0" }
    
    fn handle_signal(&mut self, signal: SignalEnvelope) -> Result<Option<SignalEnvelope>> {
        let mut response = signal.clone();
        response.payload = serde_json::json!({
            "type": "echo",
            "original": signal.payload
        });
        Ok(Some(response))
    }
}
```

### Counter 插件

统计接收到的消息数量：

```rust
struct CounterPlugin {
    count: RwLock<u64>,
}

#[plugin]
impl AxiomPlugin for CounterPlugin {
    fn name(&self) -> &str { "counter" }
    fn version(&self) -> &str { "0.1.0" }
    
    fn start(&mut self) -> Result<()> {
        *self.count.write() = 0;
        Ok(())
    }
    
    fn handle_signal(&mut self, _signal: SignalEnvelope) -> Result<Option<SignalEnvelope>> {
        *self.count.write() += 1;
        Ok(None)
    }
}
```

---

## 总结

插件系统为 Axiom Core 提供了灵活的运行时扩展能力：

- **WASM 插件**：沙箱隔离，跨语言支持，适合安全敏感场景
- **Native 插件**：高性能，完整系统访问，适合性能敏感场景
- **统一接口**：两种插件使用相同的 `AxiomPlugin` trait，API 一致
- **动态加载**：无需重新编译即可扩展功能

这种设计使 Axiom Core 成为构建可扩展智能体系统的理想选择。