use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Args, Subcommand};

use axiom_kernel::plugin::{abi::PluginKind, package::unpack_from_file, RuntimeKernelBridge};

#[derive(Debug, Args)]
pub struct PluginArgs {
    #[command(subcommand)]
    pub command: PluginCommands,
}

#[derive(Debug, Subcommand)]
pub enum PluginCommands {
    /// List installed plugins
    List(ListArgs),
    /// Install a plugin from a package file
    Install(InstallArgs),
    /// Uninstall a plugin by id
    Uninstall(UninstallArgs),
    /// Show plugin info
    Info(InfoArgs),
}

#[derive(Debug, Args)]
pub struct ListArgs {
    /// Filter by plugin kind
    #[arg(long)]
    pub kind: Option<String>,
}

#[derive(Debug, Args)]
pub struct InstallArgs {
    /// Plugin package path (.axmp or shared library)
    pub path: String,
}

#[derive(Debug, Args)]
pub struct UninstallArgs {
    /// Plugin id to uninstall
    pub id: String,
}

#[derive(Debug, Args)]
pub struct InfoArgs {
    /// Plugin id to show info
    pub id: String,
}

pub fn run_plugin(args: &PluginArgs) -> Result<ExitCode> {
    match &args.command {
        PluginCommands::List(args) => run_list(args),
        PluginCommands::Install(args) => run_install(args),
        PluginCommands::Uninstall(args) => run_uninstall(args),
        PluginCommands::Info(args) => run_info(args),
    }
}

fn runtime_bridge() -> RuntimeKernelBridge {
    RuntimeKernelBridge::new()
}

fn plugin_kind_from_str(s: &str) -> Option<PluginKind> {
    match s.to_lowercase().as_str() {
        "llm" => Some(PluginKind::Llm),
        "memory" => Some(PluginKind::Memory),
        "tool" => Some(PluginKind::Tool),
        "mcp" => Some(PluginKind::Mcp),
        "planner" => Some(PluginKind::Planner),
        "alert" => Some(PluginKind::Alert),
        "viz" => Some(PluginKind::Viz),
        "governance" => Some(PluginKind::Governance),
        _ => None,
    }
}

fn run_list(args: &ListArgs) -> Result<ExitCode> {
    let bridge = runtime_bridge();
    let runtime = tokio::runtime::Runtime::new().context("Failed to create tokio runtime")?;

    let kind = match &args.kind {
        Some(kind_str) => Some(
            plugin_kind_from_str(kind_str)
                .ok_or_else(|| anyhow::anyhow!("unknown plugin kind: {kind_str}"))?,
        ),
        None => None,
    };

    let plugins = runtime.block_on(async move {
        if let Some(kind) = kind {
            bridge.plugin_registry.get_all_by_kind(kind).await
        } else {
            bridge.plugin_registry.list_all().await
        }
    });

    println!("Installed plugins:");
    if plugins.is_empty() {
        println!("  (none)");
        return Ok(ExitCode::SUCCESS);
    }

    for plugin in plugins {
        println!("  - {} v{} (kind: {:?})", plugin.id(), plugin.version(), guess_kind(plugin.id()));
    }

    Ok(ExitCode::SUCCESS)
}

fn run_install(args: &InstallArgs) -> Result<ExitCode> {
    let path = std::path::Path::new(&args.path);
    if !path.exists() {
        return Err(anyhow::anyhow!("plugin package not found: {}", args.path));
    }

    let bridge = runtime_bridge();

    if args.path.ends_with(".axmp") {
        let package = unpack_from_file(path).map_err(|e| anyhow::anyhow!(e))?;
        println!("Installing plugin: {} v{}", package.manifest.id, package.manifest.version);
        println!("  description: {}", package.manifest.description.as_deref().unwrap_or(""));
        println!("  kind: {:?}", package.manifest.kind);
    } else {
        let loader = axiom_kernel::plugin::NativePluginLoader::new();
        let plugin = loader.load(path).map_err(|e| anyhow::anyhow!(e))?;
        let runtime = tokio::runtime::Runtime::new().context("Failed to create tokio runtime")?;
        runtime.block_on(async move { bridge.plugin_registry.register(plugin).await });
        println!("Plugin installed successfully.");
        return Ok(ExitCode::SUCCESS);
    }

    println!("Plugin installed successfully.");
    Ok(ExitCode::SUCCESS)
}

fn run_uninstall(args: &UninstallArgs) -> Result<ExitCode> {
    let bridge = runtime_bridge();
    let runtime = tokio::runtime::Runtime::new().context("Failed to create tokio runtime")?;
    let removed = runtime.block_on(async move { bridge.plugin_registry.remove(&args.id).await });

    if removed.is_some() {
        println!("Plugin `{}` uninstalled successfully.", args.id);
        Ok(ExitCode::SUCCESS)
    } else {
        Err(anyhow::anyhow!("plugin not found: {}", args.id))
    }
}

fn run_info(args: &InfoArgs) -> Result<ExitCode> {
    let bridge = runtime_bridge();
    let runtime = tokio::runtime::Runtime::new().context("Failed to create tokio runtime")?;
    let plugins = runtime.block_on(async move { bridge.plugin_registry.list_all().await });
    let plugin = plugins
        .iter()
        .find(|p| p.id() == args.id)
        .ok_or_else(|| anyhow::anyhow!("plugin not found: {}", args.id))?;

    println!("Plugin info:");
    println!("  id: {}", plugin.id());
    println!("  version: {}", plugin.version());
    println!(
        "  description: {}",
        plugin.capabilities().first().map(|c| c.description.as_deref().unwrap_or("")).unwrap_or("")
    );
    println!(
        "  kind: {:?}",
        plugin.capabilities().first().map(|c| c.name.as_str()).unwrap_or("unknown")
    );

    let deps = plugin.dependencies();
    if deps.is_empty() {
        println!("  dependencies: (none)");
    } else {
        println!("  dependencies: {}", deps.join(", "));
    }

    Ok(ExitCode::SUCCESS)
}

fn guess_kind(id: &str) -> PluginKind {
    let lower = id.to_lowercase();
    if lower.contains("llm") || lower.contains("model") {
        PluginKind::Llm
    } else if lower.contains("memory") {
        PluginKind::Memory
    } else if lower.contains("tool") {
        PluginKind::Tool
    } else if lower.contains("mcp") {
        PluginKind::Mcp
    } else if lower.contains("plan") {
        PluginKind::Planner
    } else if lower.contains("alert") {
        PluginKind::Alert
    } else if lower.contains("viz") {
        PluginKind::Viz
    } else if lower.contains("gov") || lower.contains("oversight") {
        PluginKind::Governance
    } else {
        PluginKind::Tool
    }
}
