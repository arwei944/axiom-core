use axiom_kernel::plugin::abi::{PluginContext, PluginMessage, PluginReply};
use axiom_kernel::{
    AxiomKernel, AxiomPlugin, CellKernel, HeatmapCollector, LensKernel, PluginRegistry,
    SignalKernel, WitnessKernel,
};
use axiom_plugin_example_wasm::counter::WasmCounterPlugin;
use axiom_plugin_example_wasm::echo::WasmEchoPlugin;
use axiom_plugin_example_wasm::transformer::WasmTransformerPlugin;

#[tokio::test]
async fn test_wasm_echo_plugin_basic() {
    let mut plugin = WasmEchoPlugin;
    let ctx = PluginContext::new(
        CellKernel::new(),
        SignalKernel::new(),
        LensKernel::new(),
        AxiomKernel::new(),
        WitnessKernel::new(),
        PluginRegistry::new(),
        std::sync::Arc::new(tokio::sync::RwLock::new(HeatmapCollector::new())),
    );
    plugin.init(ctx).unwrap();

    let reply = plugin
        .handle_message(PluginMessage::Custom {
            kind: "echo".into(),
            payload: b"hello".to_vec(),
        })
        .unwrap();

    match reply {
        PluginReply::Ok(data) => assert_eq!(data, b"hello"),
        PluginReply::Err(err) => panic!("{}", err),
    }
}

#[tokio::test]
async fn test_wasm_counter_plugin_increments() {
    let mut plugin = WasmCounterPlugin::default();
    let ctx = PluginContext::new(
        CellKernel::new(),
        SignalKernel::new(),
        LensKernel::new(),
        AxiomKernel::new(),
        WitnessKernel::new(),
        PluginRegistry::new(),
        std::sync::Arc::new(tokio::sync::RwLock::new(HeatmapCollector::new())),
    );
    plugin.init(ctx).unwrap();

    let reply1 = plugin
        .handle_message(PluginMessage::Custom {
            kind: "inc".into(),
            payload: Vec::new(),
        })
        .unwrap();
    let reply2 = plugin
        .handle_message(PluginMessage::Custom {
            kind: "inc".into(),
            payload: Vec::new(),
        })
        .unwrap();

    match (reply1, reply2) {
        (PluginReply::Ok(a), PluginReply::Ok(b)) => {
            assert_eq!(a, b"1");
            assert_eq!(b, b"2");
        }
        other => panic!("unexpected replies: {:?}", other),
    }
}

#[tokio::test]
async fn test_wasm_transformer_plugin_adds_one() {
    let mut plugin = WasmTransformerPlugin;
    let ctx = PluginContext::new(
        CellKernel::new(),
        SignalKernel::new(),
        LensKernel::new(),
        AxiomKernel::new(),
        WitnessKernel::new(),
        PluginRegistry::new(),
        std::sync::Arc::new(tokio::sync::RwLock::new(HeatmapCollector::new())),
    );
    plugin.init(ctx).unwrap();

    let reply = plugin
        .handle_message(PluginMessage::Custom {
            kind: "transform".into(),
            payload: b"abc".to_vec(),
        })
        .unwrap();

    match reply {
        PluginReply::Ok(data) => assert_eq!(data, b"bcd"),
        PluginReply::Err(err) => panic!("{}", err),
    }
}
