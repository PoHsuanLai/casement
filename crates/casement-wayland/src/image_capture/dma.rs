//! The dma-buf formats a capture session offers.

use smithay::backend::allocator::{Format, Fourcc, Modifier};

/// The formats and modifiers a capture client may allocate: those `renderable` (the renderer can
/// draw a capture into them) that `accepted` (the dma-buf global takes the buffer from the
/// client). Grouped by format in the order the renderer lists them, each modifier once.
///
/// ```
/// use casement_wayland::image_capture::dma_offer;
/// use smithay::backend::allocator::{Format, Fourcc, Modifier};
///
/// let linear = |code| Format { code, modifier: Modifier::Linear };
/// let offer = dma_offer(&[linear(Fourcc::Argb8888)], &[linear(Fourcc::Argb8888), linear(Fourcc::Xrgb8888)]);
/// assert_eq!(offer, [(Fourcc::Argb8888, vec![Modifier::Linear])]);
/// ```
pub fn dma_offer(renderable: &[Format], accepted: &[Format]) -> Vec<(Fourcc, Vec<Modifier>)> {
    let mut offer: Vec<(Fourcc, Vec<Modifier>)> = Vec::new();
    for format in renderable.iter().filter(|f| accepted.contains(f)) {
        match offer.iter_mut().find(|(code, _)| *code == format.code) {
            Some((_, modifiers)) => {
                if !modifiers.contains(&format.modifier) {
                    modifiers.push(format.modifier);
                }
            }
            None => offer.push((format.code, vec![format.modifier])),
        }
    }
    offer
}

#[cfg(test)]
mod tests {
    use super::*;

    fn format(code: Fourcc, modifier: Modifier) -> Format {
        Format { code, modifier }
    }

    #[test]
    fn only_formats_both_sides_know_are_offered_grouped_by_format() {
        let argb = Fourcc::Argb8888;
        let xrgb = Fourcc::Xrgb8888;
        let tiled = Modifier::from(0x0100_0000_0000_0001_u64);
        let renderable = [
            format(argb, Modifier::Linear),
            format(xrgb, Modifier::Linear),
            format(argb, tiled),
            format(argb, Modifier::Linear),
            format(Fourcc::Abgr8888, Modifier::Linear),
        ];
        let accepted = [
            format(argb, tiled),
            format(argb, Modifier::Linear),
            format(Fourcc::Abgr8888, Modifier::Linear),
        ];
        assert_eq!(
            dma_offer(&renderable, &accepted),
            [
                (argb, vec![Modifier::Linear, tiled]),
                (Fourcc::Abgr8888, vec![Modifier::Linear]),
            ],
            "xrgb is not accepted; the repeated linear is listed once"
        );
    }

    #[test]
    fn nothing_renderable_or_nothing_accepted_offers_nothing() {
        let linear = format(Fourcc::Argb8888, Modifier::Linear);
        assert!(dma_offer(&[], &[linear]).is_empty());
        assert!(dma_offer(&[linear], &[]).is_empty());
    }
}
