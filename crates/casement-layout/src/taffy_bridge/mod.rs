//! The only module naming taffy (boundary row B2): building a Taffy tree from logical sizes,
//! solving it, and rounding its float layout back to logical pixels.

mod round;
mod style;
mod tree;

pub(crate) use round::rect_of;
pub(crate) use style::fixed_size;
pub(crate) use tree::{new_tree, solve};

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};
    use taffy::{FlexDirection, Style, TaffyTree};

    #[test]
    fn a_row_of_two_fixed_boxes_lands_on_whole_pixels() {
        let mut tree: TaffyTree<()> = new_tree();
        let leaf = |tree: &mut TaffyTree<()>, w, h| {
            tree.new_leaf(Style {
                size: fixed_size(LogicalSize { w, h }),
                ..Style::default()
            })
            .expect("leaf")
        };
        let a = leaf(&mut tree, 100, 50);
        let b = leaf(&mut tree, 101, 50);
        let c = tree
            .new_leaf(Style {
                flex_grow: 1.0,
                ..Style::default()
            })
            .expect("leaf");
        let root = tree
            .new_with_children(
                Style {
                    flex_direction: FlexDirection::Row,
                    size: fixed_size(LogicalSize { w: 300, h: 100 }),
                    ..Style::default()
                },
                &[a, b, c],
            )
            .expect("root");
        solve(&mut tree, root, LogicalSize { w: 300, h: 100 }).expect("solve");
        let origin = LogicalPoint { x: 10, y: -20 };
        let rect = |node| rect_of(tree.layout(node).expect("layout"), origin);
        assert_eq!(
            rect(a),
            LogicalRect::new(
                LogicalPoint { x: 10, y: -20 },
                LogicalSize { w: 100, h: 50 }
            )
        );
        assert_eq!(
            rect(b),
            LogicalRect::new(
                LogicalPoint { x: 110, y: -20 },
                LogicalSize { w: 101, h: 50 }
            )
        );
        assert_eq!(
            rect(c),
            LogicalRect::new(
                LogicalPoint { x: 211, y: -20 },
                LogicalSize { w: 99, h: 100 }
            )
        );
    }
}
