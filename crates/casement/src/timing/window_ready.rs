//! The `window-ready` line: one toplevel's first show.

use super::stamp::Stamp;
use super::token::Token;
use casement_core::Pid;

/// A toplevel's first show, complete: what the `window-ready` line says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowReady {
    /// The app id its client gave.
    pub app_id: Token,
    /// Its client's process as the socket's peer credentials name it; 0 when none is known
    /// (X11 programs share the satellite's connection).
    pub pid: Pid,
    /// When its client connected.
    pub connected_at: Stamp,
    /// Its first commit with a buffer.
    pub first_commit_at: Stamp,
    /// The first frame that shows it.
    pub first_frame_at: Stamp,
    /// When its open animation ended (the first frame's own moment when there is none).
    pub settled_at: Stamp,
}

impl WindowReady {
    /// The line, message first, then `key=value` fields.
    pub fn line(&self) -> String {
        format!(
            "window-ready app_id={} pid={} connected_at={} first_commit_at={} first_frame_at={} settled_at={}",
            self.app_id,
            self.pid.0,
            self.connected_at,
            self.first_commit_at,
            self.first_frame_at,
            self.settled_at,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ready(app_id: &str, pid: u32) -> WindowReady {
        WindowReady {
            app_id: Token::of(app_id),
            pid: Pid(pid),
            connected_at: Stamp(100),
            first_commit_at: Stamp(250),
            first_frame_at: Stamp(900),
            settled_at: Stamp(1_400),
        }
    }

    #[test]
    fn the_line_has_the_fields_the_desktop_scripts_read_in_order() {
        // name, window, line
        let cases = [
            (
                "a plain app",
                ready("foot", 4321),
                "window-ready app_id=foot pid=4321 connected_at=100 first_commit_at=250 first_frame_at=900 settled_at=1400",
            ),
            (
                "a spaced id and no pid",
                ready("Blender 4.2", 0),
                "window-ready app_id=Blender_4.2 pid=0 connected_at=100 first_commit_at=250 first_frame_at=900 settled_at=1400",
            ),
            (
                "an empty id",
                ready("", 7),
                "window-ready app_id=- pid=7 connected_at=100 first_commit_at=250 first_frame_at=900 settled_at=1400",
            ),
        ];
        for (name, window, line) in cases {
            assert_eq!(window.line(), line, "{name}");
        }
    }
}
