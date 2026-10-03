//! Minting Space ids. Entropy is an argument: the composition root seeds a [`SpaceSeed`] from
//! the system's randomness once, tests seed it with a constant, and minting stays pure.

use porter_core::SpaceId;

/// The source of new Space ids: a seed and how many ids it has minted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpaceSeed {
    entropy: [u8; 16],
    minted: u64,
}

impl SpaceSeed {
    /// A seed from 16 random bytes.
    pub fn from_entropy(entropy: [u8; 16]) -> SpaceSeed {
        SpaceSeed { entropy, minted: 0 }
    }

    /// The next Space id and the seed after it. Distinct for every call on one seed; derived
    /// from the seed's entropy and its count, so a fixed seed mints a fixed sequence.
    ///
    /// The count becomes a step (`(count + 1) × GOLDEN`, odd multiplier, so distinct counts
    /// give distinct steps) and goes through splitmix64's finalizer, a bijection on 64 bits:
    /// the low half of the minted entropy is `mix(low + step)`, so two counts never give one
    /// id, and an all-zero seed does not mint the all-zero id.
    pub fn next(self) -> (SpaceSeed, SpaceId) {
        let SpaceSeed { entropy, minted } = self;
        let (high, low) = halves(entropy);
        let step = minted.wrapping_add(1).wrapping_mul(GOLDEN);
        let low_out = mix(low.wrapping_add(step));
        let high_out = mix(high ^ step.rotate_left(32) ^ low_out);
        let mut mixed = [0u8; 16];
        mixed[..8].copy_from_slice(&high_out.to_be_bytes());
        mixed[8..].copy_from_slice(&low_out.to_be_bytes());
        (
            SpaceSeed {
                entropy,
                minted: minted.saturating_add(1),
            },
            mint_space_id(mixed),
        )
    }
}

/// The Space id for 16 bytes of entropy: `s-` and 32 lowercase hex digits, a valid porter
/// [`SpaceId`] (it starts with a lowercase letter, holds only `[0-9a-f-]` and is 34 bytes, within
/// the 64 every porter id allows).
pub fn mint_space_id(entropy: [u8; 16]) -> SpaceId {
    let hex: String = entropy.iter().map(|byte| format!("{byte:02x}")).collect();
    SpaceId::parse(&format!("s-{hex}"))
        .expect("`s-` and 32 hex digits is a porter slug (34 bytes, [a-z0-9-])")
}

/// The odd constant splitmix64 steps by (2^64 over the golden ratio).
const GOLDEN: u64 = 0x9e37_79b9_7f4a_7c15;

/// splitmix64's output function: a bijection on `u64`.
fn mix(value: u64) -> u64 {
    let a = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    let b = (a ^ (a >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    b ^ (b >> 31)
}

/// The big-endian halves of 16 bytes.
fn halves(entropy: [u8; 16]) -> (u64, u64) {
    let mut high = [0u8; 8];
    let mut low = [0u8; 8];
    high.copy_from_slice(&entropy[..8]);
    low.copy_from_slice(&entropy[8..]);
    (u64::from_be_bytes(high), u64::from_be_bytes(low))
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::collections::BTreeSet;

    /// The ids a seed mints, in order.
    fn minted(seed: SpaceSeed, count: usize) -> Vec<SpaceId> {
        let mut ids = Vec::with_capacity(count);
        let mut seed = seed;
        for _ in 0..count {
            let (next, id) = seed.next();
            ids.push(id);
            seed = next;
        }
        ids
    }

    #[test]
    fn a_fixed_seed_mints_a_pinned_sequence() {
        const CASES: &[(&str, [u8; 16], [&str; 2])] = &[(
            "zero",
            [0; 16],
            [
                "s-777b4ba101e5379fe220a8397b1dcdaf",
                "s-f7a7d58d9012ec3c6e789e6aa1b965f4",
            ],
        )];
        for (name, entropy, want) in CASES {
            let got: Vec<String> = minted(SpaceSeed::from_entropy(*entropy), 2)
                .iter()
                .map(|id| id.as_str().to_owned())
                .collect();
            assert_eq!(got, want.to_vec(), "{name}");
        }
    }

    proptest! {
        /// Whatever the entropy, a minted id is `s-` and 32 lowercase hex digits, a porter id.
        #[test]
        fn every_minted_id_is_a_porter_id(entropy in any::<[u8; 16]>()) {
            let id = mint_space_id(entropy);
            let text = id.as_str();
            prop_assert_eq!(text.len(), 34);
            prop_assert!(text.starts_with("s-"));
            prop_assert!(text[2..].bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)));
            prop_assert_eq!(SpaceId::parse(text), Ok(id));
        }

        /// One seed never repeats an id, and a seed's ids all parse.
        #[test]
        fn a_seed_never_repeats(entropy in any::<[u8; 16]>()) {
            let ids = minted(SpaceSeed::from_entropy(entropy), 256);
            prop_assert_eq!(ids.iter().collect::<BTreeSet<_>>().len(), 256);
            prop_assert!(ids.iter().all(|id| SpaceId::parse(id.as_str()).is_ok()));
        }
    }
}
