//! The picture a copy route carries from the render GPU to the scanout GPU: which format and
//! layout both can use.

use super::{FormatEntry, Fourcc, Modifier};

impl Modifier {
    /// The linear layout, the one every GPU reads and writes.
    pub const LINEAR: Modifier = Modifier(0);
}

/// The format of the buffer a copy route draws into: the render GPU draws it and the scanout
/// GPU samples it. The first of `wanted` (best first) that both lists carry, in the linear
/// layout when both can, else in the lowest layout both name (two GPUs of one vendor share
/// tiled layouts). None when no wanted format is common to both: the route cannot be built.
///
/// ```
/// use casement_core::gpu::{FormatEntry, Fourcc, Modifier, bridge_format};
///
/// let xrgb = Fourcc(0x3432_5258);
/// let entry = |modifier| FormatEntry { format: xrgb, modifier: Modifier(modifier) };
/// let render = [entry(7), entry(0)];
/// let scanout = [entry(0)];
/// assert_eq!(bridge_format(&render, &scanout, &[xrgb]), Some(entry(0)));
/// assert_eq!(bridge_format(&render, &[], &[xrgb]), None);
/// ```
pub fn bridge_format(
    render: &[FormatEntry],
    scanout: &[FormatEntry],
    wanted: &[Fourcc],
) -> Option<FormatEntry> {
    wanted.iter().find_map(|format| {
        let in_both = |entry: &&FormatEntry| entry.format == *format && scanout.contains(entry);
        render
            .iter()
            .filter(in_both)
            .min_by_key(|entry| (entry.modifier != Modifier::LINEAR, entry.modifier))
            .copied()
    })
}
