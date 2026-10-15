//! What an app offers to be drawn as: its own pixels, a name, an app id.

/// One of a toplevel's own icon buffers, as straight (not premultiplied) RGBA8, square.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RgbaBuffer {
    /// The edge in buffer pixels.
    pub edge: u32,
    /// `edge * edge * 4` bytes: R, G, B, A.
    pub rgba: Vec<u8>,
}

impl RgbaBuffer {
    /// A buffer from `argb`, tightly packed rows of B, G, R, A premultiplied as Wayland's
    /// `Argb8888` has them (an `Xrgb8888` buffer with its alpha made opaque), `edge` pixels
    /// across. Nothing when `argb` is not `edge * edge` pixels.
    ///
    /// ```
    /// use casement_icons::RgbaBuffer;
    ///
    /// let got = RgbaBuffer::from_argb(1, &[0, 50, 100, 128]).unwrap();
    /// assert_eq!(got.rgba, [199, 100, 0, 128]);
    /// assert!(RgbaBuffer::from_argb(2, &[0; 4]).is_none());
    /// ```
    pub fn from_argb(edge: u32, argb: &[u8]) -> Option<RgbaBuffer> {
        let wanted = usize::try_from(u64::from(edge) * u64::from(edge) * 4).ok()?;
        (argb.len() == wanted).then(|| RgbaBuffer {
            edge,
            rgba: argb
                .as_chunks::<4>()
                .0
                .iter()
                .flat_map(|px| unpremultiplied(*px))
                .collect(),
        })
    }
}

/// A pixel (B, G, R, A, premultiplied) as straight R, G, B, A.
fn unpremultiplied([b, g, r, a]: [u8; 4]) -> [u8; 4] {
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
