//! The lenient policy's first commit (A37, `testing` builds only): a toplevel whose first commit
//! already carries a buffer maps at once, keyed, and its initial configure follows the keyboard
//! enter instead of preceding it.
//!
//! Some conformance clients (wlcs' text-input tests) expect to hear the keyboard enter before
//! the surface's first configure, as the compositors they were written for deliver it. The
//! strict path sends the bare initial configure at the first commit and maps at the next; this
//! path maps first (the model keys the window and queues its configure into the toplevel's
//! pending state), and [`send_initial_configure`] then sends that one configure.

use super::XdgShellPart;
use super::commit::{Buffer, Configured, Declared, Phase, mapped_events};
use crate::client::ClientState;
use casement_core::Event;
use casement_core::focus::TokenCheck;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;

/// The events of mapping `surface` keyed, when this commit is its first and carries a buffer;
/// `None` for any other commit (the ordinary path handles it). The caller feeds the events and
/// then calls [`send_initial_configure`].
pub fn map_keyed_on_first_buffer(
    part: &mut XdgShellPart,
    surface: &WlSurface,
    client: &ClientState,
) -> Option<Vec<Event>> {
    let window = part.window_of_surface(surface)?;
    let declared = Declared::read(part.toplevels.get(&window)?);
    let Buffer::Present(size) = declared.buffer else {
        return None;
    };
    let record = part.records.get_mut(&window)?;
    if declared.configured != Configured::No || record.phase != Phase::Unmapped {
        return None;
    }
    record.staged = TokenCheck::Valid;
    let events = mapped_events(window, client, &declared, size, record, &part.toplevels);
    record.phase = Phase::Mapped(size);
    Some(events)
}

/// Sends the initial configure of `surface`'s toplevel when it has not gone out yet.
pub fn send_initial_configure(part: &XdgShellPart, surface: &WlSurface) {
    let Some(toplevel) = part
        .window_of_surface(surface)
        .and_then(|window| part.toplevels.get(&window))
    else {
        return;
    };
    if !toplevel.is_initial_configure_sent() {
        toplevel.send_configure();
    }
}
