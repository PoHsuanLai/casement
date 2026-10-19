//! 03 §1.2's focus table and press routing, one row each (what P0-F1-focus makes pass).
//!
//! The fixture: apps A and B; windows 1 and 2 of A and 3 of B on the shown Space `s-1`; A active
//! with 1 key and main. A row's view is the window table after the event's own change (a mapped
//! window is in it, an unmapped one is gone). Rows check the focus exactly and that the effects
//! 03 names are among the outputs; "unchanged" rows check the outputs exactly.

pub(crate) mod fixture;
pub(crate) mod rows;
use crate::support;

use casement_core::WindowId;
use casement_core::focus::{
    Focus, FocusIn, FocusOut, FocusView, HitPart, Press, PressRoute, TokenCheck, route_press, step,
};
use casement_core::input::{DeviceClass, Origin};
use casement_core::window::Window;
use casement_protocols::vocab::{ActorKind, Attention, MacMods};
use fixture::{Outs, a, apps, b, b_active, focus, ids, press, sheet, spaces, start, windows};
use std::collections::BTreeMap;
use support::window;

/// A name, the input, the window table after it, the focus wanted, the outputs wanted.
type FocusCase = (
    &'static str,
    FocusIn,
    BTreeMap<WindowId, Window>,
    Focus,
    Outs,
);

#[test]
fn the_focus_table() {
    let none = MacMods::NONE;
    let cmd = MacMods::COMMAND;
    let cases: Vec<FocusCase> = vec![
        (
            "press on an inactive window activates, raises and swallows",
            press(3, HitPart::Content, none),
            windows(&[], vec![]),
            b_active(),
            Outs::Including(vec![
                FocusOut::Raise(ids(&[3])),
                FocusOut::Route(PressRoute::ActivateRaiseSwallow),
                FocusOut::Keyboard(Some(WindowId(3))),
                FocusOut::ActiveApp(Some(b())),
            ]),
        ),
        (
            "press in a first-mouse region also delivers",
            press(3, HitPart::FirstMouse, none),
            windows(&[], vec![]),
            b_active(),
            Outs::Including(vec![
                FocusOut::Raise(ids(&[3])),
                FocusOut::Route(PressRoute::ActivateRaiseDeliver),
            ]),
        ),
        (
            "command-press delivers without activating",
            press(3, HitPart::Content, cmd),
            windows(&[], vec![]),
            start(),
            Outs::Exactly(vec![FocusOut::Route(PressRoute::DeliverWithoutActivation)]),
        ),
        (
            "press on the key window on top delivers",
            press(1, HitPart::Content, none),
            windows(&[2, 3], vec![]),
            start(),
            Outs::Exactly(vec![FocusOut::Route(PressRoute::Deliver)]),
        ),
        (
            "press on the key window under another raises it and delivers",
            press(1, HitPart::Content, none),
            windows(&[], vec![window(5, &b(), "s-1")]),
            start(),
            Outs::Including(vec![
                FocusOut::Raise(ids(&[1])),
                FocusOut::Route(PressRoute::Deliver),
            ]),
        ),
        (
            "press on the key window under a window off the shown Space delivers",
            press(1, HitPart::Content, none),
            windows(&[2, 3], vec![window(5, &b(), "s-2")]),
            start(),
            Outs::Exactly(vec![FocusOut::Route(PressRoute::Deliver)]),
        ),
        (
            "a valid token activates",
            FocusIn::Token {
                window: WindowId(3),
                token: TokenCheck::Valid,
            },
            windows(&[], vec![]),
            b_active(),
            Outs::Including(vec![
                FocusOut::Raise(ids(&[3])),
                FocusOut::Keyboard(Some(WindowId(3))),
            ]),
        ),
        (
            "a stale token only asks for attention",
            FocusIn::Token {
                window: WindowId(3),
                token: TokenCheck::Stale,
            },
            windows(&[], vec![]),
            start(),
            Outs::Exactly(vec![FocusOut::Attention(b(), Attention::Informational)]),
        ),
        (
            "a window of the active app maps key",
            FocusIn::Mapped(WindowId(4)),
            windows(&[], vec![window(4, &a(), "s-1")]),
            focus(Some(a()), Some(4), Some(4), vec![a(), b()], &[4, 1, 2, 3]),
            Outs::Including(vec![
                FocusOut::Raise(ids(&[4])),
                FocusOut::Keyboard(Some(WindowId(4))),
            ]),
        ),
        (
            "a window of another app maps without focus",
            FocusIn::Mapped(WindowId(5)),
            windows(&[], vec![window(5, &b(), "s-1")]),
            start(),
            Outs::Exactly(vec![FocusOut::Attention(b(), Attention::Informational)]),
        ),
        (
            "activating an app raises its windows and keys its latest",
            FocusIn::ActivateApp {
                app: b(),
                by: ActorKind::User,
            },
            windows(&[], vec![]),
            b_active(),
            Outs::Including(vec![
                FocusOut::Raise(ids(&[3])),
                FocusOut::Keyboard(Some(WindowId(3))),
                FocusOut::ActiveApp(Some(b())),
            ]),
        ),
        (
            "closing the key window keys the app's next window",
            FocusIn::Unmapped(WindowId(1)),
            windows(&[1], vec![]),
            focus(Some(a()), Some(2), Some(2), vec![a(), b()], &[2, 3]),
            Outs::Including(vec![FocusOut::Keyboard(Some(WindowId(2)))]),
        ),
        (
            "closing the app's last window leaves it active and windowless",
            FocusIn::Unmapped(WindowId(1)),
            windows(&[1, 2], vec![]),
            focus(Some(a()), None, None, vec![a(), b()], &[3]),
            Outs::Including(vec![FocusOut::Keyboard(None)]),
        ),
        (
            "hiding the active app activates the previous one",
            FocusIn::HideApp(a()),
            windows(&[], vec![]),
            focus(Some(b()), Some(3), Some(3), vec![b(), a()], &[3, 1, 2]),
            Outs::Including(vec![
                FocusOut::Keyboard(Some(WindowId(3))),
                FocusOut::ActiveApp(Some(b())),
            ]),
        ),
        (
            "a sheet takes the key from its window, which stays main",
            FocusIn::SheetOpened {
                parent: WindowId(1),
                sheet: WindowId(6),
            },
            windows(&[], vec![sheet(6, 1)]),
            focus(Some(a()), Some(6), Some(1), vec![a(), b()], &[6, 1, 2, 3]),
            Outs::Including(vec![FocusOut::Keyboard(Some(WindowId(6)))]),
        ),
    ];
    let spaces = spaces();
    for (name, input, windows, want, outs) in cases {
        let apps = apps(&windows);
        let view = FocusView {
            windows: &windows,
            spaces: &spaces,
            apps: &apps,
        };
        let (got, out) = step(start(), input, &view);
        assert_eq!(got, want, "{name}: focus");
        match outs {
            Outs::Exactly(expected) => assert_eq!(out, expected, "{name}: outputs"),
            Outs::Including(expected) => {
                for e in expected {
                    assert!(out.contains(&e), "{name}: missing {e:?} in {out:?}");
                }
            }
        }
    }
}

