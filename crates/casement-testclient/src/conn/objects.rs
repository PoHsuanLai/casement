//! The objects of one connection as the tap has seen them created and destroyed, so a message
//! can be decoded by its object's interface and a fuzzer can pick live, destroyed or foreign
//! ids. Ids are the protocol's own numbers.

use super::catalogue;
use std::collections::BTreeMap;
use wayland_backend::protocol::Interface;

/// Ids from here up are the compositor's to make.
const SERVER_IDS: u32 = 0xff00_0000;

/// One object's place in its life.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Life {
    /// Created and not destroyed.
    Live,
    /// The client sent a destructor, or the compositor sent `delete_id`; kept so a stale id can
    /// be named.
    Dead,
}

#[derive(Debug, Clone)]
pub(super) struct Slot {
    pub name: String,
    pub interface: Option<&'static Interface>,
    pub life: Life,
}

/// Every object this connection ever had, by id.
#[derive(Debug, Clone)]
pub(super) struct Objects {
    slots: BTreeMap<u32, Slot>,
}

impl Default for Objects {
    fn default() -> Self {
        let mut objects = Objects {
            slots: BTreeMap::new(),
        };
        objects.create(1, "wl_display");
        objects
    }
}

impl Objects {
    /// Records `id` as a new live object of the interface called `name`.
    pub(super) fn create(&mut self, id: u32, name: &str) {
        let slot = Slot {
            name: name.to_owned(),
            interface: catalogue::lookup(name),
            life: Life::Live,
        };
        self.slots.insert(id, slot);
    }

    /// Records `id` as destroyed (a destructor request, or `delete_id`).
    pub(super) fn kill(&mut self, id: u32) {
        if let Some(slot) = self.slots.get_mut(&id) {
            slot.life = Life::Dead;
        }
    }

    /// The slot of `id`.
    pub(super) fn get(&self, id: u32) -> Option<&Slot> {
        self.slots.get(&id)
    }

    /// The first client-range id above every id seen (server-made ids start at 0xff000000).
    pub(super) fn next_client_id(&self) -> u32 {
        self.slots
            .keys()
            .filter(|id| **id < SERVER_IDS)
            .max()
            .map_or(2, |id| id + 1)
    }

    /// `interface#id`, or `#id` for an id never seen.
    pub(super) fn name_of(&self, id: u32) -> String {
        match self.slots.get(&id) {
            Some(slot) => format!("{}#{id}", slot.name),
            None => format!("#{id}"),
        }
    }

    /// The ids in `life`, in id order, with their interface names.
    pub(super) fn listed(&self, life: Life) -> Vec<(u32, String)> {
        self.slots
            .iter()
            .filter(|(_, slot)| slot.life == life)
            .map(|(id, slot)| (*id, slot.name.clone()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_display_is_object_one_and_a_killed_object_keeps_its_name() {
        let mut objects = Objects::default();
        assert_eq!(objects.name_of(1), "wl_display#1");
        objects.create(2, "wl_registry");
        objects.kill(2);
        assert_eq!(objects.name_of(2), "wl_registry#2");
        assert_eq!(objects.name_of(9), "#9");
        assert_eq!(
            objects.listed(Life::Live),
            vec![(1, "wl_display".to_owned())]
        );
        assert_eq!(
            objects.listed(Life::Dead),
            vec![(2, "wl_registry".to_owned())]
        );
    }
}
