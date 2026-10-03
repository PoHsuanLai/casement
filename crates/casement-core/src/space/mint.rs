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
    pub fn next(self) -> (SpaceSeed, SpaceId) {
        let _ = (self.entropy, self.minted);
        todo!("P0-F1-window: mix the count into the entropy, mint_space_id, count + 1")
    }
}

/// The Space id for 16 bytes of entropy: `s-` and 32 lowercase hex digits, a valid porter
/// [`SpaceId`].
pub fn mint_space_id(entropy: [u8; 16]) -> SpaceId {
    let _ = entropy;
    todo!("P0-F1-window: \"s-\" + lowercase hex, parsed as a SpaceId")
}
