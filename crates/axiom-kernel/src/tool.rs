use crate::axiom::KernelResult;
use std::any::Any;

pub trait Tool: Send + Sync {
    fn id(&self) -> &'static str;
    fn invoke(&self, args: Vec<u8>) -> KernelResult<Vec<u8>>;
    fn validate(&self, args: &[u8]) -> KernelResult<()> {
        let _ = args;
        Ok(())
    }
}

pub trait DynTool: 'static {
    fn id(&self) -> &'static str;
    fn invoke(&self, args: Vec<u8>) -> KernelResult<Vec<u8>>;
    fn validate(&self, args: &[u8]) -> KernelResult<()>;
    fn as_any(&self) -> &dyn Any;
}

impl<T: Tool + 'static> DynTool for T {
    fn id(&self) -> &'static str {
        Tool::id(self)
    }
    fn invoke(&self, args: Vec<u8>) -> KernelResult<Vec<u8>> {
        Tool::invoke(self, args)
    }
    fn validate(&self, args: &[u8]) -> KernelResult<()> {
        Tool::validate(self, args)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub type BoxedTool = Box<dyn DynTool + Send + Sync>;
