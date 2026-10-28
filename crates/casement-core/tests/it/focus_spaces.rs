//! What 03 §1.2's tables leave to the machine, part two: activation, hiding, who may move focus,
//! Spaces and sheets, and the press routes the table does not list.

use crate::focus::fixture;
use crate::focus::rows;
use crate::support;

use casement_core::WindowId;
use casement_core::focus::{
    Focus, FocusIn, FocusOut, FocusView, HitPart, Press, PressRoute, route_press,
};
use casement_core::geometry::LogicalSize;
use casement_core::input::{DeviceClass, Origin};
use casement_core::window::{Restore, Window, WindowState};
use casement_protocols::vocab::{ActorKind, Emphasis, HeldMods};
use fixture::{a, apps, b, focus, ids, sheet, start, windows};
use rows::{Row, attention, c, nothing, on, over_sheet, row, run, three_spaces};
use std::collections::BTreeMap;
use support::{headless, space_id};

#[test]
fn activation_hiding_and_who_may_move_focus() {
    let b_idle = focus(Some(b()), None, None, vec![b(), a()], &[1, 2, 3]);
    let rows = vec![
        row(
            "activating the active app again raises its windows and keeps the key",
            start(),
            FocusIn::ActivateApp {
                app: a(),
                by: ActorKind::User,
            },
            windows(&[], vec![]),
            start(),
            vec![FocusOut::Raise(ids(&[1, 2]))],
        ),
        row(
            "an app with windows only on another Space becomes active and windowless (S5)",
            start(),
            FocusIn::ActivateApp {
                app: b(),
                by: ActorKind::User,
            },
            windows(&[], vec![on(3, &b(), "s-2")]),
            b_idle.clone(),
            vec![
                FocusOut::Keyboard(None),
                FocusOut::Emphasis(vec![(WindowId(1), Emphasis::Inactive)]),
                FocusOut::ActiveApp(Some(b())),
            ],
        ),
        row(
            "a command-line tool may activate an app",
            start(),
            FocusIn::ActivateApp {
                app: b(),
                by: ActorKind::Cli,
            },
            windows(&[], vec![]),
            focus(Some(b()), Some(3), Some(3), vec![b(), a()], &[3, 1, 2]),
            vec![
                FocusOut::Raise(ids(&[3])),
                FocusOut::Keyboard(Some(WindowId(3))),
                FocusOut::Emphasis(vec![
                    (WindowId(1), Emphasis::Inactive),
                    (WindowId(3), Emphasis::Key),
                ]),
                FocusOut::ActiveApp(Some(b())),
            ],
        ),
        row(
            "an app cannot activate itself (A7)",
            start(),
            FocusIn::ActivateApp {
                app: b(),
                by: ActorKind::App,
            },
            windows(&[], vec![]),
            start(),
            attention(b()),
        ),
        row(
            "a third party cannot activate an app",
            start(),
            FocusIn::ActivateApp {
                app: b(),
                by: ActorKind::ThirdParty,
            },
            windows(&[], vec![]),
            start(),
            attention(b()),
        ),
        row(
            "hiding another app moves nothing",
            start(),
            FocusIn::HideApp(b()),
            windows(&[], vec![]),
            start(),
            nothing(),
        ),
        row(
            "hiding the active app skips apps with no visible window",
            focus(Some(a()), Some(1), Some(1), vec![a(), c(), b()], &[1, 2, 3]),
            FocusIn::HideApp(a()),
            windows(&[], vec![]),
            focus(Some(b()), Some(3), Some(3), vec![b(), a(), c()], &[3, 1, 2]),
            vec![
                FocusOut::Keyboard(Some(WindowId(3))),
                FocusOut::Emphasis(vec![
                    (WindowId(1), Emphasis::Inactive),
                    (WindowId(3), Emphasis::Key),
                ]),
                FocusOut::ActiveApp(Some(b())),
            ],
        ),
        Row {
            hidden: vec![b()],
            ..row(
                "hiding the active app when every other app is hidden leaves nothing active",
                start(),
                FocusIn::HideApp(a()),
                windows(&[], vec![]),
                Focus {
                    active: None,
                    key: None,
                    main: None,
                    ..start()
                },
                vec![
                    FocusOut::Keyboard(None),
                    FocusOut::Emphasis(vec![(WindowId(1), Emphasis::Inactive)]),
                    FocusOut::ActiveApp(None),
                ],
            )
        },
        row(
            "unhiding an app moves nothing",
            start(),
            FocusIn::UnhideApp(b()),
            windows(&[], vec![]),
            start(),
            nothing(),
        ),
    ];
    rows.into_iter().for_each(run);
}

