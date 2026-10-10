//! The `casement` binary: arguments, the environment and an exit code around [`casement::run`].

use casement::cli::{self, Command};
use casement::{DebugBus, RunConfig};
use std::process::ExitCode;

fn main() -> ExitCode {
    let filter = std::env::var("CASEMENT_LOG").unwrap_or_else(|_| "info".to_owned());
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
            // Debug1 lives on the jail's private bus, which the jail's scripts name here.
            let debug_bus = match std::env::var("DBUS_SESSION_BUS_ADDRESS") {
                Ok(address) => DebugBus::At(address),
                Err(_) => DebugBus::Unset,
            };
            match casement::run(RunConfig::new(start).with_debug_bus(debug_bus)) {
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
