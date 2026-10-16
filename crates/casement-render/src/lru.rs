//! A small cache that drops its least recently used entry when it is full.
//!
//! The renderer keeps rasters and buffers between frames under keys that change (a window being
//! resized asks for a new size every frame). A cache that empties itself when full also drops
//! what every other window is drawing, which then rasters again all at once and changes every
//! element's id; dropping only the entry nobody asked for longest costs one raster.

use std::collections::BTreeMap;

/// Values by key, at most `limit` of them; the oldest-asked-for goes first.
#[derive(Debug, Clone)]
pub(crate) struct Lru<K, V> {
    entries: BTreeMap<K, (u64, V)>,
    /// Counts the asks, so each entry knows when it was last asked for.
    asks: u64,
    limit: usize,
}

impl<K: Ord + Clone, V: Clone> Lru<K, V> {
    /// An empty cache of at most `limit` entries (at least one).
    pub(crate) fn new(limit: usize) -> Lru<K, V> {
        Lru {
            entries: BTreeMap::new(),
            asks: 0,
            limit: limit.max(1),
        }
    }

    /// The value under `key`, made by `make` when there is none (the least recently asked-for
    /// entry goes if the cache is full).
    pub(crate) fn get_or_make(&mut self, key: K, make: impl FnOnce() -> V) -> V {
        self.asks = self.asks.saturating_add(1);
        let asked = self.asks;
        if let Some((last, value)) = self.entries.get_mut(&key) {
            *last = asked;
            return value.clone();
        }
        if self.entries.len() >= self.limit {
            let oldest = self
                .entries
                .iter()
                .min_by_key(|(_, (last, _))| *last)
                .map(|(key, _)| key.clone());
            if let Some(oldest) = oldest {
                self.entries.remove(&oldest);
            }
        }
        let value = make();
        self.entries.insert(key, (asked, value.clone()));
        value
    }

    /// Drops every entry.
    pub(crate) fn clear(&mut self) {
        self.entries.clear();
    }

    /// How many entries are held.
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_full_cache_drops_only_the_entry_asked_for_longest_ago() {
        let mut cache: Lru<u32, u32> = Lru::new(3);
        for key in [1, 2, 3] {
            cache.get_or_make(key, || key * 10);
        }
        // 1 is asked for again, so 2 is now the oldest.
        cache.get_or_make(1, || unreachable!("1 is held"));
        cache.get_or_make(4, || 40);
        assert_eq!(cache.len(), 3);
        cache.get_or_make(1, || unreachable!("1 stayed"));
        cache.get_or_make(3, || unreachable!("3 stayed"));
        let made = std::cell::Cell::new(0);
        cache.get_or_make(2, || {
            made.set(made.get() + 1);
            20
        });
        assert_eq!(made.get(), 1, "2 was the one dropped");
    }

    #[test]
    fn a_window_asked_for_every_frame_survives_a_hundred_new_sizes() {
        let mut cache: Lru<(u32, u32), u32> = Lru::new(8);
        cache.get_or_make((0, 0), || 0);
        for size in 0..100 {
            cache.get_or_make((0, 0), || unreachable!("the steady window is held"));
            cache.get_or_make((1, size), || size);
        }
        assert_eq!(cache.len(), 8);
    }

    #[test]
    fn clearing_forgets_everything() {
        let mut cache: Lru<u32, u32> = Lru::new(2);
        cache.get_or_make(1, || 1);
        cache.clear();
        assert_eq!(cache.len(), 0);
    }
}