/// A run's lease keys exactly the window it holds, as its own actor: the same move an
/// activation by the person makes, where an app's own ask only asks for attention (A7, above).
#[test]
fn a_lease_keys_the_window_it_holds() {
    let b_keyed = focus(Some(b()), Some(3), Some(3), vec![b(), a()], &[3, 1, 2]);
    let lease_on = |n: u64| FocusIn::LeaseFocus {
        window: WindowId(n),
    };
    let rows = vec![
        row(
            "a lease on another app's window raises and keys it, and its app is active",
            start(),
            lease_on(3),
            windows(&[], vec![]),
            b_keyed.clone(),
            vec![
                FocusOut::Raise(ids(&[3])),
                FocusOut::Keyboard(Some(WindowId(3))),
                FocusOut::Emphasis(vec![
                    (WindowId(1), Emphasis::Inactive),
                    (WindowId(3), Emphasis::Key),
                ]),
                FocusOut::ActiveApp(Some(b())),
            ],
        ),
        row(
            "a lease on the key window only raises it",
            start(),
            lease_on(1),
            windows(&[], vec![]),
            start(),
            vec![FocusOut::Raise(ids(&[1]))],
        ),
        row(
            "a lease on a window of a Space no output shows moves nothing",
            start(),
            lease_on(3),
            windows(&[], vec![on(3, &b(), "s-2")]),
            start(),
            nothing(),
        ),
        Row {
            hidden: vec![b()],
            ..row(
                "a lease on a window of a hidden app moves nothing (the model unhides first)",
                start(),
                lease_on(3),
                windows(&[], vec![]),
                start(),
                nothing(),
            )
        },
        row(
            "a lease on a window that is gone moves nothing",
            start(),
            lease_on(9),
            windows(&[], vec![]),
            start(),
            nothing(),
        ),
    ];
    rows.into_iter().for_each(run);
}

