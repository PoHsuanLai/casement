//! sill's whole `PreferWhole` table, ported to 120ths, and properties over modes and places.

use super::{
    EdidFacts, EdidSerial, Mode, ModePreference, ModeWish, OutputConfig, OutputFacts, OutputPower,
    PhysicalMm, SavedOutputs, ScalePolicy, VrrCapability, first_sight_scale, pick_mode,
    place_outputs,
};
use crate::geometry::{LogicalPoint, PhysicalSize, Transform};
use crate::ids::{OutputKey, OutputName};
use crate::time::Millihertz;
use ds_core::geometry::scale::Scale;
use proptest::prelude::*;
use std::collections::BTreeMap;

fn mode(w: u32, h: u32, millihertz: u32, preference: ModePreference) -> Mode {
    Mode {
        size: PhysicalSize { w, h },
        refresh: Millihertz(millihertz),
        preference,
    }
}

fn display(name: &str, modes: Vec<Mode>, mm: (u32, u32)) -> OutputFacts {
    let edid = EdidFacts {
        make: "Test".to_owned(),
        model: name.to_owned(),
        serial: EdidSerial::Unknown,
    };
    let name = OutputName::parse(name).expect("name");
    OutputFacts {
        key: OutputKey::of(&edid, &name),
        name,
        edid,
        modes,
        physical: PhysicalMm { w: mm.0, h: mm.1 },
        vrr: VrrCapability::Incapable,
    }
}

/// The panel size of a `diagonal_in` display whose pixels are square, to the millimetre.
fn size_of(diagonal_in: f64, w: u32, h: u32) -> (u32, u32) {
    let diagonal_px = f64::from(w).hypot(f64::from(h));
    let mm_per_px = diagonal_in * 25.4 / diagonal_px;
    (
        (f64::from(w) * mm_per_px).round() as u32,
        (f64::from(h) * mm_per_px).round() as u32,
    )
}

