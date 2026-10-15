//! One output as a row of desktops: where each Space sits in the strip and where each of its
//! floating and new windows goes.

use super::cascade::cascade_rect;
use super::floating::{
    filled_target, fit_size, floating_target, full_screen_target, placed, placed_target,
};
use crate::{
    Arrangement, LayoutKnobs, OutputSpec, SceneMode, SpaceArrangement, SpaceSpec, SpaceTarget,
    WindowPlacement,
};
use casement_core::geometry::{LogicalPoint, LogicalRect};
use porter_core::SpaceId;

/// Adds `output`'s Space origins and window targets to `into`.
pub(super) fn arrange_output(output: &OutputSpec, knobs: &LayoutKnobs, into: &mut Arrangement) {
    let shown = shown_index(output);
    for (index, space) in output.spaces.iter().enumerate() {
        into.spaces.push(SpaceTarget {
            space: space.space.clone(),
            output: output.output.clone(),
            origin: strip_origin(output.usable, index, shown),
        });
        arrange_space(output, space, knobs, into);
    }
}

/// The row position of the Space the output shows (the first when it names none).
fn shown_index(output: &OutputSpec) -> usize {
    let shown = match &output.mode {
        SceneMode::Desktop { shown } | SceneMode::Overview { shown, .. } => Some(shown),
        SceneMode::ShowDesktop => None,
    };
    shown
        .and_then(|shown| position_of(&output.spaces, shown))
        .unwrap_or(0)
}

fn position_of(spaces: &[SpaceSpec], id: &SpaceId) -> Option<usize> {
    spaces.iter().position(|space| &space.space == id)
}

/// Where Space `index` sits when Space `shown` is at the usable origin: one usable width apart.
fn strip_origin(usable: LogicalRect, index: usize, shown: usize) -> LogicalPoint {
    let apart = i64::try_from(index).unwrap_or(i64::MAX) - i64::try_from(shown).unwrap_or(0);
    let x =
        i64::from(usable.origin.x).saturating_add(apart.saturating_mul(i64::from(usable.size.w)));
    LogicalPoint {
        x: i32::try_from(x).unwrap_or(if x < 0 { i32::MIN } else { i32::MAX }),
        y: usable.origin.y,
    }
}

/// Adds the targets of `space`'s windows: in the output's usable coordinates, as if the Space
/// were shown (motion applies the strip origin).
fn arrange_space(
    output: &OutputSpec,
    space: &SpaceSpec,
    knobs: &LayoutKnobs,
    into: &mut Arrangement,
) {
    if matches!(space.kind, SpaceArrangement::Split(_)) {
        return;
    }
    let usable = output.usable;
    let mut new_so_far = 0;
    for window in &space.windows {
        let (target, report) = match window.placement {
            WindowPlacement::Floating(rect) => {
                floating_target(window.window, &output.output, rect, window.limits, usable)
            }
            WindowPlacement::Placed(rect) => {
                placed_target(window.window, &output.output, rect, window.limits, usable)
            }
            WindowPlacement::New(asked) => {
                let size = fit_size(asked, window.limits, usable);
                let rect = cascade_rect(usable, size, new_so_far, knobs.cascade_step);
                new_so_far += 1;
                placed(window.window, &output.output, asked, rect, window.limits)
            }
            WindowPlacement::Filled => {
                filled_target(window.window, &output.output, window.limits, usable)
            }
            WindowPlacement::FullScreen => (
                full_screen_target(window.window, &output.output, output.full),
                None,
            ),
            WindowPlacement::Minimised | WindowPlacement::Tiled(_) => continue,
        };
        into.windows.push(target);
        into.reports.extend(report);
    }
}
