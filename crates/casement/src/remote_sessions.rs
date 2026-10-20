//! The remote EIS connections that are open (`org.quire.Compositor1.ConnectEis` and
//! `LeaseWindow`), numbered one slot each (CASW-19) and findable by portal session or lease, so
//! `EndEis` and `EndLease` can stop the input and close the compositor's end of each socket
//! (PORTAL-05).

use casement_agent::{RemoteFor, RemoteHandle, RemoteSlot, RemoteSlots};
use smithay::reexports::calloop::RegistrationToken;
use std::collections::BTreeMap;

/// One open connection: its handle and where its source sits in the event loop.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Open {
    handle: RemoteHandle,
    token: RegistrationToken,
    /// The bus connection (its unique name) that asked for it, when known: the sockets go with it.
    owner: Option<String>,
}

/// The slot mint and the open connections.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RemoteSessions {
    slots: RemoteSlots,
    open: BTreeMap<RemoteSlot, Open>,
}

impl RemoteSessions {
    /// The next connection's slot; none once every slot has been used (the connection is then
    /// refused rather than given a number that is taken).
    pub fn mint(&mut self) -> Option<RemoteSlot> {
        let (slots, slot) = self.slots.next()?;
        self.slots = slots;
        Some(slot)
    }

    /// The connection `handle` is open, its source in the loop at `token`, asked for by the bus
    /// connection `owner`.
    pub fn opened(
        &mut self,
        handle: RemoteHandle,
        token: RegistrationToken,
        owner: Option<String>,
    ) {
        self.open.insert(
            handle.slot(),
            Open {
                handle,
                token,
                owner,
            },
        );
    }

    /// The connection in `slot` ended by itself (the app hung up).
    pub fn closed(&mut self, slot: RemoteSlot) {
        self.open.remove(&slot);
    }

    /// Takes every open connection of `owner_of` (a portal session, an agent lease) out of the
    /// book: for each, the handle (to release what the app held) and the loop token (to remove
    /// the source).
    pub fn end_for(&mut self, owner_of: RemoteFor) -> Vec<(RemoteHandle, RegistrationToken)> {
        self.take_where(|open| open.handle.owner() == owner_of)
    }

    /// Takes every open connection the bus connection `owner` asked for out of the book, as
    /// [`RemoteSessions::end`] does (the portal's connection left the bus: NEW-PI-8).
    pub fn end_owner(&mut self, owner: &str) -> Vec<(RemoteHandle, RegistrationToken)> {
        self.take_where(|open| open.owner.as_deref() == Some(owner))
    }

    fn take_where(
        &mut self,
        wanted: impl Fn(&Open) -> bool,
    ) -> Vec<(RemoteHandle, RegistrationToken)> {
        let slots: Vec<RemoteSlot> = self
            .open
            .iter()
            .filter(|(_, open)| wanted(open))
            .map(|(slot, _)| *slot)
            .collect();
        slots
            .into_iter()
            .filter_map(|slot| self.open.remove(&slot))
            .map(|open| (open.handle, open.token))
            .collect()
    }

    /// How many leases the bus connection `owner` holds open.
    pub fn leases_of(&self, owner: Option<&str>) -> usize {
        self.open
            .values()
            .filter(|open| matches!(open.handle.owner(), RemoteFor::Agent(_)))
            .filter(|open| open.owner.as_deref() == owner)
            .count()
    }

    /// Who asked for the open connection of `owner_of`, if it is open: the bus connection
    /// (its unique name), when known.
    pub fn asked_by(&self, owner_of: RemoteFor) -> Option<Option<&str>> {
        self.open
            .values()
            .find(|open| open.handle.owner() == owner_of)
            .map(|open| open.owner.as_deref())
    }

    /// How many connections are open.
    pub fn len(&self) -> usize {
        self.open.len()
    }

    /// Whether none is open.
    pub fn is_empty(&self) -> bool {
        self.open.is_empty()
    }
}
