//! The fake logind (`org.freedesktop.login1` subset on the jail's private bus, the `fake-logind`
//! binary): Manager `Inhibit` (a pipe fd), `PrepareForSleep`, `ListSessions`; Session `Lock`,
//! `Unlock`, `SetIdleHint`, `TakeDevice`; Seat `SwitchTo`. Bodies at P1 (P1-F2).

/// A call the fake recorded, for assertions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogindCall {
    /// `Inhibit(what, who, why, mode)`.
    Inhibit(String),
    /// The inhibitor's pipe was closed.
    InhibitReleased(String),
    /// `SetIdleHint(idle)`.
    IdleHint(String),
    /// `TakeDevice(major, minor)`.
    TakeDevice(u32, u32),
    /// `SwitchTo(vt)`.
    SwitchTo(u32),
}

/// The fake logind server.
#[derive(Debug, Default)]
pub struct FakeLogind;

impl FakeLogind {
    /// Serves on the bus at `address` until the process is stopped.
    pub fn serve(address: &str) -> Result<FakeLogind, String> {
        let _ = address;
        todo!("P1-F2: FakeLogind::serve")
    }
}
