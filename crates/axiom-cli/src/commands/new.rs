use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Args;

#[derive(Args)]
pub struct NewArgs {
    /// Name of the project to create
    pub name: String,

    /// Skip git repository initialization
    #[arg(long)]
    pub no_git: bool,

    /// Use a minimal template instead of the full template
    #[arg(long)]
    pub minimal: bool,
}

pub fn run_new(args: &NewArgs) -> Result<std::process::ExitCode> {
    let project_path = PathBuf::from(&args.name);

    if project_path.exists() {
        return Err(anyhow::anyhow!("Directory '{}' already exists", args.name));
    }

    println!("Creating axiom project '{}'...", args.name);

    create_project_structure(&project_path, args.minimal)?;
    create_cargo_toml(&project_path, &args.name)?;
    create_main_rs(&project_path)?;
    create_cells_mod(&project_path)?;
    create_signals_mod(&project_path)?;
    create_axioms_mod(&project_path)?;
    create_config_toml(&project_path)?;

    if !args.no_git {
        init_git(&project_path)?;
    }

    println!("\nProject created successfully!");
    println!("Run `cd {} && cargo run` to start", args.name);

    Ok(std::process::ExitCode::SUCCESS)
}

fn create_project_structure(project_path: &Path, minimal: bool) -> Result<()> {
    fs::create_dir_all(project_path).context("Failed to create project directory")?;

    fs::create_dir_all(project_path.join("src/cells"))
        .context("Failed to create cells directory")?;

    fs::create_dir_all(project_path.join("src/signals"))
        .context("Failed to create signals directory")?;

    fs::create_dir_all(project_path.join("src/axioms"))
        .context("Failed to create axioms directory")?;

    fs::create_dir_all(project_path.join(".axiom")).context("Failed to create .axiom directory")?;

    if !minimal {
        fs::create_dir_all(project_path.join("tests"))
            .context("Failed to create tests directory")?;
        fs::create_dir_all(project_path.join("examples"))
            .context("Failed to create examples directory")?;
    }

    Ok(())
}

fn create_cargo_toml(project_path: &Path, name: &str) -> Result<()> {
    let content = format!(
        r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
axiom-kernel = {{ path = "../../crates/axiom-kernel" }}
axiom-runtime = {{ path = "../../crates/axiom-runtime" }}
axiom-store = {{ path = "../../crates/axiom-store" }}
axiom-oversight = {{ path = "../../crates/axiom-oversight" }}
axiom-macros = {{ path = "../../crates/axiom-macros" }}

serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
tokio = {{ version = "1.0", features = ["full"] }}
tracing = "0.1"
tracing-subscriber = {{ version = "0.3", features = ["fmt"] }}

[dev-dependencies]
tokio = {{ version = "1.0", features = ["full"] }}
"#,
        name
    );

    let mut file =
        File::create(project_path.join("Cargo.toml")).context("Failed to create Cargo.toml")?;
    file.write_all(content.as_bytes())
        .context("Failed to write Cargo.toml")?;

    Ok(())
}

fn create_main_rs(project_path: &Path) -> Result<()> {
    let content = r#"use axiom_kernel::id::CellId;
use axiom_kernel::layer::Layer;
use axiom_runtime::RuntimeBuilder;
use tracing_subscriber::fmt::format::FmtSpan;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_span_events(FmtSpan::ACTIVE)
        .init();

    let rt = RuntimeBuilder::new().build();

    rt.register_cell(axiom_runtime::CellRegistration {
        id: CellId::new("hello-cell"),
        layer: Layer::Exec,
        version: axiom_kernel::version::Version::new(0, 1, 0),
        supervision_strategy: axiom_kernel::cell::SupervisionStrategy::default(),
        cell: None,
        factory: None,
    })
    .await?;

    rt.start().await?;

    Ok(())
}
"#;

    let mut file =
        File::create(project_path.join("src/main.rs")).context("Failed to create main.rs")?;
    file.write_all(content.as_bytes())
        .context("Failed to write main.rs")?;

    Ok(())
}

