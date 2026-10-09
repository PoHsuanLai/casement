//! Identifiers: output names parse at the boundary, output keys follow the display, the mint
//! counts.

use casement_core::output::{EdidFacts, EdidSerial};
use casement_core::{IdMint, LayerKey, LeaseId, OutputName, WindowId};

#[test]
fn output_names_parse_once() {
    const CASES: &[(&str, bool)] = &[
        ("DP-1", true),
        ("eDP-1", true),
        ("HDMI-A-1", true),
        ("HEADLESS-12", true),
        ("DP_1", true),
        ("", false),
        ("DP 1", false),
        ("DP/1", false),
        ("Ä-1", false),
    ];
    for (text, ok) in CASES {
        assert_eq!(OutputName::parse(text).is_ok(), *ok, "{text:?}");
    }
    assert!(OutputName::parse(&"D".repeat(63)).is_ok());
    assert!(OutputName::parse(&"D".repeat(64)).is_err());
    assert_eq!(OutputName::parse("DP-1").expect("valid").as_str(), "DP-1");
}

#[test]
fn an_output_key_follows_the_display_not_the_connector() {
    let with_serial = EdidFacts {
        make: "Dell".to_owned(),
        model: "U2723QE".to_owned(),
        serial: EdidSerial::Known("7XK".to_owned()),
    };
    let no_serial = EdidFacts {
        serial: EdidSerial::Unknown,
        ..with_serial.clone()
    };
    let dp1 = OutputName::parse("DP-1").expect("name");
    let dp2 = OutputName::parse("DP-2").expect("name");
    let a = casement_core::OutputKey::of(&with_serial, &dp1);
    assert_eq!(
        a,
        casement_core::OutputKey::of(&with_serial, &dp2),
        "replugged elsewhere"
    );
    assert_eq!(a.as_str(), "edid:Dell/U2723QE/7XK");
    assert_eq!(
        casement_core::OutputKey::of(&no_serial, &dp2).as_str(),
        "connector:DP-2"
    );
    assert_ne!(
        casement_core::OutputKey::of(&no_serial, &dp1),
        casement_core::OutputKey::of(&no_serial, &dp2)
    );
}

#[test]
fn the_mint_never_repeats_across_kinds() {
    let (mint, w) = IdMint::start().window();
    let (mint, l) = mint.layer();
    let (mint, e) = mint.lease();
    let (_, w2) = mint.window();
    assert_eq!(
        (w, l, e, w2),
        (WindowId(1), LayerKey(2), LeaseId(3), WindowId(4))
    );
}

#[test]
fn ids_serialise_as_bare_values() {
    assert_eq!(serde_json::to_string(&WindowId(9)).expect("json"), "9");
    let name: OutputName = serde_json::from_str("\"DP-3\"").expect("valid");
    assert_eq!(name.as_str(), "DP-3");
    assert!(serde_json::from_str::<OutputName>("\"no spaces\"").is_err());
}
