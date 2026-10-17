//! The `xwayland-ready` line: the first xwayland-satellite of a session came up.

use super::stamp::Stamp;

/// The first satellite start of a session, complete: what the `xwayland-ready` line says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct XwaylandReady {
    /// When casement decided to start satellite: the turn it saw the first X11 client waiting.
    pub requested_at: Stamp,
    /// When satellite answered: its connection to the compositor arrived.
    pub ready_at: Stamp,
}

impl XwaylandReady {
    /// The line, message first, then `key=value` fields.
    pub fn line(&self) -> String {
        format!(
            "xwayland-ready requested_at={} ready_at={}",
            self.requested_at, self.ready_at
        )
    }
}

/// Where the session's first satellite is. Only the first start is timed; a restart after a
/// crash is not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum XwaylandStage {
    /// Not started yet.
    #[default]
    Idle,
    /// Started at this moment, not yet connected.
    Requested(Stamp),
    /// Reported (or the first start is over).
    Done,
}

impl XwaylandStage {
    /// Satellite was started at `at`. Only the first start counts.
    #[must_use]
    pub fn requested(self, at: Stamp) -> XwaylandStage {
        match self {
            XwaylandStage::Idle => XwaylandStage::Requested(at),
            other => other,
        }
    }

    /// Satellite's connection arrived at `at`: the record, once.
    #[must_use]
    pub fn connected(self, at: Stamp) -> (XwaylandStage, Option<XwaylandReady>) {
        match self {
            XwaylandStage::Requested(requested_at) => (
                XwaylandStage::Done,
                Some(XwaylandReady {
                    requested_at,
                    ready_at: at.or_later(requested_at),
                }),
            ),
            other => (other, None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_start_is_reported_once_when_satellite_connects() {
        let (stage, early) = XwaylandStage::Idle.connected(Stamp(5));
        assert_eq!((stage, early), (XwaylandStage::Idle, None));
        // A second start request does not move the first.
        let stage = XwaylandStage::Idle
            .requested(Stamp(10))
            .requested(Stamp(99));
        let (stage, ready) = stage.connected(Stamp(40));
        let ready = ready.expect("reported");
        assert_eq!(ready.line(), "xwayland-ready requested_at=10 ready_at=40");
        assert_eq!(stage, XwaylandStage::Done);
        // A restart and a second connection add nothing.
        let stage = stage.requested(Stamp(500));
        assert_eq!(stage.connected(Stamp(600)), (XwaylandStage::Done, None));
    }
}
