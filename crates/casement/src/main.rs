//! The `casement` binary: arguments, the environment and an exit code around [`casement::run`].

#[cfg(feature = "debug-inject")]
mod debug_peer_units;

use casement::cli::{self, Command};
use casement::environment::run_config;
use std::process::ExitCode;

fn main() -> ExitCode {
    let filter =
        std::env::var("CASEMENT_LOG").unwrap_or_else(|_| casement::log::DEFAULT_FILTER.to_owned());
    casement::log::init(&filter);
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
        Ok(Command::Run(start)) => {
            let config = run_config(*start);
            #[cfg(feature = "debug-inject")]
            let config = match debug_peer_units::from_env() {
                Ok(table) => config.with_debug_peer_units(table),
                Err(error) => {
                    eprintln!("casement: {error}");
                    return ExitCode::from(64);
                }
            };
            match casement::run(config) {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) => {
                    eprintln!("casement: {error}");
                    ExitCode::from(error.exit_code())
                }
            }
        }
        Err(error) => {
            eprintln!("casement: {error}\n{}", cli::USAGE);
            ExitCode::from(64)
        }
    }
}
