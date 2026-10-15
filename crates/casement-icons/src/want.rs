//! What an app offers to be drawn as: its own pixels, a name, an app id.

/// Whether a buffer's alpha byte means anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alpha {
    /// `Argb8888`: the fourth byte is the coverage.
    Used,
    /// `Xrgb8888`: the fourth byte is padding, the pixel is opaque.
    Opaque,
}

/// One of a toplevel's own icon buffers, as straight (not premultiplied) RGBA8, square.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RgbaBuffer {
    /// The edge in buffer pixels.
    pub edge: u32,
    /// `edge * edge * 4` bytes: R, G, B, A.
    pub rgba: Vec<u8>,
}

impl RgbaBuffer {
    /// A buffer from what a client's shm pool holds: `Argb8888` or `Xrgb8888` (little-endian, so
    /// the bytes are B, G, R, A), premultiplied as Wayland has it, rows `stride` bytes apart.
    /// Nothing when `bytes` is too short.
    ///
    /// ```
    /// use casement_icons::{Alpha, RgbaBuffer};
    ///
    /// let got = RgbaBuffer::from_wl(1, 4, &[0, 50, 100, 128], Alpha::Used).unwrap();
    /// assert_eq!(got.rgba, [199, 100, 0, 128]);
    /// assert!(RgbaBuffer::from_wl(2, 8, &[0; 4], Alpha::Used).is_none());
    /// ```
    pub fn from_wl(edge: u32, stride: usize, bytes: &[u8], alpha: Alpha) -> Option<RgbaBuffer> {
        let side = usize::try_from(edge).ok()?;
        let rows = (0..side).map(|row| bytes.get(row * stride..row * stride + side * 4));
        let rows: Option<Vec<&[u8]>> = rows.collect();
        let rgba = rows?
            .into_iter()
            .flat_map(|row| row.as_chunks::<4>().0.iter())
            .flat_map(|px| unpremultiplied(*px, alpha))
            .collect();
        Some(RgbaBuffer { edge, rgba })
    }
}

/// A shm pixel (B, G, R, A, premultiplied) as straight R, G, B, A.
fn unpremultiplied([b, g, r, a]: [u8; 4], alpha: Alpha) -> [u8; 4] {
    let a = match alpha {
        Alpha::Used => a,
        Alpha::Opaque => u8::MAX,
    };
    let lift = |c: u8| match a {
        0 => 0,
        _ => {
            u8::try_from((u32::from(c) * 255 + u32::from(a) / 2) / u32::from(a)).unwrap_or(u8::MAX)
        }
    };
    [lift(r), lift(g), lift(b), a]
}

/// What to find an icon for: any of the three may be missing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Want {
    /// The toplevel's `app_id`.
    pub app_id: Option<String>,
    /// The icon name the toplevel set through `xdg_toplevel_icon`.
    pub name: Option<String>,
    /// The icon buffers the toplevel set through `xdg_toplevel_icon`.
    pub buffers: Vec<RgbaBuffer>,
}
