use crate::support::entry;
use casement_icons::{entry_for, parse_desktop_entry};

#[test]
fn the_parse_reads_only_the_desktop_entry_group() {
    // name, text, icon, class
    let cases = [
        (
            "plain",
            "[Desktop Entry]\nIcon=firefox\n",
            Some("firefox"),
            None,
        ),
        (
            "both keys, spaces round them",
            "[Desktop Entry]\n Icon = org.x.App \nStartupWMClass=XApp\n",
            Some("org.x.App"),
            Some("XApp"),
        ),
        (
            "an action's icon is not the app's",
            "[Desktop Entry]\nName=a\n[Desktop Action new]\nIcon=other\n",
            None,
            None,
        ),
        (
            "a localised key is another key",
            "[Desktop Entry]\nIcon[de]=bild\nIcon=pic\n",
            Some("pic"),
            None,
        ),
        ("a comment", "[Desktop Entry]\n# Icon=no\n", None, None),
        ("an empty value", "[Desktop Entry]\nIcon=\n", None, None),
        ("before any group", "Icon=no\n", None, None),
        (
            "an absolute path",
            "[Desktop Entry]\nIcon=/opt/a/b.png\n",
            Some("/opt/a/b.png"),
            None,
        ),
    ];
    for (name, text, icon, class) in cases {
        let got = parse_desktop_entry(text);
        assert_eq!(got.icon.as_deref(), icon, "{name}: icon");
        assert_eq!(got.window_class.as_deref(), class, "{name}: class");
    }
}

#[test]
fn an_entry_is_found_by_the_shared_rules_in_any_folder_depth() {
    let first = tempfile::tempdir().expect("scratch");
    let second = tempfile::tempdir().expect("scratch");
    entry(first.path(), "org.a.Exact", "Icon=exact");
    entry(first.path(), "zzz", "Icon=by-class\nStartupWMClass=Classy");
    entry(second.path(), "org.a.exact", "Icon=other-case");
    entry(second.path(), "Mixed.Case", "Icon=by-stem");
    entry(second.path(), "dup", "Icon=second-dir");
    entry(first.path(), "dup", "Icon=first-dir");
    std::fs::create_dir(first.path().join("kde")).expect("subfolder");
    entry(&first.path().join("kde"), "tool", "Icon=in-folder");
    entry(second.path(), "org.mozilla.firefox", "Icon=by-suffix");
    entry(
        second.path(),
        "spotify-launcher",
        "Icon=by-flatpak\nX-Flatpak=com.spotify.Client",
    );
    let dirs = [first.path().to_path_buf(), second.path().to_path_buf()];
    // name, app id, icon
    let cases = [
        ("the file named so", "org.a.Exact", Some("exact")),
        ("the class", "Classy", Some("by-class")),
        ("the stem in another case", "mixed.case", Some("by-stem")),
        ("the first folder wins", "dup", Some("first-dir")),
        ("a subfolder's dashed id", "kde-tool", Some("in-folder")),
        ("the last labels of the id", "firefox", Some("by-suffix")),
        ("the Flatpak id", "com.spotify.Client", Some("by-flatpak")),
        ("none", "nothing", None),
    ];
    for (name, app_id, icon) in cases {
        let got = entry_for(&dirs, app_id).and_then(|entry| entry.icon);
        assert_eq!(got.as_deref(), icon, "{name}");
    }
}

#[test]
fn a_missing_folder_is_skipped() {
    let there = tempfile::tempdir().expect("scratch");
    entry(there.path(), "app", "Icon=found");
    let dirs = [there.path().join("absent"), there.path().to_path_buf()];
    let got = entry_for(&dirs, "app").and_then(|entry| entry.icon);
    assert_eq!(got.as_deref(), Some("found"));
}
