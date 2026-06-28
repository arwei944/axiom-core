//! Cell - Isolated stateful unit with private state + message mailbox.

use crate::signal::Signal;
use crate::Result;
use async_trait::async_trait;

/// Lifecycle states of a Cell (enforced by typestate pattern).
pub mod state {
    pub struct Created;
    pub struct Running;
    pub struct Suspended;
    pub struct Crashed;
    pub struct Stopped;
}

/// Unique identifier for a Cell.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct CellId(pub String);

/// Supervision strategy when a Cell crashes.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum SupervisionStrategy {
    /// Restart the cell with fresh state.
    Restart { max_retries: u32 },
    /// Stop the cell permanently.
    Stop,
    /// Escalate failure to parent supervisor.
    Escalate,
}

/// Core Cell trait - implement this to define a stateful unit.
#[async_trait]
pub trait Cell: Send + 'static {
    /// Type of messages this Cell can handle.
    type Message: Signal;

    /// The Cell's unique identifier.
    fn id(&self) -> &CellId;

    /// Called when the Cell starts (after being spawned).
    async fn on_start(&mut self) -> Result<()> {
        Ok(())
    }

    /// Handle an incoming signal/message.
    async fn handle(&mut self, signal: Self::Message) -> Result<()>;

    /// Called when the Cell is about to stop.
    async fn on_stop(&mut self) -> Result<()> {
        Ok(())
    }

    /// Supervision strategy for this cell.
    fn supervision_strategy(&self) -> SupervisionStrategy {
        SupervisionStrategy::Restart { max_retries: 3 }
    }
}