/// A label, the panel in pixels, its size in millimetres (None: unreported), the scale sill's
/// `PreferWhole` gives in hundredths.
type Row = (&'static str, (u32, u32), Option<(u32, u32)>, u16);

#[test]
fn whole_like_mac_matches_sills_prefer_whole_table() {
    let rows: Vec<Row> = vec![
        (
            "27in 4K",
            (3840, 2160),
            Some(size_of(27.0, 3840, 2160)),
            200,
        ),
        ("BenQ MA270U", (3840, 2160), Some((597, 336)), 200),
        (
            "27in 5K",
            (5120, 2880),
            Some(size_of(27.0, 5120, 2880)),
            200,
        ),
        (
            "24in 1080p",
            (1920, 1080),
            Some(size_of(24.0, 1920, 1080)),
            100,
        ),
        (
            "13.3in 2560x1600",
            (2560, 1600),
            Some(size_of(13.3, 2560, 1600)),
            200,
        ),
        (
            "14in 2880x1800",
            (2880, 1800),
            Some(size_of(14.0, 2880, 1800)),
            200,
        ),
        (
            "32in 4K",
            (3840, 2160),
            Some(size_of(32.0, 3840, 2160)),
            125,
        ),
        (
            "27in 1440p",
            (2560, 1440),
            Some(size_of(27.0, 2560, 1440)),
            100,
        ),
        (
            "15.6in 1080p",
            (1920, 1080),
            Some(size_of(15.6, 1920, 1080)),
            125,
        ),
        (
            "14in 1920x1200",
            (1920, 1200),
            Some(size_of(14.0, 1920, 1200)),
            150,
        ),
        (
            "16in 2560x1600",
            (2560, 1600),
            Some(size_of(16.0, 2560, 1600)),
            200,
        ),
        (
            "13.6in 2560x1664",
            (2560, 1664),
            Some(size_of(13.6, 2560, 1664)),
            200,
        ),
        (
            "34in ultrawide",
            (3440, 1440),
            Some(size_of(34.0, 3440, 1440)),
            100,
        ),
        (
            "32in 6K",
            (6016, 3384),
            Some(size_of(32.0, 6016, 3384)),
            200,
        ),
        // 2x would leave 1128 points across, under the floor: nearest step.
        (
            "13.5in 2256x1504",
            (2256, 1504),
            Some(size_of(13.5, 2256, 1504)),
            175,
        ),
        (
            "15.6in 4K stays 2x",
            (3840, 2160),
            Some(size_of(15.6, 3840, 2160)),
            200,
        ),
        ("projector 1080p", (1920, 1080), None, 100),
        ("projector 4K", (3840, 2160), None, 200),
        // A TV whose EDID says 16 x 9 cm: 13 700 ppi is not a display.
        ("4K TV, nonsense size", (3840, 2160), Some((160, 90)), 200),
    ];
    for (label, (w, h), mm, want) in rows {
        let facts = display(
            "DP-1",
            vec![mode(w, h, 60_000, ModePreference::Preferred)],
            mm.unwrap_or((0, 0)),
        );
        assert_eq!(
            first_sight_scale(&facts, &ScalePolicy::WholeLikeMac),
            Scale::from_percent(want),
            "{label}"
        );
    }
}

#[test]
fn the_scale_follows_the_preferred_mode_not_the_fastest() {
    let facts = display(
        "DP-1",
        vec![
            mode(1920, 1080, 240_000, ModePreference::Other),
            mode(3840, 2160, 60_000, ModePreference::Preferred),
        ],
        (597, 336),
    );
    assert_eq!(
        first_sight_scale(&facts, &ScalePolicy::WholeLikeMac),
        Scale(240)
    );
}

#[test]
fn an_output_without_modes_has_no_mode_and_the_smallest_scale() {
    let facts = display("DP-1", Vec::new(), (597, 336));
    assert_eq!(
        pick_mode(&facts, &ModeWish::Preferred),
        mode(0, 0, 0, ModePreference::Other)
    );
    assert_eq!(
        first_sight_scale(&facts, &ScalePolicy::WholeLikeMac),
        Scale(120)
    );
}

#[test]
fn a_saved_rotated_display_takes_its_turned_width() {
    let big = display(
        "DP-1",
        vec![mode(3840, 2160, 60_000, ModePreference::Preferred)],
        (597, 336),
    );
    let newcomer = display(
        "DP-2",
        vec![mode(1920, 1080, 60_000, ModePreference::Preferred)],
        (0, 0),
    );
    let saved = SavedOutputs(BTreeMap::from([(
        big.key.clone(),
        OutputConfig {
            mode: big.modes[0],
            scale: Scale(240),
            transform: Transform::R90,
            position: LogicalPoint { x: 100, y: -50 },
            power: OutputPower::Off,
        },
    )]));
    let got = place_outputs(
        &[big.clone(), newcomer.clone()],
        &saved,
        &ScalePolicy::WholeLikeMac,
    );
    // 3840 x 2160 turned and halved is 1080 wide: the newcomer starts at 100 + 1080, level with
    // the top of what is placed, and the saved display keeps its power state.
    assert_eq!(got[0].1, saved.0[&big.key]);
    assert_eq!(got[1].1.position, LogicalPoint { x: 1180, y: -50 });
}

fn arbitrary_mode() -> impl Strategy<Value = Mode> {
    (1u32..8000, 1u32..5000, 1u32..500_000, any::<bool>()).prop_map(|(w, h, hz, preferred)| Mode {
        size: PhysicalSize { w, h },
        refresh: Millihertz(hz),
        preference: if preferred {
            ModePreference::Preferred
        } else {
            ModePreference::Other
        },
    })
}

fn arbitrary_wish() -> impl Strategy<Value = ModeWish> {
    prop_oneof![
        Just(ModeWish::Preferred),
        arbitrary_mode().prop_map(ModeWish::Exact),
        (1u32..8000, 1u32..5000)
            .prop_map(|(w, h)| ModeWish::HighestRefreshAt(PhysicalSize { w, h })),
    ]
}

proptest! {
    #[test]
    fn a_picked_mode_is_always_one_the_display_offers(
        modes in prop::collection::vec(arbitrary_mode(), 1..8), wish in arbitrary_wish(),
    ) {
        let facts = display("DP-1", modes.clone(), (0, 0));
        prop_assert!(modes.contains(&pick_mode(&facts, &wish)));
    }

    #[test]
    fn a_denser_display_never_gets_a_smaller_scale(
        w in 800u32..8000, h in 600u32..4500, small in 100u32..900, bigger in 0u32..400,
    ) {
        // The same panel stretched over a smaller area is denser: the scale cannot fall. (A
        // density outside 40 to 600 ppi is read as a wrong EDID and falls back to a size-less
        // rule, so the claim holds inside that range.)
        let ppi = |diagonal_tenths: u32| f64::from(w).hypot(f64::from(h)) * 10.0 / f64::from(diagonal_tenths);
        prop_assume!((45.0..=590.0).contains(&ppi(small + 100)) && (45.0..=590.0).contains(&ppi(small + bigger + 100)));
        let at = |diagonal_tenths: u32| {
            let (mw, mh) = size_of(f64::from(diagonal_tenths) / 10.0, w, h);
            let facts = display("DP-1", vec![mode(w, h, 60_000, ModePreference::Preferred)], (mw, mh));
            first_sight_scale(&facts, &ScalePolicy::WholeLikeMac)
        };
        let larger_panel = at(small + bigger + 100);
        let smaller_panel = at(small + 100);
        prop_assert!(smaller_panel >= larger_panel, "{smaller_panel:?} < {larger_panel:?}");
    }

    #[test]
    fn a_first_scale_is_a_step_the_renderer_draws(
        w in 640u32..8000, h in 480u32..4500, mw in 0u32..1200, mh in 0u32..700,
    ) {
        let facts = display("DP-1", vec![mode(w, h, 60_000, ModePreference::Preferred)], (mw, mh));
        let scale = first_sight_scale(&facts, &ScalePolicy::WholeLikeMac);
        let steps = [100u16, 125, 150, 175, 200, 250, 300].map(Scale::from_percent);
        prop_assert!(steps.contains(&scale), "{scale:?}");
    }

    #[test]
    fn placing_keeps_order_keeps_saved_and_never_overlaps_newcomers(
        sizes in prop::collection::vec((640u32..4000, 480u32..2200), 1..6),
        keep in prop::collection::vec(any::<bool>(), 6),
    ) {
        let present: Vec<OutputFacts> = sizes
            .iter()
            .enumerate()
            .map(|(n, (w, h))| {
                display(&format!("DP-{n}"), vec![mode(*w, *h, 60_000, ModePreference::Preferred)], (0, 0))
            })
            .collect();
        let first = place_outputs(&present, &SavedOutputs::default(), &ScalePolicy::WholeLikeMac);
        let keys: Vec<_> = present.iter().map(|facts| facts.key.clone()).collect();
        prop_assert_eq!(first.iter().map(|(key, _)| key.clone()).collect::<Vec<_>>(), keys);
        // Every newcomer sits flush right of the one before: no gaps, no overlap.
        for pair in first.windows(2) {
            let (a, b) = (&pair[0].1, &pair[1].1);
            let width = i64::from(a.mode.size.w) * 120 / i64::from(a.scale.numerator());
            prop_assert!(i64::from(b.position.x) >= i64::from(a.position.x) + width - 1);
        }
        // Saving some and placing again keeps those exactly, and a second pass changes nothing.
        let saved = SavedOutputs(
            first.iter().zip(&keep).filter(|(_, k)| **k).map(|((key, config), _)| (key.clone(), *config)).collect(),
        );
        let again = place_outputs(&present, &saved, &ScalePolicy::WholeLikeMac);
        for ((key, config), (key2, config2)) in first.iter().zip(&again) {
            prop_assert_eq!(key, key2);
            if saved.0.contains_key(key) {
                prop_assert_eq!(config, config2);
            }
        }
        let everything = SavedOutputs(again.iter().cloned().collect());
        prop_assert_eq!(place_outputs(&present, &everything, &ScalePolicy::WholeLikeMac), again);
    }
}
