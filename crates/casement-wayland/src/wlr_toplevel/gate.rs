//! The consent gate of `zwlr_foreign_toplevel_manager_v1` (`Power::Windows`): binding it lists
//! every window, so a manager bound while the person has not answered hears nothing; refused,
//! it says `finished`. A power taken back finishes the managers in use, and their handles
//! change nothing from then on.

use super::{WlrToplevelHandler, WlrToplevelPart, list_all_on};
use crate::consent::{self, Mortal, Pushed};
use casement_core::ClientKey;
use casement_core::privilege::PowerOutcome;
use smithay::reexports::wayland_protocols_wlr::foreign_toplevel::v1::server::zwlr_foreign_toplevel_manager_v1::ZwlrForeignToplevelManagerV1;
use smithay::reexports::wayland_server::{Client, Resource};

impl Mortal for ZwlrForeignToplevelManagerV1 {
    fn alive(&self) -> bool {
        self.is_alive()
    }
}

/// Keeps `manager`, bound by `client`, until the answer comes; one past the cap of waiting
/// requests, and one from a client nobody vouches for, is finished.
pub(super) fn wait<D: WlrToplevelHandler>(
    state: &mut D,
    client: &Client,
    manager: ZwlrForeignToplevelManagerV1,
) {
    let refused = match consent::key_of(client) {
        Some(key) => match state.wlr_toplevel_part().waiting.push(key, manager) {
            Pushed::Kept => return,
            Pushed::Full(full) => full,
        },
        None => manager,
    };
    consent::finish_later(consent::Farewell::WindowList(refused));
}

/// How many managers `client` has waiting for an answer.
pub(super) fn waiting_of(part: &WlrToplevelPart, client: &Client) -> usize {
    consent::key_of(client).map_or(0, |key| part.waiting.count(key))
}

/// The answer about `client`'s power over the windows came out. Allowed, the managers that
/// waited are listed every window; refused, they are finished; taken back, so are the ones in
/// use.
pub fn on_resolved<D: WlrToplevelHandler>(state: &mut D, client: ClientKey, outcome: PowerOutcome) {
    let part = state.wlr_toplevel_part();
    let waiting = part.waiting.take(client);
    match outcome {
        PowerOutcome::Allowed => {
            for manager in waiting.into_iter().filter(|manager| manager.is_alive()) {
                part.managers.push(manager.clone());
                list_all_on(part, &manager);
            }
        }
        PowerOutcome::Denied | PowerOutcome::Revoked => {
            waiting.iter().for_each(|manager| manager.finished());
            let (ended, kept): (Vec<_>, Vec<_>) = std::mem::take(&mut part.managers)
                .into_iter()
                .partition(|manager| owner_of(manager) == Some(client));
            part.managers = kept;
            ended.iter().for_each(|manager| manager.finished());
            for held in part.handles.values_mut() {
                held.retain(|held| {
                    held.handle.client().as_ref().and_then(consent::key_of) != Some(client)
                });
            }
        }
    }
}

/// The client a manager belongs to, when casement inserted it.
fn owner_of(manager: &ZwlrForeignToplevelManagerV1) -> Option<ClientKey> {
    manager.client().as_ref().and_then(consent::key_of)
}