#[test]
fn spaces_and_sheets() {
    let on_s2 = |extra: Vec<Window>| windows(&[], extra);
    let rows = vec![
        Row {
            shown: "s-2",
            ..row(
                "a Space change keys the active app's window on the new Space",
                start(),
                FocusIn::SpaceShown {
                    output: headless("HEADLESS-1").key,
                    space: space_id("s-2"),
                },
                on_s2(vec![on(9, &a(), "s-2"), on(3, &b(), "s-2")]),
                focus(Some(a()), Some(9), Some(9), vec![a(), b()], &[9, 1, 2, 3]),
                vec![
                    FocusOut::Keyboard(Some(WindowId(9))),
                    FocusOut::Emphasis(vec![
                        (WindowId(1), Emphasis::Inactive),
                        (WindowId(9), Emphasis::Key),
                    ]),
                ],
            )
        },
        Row {
            shown: "s-2",
            ..row(
                "a Space change to another app's window moves the active app there",
                start(),
                FocusIn::SpaceShown {
                    output: headless("HEADLESS-1").key,
                    space: space_id("s-2"),
                },
                on_s2(vec![on(3, &b(), "s-2")]),
                focus(Some(b()), Some(3), Some(3), vec![b(), a()], &[3, 1, 2]),
                vec![
                    FocusOut::Keyboard(Some(WindowId(3))),
                    FocusOut::Emphasis(vec![
                        (WindowId(1), Emphasis::Inactive),
                        (WindowId(3), Emphasis::Key),
                    ]),
                    FocusOut::ActiveApp(Some(b())),
                ],
            )
        },
        Row {
            shown: "s-3",
            ..row(
                "a Space change to an empty Space leaves the app active with no key",
                start(),
                FocusIn::SpaceShown {
                    output: headless("HEADLESS-1").key,
                    space: space_id("s-3"),
                },
                windows(&[], vec![]),
                focus(Some(a()), None, None, vec![a(), b()], &[1, 2, 3]),
                vec![
                    FocusOut::Keyboard(None),
                    FocusOut::Emphasis(vec![(WindowId(1), Emphasis::Inactive)]),
                ],
            )
        },
        Row {
            shown: "s-2",
            ..row(
                "a Space change keys the activated app's window (S5 after the switch)",
                b_idle_focus(),
                FocusIn::SpaceShown {
                    output: headless("HEADLESS-1").key,
                    space: space_id("s-2"),
                },
                on_s2(vec![on(3, &b(), "s-2")]),
                focus(Some(b()), Some(3), Some(3), vec![b(), a()], &[3, 1, 2]),
                vec![
                    FocusOut::Keyboard(Some(WindowId(3))),
                    FocusOut::Emphasis(vec![(WindowId(3), Emphasis::Key)]),
                ],
            )
        },
        row(
            "a Space change that leaves the key window visible moves nothing",
            start(),
            FocusIn::SpaceShown {
                output: headless("HEADLESS-1").key,
                space: space_id("s-1"),
            },
            windows(&[], vec![]),
            start(),
            nothing(),
        ),
        row(
            "a sheet opens on the key window and takes the key",
            start(),
            FocusIn::SheetOpened {
                parent: WindowId(1),
                sheet: WindowId(6),
            },
            windows(&[], vec![sheet(6, 1)]),
            over_sheet(),
            vec![
                FocusOut::Keyboard(Some(WindowId(6))),
                FocusOut::Emphasis(vec![
                    (WindowId(1), Emphasis::Main),
                    (WindowId(6), Emphasis::Key),
                ]),
            ],
        ),
        row(
            "a sheet opening on a window that is not key waits for a press",
            start(),
            FocusIn::SheetOpened {
                parent: WindowId(2),
                sheet: WindowId(6),
            },
            windows(&[], vec![sheet(6, 2)]),
            start(),
            nothing(),
        ),
        row(
            "a sheet closed before it unmaps hands the key back",
            over_sheet(),
            FocusIn::SheetClosed {
                parent: WindowId(1),
            },
            windows(&[], vec![sheet(6, 1)]),
            focus(Some(a()), Some(1), Some(1), vec![a(), b()], &[1, 6, 2, 3]),
            vec![
                FocusOut::Keyboard(Some(WindowId(1))),
                FocusOut::Emphasis(vec![
                    (WindowId(1), Emphasis::Key),
                    (WindowId(6), Emphasis::Inactive),
                ]),
            ],
        ),
        row(
            "a sheet closed over a visible parent with a sibling hands the key to the parent",
            over_sheet(),
            FocusIn::SheetClosed {
                parent: WindowId(1),
            },
            windows(&[], vec![sheet(6, 1)]),
            focus(Some(a()), Some(1), Some(1), vec![a(), b()], &[1, 6, 2, 3]),
            vec![
                FocusOut::Keyboard(Some(WindowId(1))),
                FocusOut::Emphasis(vec![
                    (WindowId(1), Emphasis::Key),
                    (WindowId(6), Emphasis::Inactive),
                ]),
            ],
        ),
        row(
            "a sheet closed over a minimised parent re-keys to the app's next visible window",
            over_sheet(),
            FocusIn::SheetClosed {
                parent: WindowId(1),
            },
            windows_with(1, minimised, vec![sheet(6, 1)]),
            focus(Some(a()), Some(2), Some(2), vec![a(), b()], &[2, 6, 1, 3]),
            vec![
                FocusOut::Keyboard(Some(WindowId(2))),
                FocusOut::Emphasis(vec![
                    (WindowId(1), Emphasis::Inactive),
                    (WindowId(2), Emphasis::Key),
                    (WindowId(6), Emphasis::Inactive),
                ]),
            ],
        ),
        row(
            "a sheet closed over a parent on a hidden Space re-keys to the app's next visible window",
            over_sheet(),
            FocusIn::SheetClosed {
                parent: WindowId(1),
            },
            windows_with(
                1,
                |w| Window {
                    place: on(1, &a(), "s-2").place,
                    ..w
                },
                vec![sheet(6, 1)],
            ),
            focus(Some(a()), Some(2), Some(2), vec![a(), b()], &[2, 6, 1, 3]),
            vec![
                FocusOut::Keyboard(Some(WindowId(2))),
                FocusOut::Emphasis(vec![
                    (WindowId(1), Emphasis::Inactive),
                    (WindowId(2), Emphasis::Key),
                    (WindowId(6), Emphasis::Inactive),
                ]),
            ],
        ),
        row(
            "a sheet closed over a minimised parent with no other window leaves A windowless",
            over_sheet(),
            FocusIn::SheetClosed {
                parent: WindowId(1),
            },
            windows_without(2, 1, minimised, vec![sheet(6, 1)]),
            focus(Some(a()), None, None, vec![a(), b()], &[6, 1, 2, 3]),
            vec![
                FocusOut::Keyboard(None),
                FocusOut::Emphasis(vec![
                    (WindowId(1), Emphasis::Inactive),
                    (WindowId(6), Emphasis::Inactive),
                ]),
            ],
        ),
        row(
            "a sheet closed over a parent on a hidden Space with no other window leaves A windowless",
            over_sheet(),
            FocusIn::SheetClosed {
                parent: WindowId(1),
            },
            windows_without(
                2,
                1,
                |w| Window {
                    place: on(1, &a(), "s-2").place,
                    ..w
                },
                vec![sheet(6, 1)],
            ),
            focus(Some(a()), None, None, vec![a(), b()], &[6, 1, 2, 3]),
            vec![
                FocusOut::Keyboard(None),
                FocusOut::Emphasis(vec![
                    (WindowId(1), Emphasis::Inactive),
                    (WindowId(6), Emphasis::Inactive),
                ]),
            ],
        ),
        row(
            "a sheet closed after it unmapped finds the key already back",
            start(),
            FocusIn::SheetClosed {
                parent: WindowId(1),
            },
            windows(&[], vec![]),
            start(),
            nothing(),
        ),
    ];
    rows.into_iter().for_each(run);
}

