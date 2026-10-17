//! Who is cut when the session has no room for another window: the client holding the most, not
//! the one that happened to ask last. One client that fills the session's window cap would
//! otherwise get the next innocent client cut at its first window.

use super::XdgShellPart;
use casement_core::ClientKey;
use smithay::reexports::wayland_server::{Client, Resource};
use std::collections::BTreeMap;

/// The client to cut: the one of `held` (windows per client) holding the most. When `asker` holds
/// as many as the largest other client, it is `asker` (a tie is the asker's to lose), so an
/// asker is never spared for holding no less than the rest.
pub fn heaviest_holder(held: &BTreeMap<ClientKey, usize>, asker: ClientKey) -> ClientKey {
    let start = (asker, held.get(&asker).copied().unwrap_or(0));
    held.iter()
        .filter(|(client, _)| **client != asker)
        .fold(start, |best, (client, count)| match *count > best.1 {
            true => (*client, *count),
            false => best,
        })
        .0
}

impl XdgShellPart {
    /// How many toplevels each client holds, mapped or not.
    pub fn toplevels_by_client(&self) -> BTreeMap<ClientKey, usize> {
        self.records
            .values()
            .fold(BTreeMap::new(), |mut held, record| {
                *held.entry(record.client).or_insert(0) += 1;
                held
            })
    }

    /// The connection of `client`, found through one of its toplevels (this part keeps shell
    /// clients by key, but not the connection a cut needs).
    pub fn connection_of(&self, client: ClientKey) -> Option<Client> {
        self.records
            .iter()
            .filter(|(_, record)| record.client == client)
            .find_map(|(window, _)| self.toplevels.get(window)?.wl_surface().client())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// (case, windows held per client, the asker, who is cut)
    #[test]
    fn the_client_holding_the_most_windows_is_cut_and_the_asker_when_it_is_that_client() {
        let held = |rows: &[(u64, usize)]| -> BTreeMap<ClientKey, usize> {
            rows.iter()
                .map(|(n, count)| (ClientKey(*n), *count))
                .collect()
        };
        let cases = [
            (
                "a flood and an innocent asker",
                held(&[(1, 4000), (2, 96)]),
                3,
                1,
            ),
            (
                "an innocent asker holding a few",
                held(&[(1, 4000), (2, 90), (3, 6)]),
                3,
                1,
            ),
            ("the asker is the flood", held(&[(1, 4000), (2, 96)]), 1, 1),
            ("a tie is the asker's", held(&[(1, 2048), (2, 2048)]), 2, 2),
            (
                "the larger of two others",
                held(&[(1, 10), (2, 20), (3, 5)]),
                3,
                2,
            ),
            ("nobody else holds any", held(&[(7, 3)]), 7, 7),
            ("nobody holds any", held(&[]), 9, 9),
        ];
        for (case, held, asker, cut) in cases {
            assert_eq!(
                heaviest_holder(&held, ClientKey(asker)),
                ClientKey(cut),
                "{case}"
            );
        }
    }
}
