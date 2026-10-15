//! A fake theme and picture files in a scratch folder.

use casement_icons::ThemeLookup;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// A theme that knows these names.
pub struct FakeTheme(pub BTreeMap<String, PathBuf>);

impl ThemeLookup for FakeTheme {
    fn find(&self, name: &str, _edge: u32) -> Option<PathBuf> {
        self.0.get(name).cloned()
    }
}

/// A PNG of `edge` by `edge` pixels, all `rgba`, at `path`.
pub fn png(path: &Path, edge: u32, rgba: [u8; 4]) {
    image::RgbaImage::from_pixel(edge, edge, image::Rgba(rgba))
        .save(path)
        .expect("write the png");
}

/// A desktop entry file `name`.desktop in `dir` with `body`.
pub fn entry(dir: &Path, name: &str, body: &str) {
    std::fs::write(
        dir.join(format!("{name}.desktop")),
        format!("[Desktop Entry]\n{body}\n"),
    )
    .expect("write the entry");
}
