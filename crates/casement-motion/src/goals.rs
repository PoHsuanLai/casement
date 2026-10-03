//! Where an arrangement sends each window and each Space strip: the targets motion aims at.

use casement_core::geometry::LogicalRect;
use casement_core::{OutputKey, WindowId};
use casement_layout::Arrangement;
use std::collections::BTreeMap;

/// Where one window ends up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Goal {
    /// The output it is drawn on.
    pub(crate) output: OutputKey,
    /// Its rectangle: its overview slot while an overview is open, else its target.
    pub(crate) rect: LogicalRect,
    /// Its place in the arrangement's order (bottom of the stack first).
    pub(crate) stack: u32,
}

/// Every goal of one arrangement.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Goals {
    /// Each window's goal.
    pub(crate) windows: BTreeMap<WindowId, Goal>,
    /// Each output's strip: the x origin of its first Space.
    pub(crate) strips: BTreeMap<OutputKey, f64>,
}

impl Goals {
    /// The goals of `arrangement`. A window with an overview slot goes to the slot; a window
    /// with a slot but no target has no output to be drawn on (the slot names none), so it has
    /// no goal yet.
    pub(crate) fn of(arrangement: &Arrangement) -> Goals {
        let mut windows: BTreeMap<WindowId, Goal> = arrangement
            .windows
            .iter()
            .zip(0u32..)
            .map(|(target, stack)| {
                let goal = Goal {
                    output: target.output.clone(),
                    rect: target.rect,
                    stack,
                };
                (target.window, goal)
            })
            .collect();
        for slot in &arrangement.overview {
            if let Some(goal) = windows.get_mut(&slot.window) {
                goal.rect = slot.rect;
            }
        }
        let mut strips = BTreeMap::new();
        for space in &arrangement.spaces {
            strips
                .entry(space.output.clone())
                .or_insert(f64::from(space.origin.x));
        }
        Goals { windows, strips }
    }
}
