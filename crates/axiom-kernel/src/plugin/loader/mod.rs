pub mod native;

#[cfg(feature = "wasm-loader")]
pub mod wasm;

#[cfg(feature = "wasm-loader")]
pub use wasm::WasmPluginLoader;

pub use native::NativePluginLoader;

use crate::plugin::abi::PluginError;
use std::path::Path;

pub trait PluginLoader {
    fn load(&self, path: &Path) -> Result<Box<dyn crate::plugin::abi::AxiomPlugin>, PluginError>;
    fn unload(&self, _plugin: &dyn crate::plugin::abi::AxiomPlugin) -> Result<(), PluginError> {
        Ok(())
    }
}
