//! Space ids (the tables P0-F1-window makes pass).

use casement_core::space::mint_space_id;
use porter_core::SpaceId;

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
