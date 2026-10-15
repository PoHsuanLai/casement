//! Split View's row: the leading window, the divider and the trailing window side by side,
//! solved by Taffy inside the output (the first caller of the bridge outside its tests).

use super::{fixed_size, new_tree, rect_of, solve};
use casement_core::geometry::{LogicalRect, LogicalSize};
use taffy::{FlexDirection, Style, TaffyError, TaffyTree};

/// Where the three parts of a Split View sit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SplitRects {
    /// The leading (left) window's half.
    pub(crate) leading: LogicalRect,
    /// The strip between the halves that the divider is drawn in.
    pub(crate) divider: LogicalRect,
    /// The trailing (right) window's half.
    pub(crate) trailing: LogicalRect,
}

/// `area` laid out as a row: the leading half `leading_w` wide, the divider `gap` wide, and the
/// trailing half taking the rest, all as tall as `area`. None when Taffy refuses the tree.
pub(crate) fn split_rects(area: LogicalRect, leading_w: u32, gap: u32) -> Option<SplitRects> {
    solved(area, leading_w, gap).ok()
}

fn solved(area: LogicalRect, leading_w: u32, gap: u32) -> Result<SplitRects, TaffyError> {
    let mut tree: TaffyTree<()> = new_tree();
    let height = area.size.h;
    let fixed = |w: u32| Style {
        size: fixed_size(LogicalSize { w, h: height }),
        flex_shrink: 0.0,
        ..Style::default()
    };
    let leading = tree.new_leaf(fixed(leading_w))?;
    let divider = tree.new_leaf(fixed(gap))?;
    let trailing = tree.new_leaf(Style {
        flex_grow: 1.0,
        ..fixed(0)
    })?;
    let root = tree.new_with_children(
        Style {
            flex_direction: FlexDirection::Row,
            size: fixed_size(area.size),
            ..Style::default()
        },
        &[leading, divider, trailing],
    )?;
    solve(&mut tree, root, area.size)?;
    let at =
        |node| -> Result<LogicalRect, TaffyError> { Ok(rect_of(tree.layout(node)?, area.origin)) };
    Ok(SplitRects {
        leading: at(leading)?,
        divider: at(divider)?,
        trailing: at(trailing)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::geometry::LogicalPoint;

    #[test]
    fn the_row_tiles_the_area_exactly() {
        let area = LogicalRect::new(
            LogicalPoint { x: 10, y: 20 },
            LogicalSize { w: 1000, h: 500 },
        );
        let got = split_rects(area, 400, 8).expect("a row");
        let rect = |x, w| LogicalRect::new(LogicalPoint { x, y: 20 }, LogicalSize { w, h: 500 });
        assert_eq!(got.leading, rect(10, 400));
        assert_eq!(got.divider, rect(410, 8));
        assert_eq!(got.trailing, rect(418, 592));
    }
}
