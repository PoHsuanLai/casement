use super::super::state::Restore;
use super::*;
use crate::geometry::{LogicalPoint, LogicalRect, LogicalSize};
use porter_core::SpaceId;

fn rect() -> LogicalRect {
    LogicalRect::new(LogicalPoint { x: 1, y: 2 }, LogicalSize { w: 30, h: 40 })
}

fn offered(items: &[MenuItem]) -> Vec<WindowEntry> {
    items
        .iter()
        .filter(|item| item.enabled)
        .map(|item| item.entry)
        .collect()
}

fn dimmed(items: &[MenuItem]) -> Vec<WindowEntry> {
    items
        .iter()
        .filter(|item| !item.enabled)
        .map(|item| item.entry)
        .collect()
}

/// A row of the enabled-entries table: name, window state, Split View support, the entries
/// enabled and the entries dimmed.
type EnabledCase = (
    &'static str,
    WindowState,
    bool,
    Vec<WindowEntry>,
    Vec<WindowEntry>,
);

#[test]
fn each_entry_means_one_action_and_one_label() {
    use MenuAction as A;
    use PlaceTarget as P;
    let table = [
        (
            WindowEntry::EnterFullScreen,
            "Enter Full Screen",
            A::FullScreen(Toggle::On),
        ),
        (
            WindowEntry::ExitFullScreen,
            "Exit Full Screen",
            A::FullScreen(Toggle::Off),
        ),
        (WindowEntry::Left, "Left", A::Place(P::Left)),
        (WindowEntry::Right, "Right", A::Place(P::Right)),
        (WindowEntry::Top, "Top", A::Place(P::Top)),
        (WindowEntry::Bottom, "Bottom", A::Place(P::Bottom)),
        (WindowEntry::TopLeft, "Top Left", A::Place(P::TopLeft)),
        (WindowEntry::TopRight, "Top Right", A::Place(P::TopRight)),
        (
            WindowEntry::BottomLeft,
            "Bottom Left",
            A::Place(P::BottomLeft),
        ),
        (
            WindowEntry::BottomRight,
            "Bottom Right",
            A::Place(P::BottomRight),
        ),
        (
            WindowEntry::ReturnToPreviousSize,
            "Return to Previous Size",
            A::Place(P::Restore),
        ),
        (
            WindowEntry::TileLeftOfScreen,
            "Left of Screen",
            A::SplitTile(SplitSide::Left),
        ),
        (
            WindowEntry::TileRightOfScreen,
            "Right of Screen",
            A::SplitTile(SplitSide::Right),
        ),
        (WindowEntry::Fill, "Fill", A::Place(P::Fill)),
        (WindowEntry::Centre, "Center", A::Place(P::Centre)),
    ];
    assert_eq!(table.len(), WindowEntry::ALL.len());
    for (entry, label, action) in table {
        assert_eq!(entry_label(entry), label, "{entry:?} label");
        assert_eq!(entry_action(entry), action, "{entry:?} action");
    }
}

#[test]
fn wire_codes_stay_below_the_mask_width() {
    for entry in WindowEntry::ALL {
        assert!(entry.code() < 32, "{entry:?} fits the mask");
    }
}

#[test]
fn the_submenus_hold_the_entries_the_mac_groups() {
    let moves: Vec<WindowEntry> = WindowEntry::ALL
        .iter()
        .copied()
        .filter(|entry| entry_group(*entry) == MenuGroup::MoveAndResize)
        .collect();
    assert_eq!(moves.len(), 9);
    assert!(moves.contains(&WindowEntry::ReturnToPreviousSize));
    let tiles: Vec<WindowEntry> = WindowEntry::ALL
        .iter()
        .copied()
        .filter(|entry| entry_group(*entry) == MenuGroup::FullScreenTile)
        .collect();
    assert_eq!(
        tiles,
        vec![
            WindowEntry::TileLeftOfScreen,
            WindowEntry::TileRightOfScreen
        ]
    );
}

