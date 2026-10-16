//! Where a window comes from: a Wayland client of its own, or an X11 program.

/// How a toplevel reached casement. Every X11 program's windows arrive over the one Wayland
/// connection xwayland-satellite holds, so a window of this origin says nothing about which
/// process owns it: force quit asks the X server (P3-QU).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WindowOrigin {
    /// A Wayland client: its connection is its own.
    #[default]
    Wayland,
    /// An X11 program, through the satellite's shared connection.
    X11,
}