fn create_cells_mod(project_path: &Path) -> Result<()> {
    let content = r#"pub mod hello_cell;
"#;

    let mut file = File::create(project_path.join("src/cells/mod.rs"))
        .context("Failed to create cells/mod.rs")?;
    file.write_all(content.as_bytes())
        .context("Failed to write cells/mod.rs")?;

    let hello_cell_content = r#"use axiom_kernel::cell::Cell;
use axiom_kernel::context::CellContext;
use axiom_kernel::error::AxiomError;
use axiom_kernel::layer::ExecLayer;

#[derive(Debug, Default)]
pub struct HelloCell;

#[async_trait::async_trait]
impl Cell for HelloCell {
    type Message = HelloSignal;
    type Layer = ExecLayer;

    fn id(&self) -> &axiom_kernel::id::CellId {
        static ID: axiom_kernel::id::CellId = axiom_kernel::id::CellId::new_static("hello-cell");
        &ID
    }

    async fn handle(
        &mut self,
        msg: Self::Message,
        ctx: &mut CellContext<'_>,
    ) -> Result<(), AxiomError> {
        tracing::info!("HelloCell received: {}", msg.name);
        Ok(())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HelloSignal {
    pub name: String,
}

impl axiom_kernel::signal::Signal for HelloSignal {
    fn signal_type(&self) -> &str {
        "HelloSignal"
    }
}
"#;

    let mut file = File::create(project_path.join("src/cells/hello_cell.rs"))
        .context("Failed to create hello_cell.rs")?;
    file.write_all(hello_cell_content.as_bytes())
        .context("Failed to write hello_cell.rs")?;

    Ok(())
}

fn create_signals_mod(project_path: &Path) -> Result<()> {
    let content = r#"pub mod hello_signal;
"#;

    let mut file = File::create(project_path.join("src/signals/mod.rs"))
        .context("Failed to create signals/mod.rs")?;
    file.write_all(content.as_bytes())
        .context("Failed to write signals/mod.rs")?;

    let hello_signal_content = r#"#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HelloSignal {
    pub name: String,
}

impl axiom_kernel::signal::Signal for HelloSignal {
    fn signal_type(&self) -> &str {
        "HelloSignal"
    }
}
"#;

    let mut file = File::create(project_path.join("src/signals/hello_signal.rs"))
        .context("Failed to create hello_signal.rs")?;
    file.write_all(hello_signal_content.as_bytes())
        .context("Failed to write hello_signal.rs")?;

    Ok(())
}

fn create_axioms_mod(project_path: &Path) -> Result<()> {
    let content = r#"pub mod example_axiom;
"#;

    let mut file = File::create(project_path.join("src/axioms/mod.rs"))
        .context("Failed to create axioms/mod.rs")?;
    file.write_all(content.as_bytes())
        .context("Failed to write axioms/mod.rs")?;

    let example_axiom_content = r#"use axiom_kernel::axiom::Axiom;
use axiom_kernel::id::CellId;

pub struct ExampleAxiom;

impl Axiom for ExampleAxiom {
    fn name(&self) -> &str {
        "example-axiom"
    }

    fn check(
        &self,
        _cell_id: &CellId,
        _signal_type: &str,
        _payload: &serde_json::Value,
    ) -> Result<(), axiom_kernel::error::AxiomError> {
        Ok(())
    }
}
"#;

    let mut file = File::create(project_path.join("src/axioms/example_axiom.rs"))
        .context("Failed to create example_axiom.rs")?;
    file.write_all(example_axiom_content.as_bytes())
        .context("Failed to write example_axiom.rs")?;

    Ok(())
}

fn create_config_toml(project_path: &Path) -> Result<()> {
    let content = r#"[runtime]
mailbox_capacity = 1024
dispatch_poll_interval_ms = 10

[entropy]
threshold = 100.0
cooldown_ms = 60000

[logging]
level = "debug"
format = "pretty"
"#;

    let mut file = File::create(project_path.join(".axiom/config.toml"))
        .context("Failed to create .axiom/config.toml")?;
    file.write_all(content.as_bytes())
        .context("Failed to write .axiom/config.toml")?;

    Ok(())
}

fn init_git(project_path: &Path) -> Result<()> {
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(project_path)
        .status()
        .context("Failed to initialize git repository")?;

    let gitignore_content = r#"target/
Cargo.lock
.axiom/.constraints.lock
"#;

    let mut file =
        File::create(project_path.join(".gitignore")).context("Failed to create .gitignore")?;
    file.write_all(gitignore_content.as_bytes())
        .context("Failed to write .gitignore")?;

    Ok(())
}
