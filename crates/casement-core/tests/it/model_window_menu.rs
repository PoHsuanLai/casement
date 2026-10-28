//! The window menu through `Model::step`: a request tells the shell the entries and which are
//! enabled; a pick carries out the entry's model command, and only while it is enabled.

use crate::support::driver::{Op, last, map, plug, request, run, run_from};
use crate::support::proven;
use casement_core::focus::TokenCheck;
use casement_core::geometry::LogicalPoint;
use casement_core::shell::{ShellNote, ShellRequest, WindowMenuShow};
use casement_core::window::WindowState;
use casement_core::{Effect, Event, ToplevelRequest, WindowId};
use casement_protocols::vocab::{TileZone, WindowEntry};

fn pick(entry: WindowEntry) -> Op {
    Op::Event(Event::Shell(ShellRequest::WindowMenuPick {
        window: WindowId(1),
        entry,
    }))
}

fn mapped() -> casement_core::Model {
    let mail = proven("org.quire.Mail");
    run(vec![plug("HEADLESS-1"), map(1, &mail, TokenCheck::Valid)]).model
}

/// A row of the pick table: name, the entries picked in turn, and what the window's state must be.
type PickCase = (&'static str, Vec<WindowEntry>, fn(&WindowState) -> bool);

#[test]
fn a_menu_request_tells_the_shell_the_entries() {
    let at = LogicalPoint { x: 40, y: 50 };
    let run = run_from(
        mapped(),
        vec![request(1, ToplevelRequest::WindowMenu { at })],
    );
    let shown = last(&run).iter().find_map(|effect| match effect {
        Effect::Shell(ShellNote::WindowMenu(show)) => Some(show.clone()),
        _ => None,
    });
    let show: WindowMenuShow = shown.expect("the menu is shown");
    assert_eq!((show.window, show.at), (WindowId(1), at));
    assert_eq!(show.items[0].entry, WindowEntry::EnterFullScreen);
    let item = |entry| show.items.iter().find(|i| i.entry == entry).expect("entry");
    assert!(item(WindowEntry::Left).enabled);
    assert!(!item(WindowEntry::ReturnToPreviousSize).enabled);
}

#[test]
fn a_pick_carries_out_the_entries_command() {
    let cases: Vec<PickCase> = vec![
        ("left tiles to the left", vec![WindowEntry::Left], |s| {
            matches!(
                s,
                WindowState::Tiled {
                    zone: TileZone::Left,
                    ..
                }
            )
        }),
        (
            "bottom right tiles to that quarter",
            vec![WindowEntry::BottomRight],
            |s| {
                matches!(
                    s,
                    WindowState::Tiled {
                        zone: TileZone::BottomRight,
                        ..
                    }
                )
            },
        ),
        ("fill zooms", vec![WindowEntry::Fill], |s| {
            matches!(s, WindowState::Filled { .. })
        }),
        (
            "return to previous size undoes a tile",
            vec![WindowEntry::Top, WindowEntry::ReturnToPreviousSize],
            |s| matches!(s, WindowState::Floating(_)),
        ),
        ("centre floats the window", vec![WindowEntry::Centre], |s| {
            matches!(s, WindowState::Floating(_))
        }),
        (
            "enter full screen",
            vec![WindowEntry::EnterFullScreen],
            |s| matches!(s, WindowState::FullScreen { .. }),
        ),
        (
            "exit full screen",
            vec![WindowEntry::EnterFullScreen, WindowEntry::ExitFullScreen],
            |s| !matches!(s, WindowState::FullScreen { .. }),
        ),
        (
            "return to previous size on a floating window changes nothing",
            vec![WindowEntry::ReturnToPreviousSize],
            |s| matches!(s, WindowState::New(_)),
        ),
        (
            "a Full Screen Tile entry with no other window is just full screen",
            vec![WindowEntry::TileLeftOfScreen],
            |s| matches!(s, WindowState::FullScreen { .. }),
        ),
    ];
    for (name, entries, holds) in cases {
        let run = run_from(mapped(), entries.into_iter().map(pick).collect());
        let state = &run.model.windows()[&WindowId(1)].state;
        assert!(holds(state), "{name}: {state:?}");
    }
}

#[test]
fn a_dimmed_entry_or_a_gone_window_changes_nothing() {
    let filled = run_from(mapped(), vec![pick(WindowEntry::Fill)]);
    let again = run_from(
        mapped(),
        vec![pick(WindowEntry::Fill), pick(WindowEntry::Fill)],
    );
    assert_eq!(
        again.model.windows()[&WindowId(1)].state,
        filled.model.windows()[&WindowId(1)].state
    );
    let gone = run_from(
        mapped(),
        vec![Op::Event(Event::Shell(ShellRequest::WindowMenuPick {
            window: WindowId(9),
            entry: WindowEntry::Fill,
        }))],
    );
    assert!(last(&gone).is_empty());
}
