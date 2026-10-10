//! `casement-x11probe`: an X11 client for the XWayland suite (P1-X1). It sets `WM_CLASS`, maps
//! windows, reads input and selections, reports override-redirect facts, geometry, the screen's
//! size and the `RESOURCE_MANAGER` (the DPI satellite publishes), and asks for activation, so
//! `dev/accept/xwayland-*.sh` assert what an X11 client sees through xwayland-satellite.
//!
//! `casement-x11probe session` is one long-lived client driven from stdin, one command a line
//! (grammar in `x11probe/command.rs`), printing `x11: ...` lines on stdout: what the session did
//! (`x11: done N`, `x11: error ...`) and every event its windows got
//! (`x11: event NAME key-press detail=38`). Scripts wait on lines, never on sleeps.
//!
//! Exit codes: 0 passed, 1 the probe failed, 2 refused or unavailable (no `DISPLAY`, or this is
//! not the jail), 64 usage.

#[path = "x11probe/command.rs"]
mod command;
#[path = "x11probe/session.rs"]
mod session;

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args != ["session"] {
        eprintln!(
            "usage: casement-x11probe session   (commands on stdin; see x11probe/command.rs)"
        );
        return ExitCode::from(64);
    }
    if let Some(why) = outside_the_jail() {
        println!("x11: error {why}");
        return ExitCode::from(2);
    }
    match session::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(session::ProbeError::Connect(why)) => {
            println!("x11: error could not connect: {why}");
            ExitCode::from(2)
        }
        Err(why) => {
            println!("x11: error {why}");
            ExitCode::from(1)
        }
    }
}

/// The jail check in its smallest form (as `casement-probe` has it): no input devices and no
/// live compositor socket.
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
