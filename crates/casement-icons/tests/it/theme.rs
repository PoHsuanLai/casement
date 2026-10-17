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

/// NEW-CASB-3: a pipe nobody writes to, or a file past the cap, where the settings file should
/// be is no theme and never a wait: the reader opens without blocking and refuses what is not a
/// small plain file.
#[test]
fn a_settings_file_that_is_a_pipe_or_huge_is_hicolor() {
    let dir = tempfile::tempdir().expect("a scratch folder");
    let file = theme_file(dir.path());
    std::fs::create_dir_all(file.parent().expect("a parent")).expect("the folder");
    rustix::fs::mkfifoat(
        rustix::fs::CWD,
        &file,
        rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR,
    )
    .expect("a pipe");
    assert_eq!(SystemTheme::of_desktop(dir.path()), SystemTheme::default());
    std::fs::remove_file(&file).expect("the pipe goes");
    let big = format!("gtk-icon-theme-name=Late\n{}", "#".repeat(70 * 1024));
    std::fs::write(&file, big).expect("a big file");
    assert_eq!(SystemTheme::of_desktop(dir.path()), SystemTheme::default());
}