#[test]
fn press_routing() {
    let windows = windows(&[], vec![sheet(7, 2)]);
    let spaces = spaces();
    let apps = apps(&windows);
    let view = FocusView {
        windows: &windows,
        spaces: &spaces,
        apps: &apps,
    };
    let pointer = Origin::Physical(DeviceClass::Pointer);
    let cases = [
        (
            "the key window",
            1,
            HitPart::Content,
            MacMods::NONE,
            PressRoute::Deliver,
        ),
        (
            "the key window with command",
            1,
            HitPart::Content,
            MacMods::COMMAND,
            PressRoute::Deliver,
        ),
        (
            "a first-mouse region",
            3,
            HitPart::FirstMouse,
            MacMods::NONE,
            PressRoute::ActivateRaiseDeliver,
        ),
        (
            "elsewhere on an inactive window",
            3,
            HitPart::Content,
            MacMods::NONE,
            PressRoute::ActivateRaiseSwallow,
        ),
        (
            "command on an inactive window",
            3,
            HitPart::Content,
            MacMods::COMMAND,
            PressRoute::DeliverWithoutActivation,
        ),
        (
            "command on an inactive titlebar",
            3,
            HitPart::Titlebar,
            MacMods::COMMAND,
            PressRoute::MoveWithoutActivation,
        ),
        (
            "a window behind its sheet",
            2,
            HitPart::Content,
            MacMods::NONE,
            PressRoute::SwallowWithBell,
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
