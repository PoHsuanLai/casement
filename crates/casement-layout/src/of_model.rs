//! The spec of a model: the one place layout reads casement-core's model.

use crate::{
    OutputSpec, SceneMode, SceneSpec, SpaceArrangement, SpaceSpec, StackIndex, WindowPlacement,
    WindowSpec,
};
use casement_core::app::AppVisibility;
use casement_core::space::{Space, SpaceContent};
use casement_core::window::{SpacePlace, Window, WindowState};
use casement_core::{Model, OutputKey, OutputState};

/// What `model` asks layout to arrange: every plugged output with its usable area (after
/// layer exclusive zones, A26), scale, mode and Spaces, each Space with its windows in
/// stacking order (the model's stack ranks, A28). A window on every Space is arranged on the
/// shown Space of its home output. An output with no Space has nothing to arrange and is left
/// out.
pub fn spec_of(model: &Model) -> SceneSpec {
    SceneSpec {
        outputs: model
            .outputs()
            .0
            .iter()
            .filter_map(|(key, state)| output_spec(model, key, state))
            .collect(),
    }
}

/// `key`'s part of the scene.
fn output_spec(model: &Model, key: &OutputKey, state: &OutputState) -> Option<OutputSpec> {
    let row = model.spaces().row(key);
    let shown = model
        .spaces()
        .shown_on(key)
        .or_else(|| row.first().copied())?
        .id
        .clone();
    let full = state.config.logical_rect();
    let usable = model.layers().usable_area(key, full);
    let spaces = row
        .into_iter()
        .map(|space| {
            let visibility = match space.id == shown {
                true => Visibility::Shown,
                false => Visibility::Hidden,
            };
            space_spec(model, space, key, visibility)
        })
        .collect();
    Some(OutputSpec {
        output: key.clone(),
        full,
        usable,
        scale: state.config.scale,
        mode: SceneMode::Desktop { shown },
        spaces,
    })
}

/// Whether a Space is the one its output shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Visibility {
    /// The output shows it.
    Shown,
    /// Another Space is shown.
    Hidden,
}

/// One Space's part: its arrangement and its windows, bottom of the stack first. When
/// `Shown`, the windows that sit on every Space of `output` are among them.
fn space_spec(model: &Model, space: &Space, output: &OutputKey, shown: Visibility) -> SpaceSpec {
    let mut held = model
        .windows()
        .values()
        .filter(|window| match &window.place {
            SpacePlace::On(id) => *id == space.id,
            SpacePlace::AllSpaces => shown == Visibility::Shown && window.home.as_ref() == Some(output),
        })
        .collect::<Vec<&Window>>();
    held.sort_by_key(|window| (window.stack, window.id));
    let windows = held
        .into_iter()
        .enumerate()
        .map(|(index, window)| {
            let stack = StackIndex(u32::try_from(index).unwrap_or(u32::MAX));
            window_spec(model, window, stack)
        })
        .collect();
    SpaceSpec {
        space: space.id.clone(),
        kind: arrangement_of(&space.content),
        windows,
    }
}

/// One window's part. A window of a hidden app is out of sight like one in the dock.
fn window_spec(model: &Model, window: &Window, stack: StackIndex) -> WindowSpec {
    let hidden = model
        .apps()
        .get(&window.app)
        .is_some_and(|app| matches!(app.visibility, AppVisibility::Hidden { .. }));
    WindowSpec {
        window: window.id,
        app: window.app.clone(),
        placement: match hidden {
            true => WindowPlacement::Minimised,
            false => placement_of(&window.state),
        },
        limits: window.limits,
        stack,
    }
}

/// How a Space of this content is arranged (an agent's Space is a desktop).
fn arrangement_of(content: &SpaceContent) -> SpaceArrangement {
    match content {
        SpaceContent::Desktop | SpaceContent::Agent(_) => SpaceArrangement::Desktop,
        SpaceContent::FullScreen(window) => SpaceArrangement::FullScreen(*window),
        SpaceContent::SplitView(pair) => SpaceArrangement::Split(*pair),
    }
}

/// Where a window in this state goes (a full-screen window covers its output).
fn placement_of(state: &WindowState) -> WindowPlacement {
    match state {
        WindowState::New(size) => WindowPlacement::New(*size),
        WindowState::Floating(rect) => WindowPlacement::Floating(*rect),
        WindowState::Tiled { zone, .. } => WindowPlacement::Tiled(*zone),
        WindowState::Filled { .. } => WindowPlacement::Filled,
        WindowState::FullScreen { .. } => WindowPlacement::FullScreen,
        WindowState::Minimised { .. } => WindowPlacement::Minimised,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::WindowId;
    use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};
    use casement_core::space::SplitPair;
    use casement_core::window::Restore;
    use casement_protocols::vocab::TileZone;
    use ds_core::vocab::Fraction;
    use porter_core::SpaceId;

    fn rect(x: i32, y: i32, w: u32, h: u32) -> LogicalRect {
        LogicalRect::new(LogicalPoint { x, y }, LogicalSize { w, h })
    }

    #[test]
    fn states_become_placements() {
        let floor = rect(1, 2, 3, 4);
        let space = SpaceId::parse("s-1").expect("id");
        let cases = [
            (
                "unplaced",
                WindowState::New(LogicalSize { w: 4, h: 5 }),
                WindowPlacement::New(LogicalSize { w: 4, h: 5 }),
            ),
            (
                "floating",
                WindowState::Floating(floor),
                WindowPlacement::Floating(floor),
            ),
            (
                "tiled",
                WindowState::Tiled {
                    zone: TileZone::Left,
                    restore: floor,
                },
                WindowPlacement::Tiled(TileZone::Left),
            ),
            (
                "filled",
                WindowState::Filled { restore: floor },
                WindowPlacement::Filled,
            ),
            (
                "full screen covers the output",
                WindowState::FullScreen {
                    space,
                    restore: Restore::Floating(floor),
                },
                WindowPlacement::FullScreen,
            ),
            (
                "minimised",
                WindowState::Minimised {
                    restore: Restore::Floating(floor),
                },
                WindowPlacement::Minimised,
            ),
        ];
        for (name, state, want) in cases {
            assert_eq!(placement_of(&state), want, "{name}");
        }
    }

    #[test]
    fn spaces_become_arrangements() {
        let pair = SplitPair {
            leading: WindowId(1),
            trailing: WindowId(2),
            divider: Fraction(500),
        };
        use casement_core::space::TaskToken;
        let cases = [
            ("desktop", SpaceContent::Desktop, SpaceArrangement::Desktop),
            (
                "agent",
                SpaceContent::Agent(TaskToken("t".to_owned())),
                SpaceArrangement::Desktop,
            ),
            (
                "full screen",
                SpaceContent::FullScreen(WindowId(9)),
                SpaceArrangement::FullScreen(WindowId(9)),
            ),
            (
                "split view",
                SpaceContent::SplitView(pair),
                SpaceArrangement::Split(pair),
            ),
        ];
        for (name, content, want) in cases {
            assert_eq!(arrangement_of(&content), want, "{name}");
        }
    }
}
