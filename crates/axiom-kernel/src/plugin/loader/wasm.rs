use crate::plugin::abi::{
    AxiomPlugin, PluginContext, PluginError, PluginMessage, PluginReply, PluginResult,
};
use std::path::Path;
use wasmtime::*;

#[cfg(feature = "wasm-loader")]
pub struct WasmPluginLoader {
    engine: Engine,
}

#[cfg(feature = "wasm-loader")]
impl WasmPluginLoader {
    pub fn new() -> Self {
        let engine = Engine::default();
        Self { engine }
    }

    pub fn load(&self, path: &Path) -> PluginResult<Box<dyn AxiomPlugin>> {
        let module = Module::from_file(&self.engine, path)
            .map_err(|e| PluginError::LoadFailed(e.to_string()))?;

        let mut store = Store::new(&self.engine, ());
        let instance = Instance::new(&mut store, &module, &[])
            .map_err(|e| PluginError::LoadFailed(e.to_string()))?;

        // P1-5: optional axiom_abi_version export must match host when present.
        if let Ok(abi_fn) = instance.get_typed_func::<(), u32>(&mut store, "axiom_abi_version") {
            let abi = abi_fn
                .call(&mut store, ())
                .map_err(|e| PluginError::LoadFailed(e.to_string()))?;
            crate::plugin::package::check_abi_compatible(abi)?;
        }

        let create = instance
            .get_typed_func::<(), i32>(&mut store, "axiom_plugin_create")
            .map_err(|e| PluginError::MissingSymbol(e.to_string()))?;

        let ptr =
            create.call(&mut store, ()).map_err(|e| PluginError::LoadFailed(e.to_string()))?;

        if ptr <= 0 {
            return Err(PluginError::LoadFailed("plugin create returned invalid pointer".into()));
        }

        Ok(Box::new(WasmPluginInstance { store, instance, ptr }))
    }

    pub fn unload(&self, _plugin: &dyn AxiomPlugin) -> PluginResult<()> {
        let _ = _plugin;
        Ok(())
    }
}

#[cfg(feature = "wasm-loader")]
impl Default for WasmPluginLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "wasm-loader")]
struct WasmPluginInstance {
    store: Store<()>,
    instance: Instance,
    ptr: i32,
}

#[cfg(feature = "wasm-loader")]
impl AxiomPlugin for WasmPluginInstance {
    fn id(&self) -> &'static str {
        "wasm-plugin"
    }
    fn version(&self) -> &'static str {
        "0.1.0"
    }
    fn dependencies(&self) -> &[&'static str] {
        &[]
    }
    fn capabilities(&self) -> &[crate::plugin::abi::CapabilityDescriptor] {
        &[]
    }
    fn init(&mut self, _ctx: PluginContext) -> PluginResult<()> {
        Ok(())
    }
    fn start(&mut self) -> PluginResult<()> {
        Ok(())
    }
    fn stop(&mut self) -> PluginResult<()> {
        Ok(())
    }
    fn handle_message(&mut self, msg: PluginMessage) -> PluginResult<PluginReply> {
        let bytes =
            postcard::to_allocvec(&msg).map_err(|e| PluginError::LoadFailed(e.to_string()))?;

        let mut store = &mut self.store;
        let instance = &self.instance;

        let alloc = instance.get_typed_func::<(u32, u32), u32>(&mut store, "axiom_alloc").ok();
        let handle = instance
            .get_typed_func::<(u32, u32), u32>(&mut store, "axiom_plugin_handle_message")
            .ok();
        let dealloc = instance.get_typed_func::<u32, ()>(&mut store, "axiom_dealloc").ok();

        let wasm_ptr = if let Some(alloc) = alloc {
            alloc
                .call(&mut store, (bytes.len() as u32, 1))
                .map_err(|e| PluginError::LoadFailed(e.to_string()))?
        } else {
            return Err(PluginError::MissingSymbol("axiom_alloc not exported".into()));
        };

        let memory = instance.get_memory(&mut store, "memory");
        if let Some(mem) = memory {
            let data = mem.data_mut(&mut store);
            let start = wasm_ptr as usize;
            let end = start + bytes.len();
            if end <= data.len() {
                data[start..end].copy_from_slice(&bytes);
            } else {
                let _ = dealloc.as_ref().map(|d| d.call(&mut store, wasm_ptr));
                return Err(PluginError::LoadFailed("wasm memory out of bounds".into()));
            }
        }

        let reply_ptr = if let Some(handle) = handle {
            handle
                .call(&mut store, (wasm_ptr, bytes.len() as u32))
                .map_err(|e| PluginError::LoadFailed(e.to_string()))?
        } else {
            let _ = dealloc.map(|d| d.call(&mut store, wasm_ptr));
            return Err(PluginError::MissingSymbol(
                "axiom_plugin_handle_message not exported".into(),
            ));
        };

        let reply = if reply_ptr > 0 {
            if let Some(mem) = instance.get_memory(&mut store, "memory") {
                let data = mem.data(&store);
                let start = reply_ptr as usize;
                let mut len = 0;
                while start + len < data.len() && data[start + len] != 0 {
                    len += 1;
                }
                let slice = &data[start..start + len];
                postcard::from_bytes(slice).map_err(|e| PluginError::LoadFailed(e.to_string()))?
            } else {
                PluginReply::Err("wasm memory unavailable".into())
            }
        } else {
            PluginReply::Err("plugin returned null reply".into())
        };

        let _ = dealloc.as_ref().map(|d| d.call(&mut store, wasm_ptr));
        if let PluginReply::Ok(data) = &reply {
            if !data.is_empty() {
                let _ = dealloc.as_ref().map(|d| d.call(&mut store, data.as_ptr() as u32));
            }
        }

        Ok(reply)
    }

    fn clone_box(&self) -> Box<dyn AxiomPlugin> {
        // SAFETY: We're copying the wasmtime Instance handle which is safe because
        // wasmtime instances are shareable and we're creating a new Store context.
        // The ptr field contains a raw pointer to the wasm export which remains valid.
        unsafe {
            Box::new(WasmPluginInstance {
                store: Store::new(&Engine::default(), ()),
                instance: std::ptr::read(&self.instance),
                ptr: self.ptr,
            })
        }
    }
}

#[cfg(not(feature = "wasm-loader"))]
pub struct WasmPluginLoader;

#[cfg(not(feature = "wasm-loader"))]
impl WasmPluginLoader {
    pub fn new() -> Self {
        Self
    }

    pub fn load(&self, path: &Path) -> PluginResult<Box<dyn AxiomPlugin>> {
        let _ = path;
        Err(PluginError::LoadFailed("wasm-loader feature is not enabled".into()))
    }

    pub fn unload(&self, _plugin: &dyn AxiomPlugin) -> PluginResult<()> {
        let _ = plugin;
        Ok(())
    }
}

#[cfg(not(feature = "wasm-loader"))]
impl Default for WasmPluginLoader {
    fn default() -> Self {
        Self::new()
    }
}