#[test]
fn the_menu_is_enabled_per_window_state() {
    let space = SpaceId::parse("s-1").expect("id");
    let free_moves = [
        WindowEntry::Left,
        WindowEntry::Right,
        WindowEntry::Top,
        WindowEntry::Bottom,
        WindowEntry::TopLeft,
        WindowEntry::TopRight,
        WindowEntry::BottomLeft,
        WindowEntry::BottomRight,
        WindowEntry::Fill,
        WindowEntry::Centre,
    ];
    let tile_pair = [
        WindowEntry::TileLeftOfScreen,
        WindowEntry::TileRightOfScreen,
    ];
    let cases: Vec<EnabledCase> = vec![
        (
            "floating, no Split View",
            WindowState::Floating(rect()),
            false,
            [vec![WindowEntry::EnterFullScreen], free_moves.to_vec()].concat(),
            [vec![WindowEntry::ReturnToPreviousSize], tile_pair.to_vec()].concat(),
        ),
        (
            "unplaced, Split View",
            WindowState::New(LogicalSize { w: 3, h: 4 }),
            true,
            [
                vec![WindowEntry::EnterFullScreen],
                free_moves.to_vec(),
                tile_pair.to_vec(),
            ]
            .concat(),
            vec![WindowEntry::ReturnToPreviousSize],
        ),
        (
            "tiled left",
            WindowState::Tiled {
                zone: TileZone::Left,
                restore: rect(),
            },
            false,
            [
                vec![WindowEntry::EnterFullScreen],
                free_moves
                    .iter()
                    .copied()
                    .filter(|e| *e != WindowEntry::Left)
                    .collect(),
                vec![WindowEntry::ReturnToPreviousSize],
            ]
            .concat(),
            [vec![WindowEntry::Left], tile_pair.to_vec()].concat(),
        ),
        (
            "zoomed",
            WindowState::Filled { restore: rect() },
            false,
            [
                vec![WindowEntry::EnterFullScreen],
                free_moves
                    .iter()
                    .copied()
                    .filter(|e| *e != WindowEntry::Fill)
                    .collect(),
                vec![WindowEntry::ReturnToPreviousSize],
            ]
            .concat(),
            [vec![WindowEntry::Fill], tile_pair.to_vec()].concat(),
        ),
        (
            "full screen",
            WindowState::FullScreen {
                space,
                restore: Restore::Floating(rect()),
            },
            true,
            vec![WindowEntry::ExitFullScreen],
            WindowEntry::ALL
                .iter()
                .copied()
                .filter(|e| {
                    !matches!(
                        e,
                        WindowEntry::ExitFullScreen | WindowEntry::EnterFullScreen
                    )
                })
                .collect(),
        ),
        (
            "in the dock",
            WindowState::Minimised {
                restore: Restore::Floating(rect()),
            },
            true,
            vec![],
            WindowEntry::ALL
                .iter()
                .copied()
                .filter(|e| *e != WindowEntry::ExitFullScreen)
                .collect(),
        ),
    ];
    for (name, state, split, on, off) in cases {
        let items = menu_for(&state, split);
        let (mut got_on, mut got_off) = (offered(&items), dimmed(&items));
        let (mut want_on, mut want_off) = (on, off);
        got_on.sort();
        want_on.sort();
        got_off.sort();
        want_off.sort();
        assert_eq!(got_on, want_on, "{name}: offered");
        assert_eq!(got_off, want_off, "{name}: dimmed");
    }
}

#[test]
fn the_mask_has_a_bit_per_offered_entry() {
    let items = menu_for(&WindowState::Floating(rect()), false);
    let mask = enabled_mask(&items);
    for entry in WindowEntry::ALL.iter().copied() {
        let want = is_offered(&items, entry);
        assert_eq!(mask & (1 << entry.code()) != 0, want, "{entry:?}");
    }
}

#[test]
fn the_menu_opens_just_under_the_light() {
    let light = LogicalRect::new(LogicalPoint { x: 50, y: 10 }, LogicalSize { w: 12, h: 12 });
    assert_eq!(menu_point(light), LogicalPoint { x: 50, y: 26 });
}
