//! An output in Mission Control: the shown Space's windows spread over the usable area
//! (`spread.rs`). Their desktop targets stay as they are, so the windows fly back to them when
//! the overview closes.

use super::spread::{Entry, Fit, spread};
use crate::{
    Arrangement, LayoutKnobs, LayoutReport, OutputSpec, OverviewKind, SceneMode, SpaceArrangement,
};

/// Adds the overview slots of `output` to `into`, after its desktop targets are there. An
/// output that is not in Mission Control, or whose shown Space is not a desktop, adds none.
pub(super) fn arrange_overview(output: &OutputSpec, knobs: &LayoutKnobs, into: &mut Arrangement) {
    let SceneMode::Overview {
        shown,
        kind: OverviewKind::MissionControl,
    } = &output.mode
    else {
        return;
    };
    let Some(space) = output.spaces.iter().find(|space| &space.space == shown) else {
        return;
    };
    if !matches!(space.kind, SpaceArrangement::Desktop) {
        return;
    }
    let entries: Vec<Entry> = space
        .windows
        .iter()
        .filter_map(|window| {
            let target = into.windows.iter().find(|t| t.window == window.window)?;
            Some(Entry {
                window: window.window,
                rect: target.rect,
                reserve: window.reserve_top,
            })
        })
        .collect();
    let (slots, fit) = spread(&entries, output.usable, knobs);
    into.overview.extend(slots);
    if fit == Fit::Overflow {
        into.reports
            .push(LayoutReport::SpreadOverflow(output.output.clone()));
    }
}
