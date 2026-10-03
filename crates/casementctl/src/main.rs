//! `casementctl`: drives a jailed casement over its private bus.
//!
//! ```text
//! casementctl debug inject            lines on stdin, `ok <line>` each (casement-ipc's grammar)
//! casementctl debug wait PREDICATE [--timeout MS]
//! casementctl debug clock step N | advance MS
//! casementctl debug output list | add SPEC | remove NAME | set NAME KEY=VALUE
//! casementctl debug windows | scene
//! casementctl debug fault panic-after-frames N | stall-client-ms MS | drop-output NAME
//! casementctl shot [--output NAME | --window ID | --region X,Y,W,H] [--masked] OUT.png   (P1-C1)
//! casementctl report --kms --input --gpu                                              (P5, user only)
//! ```
//!
//! Every subcommand but `report` runs only inside the jail (it refuses otherwise, exit 2).

use casementctl::command;

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match command::parse(&args) {
        Ok(command) => command::run(command),
        Err(error) => {
            eprintln!("casementctl: {error}");
            ExitCode::from(64)
        }
    }
}
