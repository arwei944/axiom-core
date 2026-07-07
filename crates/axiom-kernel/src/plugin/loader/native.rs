use crate::plugin::abi::{AxiomPlugin, PluginError, PluginResult};
use libloading::{Library, Symbol};
use std::path::Path;

pub struct NativePluginLoader {
    _lib: Option<Library>,
}

impl NativePluginLoader {
    pub fn new() -> Self {
        Self { _lib: None }
    }

    pub fn load(&self, path: &Path) -> PluginResult<Box<dyn AxiomPlugin>> {
        // SAFETY: Loading a native plugin requires unsafe operations to access
        // the dynamic library and call C ABI functions. We verify the symbol
        // exists and the returned pointer is non-null before using it.
        unsafe {
            let lib = Library::new(path)
                .map_err(|e| PluginError::LoadFailed(format!("failed to load library: {e}")))?;

            let create: Symbol<unsafe extern "C" fn() -> *mut dyn AxiomPlugin> = lib
                .get(b"axiom_plugin_create")
                .map_err(|e| PluginError::MissingSymbol(format!("axiom_plugin_create: {e}")))?;

            let ptr = create();
            if ptr.is_null() {
                return Err(PluginError::LoadFailed("axiom_plugin_create returned null".into()));
            }

            Ok(Box::from_raw(ptr))
        }
    }

    pub fn unload(&self, _plugin: &dyn AxiomPlugin) -> PluginResult<()> {
        let _ = _plugin;
        Ok(())
    }
}

impl Default for NativePluginLoader {
    fn default() -> Self {
        Self::new()
    }
}
