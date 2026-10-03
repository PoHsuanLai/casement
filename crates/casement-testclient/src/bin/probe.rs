//! `casement-probe`: one-shot probes against `$WAYLAND_DISPLAY` for dev scripts, JSON lines on
//! stdout: `globals`, `toplevel`, `layer`, `transcript`; `capture` (P1) and `lease` (P5) say
//! they are not there yet. Runs only inside the jail: it refuses (exit 2, "could not run
//! safely") when a live session's sockets or `/dev/input` are visible.
//!
//! Exit codes: 0 passed, 1 the probe failed, 2 refused or unavailable, 64 usage.

#[path = "probe/shell.rs"]
mod shell;

use casement_testclient::TestClient;
use serde_json::json;
use std::process::ExitCode;

fn main() -> ExitCode {
    if let Some(why) = outside_the_jail() {
        println!("{}", json!({ "error": why }));
        return ExitCode::from(2);
    }
    let mut args = std::env::args().skip(1);
    let command = args.next();
    let rest: Vec<String> = args.collect();
    let outcome = match command.as_deref() {
        Some("globals") => globals(),
        Some("transcript") => transcript(),
        Some("toplevel") => shell::toplevel(&rest),
        Some("layer") => shell::layer(&rest),
        Some("capture") => Err(Failure::Unavailable("capture arrives with P1")),
        Some("lease") => Err(Failure::Unavailable("lease arrives with P5")),
        _ => Err(Failure::Usage),
    };
    match outcome {
        Ok(()) => ExitCode::SUCCESS,
        Err(Failure::Probe(why)) => {
            println!("{}", json!({ "error": why }));
            ExitCode::from(1)
        }
        Err(Failure::Unavailable(why)) => {
            println!("{}", json!({ "error": why }));
            ExitCode::from(2)
        }
        Err(Failure::Usage) => {
            eprintln!(
                "usage: casement-probe globals | transcript | toplevel [--title T] [--app-id A] [--size WxH] [--hold] | layer [--layer L] [--namespace N] [--size WxH] [--hold] | capture | lease"
            );
            ExitCode::from(64)
        }
    }
}

/// Why a probe did not pass.
pub(crate) enum Failure {
    /// The probe ran and the compositor did not do what it should.
    Probe(String),
    /// The probe cannot run here or yet.
    Unavailable(&'static str),
    /// Not a command.
    Usage,
}

impl From<casement_testclient::ClientError> for Failure {
    fn from(error: casement_testclient::ClientError) -> Failure {
        Failure::Probe(error.to_string())
    }
}

/// The jail check in its smallest form (the full one is `casement::jail_check`): no input
/// devices and no live compositor socket.
fn outside_the_jail() -> Option<String> {
    let uid = rustix::process::getuid().as_raw();
    let live = format!("/run/user/{uid}/wayland-0");
    if std::path::Path::new("/dev/input").exists() {
        return Some("/dev/input is visible: this is not the jail".to_owned());
    }
    if std::path::Path::new(&live).exists() {
        return Some(format!("{live} exists: this is not the jail"));
    }
    None
}

/// Every global the compositor advertises, one line each.
fn globals() -> Result<(), Failure> {
    let client = TestClient::connect_env()?;
    for global in client.globals().contents().clone_list() {
        println!(
            "{}",
            json!({ "global": { "name": global.name, "interface": global.interface, "version": global.version } })
        );
    }
    Ok(())
}

/// What a client sees on connecting: every event of the first roundtrip, one line each.
fn transcript() -> Result<(), Failure> {
    let client = TestClient::connect_env()?;
    for line in &client.transcript().0 {
        println!(
            "{}",
            json!({ "object": line.object, "event": line.event, "args": line.args })
        );
    }
    Ok(())
}
