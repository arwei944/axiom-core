use crate::axiom::KernelResult;
use crate::RuntimeTier;
use crate::signal::Signal;

pub trait Guard: Send + Sync {
    fn id(&self) -> &'static str;
    fn layer(&self) -> Option<RuntimeTier>;
    fn check(&self, signal: &dyn Signal) -> KernelResult<()>;
}

pub trait DynGuard: 'static {
    fn id(&self) -> &'static str;
    fn layer(&self) -> Option<RuntimeTier>;
    fn check(&self, signal: &dyn Signal) -> KernelResult<()>;
    fn as_any(&self) -> &dyn std::any::Any;
}

impl<T: Guard + 'static> DynGuard for T {
    fn id(&self) -> &'static str {
        Guard::id(self)
    }
    fn layer(&self) -> Option<RuntimeTier> {
        Guard::layer(self)
    }
    fn check(&self, signal: &dyn Signal) -> KernelResult<()> {
        Guard::check(self, signal)
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub type BoxedGuard = Box<dyn DynGuard + Send + Sync>;
