//! The soft sides of a genie strip. A strip covers whole pixel columns, but the curve puts its
//! edges between pixels, and a side drawn in whole pixels steps from row to row. The strip is
//! cut into its body (drawn at full strength) and its two edge columns, each drawn at the share
//! of the pixel the window covers, in `STEPS` steps: the window's own pixels are drawn again at
//! that fraction of their alpha (`genie.rs` builds them), so it works the same on every
//! renderer. A column that is all but covered joins the body; one that is all but empty is
//! left out.

use super::strip::StripPlan;
use ds_core::vocab::Fraction;
use smithay::utils::Rectangle;

/// How many steps of strength an edge column is drawn at (the error is under `1/(2 * STEPS)`).
pub const STEPS: u8 = 4;

/// How many ids one strip's parts use: the body, and each side at each partial strength.
const PARTS: usize = 2 * STEPS as usize;

/// How strongly a piece is drawn: `steps` of [`STEPS`] of the window's own alpha.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Weight(u8);

impl Weight {
    /// Drawn as the window is.
    pub const FULL: Weight = Weight(STEPS);

    /// The step nearest `coverage` (0 to 1); none when that is nothing.
    pub fn of(coverage: f64) -> Option<Weight> {
        let steps = (coverage * f64::from(STEPS))
            .round()
            .clamp(0.0, f64::from(STEPS)) as u8;
        (steps > 0).then_some(Weight(steps))
    }

    /// The strengths below full, weakest first.
    pub fn partial() -> impl Iterator<Item = Weight> {
        (1..STEPS).map(Weight)
    }

    /// The window's `opacity` at this strength.
    pub fn of_opacity(self, opacity: Fraction) -> Fraction {
        let scaled = u32::from(opacity.0) * u32::from(self.0) / u32::from(STEPS);
        Fraction(u16::try_from(scaled).unwrap_or(opacity.0))
    }

    /// Whether this is full strength.
    pub fn is_full(self) -> bool {
        self == Weight::FULL
    }
}

/// One piece of a strip's width and how strongly it is drawn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Part {
    /// How strongly it is drawn.
    pub weight: Weight,
    /// Where: the strip's plan with the band narrowed to this piece and its own id.
    pub plan: StripPlan,
}

impl StripPlan {
    /// The pieces of this strip: its body at full strength and each edge column that is only
    /// partly covered at its share. They never overlap and lie inside the band.
    pub fn soft_parts(&self) -> Vec<Part> {
        let (width, height) = (self.band.size.w, self.band.size.h);
        let part = |code: usize, from: i32, wide: i32, weight: Weight| Part {
            weight,
            plan: StripPlan {
                index: self.index * PARTS + code,
                band: Rectangle::new(
                    (self.band.loc.x + from, self.band.loc.y).into(),
                    (wide, height).into(),
                ),
                ..*self
            },
        };
        let (left, right) = self.edges;
        if width == 1 {
            let both = Weight::of(left + right - 1.0);
            return both
                .map(|weight| part(code(weight, 0), 0, 1, weight))
                .into_iter()
                .collect();
        }
        let sides = (Weight::of(left), Weight::of(right));
        let from = i32::from(!sides.0.is_some_and(Weight::is_full));
        let to = width - i32::from(!sides.1.is_some_and(Weight::is_full));
        let soft = |weight: Option<Weight>| weight.filter(|w| !w.is_full());
        let body = (to > from).then(|| part(0, from, to - from, Weight::FULL));
        let left_edge = soft(sides.0).map(|w| part(code(w, 0), 0, 1, w));
        let right_edge = soft(sides.1).map(|w| part(code(w, 1), width - 1, 1, w));
        body.into_iter().chain(left_edge).chain(right_edge).collect()
    }
}

/// The id of a piece: 0 for the body (full strength), else its side and strength.
fn code(weight: Weight, side: usize) -> usize {
    match weight.is_full() {
        true => 0,
        false => side * usize::from(STEPS - 1) + usize::from(weight.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(width: i32, edges: (f64, f64)) -> StripPlan {
        StripPlan {
            index: 2,
            origin: (0, 0).into(),
            scale: (1.0, 1.0),
            band: Rectangle::new((10, 20).into(), (width, 3).into()),
            shift: (0, 0).into(),
            source: (0.0, 3.0),
            edges,
        }
    }

    fn columns(parts: &[Part]) -> Vec<(i32, i32, u8)> {
        parts
            .iter()
            .map(|p| (p.plan.band.loc.x, p.plan.band.size.w, p.weight.0))
            .collect()
    }

    #[test]
    fn a_strip_with_whole_edges_is_one_body() {
        let parts = plan(40, (1.0, 0.95)).soft_parts();
        assert_eq!(columns(&parts), [(10, 40, STEPS)]);
    }

    #[test]
    fn a_half_covered_edge_is_a_column_at_half_strength() {
        let parts = plan(40, (0.5, 0.1)).soft_parts();
        // the right column is all but empty, so it is left out; the left one is drawn alone
        assert_eq!(columns(&parts), [(11, 38, STEPS), (10, 1, 2)]);
        let both = plan(40, (0.5, 0.75)).soft_parts();
        assert_eq!(columns(&both), [(11, 38, STEPS), (10, 1, 2), (49, 1, 3)]);
    }

    #[test]
    fn the_parts_never_overlap_and_have_their_own_ids() {
        for edges in [(0.3, 0.6), (1.0, 0.4), (0.0, 0.0), (0.7, 1.0)] {
            for width in [1, 2, 3, 40] {
                let parts = plan(width, edges).soft_parts();
                for (i, one) in parts.iter().enumerate() {
                    assert!(one.plan.band.size.w > 0);
                    for other in &parts[i + 1..] {
                        assert!(!one.plan.band.overlaps(other.plan.band), "{edges:?}");
                        assert_ne!(one.plan.index, other.plan.index, "{edges:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn a_one_column_strip_is_drawn_at_what_the_window_covers_of_it() {
        let parts = plan(1, (0.75, 0.5)).soft_parts();
        assert_eq!(columns(&parts), [(10, 1, 1)]);
        assert!(plan(1, (0.3, 0.3)).soft_parts().is_empty());
    }

    #[test]
    fn a_weight_scales_the_opacity() {
        let half = Weight::of(0.5).expect("a weight");
        assert_eq!(half.of_opacity(Fraction(1000)), Fraction(500));
        assert_eq!(Weight::FULL.of_opacity(Fraction(700)), Fraction(700));
        assert_eq!(Weight::of(0.05), None);
        assert_eq!(Weight::partial().count(), usize::from(STEPS) - 1);
    }
}
