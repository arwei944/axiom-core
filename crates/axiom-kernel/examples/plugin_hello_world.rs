use axiom_kernel::plugin::abi::{AxiomPlugin, PluginContext, PluginMessage, PluginReply, PluginError};
use axiom_kernel::plugin::registry::PluginRegistry;

struct HelloWorldPlugin;

impl AxiomPlugin for HelloWorldPlugin {
    fn id(&self) -> &'static str {
        "hello-world"
    }

    fn version(&self) -> &'static str {
        "0.1.0"
    }

    fn dependencies(&self) -> &[&'static str] {
        &[]
    }

    fn capabilities(&self) -> &[axiom_kernel::plugin::abi::PluginCapabilityDescriptor] {
        &[]
    }

    fn init(&mut self, _ctx: PluginContext) -> Result<(), PluginError> {
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn AxiomPlugin> {
        Box::new(HelloWorldPlugin)
    }

    fn handle_message(&mut self, msg: PluginMessage) -> Result<PluginReply, PluginError> {
        match msg {
            PluginMessage::SendSignal { signal, payload } => {
                println!("plugin received signal `{}` with {} bytes", signal, payload.len());
                Ok(PluginReply::Ok(vec![]))
            }
            _ => Ok(PluginReply::Ok(vec![])),
        }
    }
}

fn main() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let registry = PluginRegistry::new();
    rt.block_on(async {
        registry.register(Box::new(HelloWorldPlugin)).await;
    });

    let key = axiom_kernel::plugin::registry::PluginKey::new("hello-world", "0.1.0");
    let instance = rt.block_on(registry.get(key));

    if let Some(plugin) = instance {
        let reply = plugin.handle_message(PluginMessage::SendSignal {
            signal: "greeting".to_string(),
            payload: b"hello world".to_vec(),
        });
        println!("plugin reply: {:?}", reply);
    }

    println!("plugin-hello-world example completed");
}
