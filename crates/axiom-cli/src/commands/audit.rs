use std::path::PathBuf;

use clap::Args;
use std::process::ExitCode;

use crate::checks::Check;
use crate::checks::deps_audit::DepsAuditCheck;

#[derive(Debug, Args, Clone)]
pub struct AuditArgs {
    /// Path to audit.toml policy file (default: .axiom/audit.toml)
    #[arg(long, value_name = "FILE")]
    pub policy: Option<PathBuf>,

    /// Output machine-readable JSON report
    #[arg(long)]
    pub json: bool,

    /// Suggest safe version bumps for vulnerable dependencies
    #[arg(long)]
    pub fix: bool,
}

pub fn run_audit(args: &AuditArgs) -> anyhow::Result<ExitCode> {
    let policy_path = args
        .policy
        .clone()
        .unwrap_or_else(|| PathBuf::from(".axiom/audit.toml"));

    let check = DepsAuditCheck::new(policy_path, args.json, args.fix);
    let result = check.run();

    if result.passed {
        if args.json {
            let report = serde_json::json!({
                "status": "passed",
                "violations": [],
                "message": result.message,
            });
            println!("{}", serde_json::to_string_pretty(&report)?);
        } else {
            println!("✓ {}", result.message);
        }
        Ok(ExitCode::SUCCESS)
    } else {
        if args.json {
            let report = serde_json::json!({
                "status": "failed",
                "violations": result.message.lines().collect::<Vec<_>>(),
                "message": result.message,
            });
            println!("{}", serde_json::to_string_pretty(&report)?);
        } else {
            eprintln!("✗ {}", result.message);
        }
        Ok(ExitCode::from(1))
    }
}
