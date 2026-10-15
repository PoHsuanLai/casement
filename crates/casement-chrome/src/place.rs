//! Where the title goes in the titlebar, in device pixels: centred in the bar, pushed off the
//! lights and the right edge when it does not fit, truncated at its end when even the whole room
//! is too little.

use ds_core::geometry::scale::Scale;

/// What the titlebar keeps clear on each side of the title, in logical pixels: the lights on the
/// left (with a gap after them), a small margin on the right.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Margins {
    /// From the bar's left edge to where the title may start.
    pub left: u32,
    /// From the bar's right edge to where the title must end.
    pub right: u32,
}

impl Margins {
    /// The margins in device pixels at `scale`.
    ///
    /// ```
    /// use casement_chrome::Margins;
    /// use ds_core::geometry::scale::Scale;
    ///
    /// let margins = Margins { left: 73, right: 13 };
    /// assert_eq!(margins.in_device(Scale(240)), (146, 26));
    /// ```
    pub fn in_device(self, scale: Scale) -> (u32, u32) {
        (device(self.left, scale), device(self.right, scale))
    }
}

/// `logical` pixels at `scale`, rounded half up, the rounding of the surface buffers (so a
/// title raster is as big as the titlebar's).
///
/// ```
/// use casement_chrome::place::device;
/// use ds_core::geometry::scale::Scale;
///
/// assert_eq!(device(13, Scale(180)), 20);
/// ```
pub fn device(logical: u32, scale: Scale) -> u32 {
    let half = u64::from(Scale::DENOMINATOR / 2);
    let scaled =
        (u64::from(logical) * u64::from(scale.numerator()) + half) / u64::from(Scale::DENOMINATOR);
    u32::try_from(scaled).unwrap_or(u32::MAX)
}

/// The width the title may take in a bar `bar` wide with `left` and `right` kept clear.
///
/// ```
/// use casement_chrome::place::room;
///
/// assert_eq!(room(800, 73, 13), 714);
/// assert_eq!(room(60, 73, 13), 0);
/// ```
pub fn room(bar: u32, left: u32, right: u32) -> u32 {
    bar.saturating_sub(left.saturating_add(right))
}

/// Where a title `text` wide starts in a bar `bar` wide: centred, but never nearer the left
/// edge than `left` and never so far right that it passes `bar - right`. A title as wide as the
/// room (or wider) starts at `left`.
///
/// ```
/// use casement_chrome::place::left_edge;
///
/// assert_eq!(left_edge(100, 800, 73, 13), 350);
/// assert_eq!(left_edge(100, 200, 73, 13), 73);
/// ```
pub fn left_edge(text: u32, bar: u32, left: u32, right: u32) -> u32 {
    let centred = bar.saturating_sub(text) / 2;
    let latest = bar.saturating_sub(right).saturating_sub(text);
    centred.max(left).min(latest.max(left))
}

/// The most of `len` items that fit, when `fits(n)` says whether the first `n` do and fitting is
/// monotone (if `n` fit, so does every smaller number). Zero when not even one does.
///
/// ```
/// use casement_chrome::place::longest_fit;
///
/// assert_eq!(longest_fit(10, |n| n * 7 <= 30), 4);
/// ```
pub fn longest_fit(len: usize, mut fits: impl FnMut(usize) -> bool) -> usize {
    let (mut low, mut high) = (0, len);
    while low < high {
        let mid = low + (high - low).div_ceil(2);
        if fits(mid) {
            low = mid;
        } else {
            high = mid - 1;
        }
    }
    low
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logical_lengths_round_half_up_at_every_scale() {
        const CASES: &[(u32, u32, u32)] = &[
            (28, 120, 28),
            (28, 150, 35),
            (28, 180, 42),
            (28, 240, 56),
            (13, 180, 20),
            (13, 150, 16),
            (1, 150, 1),
            (0, 240, 0),
            (13, 0, 0),
        ];
        for (logical, scale, want) in CASES {
            let got = device(*logical, Scale(*scale));
            assert_eq!(got, *want, "{logical} at {scale}");
        }
    }

    #[test]
    fn the_room_is_what_the_margins_leave() {
        const CASES: &[(u32, u32, u32, u32)] = &[
            (800, 73, 13, 714),
            (86, 73, 13, 0),
            (60, 73, 13, 0),
            (0, 0, 0, 0),
            (100, 0, 0, 100),
        ];
        for (bar, left, right, want) in CASES {
            assert_eq!(room(*bar, *left, *right), *want, "{bar}");
        }
    }

    #[test]
    fn the_title_is_centred_unless_the_lights_or_the_edge_push_it() {
        // (text, bar, left, right, start)
        const CASES: &[(&str, u32, u32, u32, u32, u32)] = &[
            ("centred", 100, 800, 73, 13, 350),
            ("pushed off the lights", 100, 200, 73, 13, 73),
            ("as wide as the room", 714, 800, 73, 13, 73),
            ("wider than the room", 900, 800, 73, 13, 73),
            ("off centre by the lights", 300, 460, 73, 13, 80),
            ("no text", 0, 800, 73, 13, 400),
            ("no bar", 10, 0, 73, 13, 73),
        ];
        for (name, text, bar, left, right, want) in CASES {
            assert_eq!(left_edge(*text, *bar, *left, *right), *want, "{name}");
        }
    }

    #[test]
    fn the_longest_fit_is_found_for_any_width() {
        for len in 0..40 {
            for limit in 0..=(len * 7 + 3) {
                let got = longest_fit(len, |n| n * 7 <= limit);
                assert_eq!(got, (limit / 7).min(len), "len {len} limit {limit}");
            }
        }
    }

    #[test]
    fn margins_scale_to_device_pixels() {
        let margins = Margins {
            left: 73,
            right: 13,
        };
        assert_eq!(margins.in_device(Scale(120)), (73, 13));
        assert_eq!(margins.in_device(Scale(180)), (110, 20));
        assert_eq!(margins.in_device(Scale(240)), (146, 26));
    }
}
