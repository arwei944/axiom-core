use axiom_kernel::plugin::abi::{PluginContext, PluginMessage, PluginReply};
use axiom_kernel::AxiomPlugin;
use axiom_plugin_test::{CounterPlugin, EchoPlugin};

#[tokio::test]
async fn test_echo_plugin_basic() {
    let mut plugin = EchoPlugin;
    let ctx = PluginContext::new(
        axiom_kernel::CellKernel::new(),
        axiom_kernel::SignalKernel::new(),
        axiom_kernel::LensKernel::new(),
        axiom_kernel::AxiomKernel::new(),
        axiom_kernel::WitnessKernel::new(),
        axiom_kernel::PluginRegistry::new(),
        std::sync::Arc::new(tokio::sync::RwLock::new(axiom_kernel::HeatmapCollector::new())),
    );
    plugin.init(ctx).unwrap();

    let reply = plugin
        .handle_message(PluginMessage::Custom { kind: "echo".into(), payload: b"hello".to_vec() })
        .unwrap();

    match reply {
        PluginReply::Ok(data) => assert_eq!(data, b"hello"),
        PluginReply::Err(err) => panic!("{}", err),
    }
}

#[tokio::test]
async fn test_counter_plugin_increments() {
    let mut plugin = CounterPlugin::default();
    let ctx = PluginContext::new(
        axiom_kernel::CellKernel::new(),
        axiom_kernel::SignalKernel::new(),
        axiom_kernel::LensKernel::new(),
        axiom_kernel::AxiomKernel::new(),
        axiom_kernel::WitnessKernel::new(),
        axiom_kernel::PluginRegistry::new(),
        std::sync::Arc::new(tokio::sync::RwLock::new(axiom_kernel::HeatmapCollector::new())),
    );
    plugin.init(ctx).unwrap();

    let reply1 = plugin
        .handle_message(PluginMessage::Custom { kind: "inc".into(), payload: Vec::new() })
        .unwrap();
    let reply2 = plugin
        .handle_message(PluginMessage::Custom { kind: "inc".into(), payload: Vec::new() })
        .unwrap();

    match (reply1, reply2) {
        (PluginReply::Ok(a), PluginReply::Ok(b)) => {
            assert_eq!(a, b"1");
            assert_eq!(b, b"2");
        }
        other => panic!("unexpected replies: {:?}", other),
    }
}
