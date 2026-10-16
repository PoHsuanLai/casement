//! What a monitor's EDID can say, and how two monitors that say the same are told apart.

use casement_backend::tty::{
    ConnectorFacts, ConnectorId, ConnectorStatus, KmsResources, attached_outputs, parse_edid,
};
use casement_core::OutputName;
use casement_core::output::EdidSerial;

/// A valid base block (header, make `ACM`, checksum) with `name` bytes in the monitor-name
/// descriptor and `serial` bytes in the serial descriptor, both exactly as given.
fn block(name: &[u8], serial: Option<&[u8]>) -> Vec<u8> {
    let mut block = vec![0u8; 128];
    block[..8].copy_from_slice(&[0, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0]);
    block[8..10].copy_from_slice(&0x046Du16.to_be_bytes());
    block[10..12].copy_from_slice(&0x1234u16.to_le_bytes());
    let mut put = |at: usize, tag: u8, text: &[u8]| {
        block[at + 3] = tag;
        block[at + 5..at + 18].fill(b' ');
        let used = text.len().min(13);
        block[at + 5..at + 5 + used].copy_from_slice(&text[..used]);
    };
    put(54, 0xfc, name);
    if let Some(serial) = serial {
        put(72, 0xff, serial);
    }
    let sum = block[..127]
        .iter()
        .fold(0u8, |sum, byte| sum.wrapping_add(*byte));
    block[127] = 0u8.wrapping_sub(sum);
    block
}

/// A case's name, the model and serial bytes in the EDID, and the model and serial read back.
type EdidCase<'a> = (&'a str, &'a [u8], &'a [u8], &'a str, &'a str);

#[test]
fn edid_text_keeps_printable_ascii_only() {
    let cases: [EdidCase; 4] = [
        ("plain", b"U2723QE\n", b"AB12\n", "U2723QE", "AB12"),
        ("a NUL inside", b"DE\0LL\n", b"7G\0K\n", "DELL", "7GK"),
        (
            "high bytes and controls",
            b"\xe3\x81\x82X\x07Y\n",
            b"\x80\x1b[0m1\n",
            "XY",
            "[0m1",
        ),
        (
            "no newline: the field ends it",
            b"ABCDEFGHIJKLM",
            b"1234567890123",
            "ABCDEFGHIJKLM",
            "1234567890123",
        ),
    ];
    for (name, model, serial, want_model, want_serial) in cases {
        let facts = parse_edid(&block(model, Some(serial))).expect(name);
        assert_eq!(facts.model, want_model, "{name}");
        assert_eq!(
            facts.serial,
            EdidSerial::Known(want_serial.to_owned()),
            "{name}"
        );
        assert!(
            facts
                .model
                .bytes()
                .chain(facts.make.bytes())
                .all(|byte| matches!(byte, 0x20..=0x7e)),
            "{name}"
        );
    }
}

#[test]
fn a_name_of_nothing_printable_falls_back_to_the_product_code() {
    let facts = parse_edid(&block(b"\0\x01\x02\n", None)).expect("edid");
    assert_eq!(facts.model, "1234");
}

fn connected(id: u32, name: &str, edid: Vec<u8>) -> ConnectorFacts {
    ConnectorFacts::new(
        ConnectorId(id),
        OutputName::parse(name).expect("a name"),
        ConnectorStatus::Connected,
    )
    .with_edid(edid)
}

#[test]
fn two_displays_with_one_serial_get_two_keys_and_the_first_keeps_its_edid_key() {
    let same = block(b"CLONE\n", Some(b"0001\n"));
    let alone = KmsResources::new(vec![connected(1, "DP-1", same.clone())], vec![], vec![]);
    let alone_key = attached_outputs(&alone)[&ConnectorId(1)].key.clone();
    assert_eq!(alone_key.as_str(), "edid:ACM/CLONE/0001");

    // Listed in the other order on purpose: the connector named first still keeps the EDID key.
    let both = KmsResources::new(
        vec![
            connected(2, "DP-2", same.clone()),
            connected(1, "DP-1", same),
        ],
        vec![],
        vec![],
    );
    let keys = attached_outputs(&both);
    assert_eq!(keys[&ConnectorId(1)].key, alone_key);
    assert_eq!(keys[&ConnectorId(2)].key.as_str(), "connector:DP-2");
}

#[test]
fn displays_with_different_serials_keep_their_edid_keys() {
    let resources = KmsResources::new(
        vec![
            connected(1, "DP-1", block(b"M\n", Some(b"A\n"))),
            connected(2, "DP-2", block(b"M\n", Some(b"B\n"))),
        ],
        vec![],
        vec![],
    );
    let keys = attached_outputs(&resources);
    assert_eq!(keys[&ConnectorId(1)].key.as_str(), "edid:ACM/M/A");
    assert_eq!(keys[&ConnectorId(2)].key.as_str(), "edid:ACM/M/B");
}
