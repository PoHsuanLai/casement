//! The `casement` binary.

use casement::cli::{self, Command};
use std::process::ExitCode;

fn main() -> ExitCode {
    casement::log::init();
    let args: Vec<String> = std::env::args().skip(1).collect();
    match cli::parse_args(&args) {
        Ok(Command::Help) => {
            print!("{}", cli::USAGE);
            ExitCode::SUCCESS
        }
        Ok(Command::WriteSchema(dir)) => match casement::config::write_schema(&dir) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("casement: {error}");
                ExitCode::FAILURE
            }
        },
        Ok(Command::Run(config)) => run(config),
        Err(error) => {
            eprintln!("casement: {error}\n{}", cli::USAGE);
            ExitCode::from(64)
        }
    }
}

/// Runs the compositor until it is asked to stop. A binary built with `testing` or
/// `debug-inject` refuses every backend but headless and refuses to start outside the jail
/// (exit 5, §3.2).
fn run(config: cli::StartConfig) -> ExitCode {
    let _ = config;
    todo!("P0-B-loop: main::run")
}
