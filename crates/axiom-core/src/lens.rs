//! Lens - On-demand state projection from the event log.
//!
//! Instead of stuffing all history into a context window, Lenses project
//! exactly the state needed at the right granularity, with permission boundaries.

use crate::Result;
use async_trait::async_trait;

/// A Lens projects a view of state from the event log.
#[async_trait]
pub trait Lens: Send + Sync {
    /// Type of state this lens projects.
    type View: Send + Sync;

    /// Unique lens identifier (also used for permission boundaries).
    fn lens_id(&self) -> &'static str;

    /// Project the current state view.
    async fn project(&self) -> Result<Self::View>;

    /// Project state as of a specific Vector Clock (time travel).
    async fn project_at(&self, clock: &crate::signal::VectorClock) -> Result<Self::View>;
}

/// Composable lens - combine two lenses into one.
pub struct Lens2<L1, L2> {
    l1: L1,
    l2: L2,
}

impl<L1, L2> Lens2<L1, L2> {
    pub fn new(l1: L1, l2: L2) -> Self {
        Self { l1, l2 }
    }
}

#[async_trait]
impl<L1: Lens, L2: Lens> Lens for Lens2<L1, L2> {
    type View = (L1::View, L2::View);

    fn lens_id(&self) -> &'static str {
        "Lens2"
    }

    async fn project(&self) -> Result<Self::View> {
        let v1 = self.l1.project().await?;
        let v2 = self.l2.project().await?;
        Ok((v1, v2))
    }

    async fn project_at(&self, clock: &crate::signal::VectorClock) -> Result<Self::View> {
        let v1 = self.l1.project_at(clock).await?;
        let v2 = self.l2.project_at(clock).await?;
        Ok((v1, v2))
    }
}
