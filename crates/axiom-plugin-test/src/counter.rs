use axiom_kernel::plugin::abi::{AxiomPlugin, PluginContext, PluginMessage, PluginReply};
use std::sync::Mutex;

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
        let mut count = self.count.lock().unwrap();
        *count += 1;
        Ok(PluginReply::Ok(count.to_string().into_bytes()))
    }
    fn clone_box(&self) -> Box<dyn AxiomPlugin> {
        Box::new(CounterPlugin {
            count: Mutex::new(*self.count.lock().unwrap()),
        })
    }
}
