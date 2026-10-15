use casement_icons::{SystemTheme, theme_file};

fn settings_in(text: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("a scratch folder");
    let file = theme_file(dir.path());
    std::fs::create_dir_all(file.parent().expect("a parent")).expect("the folder");
    std::fs::write(file, text).expect("the file");
    dir
}

#[test]
fn the_desktop_theme_is_the_one_the_settings_file_names() {
    // name, file text, want
    let cases = [
        (
            "named",
            "[Settings]\ngtk-icon-theme-name=Adwaita\n",
            Some("Adwaita"),
        ),
        (
            "spaces and other keys",
            "gtk-font-name=x\n gtk-icon-theme-name = Papirus \n",
            Some("Papirus"),
        ),
        ("empty value", "gtk-icon-theme-name=\n", None),
        ("not there", "gtk-theme-name=Dark\n", None),
    ];
    for (name, text, want) in cases {
        let dir = settings_in(text);
        let got = SystemTheme::of_desktop(dir.path());
        assert_eq!(got, SystemTheme::new(want.map(str::to_owned)), "{name}");
    }
}

#[test]
fn no_settings_file_means_hicolor() {
    let dir = tempfile::tempdir().expect("a scratch folder");
    assert_eq!(SystemTheme::of_desktop(dir.path()), SystemTheme::default());
}
