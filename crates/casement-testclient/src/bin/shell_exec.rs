//! `casement-shell-exec -- PROGRAM [ARGS...]`: runs PROGRAM as the shell. It asks casement for
//! the shell connection (`org.quire.Compositor1.Connect("shell")`, which casement gives only to
//! a caller holding the shell role) and leaves the socket to PROGRAM in `WAYLAND_SOCKET`, which
//! libwayland reads before `WAYLAND_DISPLAY`. Scenarios run shell-host's demos through it: an
//! exclusive-keyboard layer keeps the keyboard only on the shell's connection (CASC-02).
//!
//! Runs only inside the jail, like `casement-probe`. Exit codes: 2 outside the jail, 64 usage,
//! 69 casement handed out no connection, 126 PROGRAM could not be run; else PROGRAM's own.

use std::os::fd::{AsFd, IntoRawFd};
use std::os::unix::process::CommandExt;
use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    let mut args = std::env::args_os().skip(1);
    let program = match (args.next(), args.next()) {
        (Some(dashes), Some(program)) if dashes == "--" => program,
        _ => {
            eprintln!("usage: casement-shell-exec -- PROGRAM [ARGS...]");
            return ExitCode::from(64);
        }
    };
    if let Some(why) = outside_the_jail() {
        eprintln!("casement-shell-exec: {why}");
        return ExitCode::from(2);
    }
    let socket = match casement_testclient::quire::handout::shell() {
        Ok(socket) => socket,
        Err(why) => {
            eprintln!("casement-shell-exec: no shell connection: {why}");
            return ExitCode::from(69);
        }
    };
    // The handed socket is close-on-exec; PROGRAM must inherit it.
    if let Err(why) = rustix::io::fcntl_setfd(socket.as_fd(), rustix::io::FdFlags::empty()) {
        eprintln!("casement-shell-exec: {why}");
        return ExitCode::from(126);
    }
    let fd = socket.into_raw_fd();
    let why = Command::new(&program)
        .args(args)
        .env("WAYLAND_SOCKET", fd.to_string())
        .exec();
    eprintln!("casement-shell-exec: {}: {why}", program.to_string_lossy());
    ExitCode::from(126)
}

/// Why this is not the jail, if it is not: the live session's input or display is visible.
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
