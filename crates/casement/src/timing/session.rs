//! The session's start and the first satellite.

use super::book::Timing;
use super::session_ready::ShellSurface;
use super::stamp::Stamp;

impl Timing {
    /// xwayland-satellite was started at `at` (the first time in this session counts).
    pub fn satellite_started(&mut self, at: Stamp) {
        self.xwayland = self.xwayland.requested(at);
    }

    /// xwayland-satellite's connection arrived at `at`: the first writes its line.
    pub fn satellite_connected(&mut self, at: Stamp) {
        let (stage, ready) = self.xwayland.connected(at);
        self.xwayland = stage;
        if let Some(ready) = ready {
            self.sink.write(ready.line());
        }
    }

    /// The shell's connection was taken in at `at`.
    pub fn shell_connected(&mut self, at: Stamp) {
        self.session = self.session.shell_connected(at);
    }

    /// A frame at `at` showed the shell's `surface`: when the bar and the dock are both up the
    /// line is written.
    pub fn shell_shown(&mut self, surface: ShellSurface, at: Stamp) {
        let (stage, ready) = self.session.shown(surface, at);
        self.session = stage;
        if let Some(ready) = ready {
            self.sink.write(ready.line());
        }
    }
}
