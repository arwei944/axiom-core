//! Cell - Isolated stateful unit with private state + message mailbox.
//!
//! Every Cell belongs to exactly one Layer. The layer is enforced at compile time
//! through specialized traits: ExecCell, ValidateCell, AgentCell, OversightCell.
//!
//! Each layer-specific CellContext only exposes the send methods that are legal
//! for that layer, preventing illegal cross-layer calls at compile time.

use crate::context::{CellContext, LayeredCellContext, OutgoingEnvelope, OutgoingWitness};
use crate::id::CellId;
use crate::layer::Layer;
use crate::sealed::LayerMarker;
use crate::signal::{Signal, SignalEnvelope};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;

/// Type alias for the boxed future returned by `handle_dyn`.
///
/// Without this alias, clippy flags the raw `Pin<Box<dyn Future<...>>>` type as
/// `type_complexity` in the trait declaration, impl, and wrapper.
pub type BoxHandleFuture<'a> = Pin<
    Box<
        dyn Future<
                Output = (
                    crate::Result<()>,
                    Vec<OutgoingEnvelope>,
                    Vec<OutgoingWitness>,
                ),
            > + Send
            + 'a,
    >,
>;

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
    type Layer: LayerMarker;
    fn id(&self) -> &CellId;
    fn layer() -> Layer
    where
        Self: Sized,
    {
        Self::Layer::LAYER
    }
    fn supervision_strategy(&self) -> SupervisionStrategy {
        SupervisionStrategy::default()
    }
    fn heartbeat_interval_ms(&self) -> Option<u64> {
        None
    }

    fn on_start<'a>(
        &'a mut self,
        _ctx: &'a mut CellContext<'a>,
    ) -> impl Future<Output = crate::Result<()>> + Send + 'a {
        async { Ok(()) }
    }
    /// Handle a signal and return outgoing envelopes + witnesses drained from `ctx`.
    ///
    /// Implementations receive a `LayeredCellContext<Self::Layer>` which only
    /// exposes `send_to` / `emit_to` methods for target layers allowed by the
    /// architecture. Illegal cross-layer calls are rejected at compile time.
    ///
    /// Implementations MUST call `ctx.end_processing()` before returning, because
    /// the framework does NOT access `ctx` after this future resolves. This design
    /// avoids borrow-checker conflicts with the opaque `impl Future + 'a` return
    /// type, which ties the `&mut ctx` borrow to the entire function scope.
    fn handle<'a>(
        &'a mut self,
        signal: Self::Message,
        ctx: LayeredCellContext<'a, Self::Layer>,
    ) -> impl Future<
        Output = (
            crate::Result<()>,
            Vec<OutgoingEnvelope>,
            Vec<OutgoingWitness>,
        ),
    > + Send
           + 'a;
    fn on_stop<'a>(
        &'a mut self,
        _ctx: &'a mut CellContext<'a>,
    ) -> impl Future<Output = crate::Result<()>> + Send + 'a {
        async { Ok(()) }
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

/// Type-erased cell capable of dispatching `SignalEnvelope` payloads to `Cell::handle`.
///
/// Requires `Cell::Message: for<'de> Deserialize<'de>` so that the type-erased
/// JSON payload inside a `SignalEnvelope` can be deserialized back into the
/// strongly-typed `Self::Message` before invoking `Cell::handle`.
///
/// `handle_dyn` also calls `CellContext::end_processing()` internally and returns
/// the outgoing envelopes, so callers do not need to touch `ctx` after the call.
/// This avoids borrow-checker conflicts with boxed futures capturing `&mut ctx`.
pub trait DynHandleCell: DynCell {
    fn handle_dyn<'a>(
        &'a mut self,
        env: SignalEnvelope,
        ctx: &'a mut CellContext<'a>,
    ) -> BoxHandleFuture<'a>;
}

impl<C> DynHandleCell for C
where
    C: Cell,
    C::Message: for<'de> serde::Deserialize<'de>,
{
    fn handle_dyn<'a>(
        &'a mut self,
        env: SignalEnvelope,
        ctx: &'a mut CellContext<'a>,
    ) -> BoxHandleFuture<'a> {
        Box::pin(async move {
            let msg: C::Message = match serde_json::from_value(env.payload) {
                Ok(m) => m,
                Err(e) => {
                    // ctx is not yet borrowed by handle(), so we can drain here.
                    let outgoing = ctx.take_outgoing();
                    let witnesses = ctx.take_witnesses();
                    return (
                        Err(crate::AxiomError::SignalSerialization {
                            signal_type: env.signal_type.clone(),
                            message: format!("dispatch deserialize: {e}"),
                        }),
                        outgoing,
                        witnesses,
                    );
                }
            };
            // handle() returns (Result, Vec<OutgoingEnvelope>, Vec<OutgoingWitness>)
            // and drains ctx internally via end_processing().
            let layered = ctx.as_layered::<C::Layer>();
            let (result, outgoing, witnesses) = self.handle(msg, layered).await;
            (result, outgoing, witnesses)
        })
    }
}

pub struct CellHandle {
    inner: Box<dyn DynHandleCell>,
}

impl CellHandle {
    pub fn new<C>(cell: C) -> Self
    where
        C: Cell,
        C::Message: for<'de> serde::Deserialize<'de>,
    {
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

    /// Dispatch a type-erased `SignalEnvelope` to the wrapped cell's `handle` method.
    ///
    /// Returns the handle result plus outgoing envelopes collected via
    /// `CellContext::end_processing()`.
    pub fn handle_dyn<'a>(
        &'a mut self,
        env: SignalEnvelope,
        ctx: &'a mut CellContext<'a>,
    ) -> BoxHandleFuture<'a> {
        self.inner.handle_dyn(env, ctx)
    }
}

impl std::ops::Deref for CellHandle {
    type Target = dyn DynHandleCell;
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
        fn serialize_to_json(&self) -> crate::Result<serde_json::Value> {
            serde_json::to_value(self).map_err(|e| crate::AxiomError::SignalSerialization {
                signal_type: "TestSignal".into(),
                message: e.to_string(),
            })
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
        type Layer = crate::sealed::ExecLayer;
        fn id(&self) -> &CellId {
            &self.id
        }

        #[allow(clippy::manual_async_fn)]
        fn handle<'a>(
            &'a mut self,
            signal: ExecCmd,
            ctx: LayeredCellContext<'a, Self::Layer>,
        ) -> impl Future<
            Output = (
                crate::Result<()>,
                Vec<OutgoingEnvelope>,
                Vec<OutgoingWitness>,
            ),
        > + Send
               + 'a {
            async move {
                let mut ctx = ctx;
                self.received.push(signal.data);
                let (outgoing, witnesses) = ctx.end_processing();
                (Ok(()), outgoing, witnesses)
            }
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
        let layered = ctx.as_layered::<crate::sealed::ExecLayer>();
        let (result, _outgoing, _witnesses) = cell.handle(cmd, layered).await;
        result.unwrap();
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
