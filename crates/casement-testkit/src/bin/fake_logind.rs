//! `fake-logind`: the logind subset on the jail's private bus (P1-F2). Takes the bus address as
//! its first argument, else `DBUS_SYSTEM_BUS_ADDRESS` (the jail's scripts export it), owns
//! `org.freedesktop.login1` there and serves until it is stopped. Never the system bus: with no
//! address it refuses to start.

use casement_testkit::fakes::FakeLogind;
use std::process::ExitCode;

fn main() -> ExitCode {
    let address = std::env::args()
        .nth(1)
        .or_else(|| std::env::var("DBUS_SYSTEM_BUS_ADDRESS").ok());
    let Some(address) = address else {
        eprintln!("fake-logind: no bus address (argument or DBUS_SYSTEM_BUS_ADDRESS)");
        return ExitCode::FAILURE;
    };
    match FakeLogind::serve(&address) {
        Ok(_logind) => loop {
            std::thread::park();
        },
        Err(error) => {
            eprintln!("fake-logind: {error}");
            ExitCode::FAILURE
        }
    }
}
