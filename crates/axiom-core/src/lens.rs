//! Lens - On-demand state projection from the event log.
//!
//! Instead of stuffing all history into a context window, Lenses project
//! exactly the state needed at the right granularity, with permission boundaries
//! enforced by the type system (LensId as a compile-time marker).

use crate::id::LensId;
use crate::signal::VectorClock;
use crate::Result;

pub trait Lens: Send + Sync {
    type View: Send + Sync;

    fn lens_id(&self) -> LensId;
    async fn project(&self) -> Result<Self::View>;
    async fn project_at(&self, clock: &VectorClock) -> Result<Self::View>;
    fn token_estimate(&self) -> usize {
        0
    }
}

pub struct Lens2<L1, L2> {
    l1: L1,
    l2: L2,
}

impl<L1, L2> Lens2<L1, L2> {
    pub fn new(l1: L1, l2: L2) -> Self {
        Self { l1, l2 }
    }
}

impl<L1: Lens, L2: Lens> Lens for Lens2<L1, L2> {
    type View = (L1::View, L2::View);

    fn lens_id(&self) -> LensId {
        LensId::new("Lens2")
    }
    async fn project(&self) -> Result<Self::View> {
        let (v1, v2) = futures::join!(self.l1.project(), self.l2.project());
        Ok((v1?, v2?))
    }
    async fn project_at(&self, clock: &VectorClock) -> Result<Self::View> {
        let (v1, v2) = futures::join!(self.l1.project_at(clock), self.l2.project_at(clock));
        Ok((v1?, v2?))
    }
}

pub struct Lens3<L1, L2, L3> {
    l1: L1,
    l2: L2,
    l3: L3,
}

impl<L1, L2, L3> Lens3<L1, L2, L3> {
    pub fn new(l1: L1, l2: L2, l3: L3) -> Self {
        Self { l1, l2, l3 }
    }
}

impl<L1: Lens, L2: Lens, L3: Lens> Lens for Lens3<L1, L2, L3> {
    type View = (L1::View, L2::View, L3::View);

    fn lens_id(&self) -> LensId {
        LensId::new("Lens3")
    }
    async fn project(&self) -> Result<Self::View> {
        let (v1, v2, v3) = futures::join!(self.l1.project(), self.l2.project(), self.l3.project());
        Ok((v1?, v2?, v3?))
    }
    async fn project_at(&self, clock: &VectorClock) -> Result<Self::View> {
        let (v1, v2, v3) = futures::join!(
            self.l1.project_at(clock),
            self.l2.project_at(clock),
            self.l3.project_at(clock)
        );
        Ok((v1?, v2?, v3?))
    }
}

pub struct CachedLens<L: Lens> {
    inner: L,
    cached_at: Option<VectorClock>,
    cached_value: Option<L::View>,
}

impl<L: Lens> CachedLens<L>
where
    L::View: Clone,
{
    pub fn new(inner: L) -> Self {
        Self {
            inner,
            cached_at: None,
            cached_value: None,
        }
    }
    pub fn invalidate(&mut self) {
        self.cached_at = None;
        self.cached_value = None;
    }
}

impl<L: Lens + Clone> Lens for CachedLens<L>
where
    L::View: Clone,
{
    type View = L::View;
    fn lens_id(&self) -> LensId {
        self.inner.lens_id()
    }
    async fn project(&self) -> Result<Self::View> {
        self.inner.project().await
    }
    async fn project_at(&self, clock: &VectorClock) -> Result<Self::View> {
        if let (Some(cached_clock), Some(cached_value)) = (&self.cached_at, &self.cached_value) {
            if cached_clock == clock {
                return Ok(Clone::clone(cached_value));
            }
        }
        self.inner.project_at(clock).await
    }
}
