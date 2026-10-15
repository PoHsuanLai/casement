//! The icons found so far, owned by the caller.

use crate::{Finder, Pixels, ThemeLookup, Want};
use std::collections::BTreeMap;
use std::collections::btree_map::Entry;

/// What a lookup was asked: the same key is the same question, whatever the files say later.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct IconKey {
    app_id: Option<String>,
    name: Option<String>,
    /// A fingerprint of the toplevel's own pixels (zero for none).
    pixels: u64,
}

impl Want {
    /// The key of this question.
    pub fn key(&self) -> IconKey {
        let pixels = match self.buffers.is_empty() {
            true => 0,
            false => self.buffers.iter().fold(FNV_BASIS, |hash, buffer| {
                fnv(fnv(hash, &buffer.edge.to_le_bytes()), &buffer.rgba)
            }),
        };
        IconKey {
            app_id: self.app_id.clone(),
            name: self.name.clone(),
            pixels,
        }
    }
}

const FNV_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// FNV-1a over `bytes`, continued from `hash`.
fn fnv(hash: u64, bytes: &[u8]) -> u64 {
    bytes.iter().fold(hash, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(FNV_PRIME)
    })
}

/// An answer: which raster it is, and its pixels (none when no icon was found).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slot {
    /// Unique among this cache's answers: a caller can tell a new icon from the one it drew
    /// last frame without comparing bytes.
    pub id: u64,
    /// The pixels, or nothing when the app has no icon the lookup could find.
    pub pixels: Option<Pixels>,
}

/// Icons by what was asked and the edge they were made at. Never read per frame without a
/// lookup in it already: [`IconCache::get`] reads no file.
#[derive(Debug, Clone, Default)]
pub struct IconCache {
    slots: BTreeMap<(IconKey, u32), Slot>,
    next: u64,
}

impl IconCache {
    /// An empty cache.
    pub fn new() -> IconCache {
        IconCache::default()
    }

    /// The answer kept for `key` at `edge`, if there is one.
    pub fn get(&self, key: &IconKey, edge: u32) -> Option<&Slot> {
        self.slots.get(&(key.clone(), edge))
    }

    /// The answer for `want` at `edge`: the kept one, else `finder`'s (which reads files), kept
    /// whether or not it found an icon, so a missing icon is not looked for again.
    pub fn ensure<T: ThemeLookup>(&mut self, want: &Want, edge: u32, finder: &Finder<T>) -> &Slot {
        match self.slots.entry((want.key(), edge)) {
            Entry::Occupied(kept) => kept.into_mut(),
            Entry::Vacant(room) => {
                self.next += 1;
                room.insert(Slot {
                    id: self.next,
                    pixels: finder.find(want, edge),
                })
            }
        }
    }

    /// Forgets everything (the overview closed).
    pub fn clear(&mut self) {
        self.slots.clear();
    }

    /// How many answers are kept.
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    /// Whether none is.
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }
}
