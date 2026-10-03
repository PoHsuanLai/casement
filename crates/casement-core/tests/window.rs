//! App keys and Space ids (the tables P0-F1-window makes pass).

mod support;

use casement_core::app::AppKey;
use casement_core::space::{SpaceSeed, mint_space_id};
use porter_core::{AppId, AppName, Isolation, PeerIdentity, SpaceId};
use std::collections::BTreeSet;
use support::claimed;

#[test]
fn an_app_key_is_proven_only_by_its_peer() {
    let mail = AppId {
        name: AppName::parse("org.quire.Mail").expect("name"),
        isolation: Isolation::Unsandboxed,
    };
    let cases = [
        (
            "proven peer",
            PeerIdentity::Proven(mail.clone()),
            "foot",
            AppKey::Proven(mail),
        ),
        (
            "unproven peer",
            PeerIdentity::Unproven,
            "foot",
            AppKey::Claimed(claimed("foot")),
        ),
        (
            "an unproven claim of a real id",
            PeerIdentity::Unproven,
            "org.quire.Mail",
            AppKey::Claimed(claimed("org.quire.Mail")),
        ),
    ];
    for (name, peer, claim, want) in cases {
        assert_eq!(AppKey::of(&peer, &claimed(claim)), want, "{name}");
    }
}

#[test]
fn minted_space_ids_are_porter_ids() {
    let cases: [([u8; 16], &str); 3] = [
        ([0; 16], "s-00000000000000000000000000000000"),
        ([0xff; 16], "s-ffffffffffffffffffffffffffffffff"),
        (
            [
                0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x10, 0x32, 0x54, 0x76, 0x98, 0xba,
                0xdc, 0xfe,
            ],
            "s-0123456789abcdef1032547698badcfe",
        ),
    ];
    for (entropy, want) in cases {
        let id = mint_space_id(entropy);
        assert_eq!(id.as_str(), want);
        assert_eq!(SpaceId::parse(id.as_str()), Ok(id));
    }
}

#[test]
fn a_seed_mints_a_fixed_sequence_of_distinct_ids() {
    let mint = |seed: SpaceSeed| -> Vec<SpaceId> {
        (0..64)
            .scan(seed, |seed, _| {
                let (next, id) = seed.clone().next();
                *seed = next;
                Some(id)
            })
            .collect()
    };
    let a = mint(SpaceSeed::from_entropy([3; 16]));
    assert_eq!(
        a,
        mint(SpaceSeed::from_entropy([3; 16])),
        "a fixed seed is a fixed sequence"
    );
    assert_eq!(a.iter().collect::<BTreeSet<_>>().len(), 64, "no id repeats");
    assert_ne!(
        a[0],
        mint(SpaceSeed::from_entropy([4; 16]))[0],
        "the seed matters"
    );
}