/// `w` minimised into the dock.
fn minimised(w: Window) -> Window {
    Window {
        state: WindowState::Minimised {
            restore: Restore::New(LogicalSize { w: 800, h: 600 }),
        },
        ..w
    }
}

/// The fixture's windows with window `n` changed by `change`, plus `extra`.
fn windows_with(
    n: u64,
    change: impl Fn(Window) -> Window,
    extra: Vec<Window>,
) -> BTreeMap<WindowId, Window> {
    windows_without(0, n, change, extra)
}

/// The same without window `gone`.
fn windows_without(
    gone: u64,
    n: u64,
    change: impl Fn(Window) -> Window,
    extra: Vec<Window>,
) -> BTreeMap<WindowId, Window> {
    windows(&[gone], extra)
        .into_iter()
        .map(|(id, w)| (id, if id.0 == n { change(w) } else { w }))
        .collect()
}

/// B active, nothing key: where rule S5 leaves focus until the Space switch lands.
fn b_idle_focus() -> Focus {
    focus(Some(b()), None, None, vec![b(), a()], &[1, 2, 3])
}

#[test]
fn press_routes_beyond_the_table() {
    let windows = windows(&[], vec![sheet(7, 2)]);
    let spaces = three_spaces("s-1");
    let apps = apps(&windows);
    let view = FocusView {
        windows: &windows,
        spaces: &spaces,
        apps: &apps,
    };
    let pointer = Origin::Physical(DeviceClass::Pointer);
    let cmd_shift = HeldMods::COMMAND.with(HeldMods::SHIFT);
    let cases = [
        (
            "a resize edge of an inactive window is chrome: activate and deliver",
            3,
            HitPart::Edge,
            HeldMods::NONE,
            PressRoute::ActivateRaiseDeliver,
        ),
        (
            "command on an inactive edge delivers without activating",
            3,
            HitPart::Edge,
            HeldMods::COMMAND,
            PressRoute::DeliverWithoutActivation,
        ),
        (
            "command with shift is still command",
            3,
            HitPart::Content,
            cmd_shift,
            PressRoute::DeliverWithoutActivation,
        ),
        (
            "shift alone is nothing",
            3,
            HitPart::Content,
            HeldMods::SHIFT,
            PressRoute::ActivateRaiseSwallow,
        ),
        (
            "command on the key window's titlebar delivers",
            1,
            HitPart::Titlebar,
            HeldMods::COMMAND,
            PressRoute::Deliver,
        ),
        (
            "a window behind its sheet swallows even with command",
            2,
            HitPart::Titlebar,
            HeldMods::COMMAND,
            PressRoute::SwallowWithBell,
        ),
        (
            "the sheet itself is an inactive window like any other",
            7,
            HitPart::Content,
            HeldMods::NONE,
            PressRoute::ActivateRaiseSwallow,
        ),
        (
            "a window the view does not hold is delivered untouched",
            99,
            HitPart::Content,
            HeldMods::NONE,
            PressRoute::Deliver,
        ),
    ];
    for (name, n, part, mods, want) in cases {
        let p = Press {
            window: WindowId(n),
            part,
            mods,
            origin: pointer,
        };
        assert_eq!(route_press(&p, &start(), &view), want, "{name}");
    }
}
