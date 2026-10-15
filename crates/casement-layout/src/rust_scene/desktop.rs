//! One output as a row of desktops: where each Space sits in the strip and where each of its
//! floating and new windows goes.

use super::cascade::cascade_rect;
use super::floating::{
    filled_target, fit_size, floating_target, full_screen_target, placed, placed_target,
};
use super::split;
use super::tile_zone::tiled_target;
use crate::{
    Arrangement, LayoutKnobs, OutputSpec, SceneMode, SpaceArrangement, SpaceSpec, SpaceTarget,
    WindowPlacement,
};
use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};
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

/// `usable` with its top `reserve` rows taken away: where a window's content may sit so that
/// its frame, which reaches `reserve` above the content, stays inside `usable`. A reserve
/// larger than the area leaves a zero-height area at its bottom.
pub(super) fn below_chrome(usable: LogicalRect, reserve: u32) -> LogicalRect {
    let taken = reserve.min(usable.size.h);
    let y = i64::from(usable.origin.y) + i64::from(taken);
    LogicalRect::new(
        LogicalPoint {
            x: usable.origin.x,
            y: i32::try_from(y).unwrap_or(i32::MAX),
        },
        LogicalSize {
            w: usable.size.w,
            h: usable.size.h - taken,
        },
    )
}

/// Adds the targets of `space`'s windows: in the output's usable coordinates, as if the Space
/// were shown (motion applies the strip origin).
fn arrange_space(
    output: &OutputSpec,
    space: &SpaceSpec,
    knobs: &LayoutKnobs,
    into: &mut Arrangement,
) {
    if let SpaceArrangement::Split(pair) = space.kind {
        split::arrange_split(output, space, pair, into);
        return;
    }
    let mut new_so_far = 0;
    for window in &space.windows {
        let usable = below_chrome(output.usable, window.reserve_top);
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
            WindowPlacement::Tiled(zone) => tiled_target(
                window.window,
                &output.output,
                zone,
                window.limits,
                window.reserve_top,
                output.usable,
                knobs.tile_margin,
            ),
            WindowPlacement::Minimised => continue,
        };
        into.windows.push(target);
        into.reports.extend(report);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: i32, y: i32, w: u32, h: u32) -> LogicalRect {
        LogicalRect::new(LogicalPoint { x, y }, LogicalSize { w, h })
    }

    #[test]
    fn the_reserve_comes_off_the_top() {
        let usable = rect(0, 24, 1920, 1056);
        let cases = [
            ("no reserve", 0, rect(0, 24, 1920, 1056)),
            ("a titlebar", 28, rect(0, 52, 1920, 1028)),
            ("all of it", 1056, rect(0, 1080, 1920, 0)),
            ("more than all of it", 5000, rect(0, 1080, 1920, 0)),
        ];
        for (name, reserve, want) in cases {
            assert_eq!(below_chrome(usable, reserve), want, "{name}");
        }
    }
}
