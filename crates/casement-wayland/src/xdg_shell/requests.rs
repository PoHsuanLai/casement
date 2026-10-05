//! What a mapped toplevel asks for: an interactive move or resize, zoom, minimise, full screen,
//! the window menu, and the pong that answers a ping. The model decides; this module reports
//! the ask. A request for a window that has not mapped is dropped (the model knows no such
//! window).
//!
//! A move or resize must answer a real press: the handler asks [`grab_proven`] of the press
//! the seat's `SerialBook` recorded before it calls [`on_move`] or [`on_resize`].

use super::XdgShellPart;
use super::commit::Phase;
use crate::serial::{PressKind, PressRecord};
use casement_core::geometry::LogicalPoint;
use casement_core::{
    ClientKey, Event, InputSerial, OutputKey, ResizeEdges, ToplevelRequest, WindowId,
};
use casement_protocols::vocab::Toggle;
use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::ResizeEdge;
use smithay::wayland::shell::xdg::ToplevelSurface;

/// What a serial is offered as proof for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GrabUse {
    /// An interactive move, resize or window menu: a pointer button, a touch or a pen tip.
    Drag,
    /// A popup grab or a placement request: any press, a key included (a menu opened from the
    /// keyboard, a placement chosen with a shortcut).
    AnyPress,
}

/// Whether the press `record` (what the seat remembers of the serial the client sent, if it
/// was delivered to that client and is recent) is proof enough for `use_`.
pub fn grab_proven(record: Option<&PressRecord>, use_: GrabUse) -> bool {
    match (record.map(|press| press.kind), use_) {
        (None, _) => false,
        (Some(PressKind::Key), GrabUse::Drag) => false,
        (Some(_), GrabUse::Drag | GrabUse::AnyPress) => true,
    }
}

/// The edges a resize names (`None` for the protocol's `none`, and for a value we do not know).
pub fn edges_of(edge: ResizeEdge) -> Option<ResizeEdges> {
    match edge {
        ResizeEdge::Top => Some(ResizeEdges::Top),
        ResizeEdge::Bottom => Some(ResizeEdges::Bottom),
        ResizeEdge::Left => Some(ResizeEdges::Left),
        ResizeEdge::Right => Some(ResizeEdges::Right),
        ResizeEdge::TopLeft => Some(ResizeEdges::TopLeft),
        ResizeEdge::TopRight => Some(ResizeEdges::TopRight),
        ResizeEdge::BottomLeft => Some(ResizeEdges::BottomLeft),
        ResizeEdge::BottomRight => Some(ResizeEdges::BottomRight),
        _ => None,
    }
}

/// The request as an event, when `surface` is a mapped toplevel.
fn request(part: &XdgShellPart, surface: &ToplevelSurface, request: ToplevelRequest) -> Vec<Event> {
    mapped_window(part, surface)
        .map(|window| Event::ToplevelRequest { window, request })
        .into_iter()
        .collect()
}

fn mapped_window(part: &XdgShellPart, surface: &ToplevelSurface) -> Option<WindowId> {
    let window = part.window_of_surface(surface.wl_surface())?;
    part.records
        .get(&window)
        .is_some_and(|record| matches!(record.phase, Phase::Mapped(_)))
        .then_some(window)
}

/// An interactive move answering the press `serial`.
pub fn on_move(
    part: &mut XdgShellPart,
    surface: &ToplevelSurface,
    serial: InputSerial,
) -> Vec<Event> {
    request(part, surface, ToplevelRequest::Move { serial })
}

/// An interactive resize from `edges` answering the press `serial`.
pub fn on_resize(
    part: &mut XdgShellPart,
    surface: &ToplevelSurface,
    serial: InputSerial,
    edges: ResizeEdges,
) -> Vec<Event> {
    request(part, surface, ToplevelRequest::Resize { serial, edges })
}

/// Maximise (zoom) on or off.
pub fn on_maximise(
    part: &mut XdgShellPart,
    surface: &ToplevelSurface,
    toggle: Toggle,
) -> Vec<Event> {
    request(part, surface, ToplevelRequest::Maximise(toggle))
}

/// Minimise.
pub fn on_minimise(part: &mut XdgShellPart, surface: &ToplevelSurface) -> Vec<Event> {
    request(part, surface, ToplevelRequest::Minimise)
}

/// Full screen on or off, optionally on `output`.
pub fn on_full_screen(
    part: &mut XdgShellPart,
    surface: &ToplevelSurface,
    toggle: Toggle,
    output: Option<OutputKey>,
) -> Vec<Event> {
    request(
        part,
        surface,
        ToplevelRequest::FullScreen { toggle, output },
    )
}

/// Show the window menu at `at` (global logical).
pub fn on_window_menu(
    part: &mut XdgShellPart,
    surface: &ToplevelSurface,
    at: LogicalPoint,
) -> Vec<Event> {
    request(part, surface, ToplevelRequest::WindowMenu { at })
}

/// A client answered a ping.
pub fn on_pong(part: &mut XdgShellPart, client: ClientKey) -> Vec<Event> {
    let _ = part;
    vec![Event::Pong { client }]
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::input::{DeviceClass, Origin};
    use casement_core::time::Mono;

    fn press(kind: PressKind) -> PressRecord {
        PressRecord {
            client: ClientKey(1),
            at: Mono(std::time::Duration::from_millis(5)),
            kind,
            origin: Origin::Physical(DeviceClass::Pointer),
            button: None,
        }
    }

    /// A press proves a drag unless it was a key; no press proves nothing.
    #[test]
    fn a_grab_needs_a_press_that_can_drag() {
        let rows = [
            ("no press", None, GrabUse::Drag, false),
            ("no press, popup", None, GrabUse::AnyPress, false),
            ("button", Some(PressKind::Button), GrabUse::Drag, true),
            ("touch", Some(PressKind::Touch), GrabUse::Drag, true),
            ("pen", Some(PressKind::TabletTip), GrabUse::Drag, true),
            ("key", Some(PressKind::Key), GrabUse::Drag, false),
            (
                "key opens a menu",
                Some(PressKind::Key),
                GrabUse::AnyPress,
                true,
            ),
            (
                "button grabs a popup",
                Some(PressKind::Button),
                GrabUse::AnyPress,
                true,
            ),
        ];
        for (case, kind, use_, want) in rows {
            let record = kind.map(press);
            assert_eq!(grab_proven(record.as_ref(), use_), want, "{case}");
        }
    }

    #[test]
    fn every_resize_edge_maps_and_none_does_not() {
        let rows = [
            (ResizeEdge::Top, Some(ResizeEdges::Top)),
            (ResizeEdge::Bottom, Some(ResizeEdges::Bottom)),
            (ResizeEdge::Left, Some(ResizeEdges::Left)),
            (ResizeEdge::Right, Some(ResizeEdges::Right)),
            (ResizeEdge::TopLeft, Some(ResizeEdges::TopLeft)),
            (ResizeEdge::TopRight, Some(ResizeEdges::TopRight)),
            (ResizeEdge::BottomLeft, Some(ResizeEdges::BottomLeft)),
            (ResizeEdge::BottomRight, Some(ResizeEdges::BottomRight)),
            (ResizeEdge::None, None),
        ];
        for (edge, want) in rows {
            assert_eq!(edges_of(edge), want, "{edge:?}");
        }
    }
}
