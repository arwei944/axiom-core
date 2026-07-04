use axiom_core::cell::{Cell, ExecCell, LayerOf};
use axiom_core::context::{CellContext, LayeredCellContext, OutgoingEnvelope, OutgoingWitness};
use axiom_core::id::{CellId, CorrelationId, MsgId};
use axiom_core::layer::Layer;
use axiom_core::schema::ValidationResult;
use axiom_core::sealed::ExecLayer;
use axiom_core::signal::{now_ns, Signal, SignalKind, VectorClock};
use axiom_core::version::{Migration, SchemaVersion, Versioned};
use axiom_core::{axiom::Axiom, AxiomError, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::future::Future;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GreetCmd {
    msg_id: MsgId,
    correlation_id: CorrelationId,
    vector_clock: VectorClock,
    name: String,
}

impl Signal for GreetCmd {
    fn signal_type(&self) -> &'static str {
        "GreetCmd"
    }
    fn msg_id(&self) -> &MsgId {
        &self.msg_id
    }
    fn correlation_id(&self) -> &CorrelationId {
        &self.correlation_id
    }
    fn vector_clock(&self) -> &VectorClock {
        &self.vector_clock
    }
    fn timestamp_ns(&self) -> u64 {
        now_ns()
    }
    fn kind(&self) -> SignalKind {
        SignalKind::Command
    }
    fn layer(&self) -> Layer {
        Layer::Exec
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn clone_signal(&self) -> Box<dyn Signal> {
        Box::new(self.clone())
    }
    fn validate(&self) -> ValidationResult {
        ValidationResult::ok()
    }
    fn serialize_to_json(&self) -> ::axiom_core::Result<serde_json::Value> {
        serde_json::to_value(self).map_err(|e| ::axiom_core::AxiomError::SignalSerialization {
            signal_type: "Greet".into(),
            message: e.to_string(),
        })
    }
}

struct GreeterCell {
    id: CellId,
    greeted: Vec<String>,
}

impl GreeterCell {
    fn new() -> Self {
        Self {
            id: CellId::new("greeter"),
            greeted: Vec::new(),
        }
    }
}

#[axiom_macros::cell("exec")]
impl Cell for GreeterCell {
    type Message = GreetCmd;

    fn id(&self) -> &CellId {
        &self.id
    }

    #[allow(clippy::manual_async_fn)]
    fn handle<'a>(
        &'a mut self,
        signal: GreetCmd,
        ctx: LayeredCellContext<'a, Self::Layer>,
    ) -> impl Future<
        Output = (
            axiom_core::Result<()>,
            Vec<OutgoingEnvelope>,
            Vec<OutgoingWitness>,
        ),
    > + Send
           + 'a {
        async move {
            let mut ctx = ctx;
            self.greeted.push(signal.name);
            let (outgoing, witnesses) = ctx.end_processing();
            (Ok(()), outgoing, witnesses)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[axiom_macros::schema_version(2)]
struct V2Signal {
    msg_id: MsgId,
    correlation_id: CorrelationId,
    vector_clock: VectorClock,
    value: u32,
}

impl Signal for V2Signal {
    fn signal_type(&self) -> &'static str {
        "V2Signal"
    }
    fn msg_id(&self) -> &MsgId {
        &self.msg_id
    }
    fn correlation_id(&self) -> &CorrelationId {
        &self.correlation_id
    }
    fn vector_clock(&self) -> &VectorClock {
        &self.vector_clock
    }
    fn timestamp_ns(&self) -> u64 {
        now_ns()
    }
    fn kind(&self) -> SignalKind {
        SignalKind::Event
    }
    fn layer(&self) -> Layer {
        Layer::Validate
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn clone_signal(&self) -> Box<dyn Signal> {
        Box::new(self.clone())
    }
    fn validate(&self) -> ValidationResult {
        ValidationResult::ok()
    }
    fn serialize_to_json(&self) -> ::axiom_core::Result<serde_json::Value> {
        serde_json::to_value(self).map_err(|e| ::axiom_core::AxiomError::SignalSerialization {
            signal_type: "Greet".into(),
            message: e.to_string(),
        })
    }
}

#[derive(Debug)]
struct MigrateV1toV2;

#[axiom_macros::migration(from = 1)]
impl Migration for MigrateV1toV2 {
    fn migrate(&self, input: Value) -> Result<Value> {
        Ok(input)
    }
}

#[axiom_macros::axiom]
#[derive(Default)]
struct TestAxiom;

impl axiom_core::axiom::Axiom for TestAxiom {
    type State = i32;
    type Message = GreetCmd;

    fn name(&self) -> &'static str {
        "TestAxiom"
    }

    fn check(&self, _current: &i32, new: &i32, _msg: &GreetCmd) -> Result<()> {
        if *new < 0 {
            Err(AxiomError::InvariantViolated {
                message: "negative value not allowed".into(),
            })
        } else {
            Ok(())
        }
    }
}

#[tokio::test]
async fn test_cell_macro_adds_exec_marker() {
    let mut cell = GreeterCell::new();
    let id = CellId::new("greeter");
    let mut ctx = CellContext::new(&id, Layer::Exec);
    let cmd = GreetCmd {
        msg_id: MsgId::new("m1"),
        correlation_id: CorrelationId::new("c1"),
        vector_clock: VectorClock::new(),
        name: "world".to_string(),
    };
    let layered = ctx.as_layered::<ExecLayer>();
    let (result, _outgoing, _witnesses) = cell.handle(cmd, layered).await;
    result.unwrap();
    assert_eq!(cell.greeted, vec!["world"]);
}

#[test]
fn test_cell_macro_layer_of() {
    assert_eq!(<GreeterCell as LayerOf>::LAYER, Layer::Exec);
}

#[test]
fn test_exec_cell_marker_is_present() {
    fn assert_exec<T: ExecCell>() {}
    assert_exec::<GreeterCell>();
}

#[test]
fn test_schema_version_macro() {
    assert_eq!(
        <V2Signal as Versioned>::schema_version(),
        SchemaVersion::new(2)
    );
}

#[test]
fn test_migration_macro_versions() {
    let m = MigrateV1toV2;
    assert_eq!(m.source_version(), SchemaVersion::new(1));
    assert_eq!(m.target_version(), SchemaVersion::new(2));
}

#[test]
fn test_migration_registry_discovery() {
    let migrations = axiom_core::registered_migration_chains();
    assert!(
        !migrations.is_empty(),
        "migration registry should contain at least MigrateV1toV2"
    );
    let found_v1_to_v2 = migrations
        .iter()
        .any(|(from, to, _, _)| *from == 1 && *to == 2);
    assert!(
        found_v1_to_v2,
        "expected migration 1->2 to be registered via linkme, got: {:?}",
        migrations
    );
}

#[test]
fn test_axiom_registry_discovery() {
    let count = axiom_core::count_registered_axioms();
    assert!(
        count >= 1,
        "expected at least 1 axiom registered, got {}",
        count
    );
}

#[test]
fn test_axiom_macro_adds_debug() {
    let a = TestAxiom;
    let _ = format!("{:?}", a);
    assert_eq!(a.name(), "TestAxiom");
}
