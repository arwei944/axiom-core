use std::process::ExitCode;

use axiom_cli::commands::Cli;
use clap::Parser;

fn main() -> ExitCode {
    let cli = Cli::parse();
    match axiom_cli::commands::run(&cli) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("Error: {e}");
            ExitCode::from(1)
        }
    }
}
