//! An icon's pixels.

use std::sync::Arc;

/// A square icon: premultiplied RGBA8, rows top to bottom, exactly `edge` by `edge` pixels (the
/// same form as the title rasters the renderer takes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pixels {
    /// The edge in device pixels.
    pub edge: u32,
    /// `edge * edge * 4` bytes, shared so a copy is cheap.
    pub rgba: Arc<[u8]>,
}

impl Pixels {
    /// `straight` (R, G, B, A, not premultiplied) as premultiplied pixels, or nothing when it
    /// is not `edge * edge` pixels.
    ///
    /// ```
    /// use casement_icons::Pixels;
    ///
    /// let one = Pixels::from_straight(1, &[200, 100, 0, 128]).unwrap();
    /// assert_eq!(&*one.rgba, &[100, 50, 0, 128]);
    /// assert!(Pixels::from_straight(2, &[0; 4]).is_none());
    /// ```
    pub fn from_straight(edge: u32, straight: &[u8]) -> Option<Pixels> {
        let wanted = usize::try_from(u64::from(edge) * u64::from(edge) * 4).ok()?;
        (straight.len() == wanted).then(|| Pixels {
            edge,
            rgba: straight
                .as_chunks::<4>()
                .0
                .iter()
                .flat_map(|px| premultiplied(*px))
                .collect(),
        })
    }
}

/// One pixel premultiplied, rounded to nearest.
fn premultiplied([r, g, b, a]: [u8; 4]) -> [u8; 4] {
    let mul = |c: u8| u8::try_from((u32::from(c) * u32::from(a) + 127) / 255).unwrap_or(u8::MAX);
    [mul(r), mul(g), mul(b), a]
}
