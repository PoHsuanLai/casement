//! Names and files an app or a person can make up: none may hang, fill memory or lead out of the
//! icon folders.

use crate::support::{FakeTheme, entry, png};
use casement_icons::{Finder, IconError, IconName, Want, load};
use std::collections::BTreeMap;

const RED: [u8; 4] = [255, 0, 0, 255];

fn finder_in(dir: &std::path::Path) -> Finder<FakeTheme> {
    Finder {
        applications: vec![dir.to_owned()],
        theme: FakeTheme(BTreeMap::new()),
    }
}

#[test]
fn only_plain_names_are_icon_names() {
    let long = "a".repeat(129);
    // name, text, is a name
    let cases = [
        ("a theme name", "firefox", true),
        ("a reverse-dns name", "org.gnome.Nautilus", true),
        ("dashes, underscores, plus", "gtk-a_b+c", true),
        ("empty", "", false),
        ("an absolute path", "/dev/zero", false),
        ("a relative path", "../../etc/passwd", false),
        ("a folder inside", "apps/x", false),
        ("a hidden name", ".hidden", false),
        ("dots only", "..", false),
        ("a space", "my icon", false),
        ("a control byte", "a\u{0}b", false),
        ("too long", long.as_str(), false),
    ];
    for (name, text, plain) in cases {
        assert_eq!(IconName::parse(text).is_some(), plain, "{name}");
    }
}

#[test]
fn a_name_a_toplevel_sets_is_never_a_path() {
    let dir = tempfile::tempdir().expect("scratch");
    let file = dir.path().join("secret.png");
    png(&file, 16, RED);
    let finder = finder_in(dir.path());
    for name in [file.display().to_string(), "../secret.png".to_owned()] {
        let want = Want {
            name: Some(name.clone()),
            ..Want::default()
        };
        assert_eq!(finder.find(&want, 16), None, "{name}");
    }
}

#[test]
fn an_app_id_is_never_a_path() {
    let dir = tempfile::tempdir().expect("scratch");
    let file = dir.path().join("secret.png");
    png(&file, 16, RED);
    let want = Want {
        app_id: Some(file.display().to_string()),
        ..Want::default()
    };
    assert_eq!(finder_in(dir.path()).find(&want, 16), None);
}

#[test]
fn a_desktop_entry_may_name_an_icon_by_a_path() {
    let dir = tempfile::tempdir().expect("scratch");
    let file = dir.path().join("direct.png");
    png(&file, 16, RED);
    entry(dir.path(), "app", &format!("Icon={}", file.display()));
    let want = Want {
        app_id: Some("app".to_owned()),
        ..Want::default()
    };
    let got = finder_in(dir.path()).find(&want, 16).expect("found");
    assert_eq!(&got.rgba[8 * 16 * 4 + 8 * 4..][..4], &RED);
}

#[test]
fn a_pipe_named_by_an_entry_is_refused_at_once() {
    let dir = tempfile::tempdir().expect("scratch");
    let pipe = dir.path().join("pipe.png");
    rustix::fs::mkfifoat(
        rustix::fs::CWD,
        pipe.as_path(),
        rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR,
    )
    .expect("a pipe");
    entry(dir.path(), "app", &format!("Icon={}", pipe.display()));
    let want = Want {
        app_id: Some("app".to_owned()),
        ..Want::default()
    };
    assert_eq!(finder_in(dir.path()).find(&want, 16), None);
    assert!(matches!(load(&pipe, 16), Err(IconError::NotAFile { .. })));
}

#[test]
fn a_folder_is_not_an_icon() {
    let dir = tempfile::tempdir().expect("scratch");
    assert!(matches!(
        load(dir.path(), 16),
        Err(IconError::NotAFile { .. })
    ));
}

#[test]
fn a_file_over_the_cap_is_refused_unread() {
    let dir = tempfile::tempdir().expect("scratch");
    let file = dir.path().join("huge.png");
    let huge = std::fs::File::create(&file).expect("create");
    huge.set_len(16 * 1024 * 1024 + 1).expect("a sparse file");
    assert!(matches!(load(&file, 16), Err(IconError::TooLarge { .. })));
}

#[test]
fn an_svg_reads_no_file_outside_itself() {
    let dir = tempfile::tempdir().expect("scratch");
    let outside = dir.path().join("red.png");
    png(&outside, 16, RED);
    let svg = dir.path().join("sneaky.svg");
    std::fs::write(
        &svg,
        format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="16" height="16" viewBox="0 0 16 16"><image width="16" height="16" xlink:href="{}"/></svg>"#,
            outside.display()
        ),
    )
    .expect("write");
    let got = load(&svg, 16).expect("draws, empty");
    assert!(got.rgba.as_chunks::<4>().0.iter().all(|px| px[3] == 0));
}
