use axiom_kernel::plugin::abi::AxiomPlugin;
use std::sync::{Arc, Mutex};

pub struct WasmPluginBuilder {
    plugin: Arc<Mutex<Option<Box<dyn AxiomPlugin>>>>,
}

impl WasmPluginBuilder {
    pub fn new() -> Self {
        Self {
            plugin: Arc::new(Mutex::new(None)),
        }
    }

    pub fn build(self) -> *mut () {
        let mut plugin = self.plugin.lock().unwrap();
        if let Some(p) = plugin.take() {
            let raw = Box::into_raw(p);
            raw as *mut ()
        } else {
            std::ptr::null_mut()
        }
    }
}

impl Default for WasmPluginBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[macro_export]
macro_rules! axiom_wasm_plugin {
    ($plugin_type:ty) => {
        use $crate::wasm_sdk::WasmPluginBuilder;
        use std::sync::{Arc, Mutex};

        #[no_mangle]
        pub extern "C" fn axiom_plugin_create() -> *mut () {
            let builder = WasmPluginBuilder::new();
            let plugin = <$plugin_type>::default();
            {
                let mut b = builder.plugin.lock().unwrap();
                *b = Some(Box::new(plugin));
            }
            builder.build()
        }

        #[no_mangle]
        pub extern "C" fn axiom_plugin_destroy(ptr: *mut ()) {
            if !ptr.is_null() {
                unsafe {
                    let _ = Box::from_raw(ptr as *mut dyn $crate::AxiomPlugin);
                }
            }
        }

        #[no_mangle]
        pub extern "C" fn axiom_plugin_handle_message(
            ptr: *mut (),
            msg_ptr: *const u8,
            msg_len: usize,
        ) -> *mut u8 {
            if ptr.is_null() || msg_ptr.is_null() || msg_len == 0 {
                return std::ptr::null_mut();
            }
            unsafe {
                let plugin = &mut *(ptr as *mut dyn $crate::AxiomPlugin);
                let msg_slice = std::slice::from_raw_parts(msg_ptr, msg_len);
                let msg: $crate::PluginMessage = match postcard::from_bytes(msg_slice) {
                    Ok(m) => m,
                    Err(_) => return std::ptr::null_mut(),
                };
                let reply = match plugin.handle_message(msg) {
                    Ok(r) => r,
                    Err(_) => return std::ptr::null_mut(),
                };
                let bytes = match postcard::to_allocvec(&reply) {
                    Ok(b) => b,
                    Err(_) => return std::ptr::null_mut(),
                };
                let ptr = std::alloc::alloc(std::alloc::Layout::from_vec(bytes.clone()).unwrap());
                ptr.copy_from(bytes.as_ptr(), bytes.len());
                std::mem::forget(bytes);
                ptr
            }
        }
    };
}
