//! Cell - Isolated stateful unit with private state + message mailbox.
//!
//! Every Cell belongs to exactly one Layer. The layer is enforced at compile time
//! through specialized traits: ExecCell, ValidateCell, AgentCell, OversightCell.
//!
//! Each layer-specific CellContext only exposes the send methods that are legal
//! for that layer, preventing illegal cross-layer calls at compile time.

use crate::context::CellContext;
use crate::id::CellId;
use crate::layer::Layer;
use crate::signal::Signal;
use serde::{Deserialize, Serialize};

pub mod state {
    pub struct Created;
    pub struct Running;
    pub struct Suspended;
    pub struct Crashed;
    pub struct Stopped;
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SupervisionStrategy {
    Restart {
        max_retries: u32,
    },
    Stop,
    Escalate,
    CircuitBreak {
        failure_threshold: u32,
        reset_after_ms: u64,
    },
}

impl Default for SupervisionStrategy {
    fn default() -> Self {
        SupervisionStrategy::Restart { max_retries: 3 }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CellHealth {
    Healthy,
    Degraded { warnings: u32 },
    Unhealthy,
    Crashed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellMeta {
    pub cell_id: String,
    pub layer: Layer,
    pub supervision: SupervisionStrategy,
}

pub trait Cell: Send + 'static {
    type Message: Signal;
    fn id(&self) -> &CellId;
    fn layer() -> Layer
    where
        Self: Sized;
    fn supervision_strategy(&self) -> SupervisionStrategy {
        SupervisionStrategy::default()
    }
    fn heartbeat_interval_ms(&self) -> Option<u64> {
        None
    }

    async fn on_start(&mut self, _ctx: &mut CellContext<'_>) -> crate::Result<()> {
        Ok(())
    }
    async fn handle(
        &mut self,
        signal: Self::Message,
        ctx: &mut CellContext<'_>,
    ) -> crate::Result<()>;
    async fn on_stop(&mut self, _ctx: &mut CellContext<'_>) -> crate::Result<()> {
        Ok(())
    }

    fn state_hash(&self) -> Option<[u8; 32]> {
        None
    }
}

pub trait DynCell: Send + 'static {
    fn id(&self) -> &CellId;
    fn layer(&self) -> Layer;
    fn supervision_strategy(&self) -> SupervisionStrategy;
    fn meta(&self) -> CellMeta;
    fn state_hash(&self) -> Option<[u8; 32]>;
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

impl<C: Cell> DynCell for C {
    fn id(&self) -> &CellId {
        self.id()
    }
    fn layer(&self) -> Layer {
        C::layer()
    }
    fn supervision_strategy(&self) -> SupervisionStrategy {
        self.supervision_strategy()
    }
    fn meta(&self) -> CellMeta {
        CellMeta {
            cell_id: self.id().as_str().to_string(),
            layer: C::layer(),
            supervision: self.supervision_strategy(),
        }
    }
    fn state_hash(&self) -> Option<[u8; 32]> {
        self.state_hash()
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub struct CellHandle {
    inner: Box<dyn DynCell>,
}

impl CellHandle {
    pub fn new<C: Cell + 'static>(cell: C) -> Self {
        Self {
            inner: Box::new(cell),
        }
    }

    pub fn downcast_ref<C: Cell + 'static>(&self) -> Option<&C> {
        self.inner.as_any().downcast_ref::<C>()
    }

    pub fn downcast_mut<C: Cell + 'static>(&mut self) -> Option<&mut C> {
        self.inner.as_any_mut().downcast_mut::<C>()
    }
}

impl std::ops::Deref for CellHandle {
    type Target = dyn DynCell;
    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}

pub trait ExecCell: Cell {}
pub trait ValidateCell: Cell {}
pub trait AgentCell: Cell {}
pub trait OversightCell: Cell {}

pub trait LayerOf {
    const LAYER: Layer;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::CellContext;
    use crate::id::{CorrelationId, MsgId};
    use crate::schema::ValidationResult;
    use crate::signal::{now_ns, SignalKind, VectorClock};

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    struct ExecCmd {
        msg_id: MsgId,
        correlation_id: CorrelationId,
        vector_clock: VectorClock,
        data: String,
    }

    impl crate::signal::Signal for ExecCmd {
        fn signal_type(&self) -> &'static str {
            "ExecCmd"
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
        fn clone_signal(&self) -> Box<dyn crate::signal::Signal> {
            Box::new(self.clone())
        }
        fn validate(&self) -> ValidationResult {
            ValidationResult::ok()
        }
        fn serialize_to_json(&self) -> serde_json::Value {
            serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
        }
    }

    struct TestExecCell {
        id: CellId,
        received: Vec<String>,
    }

    impl TestExecCell {
        fn new() -> Self {
            Self {
                id: CellId::new("test-exec"),
                received: Vec::new(),
            }
        }
    }

    impl Cell for TestExecCell {
        type Message = ExecCmd;
        fn id(&self) -> &CellId {
            &self.id
        }
        fn layer() -> Layer {
            Layer::Exec
        }

        async fn handle(&mut self, signal: ExecCmd, _ctx: &mut CellContext<'_>) -> crate::Result<()> {
            self.received.push(signal.data);
            Ok(())
        }
    }

    impl ExecCell for TestExecCell {}

    #[tokio::test]
    async fn test_exec_cell_receives_message() {
        let mut cell = TestExecCell::new();
        let cmd = ExecCmd {
            msg_id: MsgId::new("m1"),
            correlation_id: CorrelationId::new("c1"),
            vector_clock: VectorClock::new(),
            data: "hello".to_string(),
        };
        let cell_id = CellId::new("test-exec");
        let mut ctx = CellContext::new(&cell_id, Layer::Exec);
        cell.handle(cmd, &mut ctx).await.unwrap();
        assert_eq!(cell.received, vec!["hello"]);
    }

    #[test]
    fn test_cell_handle_downcast() {
        let cell = TestExecCell::new();
        let handle = CellHandle::new(cell);
        assert!(handle.downcast_ref::<TestExecCell>().is_some());
        assert_eq!(handle.id().as_str(), "test-exec");
        assert_eq!(handle.layer(), Layer::Exec);
    }
}
