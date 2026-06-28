use std::process::ExitCode;

use axiom_cli::commands::Cli;
use clap::Parser;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let cli = if args.get(1).map(|s| s.as_str()) == Some("axiom") {
        Cli::parse_from(&args[2..])
    } else {
        Cli::parse_from(&args[1..])
    };
    match axiom_cli::commands::run(&cli) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("Error: {e}");
            ExitCode::from(1)
        }
    }
}
