use crate::axiom::{KernelError, KernelResult, Message};
use crate::context::{CellContext, OutgoingEnvelope, OutgoingWitness};
use crate::id::CellId;
use crate::signal::SignalEnvelope;
use crate::HeatmapCollector;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use tokio::sync::RwLock;

pub type BoxHandleFuture<'a> = Pin<
    Box<
        dyn Future<Output = (KernelResult<()>, Vec<OutgoingEnvelope>, Vec<OutgoingWitness>)>
            + Send
            + 'a,
    >,
>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CellKind {
    Exec,
    Validate,
    Agent,
    Oversight,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SupervisionStrategy {
    Restart { max_retries: u32 },
    Stop,
    Escalate,
    CircuitBreak { failure_threshold: u32, reset_after_ms: u64 },
}

impl Default for SupervisionStrategy {
    fn default() -> Self {
        SupervisionStrategy::Restart { max_retries: 3 }
    }
}

#[derive(Debug, Clone)]
pub struct CellHandle {
    pub id: CellId,
    pub kind: CellKind,
}

pub trait DynCell: Send + 'static {
    fn id(&self) -> &CellId;
    fn layer(&self) -> crate::Layer;
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

pub trait DynHandleCell: DynCell {
    fn handle_dyn<'a>(
        &'a mut self,
        env: SignalEnvelope,
        ctx: &'a mut CellContext<'a>,
    ) -> BoxHandleFuture<'a>;
}

pub struct RuntimeCellHandle {
    inner: Box<dyn DynHandleCell>,
}

impl RuntimeCellHandle {
    pub fn new(inner: Box<dyn DynHandleCell>) -> Self {
        Self { inner }
    }

    pub fn downcast_ref<C: 'static>(&self) -> Option<&C> {
        self.inner.as_any().downcast_ref::<C>()
    }

    pub fn downcast_mut<C: 'static>(&mut self) -> Option<&mut C> {
        self.inner.as_any_mut().downcast_mut::<C>()
    }

    pub fn handle_dyn<'a>(
        &'a mut self,
        env: SignalEnvelope,
        ctx: &'a mut CellContext<'a>,
    ) -> BoxHandleFuture<'a> {
        self.inner.handle_dyn(env, ctx)
    }
}

impl std::ops::Deref for RuntimeCellHandle {
    type Target = dyn DynHandleCell;
    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}

pub trait Cell: Send + Sync {
    fn cell_id(&self) -> CellId;
    fn cell_kind(&self) -> CellKind;
}

pub struct CellKernel {
    cells: RwLock<Vec<(CellHandle, CellState)>>,
    heatmap: std::sync::Arc<RwLock<HeatmapCollector>>,
}

impl CellKernel {
    pub fn new() -> Self {
        Self {
            cells: RwLock::new(Vec::new()),
            heatmap: std::sync::Arc::new(RwLock::new(HeatmapCollector::new())),
        }
    }

    pub fn with_heatmap(heatmap: std::sync::Arc<RwLock<HeatmapCollector>>) -> Self {
        Self { cells: RwLock::new(Vec::new()), heatmap }
    }

    pub fn heatmap(&self) -> std::sync::Arc<RwLock<HeatmapCollector>> {
        self.heatmap.clone()
    }

    pub async fn create(&self, kind: CellKind) -> CellHandle {
        let handle = CellHandle { id: CellId::new(uuid::Uuid::new_v4().to_string()), kind };
        let mut cells = self.cells.write().await;
        cells.push((handle.clone(), CellState::new()));
        handle
    }

    pub async fn send(&self, handle: &CellHandle, msg: Message) -> KernelResult<()> {
        let mut cells = self.cells.write().await;
        if let Some((_, state)) = cells.iter_mut().find(|(h, _)| h.id == handle.id) {
            state.inbox.push_back(msg);
            drop(cells);
            self.heatmap.write().await.record_cell_message(handle.id.to_string());
            Ok(())
        } else {
            Err(KernelError::CellNotFound(handle.id.to_string()))
        }
    }

    pub async fn receive(&self, handle: &CellHandle) -> KernelResult<Option<Message>> {
        let mut cells = self.cells.write().await;
        if let Some((_, state)) = cells.iter_mut().find(|(h, _)| h.id == handle.id) {
            Ok(state.inbox.pop_front())
        } else {
            Err(KernelError::CellNotFound(handle.id.to_string()))
        }
    }

    pub async fn count(&self) -> usize {
        self.cells.read().await.len()
    }

    pub async fn list(&self) -> Vec<(CellHandle, usize)> {
        let cells = self.cells.read().await;
        cells.iter().map(|(handle, state)| (handle.clone(), state.inbox.len())).collect()
    }
}

impl Default for CellKernel {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
struct CellState {
    inbox: std::collections::VecDeque<Message>,
}

impl CellState {
    fn new() -> Self {
        Self { inbox: std::collections::VecDeque::new() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellStatus {
    pub id: String,
    pub kind: String,
    pub queued: usize,
}

impl From<(CellHandle, usize)> for CellStatus {
    fn from((handle, queued): (CellHandle, usize)) -> Self {
        Self { id: handle.id.to_string(), kind: format!("{:?}", handle.kind), queued }
    }
}

impl CellKernel {
    pub async fn status(&self) -> Vec<CellStatus> {
        self.list().await.into_iter().map(Into::into).collect()
    }
}
