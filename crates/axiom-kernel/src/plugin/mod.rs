pub mod abi;
pub mod composer;
pub mod kernel_bridge;
pub mod loader;
pub mod package;
pub mod registry;
pub mod sandbox;
pub mod version;

pub use abi::{
    AxiomPlugin, CapabilityDescriptor, PluginContext, PluginError, PluginKind, PluginMessage,
    PluginReply,
};
pub use composer::Composer;
pub use kernel_bridge::RuntimeKernelBridge;
pub use loader::NativePluginLoader;
pub use package::{pack, pack_to_file, unpack, unpack_from_file, PluginManifest, PluginPackage};
pub use registry::PluginRegistry;
pub use sandbox::{
    NativePluginSandbox, PluginSandbox, SandboxLimits, SandboxOutcome, WasmPluginSandbox,
};
pub use version::{load_index, Dependency, PluginVersion, RepositoryIndex};
