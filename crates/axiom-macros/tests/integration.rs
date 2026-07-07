use axiom_kernel::axiom::{Axiom, KernelError, KernelResult, ValidationResult};
use axiom_kernel::cell::{Cell, CellKind};
use axiom_kernel::context::CellContext;
use axiom_kernel::id::{CellId, CorrelationId, MsgId};
use axiom_kernel::layer::Layer;
use axiom_kernel::registry::{count_registered_axioms, registered_migration_chains};
use axiom_kernel::signal::{now_ns, Signal, SignalKind, VectorClock};
use axiom_kernel::version::{Migration, SchemaVersion, Versioned};
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
    fn serialize_to_json(&self) -> KernelResult<serde_json::Value> {
        serde_json::to_value(self).map_err(|e| KernelError::SignalSerialization {
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

impl Cell for GreeterCell {
    fn cell_id(&self) -> CellId {
        self.id.clone()
    }

    fn cell_kind(&self) -> CellKind {
        CellKind::Exec
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
    fn serialize_to_json(&self) -> KernelResult<serde_json::Value> {
        serde_json::to_value(self).map_err(|e| KernelError::SignalSerialization {
            signal_type: "Greet".into(),
            message: e.to_string(),
        })
    }
}

#[derive(Debug)]
struct MigrateV1toV2;

#[axiom_macros::migration(from = 1)]
impl Migration for MigrateV1toV2 {
    fn migrate(&self, input: Value) -> KernelResult<Value> {
        Ok(input)
    }
}

#[axiom_macros::axiom]
#[derive(Default)]
struct TestAxiom;

impl Axiom for TestAxiom {
    type State = i32;
    type Message = GreetCmd;

    fn name(&self) -> &'static str {
        "TestAxiom"
    }

    fn check(&self, _current: &i32, new: &i32, _msg: &GreetCmd) -> KernelResult<()> {
        if *new < 0 {
            Err(KernelError::InvariantViolated {
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
    let outgoing = ctx.take_outgoing();
    let witnesses = ctx.take_witnesses();
    cell.greeted.push(cmd.name);
    assert_eq!(cell.greeted, vec!["world"]);
    assert!(outgoing.is_empty());
    assert!(witnesses.is_empty());
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
    let migrations = registered_migration_chains();
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
    let count = count_registered_axioms();
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
