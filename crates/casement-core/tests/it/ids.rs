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

/// CASC-08: a `/` or `%` inside the EDID text cannot make two displays share a key; text with
/// neither keeps the key it always had.
#[test]
fn separators_inside_edid_text_are_encoded_so_distinct_displays_keep_distinct_keys() {
    let facts = |make: &str, model: &str, serial: &str| EdidFacts {
        make: make.to_owned(),
        model: model.to_owned(),
        serial: EdidSerial::Known(serial.to_owned()),
    };
    let dp1 = OutputName::parse("DP-1").expect("name");
    let key = |make: &str, model: &str, serial: &str| {
        casement_core::OutputKey::of(&facts(make, model, serial), &dp1)
            .as_str()
            .to_owned()
    };
    let cases = [
        (
            "plain text is unchanged",
            ("Dell", "U2723QE", "7XK"),
            "edid:Dell/U2723QE/7XK",
        ),
        ("a slash in the make", ("a/b", "c", "s"), "edid:a%2Fb/c/s"),
        ("a slash in the model", ("a", "b/c", "s"), "edid:a/b%2Fc/s"),
        ("a slash in the serial", ("a", "b", "c/d"), "edid:a/b/c%2Fd"),
        ("a percent sign", ("100%", "m", "s"), "edid:100%25/m/s"),
        (
            "a literal escape is itself escaped",
            ("a", "b%2Fc", "s"),
            "edid:a/b%252Fc/s",
        ),
    ];
    for (name, (make, model, serial), want) in cases {
        assert_eq!(key(make, model, serial), want, "{name}");
    }
    assert_ne!(key("a/b", "c", "s"), key("a", "b/c", "s"));
    assert_ne!(
        key("a", "b/c", "s"),
        key("a", "b%2Fc", "s"),
        "an escape in the text is not a slash"
    );
    assert_ne!(key("a", "b", "c/d"), key("a", "b/c", "d"));
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
