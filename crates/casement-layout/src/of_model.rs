//! The spec of a model: the one place layout reads casement-core's model.

use crate::{
    OutputSpec, SceneMode, SceneSpec, SpaceArrangement, SpaceSpec, StackIndex, WindowPlacement,
    WindowSpec,
};
use casement_core::geometry::{LogicalRect, LogicalSize, PhysicalSize, Transform};
use casement_core::output::OutputConfig;
use casement_core::space::{Space, SpaceContent};
use casement_core::window::{SpacePlace, Window, WindowState};
use casement_core::{Model, OutputKey, OutputState, WindowId};
use ds_core::geometry::scale::Scale;
use std::cmp::Reverse;

/// What `model` asks layout to arrange: every plugged output with its usable area (after
/// layer exclusive zones), scale, mode and Spaces, each Space with its windows in stacking
/// order. An output with no Space has nothing to arrange and is left out.
///
/// The model keeps no stacking list, so a Space's stack is its windows' recency: windows
/// never key sit at the bottom (by id), then the others from least to most recently key. A
/// window on every Space has no home output in the model; it is arranged on the first
/// output's shown Space.
pub fn spec_of(model: &Model) -> SceneSpec {
    let home = model.outputs().0.keys().next();
    SceneSpec {
        outputs: model
            .outputs()
            .0
            .iter()
            .filter_map(|(key, state)| output_spec(model, key, state, home))
            .collect(),
    }
}

/// `key`'s part of the scene; `home` is the output that takes the windows on every Space.
fn output_spec(
    model: &Model,
    key: &OutputKey,
    state: &OutputState,
    home: Option<&OutputKey>,
) -> Option<OutputSpec> {
    let row = model.spaces().row(key);
    let shown = model
        .spaces()
        .shown_on(key)
        .or_else(|| row.first().copied())?
        .id
        .clone();
    let takes_all_spaces = home == Some(key);
    let usable = model.layers().usable_area(key, logical_rect(&state.config));
    let spaces = row
        .into_iter()
        .map(|space| space_spec(model, space, takes_all_spaces && space.id == shown))
        .collect();
    Some(OutputSpec {
        output: key.clone(),
        usable,
        scale: state.config.scale,
        mode: SceneMode::Desktop { shown },
        spaces,
    })
}

/// One Space's part: its arrangement and its windows, bottom of the stack first. With
/// `all_spaces`, the windows that sit on every Space are among them.
fn space_spec(model: &Model, space: &Space, all_spaces: bool) -> SpaceSpec {
    let held = model
        .windows()
        .values()
        .filter(|window| match &window.place {
            SpacePlace::On(id) => *id == space.id,
            SpacePlace::AllSpaces => all_spaces,
        })
        .collect::<Vec<&Window>>();
    let ids = held.iter().map(|window| window.id).collect::<Vec<_>>();
    let windows = stacked(&ids, &model.focus().window_mru)
        .into_iter()
        .enumerate()
        .filter_map(|(index, id)| {
            let window = held.iter().find(|window| window.id == id)?;
            let stack = StackIndex(u32::try_from(index).unwrap_or(u32::MAX));
            Some(window_spec(window, stack))
        })
        .collect();
    SpaceSpec {
        space: space.id.clone(),
        kind: arrangement_of(&space.content),
        windows,
    }
}

