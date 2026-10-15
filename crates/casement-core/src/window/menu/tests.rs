use super::super::state::Restore;
use super::*;
use crate::geometry::{LogicalPoint, LogicalRect, LogicalSize};
use porter_core::SpaceId;

fn rect() -> LogicalRect {
    LogicalRect::new(LogicalPoint { x: 1, y: 2 }, LogicalSize { w: 30, h: 40 })
}

fn offered(items: &[MenuItem]) -> Vec<MenuEntry> {
    items
        .iter()
        .filter(|item| item.offer == Offer::Enabled)
        .map(|item| item.entry)
        .collect()
}

fn dimmed(items: &[MenuItem]) -> Vec<MenuEntry> {
    items
        .iter()
        .filter(|item| item.offer == Offer::Disabled)
        .map(|item| item.entry)
        .collect()
}

#[test]
fn each_entry_means_one_action_and_one_label() {
    use MenuAction as A;
    use PlaceTarget as P;
    let table = [
        (
            MenuEntry::EnterFullScreen,
            "Enter Full Screen",
            A::FullScreen(Toggle::On),
        ),
        (
            MenuEntry::ExitFullScreen,
            "Exit Full Screen",
            A::FullScreen(Toggle::Off),
        ),
        (MenuEntry::Left, "Left", A::Place(P::Left)),
        (MenuEntry::Right, "Right", A::Place(P::Right)),
        (MenuEntry::Top, "Top", A::Place(P::Top)),
        (MenuEntry::Bottom, "Bottom", A::Place(P::Bottom)),
        (MenuEntry::TopLeft, "Top Left", A::Place(P::TopLeft)),
        (MenuEntry::TopRight, "Top Right", A::Place(P::TopRight)),
        (
            MenuEntry::BottomLeft,
            "Bottom Left",
            A::Place(P::BottomLeft),
        ),
        (
            MenuEntry::BottomRight,
            "Bottom Right",
            A::Place(P::BottomRight),
        ),
        (
            MenuEntry::ReturnToPreviousSize,
            "Return to Previous Size",
            A::Place(P::Restore),
        ),
        (
            MenuEntry::TileLeftOfScreen,
            "Left of Screen",
            A::SplitTile(SplitSide::Left),
        ),
        (
            MenuEntry::TileRightOfScreen,
            "Right of Screen",
            A::SplitTile(SplitSide::Right),
        ),
        (MenuEntry::Fill, "Fill", A::Place(P::Fill)),
        (MenuEntry::Centre, "Center", A::Place(P::Centre)),
    ];
    assert_eq!(table.len(), MenuEntry::ALL.len());
    for (entry, label, action) in table {
        assert_eq!(entry.label(), label, "{entry:?} label");
        assert_eq!(entry.action(), action, "{entry:?} action");
    }
}

#[test]
fn wire_codes_round_trip_and_stay_below_the_mask_width() {
    for entry in MenuEntry::ALL {
        assert_eq!(MenuEntry::from_code(entry.code()), Some(entry), "{entry:?}");
        assert!(entry.code() < 32, "{entry:?} fits the mask");
    }
    assert_eq!(MenuEntry::from_code(15), None);
}

#[test]
fn the_submenus_hold_the_entries_the_mac_groups() {
    let moves: Vec<MenuEntry> = MenuEntry::ALL
        .into_iter()
        .filter(|entry| entry.group() == MenuGroup::MoveAndResize)
        .collect();
    assert_eq!(moves.len(), 9);
    assert!(moves.contains(&MenuEntry::ReturnToPreviousSize));
    let tiles: Vec<MenuEntry> = MenuEntry::ALL
        .into_iter()
        .filter(|entry| entry.group() == MenuGroup::FullScreenTile)
        .collect();
    assert_eq!(
        tiles,
        vec![MenuEntry::TileLeftOfScreen, MenuEntry::TileRightOfScreen]
    );
}

#[test]
fn the_menu_is_enabled_per_window_state() {
    let space = SpaceId::parse("s-1").expect("id");
    let free_moves = [
        MenuEntry::Left,
        MenuEntry::Right,
        MenuEntry::Top,
        MenuEntry::Bottom,
        MenuEntry::TopLeft,
        MenuEntry::TopRight,
        MenuEntry::BottomLeft,
        MenuEntry::BottomRight,
        MenuEntry::Fill,
        MenuEntry::Centre,
    ];
    let tile_pair = [MenuEntry::TileLeftOfScreen, MenuEntry::TileRightOfScreen];
    let cases: Vec<(
        &str,
        WindowState,
        SplitSupport,
        Vec<MenuEntry>,
        Vec<MenuEntry>,
    )> = vec![
        (
            "floating, no Split View",
            WindowState::Floating(rect()),
            SplitSupport::Unavailable,
            [vec![MenuEntry::EnterFullScreen], free_moves.to_vec()].concat(),
            [vec![MenuEntry::ReturnToPreviousSize], tile_pair.to_vec()].concat(),
        ),
        (
            "unplaced, Split View",
            WindowState::New(LogicalSize { w: 3, h: 4 }),
            SplitSupport::Available,
            [
                vec![MenuEntry::EnterFullScreen],
                free_moves.to_vec(),
                tile_pair.to_vec(),
            ]
            .concat(),
            vec![MenuEntry::ReturnToPreviousSize],
        ),
        (
            "tiled left",
            WindowState::Tiled {
                zone: TileZone::Left,
                restore: rect(),
            },
            SplitSupport::Unavailable,
            [
                vec![MenuEntry::EnterFullScreen],
                free_moves
                    .iter()
                    .copied()
                    .filter(|e| *e != MenuEntry::Left)
                    .collect(),
                vec![MenuEntry::ReturnToPreviousSize],
            ]
            .concat(),
            [vec![MenuEntry::Left], tile_pair.to_vec()].concat(),
        ),
        (
            "zoomed",
            WindowState::Filled { restore: rect() },
            SplitSupport::Unavailable,
            [
                vec![MenuEntry::EnterFullScreen],
                free_moves
                    .iter()
                    .copied()
                    .filter(|e| *e != MenuEntry::Fill)
                    .collect(),
                vec![MenuEntry::ReturnToPreviousSize],
            ]
            .concat(),
            [vec![MenuEntry::Fill], tile_pair.to_vec()].concat(),
        ),
        (
            "full screen",
            WindowState::FullScreen {
                space,
                restore: Restore::Floating(rect()),
            },
            SplitSupport::Available,
            vec![MenuEntry::ExitFullScreen],
            MenuEntry::ALL
                .into_iter()
                .filter(|e| !matches!(e, MenuEntry::ExitFullScreen | MenuEntry::EnterFullScreen))
                .collect(),
        ),
        (
            "in the dock",
            WindowState::Minimised {
                restore: Restore::Floating(rect()),
            },
            SplitSupport::Available,
            vec![],
            MenuEntry::ALL
                .into_iter()
                .filter(|e| *e != MenuEntry::ExitFullScreen)
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
    let items = menu_for(&WindowState::Floating(rect()), SplitSupport::Unavailable);
    let mask = enabled_mask(&items);
    for entry in MenuEntry::ALL {
        let want = is_offered(&items, entry);
        assert_eq!(mask & (1 << entry.code()) != 0, want, "{entry:?}");
    }
}

#[test]
fn the_menu_opens_just_under_the_light() {
    let light = LogicalRect::new(LogicalPoint { x: 50, y: 10 }, LogicalSize { w: 12, h: 12 });
    assert_eq!(menu_point(light), LogicalPoint { x: 50, y: 26 });
}
