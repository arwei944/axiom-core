use axiom_kernel::plugin::abi::{AxiomPlugin, PluginContext, PluginMessage, PluginReply};
use parking_lot::Mutex;

#[derive(Default)]
pub struct WasmCounterPlugin {
    count: Mutex<u64>,
}

impl AxiomPlugin for WasmCounterPlugin {
    fn id(&self) -> &'static str {
        "wasm-counter"
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
        Box::new(WasmCounterPlugin { count: Mutex::new(*self.count.lock()) })
    }
}