fn window_spec(window: &Window, stack: StackIndex) -> WindowSpec {
    WindowSpec {
        window: window.id,
        app: window.app.clone(),
        placement: placement_of(&window.state),
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

/// Where a window in this state goes (a full-screen window fills its own Space).
fn placement_of(state: &WindowState) -> WindowPlacement {
    match state {
        WindowState::Floating(rect) => WindowPlacement::Floating(*rect),
        WindowState::Tiled { zone, .. } => WindowPlacement::Tiled(*zone),
        WindowState::Filled { .. } | WindowState::FullScreen { .. } => WindowPlacement::Filled,
        WindowState::Minimised { .. } => WindowPlacement::Minimised,
    }
}

/// `windows` bottom of the stack first: those never key (by id), then the rest from the least
/// to the most recently key (`mru` lists the most recent first).
fn stacked(windows: &[WindowId], mru: &[WindowId]) -> Vec<WindowId> {
    let mut order = windows.to_vec();
    order.sort_by_key(|id| {
        (
            Reverse(mru.iter().position(|m| m == id).unwrap_or(usize::MAX)),
            *id,
        )
    });
    order
}

/// The output's whole rectangle in the global logical space: its mode (turned for a sideways
/// transform) divided by its scale, at its position.
fn logical_rect(config: &OutputConfig) -> LogicalRect {
    let PhysicalSize { w, h } = turned(config.mode.size, config.transform);
    LogicalRect::new(
        config.position,
        LogicalSize {
            w: logical(w, config.scale),
            h: logical(h, config.scale),
        },
    )
}

/// `size` as scanout sees it after `transform`: the sides swap when it turns a quarter.
fn turned(size: PhysicalSize, transform: Transform) -> PhysicalSize {
    match transform {
        Transform::R90 | Transform::R270 | Transform::Flipped90 | Transform::Flipped270 => {
            PhysicalSize {
                w: size.h,
                h: size.w,
            }
        }
        Transform::Normal | Transform::R180 | Transform::Flipped | Transform::Flipped180 => size,
    }
}

/// `device` pixels as logical pixels at `scale`, to the nearest.
fn logical(device: u32, scale: Scale) -> u32 {
    let n = u64::from(scale.numerator()).max(1);
    let d = u64::from(Scale::DENOMINATOR);
    u32::try_from((u64::from(device) * d + n / 2) / n).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::geometry::LogicalPoint;
    use casement_core::output::{Mode, ModePreference, OutputPower};
    use casement_core::space::SplitPair;
    use casement_core::time::Millihertz;
    use casement_core::window::Restore;
    use casement_protocols::vocab::TileZone;
    use ds_core::vocab::Fraction;
    use porter_core::SpaceId;

    fn rect(x: i32, y: i32, w: u32, h: u32) -> LogicalRect {
        LogicalRect::new(LogicalPoint { x, y }, LogicalSize { w, h })
    }

    fn config(w: u32, h: u32, scale: u32, transform: Transform, at: (i32, i32)) -> OutputConfig {
        OutputConfig {
            mode: Mode {
                size: PhysicalSize { w, h },
                refresh: Millihertz(60_000),
                preference: ModePreference::Preferred,
            },
            scale: Scale(scale),
            transform,
            position: LogicalPoint { x: at.0, y: at.1 },
            power: OutputPower::On,
        }
    }

    #[test]
    fn an_output_in_logical_pixels() {
        let cases = [
            (
                "1x",
                config(1920, 1080, 120, Transform::Normal, (0, 0)),
                rect(0, 0, 1920, 1080),
            ),
            (
                "1.5x",
                config(3840, 2160, 180, Transform::Normal, (1920, 0)),
                rect(1920, 0, 2560, 1440),
            ),
            (
                "2x",
                config(3840, 2160, 240, Transform::Normal, (0, 0)),
                rect(0, 0, 1920, 1080),
            ),
            (
                "1.25x rounds",
                config(1920, 1080, 150, Transform::Normal, (0, 0)),
                rect(0, 0, 1536, 864),
            ),
            (
                "a quarter turn swaps the sides",
                config(1920, 1080, 120, Transform::R90, (0, 0)),
                rect(0, 0, 1080, 1920),
            ),
            (
                "a half turn does not",
                config(1920, 1080, 120, Transform::R180, (-5, 7)),
                rect(-5, 7, 1920, 1080),
            ),
            (
                "flipped and turned swaps",
                config(1920, 1080, 120, Transform::Flipped270, (0, 0)),
                rect(0, 0, 1080, 1920),
            ),
        ];
        for (name, config, want) in cases {
            assert_eq!(logical_rect(&config), want, "{name}");
        }
    }

    #[test]
    fn states_become_placements() {
        let floor = rect(1, 2, 3, 4);
        let space = SpaceId::parse("s-1").expect("id");
        let cases = [
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
                "full screen fills its Space",
                WindowState::FullScreen {
                    space,
                    restore: Restore::Floating(floor),
                },
                WindowPlacement::Filled,
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

    #[test]
    fn the_stack_is_recency_with_never_key_windows_at_the_bottom() {
        let ids = |ns: &[u64]| ns.iter().map(|n| WindowId(*n)).collect::<Vec<_>>();
        let cases = [
            (
                "no recency: by id",
                ids(&[3, 1, 2]),
                ids(&[]),
                ids(&[1, 2, 3]),
            ),
            (
                "most recent is on top",
                ids(&[1, 2, 3]),
                ids(&[2, 3, 1]),
                ids(&[1, 3, 2]),
            ),
            (
                "never key under every key one",
                ids(&[1, 2, 3]),
                ids(&[3]),
                ids(&[1, 2, 3]),
            ),
            (
                "recency of other Spaces' windows is ignored",
                ids(&[4, 5]),
                ids(&[9, 5, 8, 4]),
                ids(&[4, 5]),
            ),
        ];
        for (name, windows, mru, want) in cases {
            assert_eq!(stacked(&windows, &mru), want, "{name}");
        }
    }
}
