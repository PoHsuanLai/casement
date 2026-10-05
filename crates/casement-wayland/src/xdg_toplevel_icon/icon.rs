//! One `xdg_toplevel_icon_v1`: what it is made of, when it stops changing, and which buffers
//! must outlive it. The rules are pure functions over [`IconParts`]; the dispatch around them
//! turns a refusal into the protocol error and returns.

use smithay::reexports::wayland_protocols::xdg::toplevel_icon::v1::server::xdg_toplevel_icon_v1::{
    self, XdgToplevelIconV1,
};
use smithay::reexports::wayland_server::backend::ClientId;
use smithay::reexports::wayland_server::protocol::wl_buffer::WlBuffer;
use smithay::reexports::wayland_server::{Client, DataInit, DisplayHandle, Resource};
use smithay::wayland::Dispatch2;
use smithay::wayland::shm::{BufferAccessError, with_buffer_contents};
use std::sync::{Arc, Mutex, MutexGuard};

/// A pixel buffer of an icon: shm-backed and square, at a scale.
#[derive(Debug, Clone, PartialEq)]
pub struct IconBuffer {
    /// The client's buffer.
    pub buffer: WlBuffer,
    /// The edge in buffer pixels.
    pub edge: u32,
    /// The scale the pixels are drawn for.
    pub scale: i32,
}

/// What an icon is made of: an optional stock name and its buffers, in the order added.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct IconParts {
    /// The XDG icon-theme name.
    pub name: Option<String>,
    /// The pixel data.
    pub buffers: Vec<IconBuffer>,
}

impl IconParts {
    /// An icon with neither a name nor a buffer resets the toplevel's icon, like no icon.
    fn is_empty(&self) -> bool {
        self.name.is_none() && self.buffers.is_empty()
    }

    /// These parts with `added`; a buffer of the same edge and scale replaces the earlier one,
    /// and says which buffer it displaced.
    fn with_buffer(mut self, added: IconBuffer) -> (IconParts, Option<WlBuffer>) {
        let same = |held: &IconBuffer| held.edge == added.edge && held.scale == added.scale;
        let displaced = self
            .buffers
            .iter()
            .position(same)
            .map(|at| self.buffers.remove(at).buffer);
        self.buffers.push(added);
        (self, displaced)
    }
}

/// Where an icon is in its life.
#[derive(Debug, Clone)]
enum Phase {
    /// Being made: requests change it.
    Building(IconParts),
    /// Given to a toplevel: it never changes again, and a request to change it is an error.
    Assigned(IconParts),
}

/// What an `add_buffer` can be refused for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Refusal {
    /// Not backed by `wl_shm`.
    NotShm,
    /// Not square.
    NotSquare,
}

impl Refusal {
    fn text(self) -> &'static str {
        match self {
            Refusal::NotShm => "The wl_buffer must be backed by wl_shm",
            Refusal::NotSquare => "The wl_buffer must be a square",
        }
    }
}

/// The edge of `buffer`, or why it is no icon buffer.
fn edge_of(buffer: &WlBuffer) -> Result<u32, Refusal> {
    let size = with_buffer_contents(buffer, |_, _, data| (data.width, data.height))
        .map_err(|_: BufferAccessError| Refusal::NotShm)?;
    match size {
        (w, h) if w == h => u32::try_from(w).map_err(|_| Refusal::NotSquare),
        _ => Err(Refusal::NotSquare),
    }
}

/// The icons that hold buffers, shared by the global, every manager and every icon.
#[derive(Debug, Clone, Default)]
pub(super) struct Registry(Arc<Mutex<Vec<(WlBuffer, XdgToplevelIconV1)>>>);

impl Registry {
    fn held(&self) -> MutexGuard<'_, Vec<(WlBuffer, XdgToplevelIconV1)>> {
        // A poisoned list is still a list: the pairs are plain handles.
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn hold(&self, buffer: WlBuffer, icon: &XdgToplevelIconV1) {
        self.held().push((buffer, icon.clone()));
    }

