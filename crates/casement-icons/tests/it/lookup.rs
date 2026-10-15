use crate::support::{FakeTheme, entry, png};
use casement_icons::{Finder, IconCache, RgbaBuffer, Want};
use std::collections::BTreeMap;

fn middle_pixel(want: &Want, finder: &Finder<FakeTheme>, edge: u32) -> Option<[u8; 4]> {
    let pixels = finder.find(want, edge)?;
    assert_eq!(
        (pixels.edge, pixels.rgba.len()),
        (edge, (edge * edge * 4) as usize)
    );
    let at = ((edge / 2 * edge + edge / 2) * 4) as usize;
    Some([
        pixels.rgba[at],
        pixels.rgba[at + 1],
        pixels.rgba[at + 2],
        pixels.rgba[at + 3],
    ])
}

const RED: [u8; 4] = [255, 0, 0, 255];
const GREEN: [u8; 4] = [0, 255, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];
const WHITE: [u8; 4] = [255, 255, 255, 255];

fn buffer(edge: u32, rgba: [u8; 4]) -> RgbaBuffer {
    RgbaBuffer {
        edge,
        rgba: rgba.repeat((edge * edge) as usize),
    }
}

#[test]
fn the_places_are_tried_in_order() {
    let dir = tempfile::tempdir().expect("scratch");
    let apps = dir.path().join("applications");
    std::fs::create_dir(&apps).expect("folder");
    let (by_name, by_entry, by_id) = (
        dir.path().join("name.png"),
        dir.path().join("entry.png"),
        dir.path().join("id.png"),
    );
    png(&by_name, 32, GREEN);
    png(&by_entry, 32, BLUE);
    png(&by_id, 32, WHITE);
    entry(&apps, "app", "Icon=from-entry");
    let theme = BTreeMap::from([
        ("named".to_owned(), by_name),
        ("from-entry".to_owned(), by_entry),
        ("app".to_owned(), by_id.clone()),
        ("lonely".to_owned(), by_id),
    ]);
    let finder = Finder {
        applications: vec![apps],
        theme: FakeTheme(theme),
    };
    let want = |app_id: &str, name: Option<&str>, buffers: Vec<RgbaBuffer>| Want {
        app_id: Some(app_id.to_owned()),
        name: name.map(str::to_owned),
        buffers,
    };
    // name, want, middle pixel
    let cases = [
        (
            "buffers first",
            want("app", Some("named"), vec![buffer(32, RED)]),
            Some(RED),
        ),
        (
            "then the name",
            want("app", Some("named"), vec![]),
            Some(GREEN),
        ),
        (
            "a name the theme lacks falls to the entry",
            want("app", Some("zzz"), vec![]),
            Some(BLUE),
        ),
        ("then the entry", want("app", None, vec![]), Some(BLUE)),
        (
            "then the app id as a name",
            want("lonely", None, vec![]),
            Some(WHITE),
        ),
        ("then nothing", want("none", None, vec![]), None),
    ];
    for (name, want, middle) in cases {
        assert_eq!(middle_pixel(&want, &finder, 32), middle, "{name}");
    }
}

#[test]
fn an_absolute_icon_path_is_used_directly() {
    let dir = tempfile::tempdir().expect("scratch");
    let apps = dir.path().join("applications");
    std::fs::create_dir(&apps).expect("folder");
    let file = dir.path().join("direct.png");
    png(&file, 16, RED);
    entry(&apps, "app", &format!("Icon={}", file.display()));
    let finder = Finder {
        applications: vec![apps],
        theme: FakeTheme(BTreeMap::new()),
    };
    let want = Want {
        app_id: Some("app".to_owned()),
        ..Want::default()
    };
    assert_eq!(middle_pixel(&want, &finder, 24), Some(RED));
}

#[test]
fn the_buffer_nearest_the_edge_wins_and_a_larger_one_breaks_a_tie() {
    let finder = Finder {
        applications: vec![],
        theme: FakeTheme(BTreeMap::new()),
    };
    // name, wanted edge, buffers, middle pixel
    let cases = [
        (
            "exact",
            64,
            vec![(32, RED), (64, GREEN), (128, BLUE)],
            GREEN,
        ),
        ("nearest", 100, vec![(32, RED), (128, BLUE)], BLUE),
        (
            "a tie goes to the larger",
            48,
            vec![(32, RED), (64, BLUE)],
            BLUE,
        ),
    ];
    for (name, edge, buffers, middle) in cases {
        let want = Want {
            buffers: buffers.into_iter().map(|(e, c)| buffer(e, c)).collect(),
            ..Want::default()
        };
        assert_eq!(middle_pixel(&want, &finder, edge), Some(middle), "{name}");
    }
}

#[test]
fn the_cache_looks_once_per_question_and_edge_and_remembers_a_miss() {
    let dir = tempfile::tempdir().expect("scratch");
    let file = dir.path().join("a.png");
    png(&file, 16, RED);
    let finder = Finder {
        applications: vec![],
        theme: FakeTheme(BTreeMap::from([("a".to_owned(), file.clone())])),
    };
    let named = |name: &str| Want {
        name: Some(name.to_owned()),
        ..Want::default()
    };
    let mut cache = IconCache::new();
    let first = cache.ensure(&named("a"), 32, &finder).clone();
    std::fs::remove_file(&file).expect("remove");
    assert_eq!(
        cache.ensure(&named("a"), 32, &finder),
        &first,
        "kept after the file went"
    );
    assert!(first.pixels.is_some());
    let other_edge = cache.ensure(&named("a"), 64, &finder).clone();
    assert!(
        other_edge.pixels.is_none(),
        "another edge is another question"
    );
    assert_ne!(other_edge.id, first.id);
    let miss = cache.ensure(&named("zzz"), 32, &finder).clone();
    assert_eq!(miss.pixels, None);
    assert_eq!(cache.len(), 3);
    assert_eq!(
        cache.get(&named("zzz").key(), 32).map(|slot| slot.id),
        Some(miss.id)
    );
    cache.clear();
    assert!(cache.is_empty());
}

#[test]
fn a_wayland_buffer_becomes_straight_rgba() {
    // B, G, R, A premultiplied.
    let got = RgbaBuffer::from_argb(1, &[0, 0, 128, 128]).expect("one pixel");
    assert_eq!(got.rgba, [255, 0, 0, 128]);
    let clear = RgbaBuffer::from_argb(1, &[9, 9, 9, 0]).expect("one pixel");
    assert_eq!(clear.rgba, [0, 0, 0, 0]);
    let opaque = RgbaBuffer::from_argb(1, &[1, 2, 3, 255]).expect("one pixel");
    assert_eq!(opaque.rgba, [3, 2, 1, 255]);
    assert!(RgbaBuffer::from_argb(2, &[0; 4]).is_none());
}
