use axiom_kernel::plugin::abi::{AxiomPlugin, PluginContext, PluginMessage, PluginReply};

#[derive(Default)]
pub struct WasmTransformerPlugin;

impl AxiomPlugin for WasmTransformerPlugin {
    fn id(&self) -> &'static str {
        "wasm-transformer"
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
        msg: PluginMessage,
    ) -> axiom_kernel::plugin::abi::PluginResult<PluginReply> {
        let payload = match msg {
            PluginMessage::Custom { payload, .. } => payload,
            _ => Vec::new(),
        };
        let transformed = payload.iter().map(|b| b.wrapping_add(1)).collect::<Vec<_>>();
        Ok(PluginReply::Ok(transformed))
    }
    fn clone_box(&self) -> Box<dyn AxiomPlugin> {
        Box::new(WasmTransformerPlugin)
    }
}