    fn release(&self, buffer: &WlBuffer, icon: &XdgToplevelIconV1) {
        self.held()
            .retain(|(held, owner)| !(held == buffer && owner == icon));
    }

    fn forget(&self, icon: &XdgToplevelIconV1) {
        self.held().retain(|(_, owner)| owner != icon);
    }

    /// Every icon that holds `buffer` is told (`no_buffer`) and lets go of it.
    pub(super) fn buffer_destroyed(&self, buffer: &WlBuffer) {
        let owners: Vec<XdgToplevelIconV1> = {
            let mut held = self.held();
            let owners = held
                .iter()
                .filter(|(held, _)| held == buffer)
                .map(|(_, owner)| owner.clone())
                .collect();
            held.retain(|(held, _)| held != buffer);
            owners
        };
        for owner in owners {
            // A client that hangs up destroys its buffers and its icons together, and an error
            // for a client already being torn down has nobody to go to (the backend aborts).
            if owner.client().is_none() {
                continue;
            }
            owner.post_error(
                xdg_toplevel_icon_v1::Error::NoBuffer,
                "The provided buffer has been destroyed before the toplevel icon",
            );
        }
    }
}

/// The user data of an icon.
#[derive(Debug)]
pub struct IconData {
    phase: Mutex<Phase>,
    registry: Registry,
}

impl IconData {
    pub(super) fn new(registry: Registry) -> IconData {
        IconData {
            phase: Mutex::new(Phase::Building(IconParts::default())),
            registry,
        }
    }

    fn phase(&self) -> MutexGuard<'_, Phase> {
        self.phase
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Hands the icon to a toplevel: from now on it is immutable. `None` when it has nothing
    /// to show (it resets the toplevel's icon).
    pub(super) fn assign(&self) -> Option<IconParts> {
        let mut phase = self.phase();
        let (Phase::Building(parts) | Phase::Assigned(parts)) = &*phase;
        let parts = parts.clone();
        *phase = Phase::Assigned(parts.clone());
        (!parts.is_empty()).then_some(parts)
    }
}

impl<D> Dispatch2<XdgToplevelIconV1, D> for IconData
where
    D: 'static,
{
    fn request(
        &self,
        _state: &mut D,
        _client: &Client,
        icon: &XdgToplevelIconV1,
        request: xdg_toplevel_icon_v1::Request,
        _display: &DisplayHandle,
        _data_init: &mut DataInit<'_, D>,
    ) {
        use xdg_toplevel_icon_v1::Request;
        let mut phase = self.phase();
        let Phase::Building(parts) = &mut *phase else {
            if matches!(request, Request::SetName { .. } | Request::AddBuffer { .. }) {
                icon.post_error(
                    xdg_toplevel_icon_v1::Error::Immutable,
                    "Request made after the icon has been assigned to a toplevel via 'set_icon'",
                );
            }
            return;
        };
        match request {
            Request::SetName { icon_name } => parts.name = Some(icon_name),
            Request::AddBuffer { buffer, scale } => match edge_of(&buffer) {
                Ok(edge) => {
                    let added = IconBuffer {
                        buffer: buffer.clone(),
                        edge,
                        scale,
                    };
                    let (next, displaced) = std::mem::take(parts).with_buffer(added);
                    *parts = next;
                    if let Some(displaced) = displaced {
                        self.registry.release(&displaced, icon);
                    }
                    self.registry.hold(buffer, icon);
                }
                Err(refusal) => icon.post_error(
                    xdg_toplevel_icon_v1::Error::InvalidBuffer,
                    refusal.text().to_owned(),
                ),
            },
            Request::Destroy => {}
            _ => {}
        }
    }

    fn destroyed(&self, _state: &mut D, _client: ClientId, icon: &XdgToplevelIconV1) {
        self.registry.forget(icon);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A name or a buffer makes an icon; nothing does not (it resets the toplevel's icon).
    #[test]
    fn an_icon_with_nothing_in_it_is_empty() {
        assert!(IconParts::default().is_empty());
        let named = IconParts {
            name: Some("mail".to_owned()),
            buffers: Vec::new(),
        };
        assert!(!named.is_empty());
    }
}
