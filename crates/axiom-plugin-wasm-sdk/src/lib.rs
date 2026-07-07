use axiom_kernel::plugin::abi::AxiomPlugin;
use parking_lot::Mutex;
use std::sync::Arc;

pub struct WasmPluginBuilder {
    plugin: Arc<Mutex<Option<Box<dyn AxiomPlugin>>>>,
}

impl WasmPluginBuilder {
    pub fn new() -> Self {
        Self { plugin: Arc::new(Mutex::new(None)) }
    }

    pub fn build(self) -> *mut () {
        let mut plugin = self.plugin.lock();
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
        use parking_lot::Mutex;
        use std::sync::Arc;
        use $crate::wasm_sdk::WasmPluginBuilder;

        #[no_mangle]
        pub extern "C" fn axiom_plugin_create() -> *mut () {
            let builder = WasmPluginBuilder::new();
            let plugin = <$plugin_type>::default();
            {
                let mut b = builder.plugin.lock();
                *b = Some(Box::new(plugin));
            }
            builder.build()
        }

        #[no_mangle]
        pub extern "C" fn axiom_plugin_destroy(ptr: *mut ()) {
            if !ptr.is_null() {
                // SAFETY: The pointer was allocated by axiom_plugin_create using Box::into_raw,
                // so it's safe to reconstruct and drop it here.
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
            // SAFETY: Pointers validated non-null, ptr allocated by create and valid until destroy,
            // msg_ptr points to valid byte slice of length msg_len
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
                let layout = match std::alloc::Layout::from_vec(bytes.clone()) {
                    Ok(l) => l,
                    Err(_) => return std::ptr::null_mut(),
                };
                let ptr = std::alloc::alloc(layout);
                if ptr.is_null() {
                    return std::ptr::null_mut();
                }
                ptr.copy_from(bytes.as_ptr(), bytes.len());
                std::mem::forget(bytes);
                ptr
            }
        }
    };
}
