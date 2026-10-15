//! A small bounded cache: the least recently used entry goes first.

/// At most `limit` entries; looking one up makes it the newest.
#[derive(Debug)]
pub(crate) struct Bounded<K, V> {
    limit: usize,
    entries: Vec<(K, V)>,
}

impl<K: PartialEq, V: Clone> Bounded<K, V> {
    /// An empty cache holding at most `limit` entries (at least one).
    pub(crate) fn new(limit: usize) -> Self {
        Bounded {
            limit: limit.max(1),
            entries: Vec::new(),
        }
    }

    /// The value under `key`, now the newest entry.
    pub(crate) fn get(&mut self, key: &K) -> Option<V> {
        let at = self.entries.iter().position(|(k, _)| k == key)?;
        let entry = self.entries.remove(at);
        let value = entry.1.clone();
        self.entries.push(entry);
        Some(value)
    }

    /// Remember `value` under `key` as the newest entry, dropping the oldest when full.
    pub(crate) fn put(&mut self, key: K, value: V) {
        self.entries.retain(|(k, _)| *k != key);
        if self.entries.len() >= self.limit {
            self.entries.remove(0);
        }
        self.entries.push((key, value));
    }

    /// How many entries it holds.
    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::Bounded;

    #[test]
    fn the_oldest_goes_first_and_a_hit_makes_an_entry_new() {
        let mut cache = Bounded::new(3);
        for n in 0..3u32 {
            cache.put(n, n * 10);
        }
        assert_eq!(cache.get(&0), Some(0));
        cache.put(3, 30);
        assert_eq!(cache.len(), 3);
        assert_eq!(cache.get(&1), None, "1 was the oldest");
        assert_eq!(cache.get(&0), Some(0));
        assert_eq!(cache.get(&2), Some(20));
        assert_eq!(cache.get(&3), Some(30));
    }

    #[test]
    fn a_key_put_twice_is_held_once() {
        let mut cache = Bounded::new(2);
        cache.put("a", 1);
        cache.put("a", 2);
        assert_eq!((cache.len(), cache.get(&"a")), (1, Some(2)));
    }

    #[test]
    fn a_limit_of_zero_still_holds_one() {
        let mut cache = Bounded::new(0);
        cache.put(1, 1);
        cache.put(2, 2);
        assert_eq!(
            (cache.len(), cache.get(&1), cache.get(&2)),
            (1, None, Some(2))
        );
    }
}
