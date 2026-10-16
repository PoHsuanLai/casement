//! What 03 §1.2's tables leave to the machine, part one: presses and tokens, mapping and
//! unmapping, utilities and sheets. Every row checks the focus and the outputs exactly
//! (`tests/it/focus.rs` holds the tables' own rows).

use crate::focus::fixture;
use crate::focus::rows;

use casement_core::WindowId;
use casement_core::focus::{Focus, FocusIn, FocusOut, HitPart, PressRoute, TokenCheck};
use casement_protocols::vocab::Emphasis;
use fixture::{a, b, focus, ids, sheet, start, windows};
use rows::{attention, nothing, on, over_sheet, press_on, row, run, utility};

#[test]
fn presses_and_tokens_beyond_the_table() {
    let rows = vec![
        row(
            "a press on another window of the active app keys it and keeps the app",
            start(),
            press_on(2, HitPart::Content),
            windows(&[], vec![]),
            focus(Some(a()), Some(2), Some(2), vec![a(), b()], &[2, 1, 3]),
            vec![
                FocusOut::Raise(ids(&[2])),
                FocusOut::Route(PressRoute::ActivateRaiseSwallow),
                FocusOut::Keyboard(Some(WindowId(2))),
                FocusOut::Emphasis(vec![
                    (WindowId(1), Emphasis::Inactive),
                    (WindowId(2), Emphasis::Key),
                ]),
            ],
        ),
        row(
            "a press on an inactive titlebar is a first-mouse region: activate and deliver",
            start(),
            press_on(3, HitPart::Titlebar),
            windows(&[], vec![]),
            focus(Some(b()), Some(3), Some(3), vec![b(), a()], &[3, 1, 2]),
            vec![
                FocusOut::Raise(ids(&[3])),
                FocusOut::Route(PressRoute::ActivateRaiseDeliver),
                FocusOut::Keyboard(Some(WindowId(3))),
                FocusOut::Emphasis(vec![
                    (WindowId(1), Emphasis::Inactive),
                    (WindowId(3), Emphasis::Key),
                ]),
                FocusOut::ActiveApp(Some(b())),
            ],
        ),
        row(
            "a press behind a sheet is swallowed with the bell",
            start(),
            press_on(2, HitPart::Content),
            windows(&[], vec![sheet(7, 2)]),
            start(),
            vec![
                FocusOut::Route(PressRoute::SwallowWithBell),
                FocusOut::Bell(a()),
            ],
        ),
        row(
            "a press on a utility keys it and the document stays main",
            start(),
            press_on(8, HitPart::Content),
            windows(&[], vec![utility(8)]),
            focus(Some(a()), Some(8), Some(1), vec![a(), b()], &[8, 1, 2, 3]),
            vec![
                FocusOut::Raise(ids(&[8])),
                FocusOut::Route(PressRoute::ActivateRaiseSwallow),
                FocusOut::Keyboard(Some(WindowId(8))),
                FocusOut::Emphasis(vec![
                    (WindowId(1), Emphasis::Main),
                    (WindowId(8), Emphasis::Key),
                ]),
            ],
        ),
        row(
            "a valid token for the key window only raises it",
            start(),
            FocusIn::Token {
                window: WindowId(1),
                token: TokenCheck::Valid,
            },
            windows(&[], vec![]),
            start(),
            vec![FocusOut::Raise(ids(&[1]))],
        ),
        row(
            "a missing token asks for attention",
            start(),
            FocusIn::Token {
                window: WindowId(3),
                token: TokenCheck::Missing,
            },
            windows(&[], vec![]),
            start(),
            attention(b()),
        ),
        row(
            "a token for a window that is not there does nothing",
            start(),
            FocusIn::Token {
                window: WindowId(99),
                token: TokenCheck::Valid,
            },
            windows(&[], vec![]),
            start(),
            nothing(),
        ),
    ];
    rows.into_iter().for_each(run);
}

#[test]
fn mapping_beyond_the_table() {
    let rows = vec![
        row(
            "a modeless utility of the active app maps without taking the key",
            start(),
            FocusIn::Mapped(WindowId(8)),
            windows(&[], vec![utility(8)]),
            start(),
            nothing(),
        ),
        row(
            "a window of the active app on a Space nobody shows asks for attention",
            start(),
            FocusIn::Mapped(WindowId(9)),
            windows(&[], vec![on(9, &a(), "s-2")]),
            start(),
            attention(a()),
        ),
        row(
            "a sheet maps key over its window, which stays main",
            start(),
            FocusIn::Mapped(WindowId(6)),
            windows(&[], vec![sheet(6, 1)]),
            over_sheet(),
            vec![
                FocusOut::Raise(ids(&[6])),
                FocusOut::Keyboard(Some(WindowId(6))),
                FocusOut::Emphasis(vec![
                    (WindowId(1), Emphasis::Main),
                    (WindowId(6), Emphasis::Key),
                ]),
            ],
        ),
        row(
            "with no app active a mapped window only asks for attention (A7)",
            Focus::none(),
            FocusIn::Mapped(WindowId(1)),
            windows(&[], vec![]),
            Focus::none(),
            attention(a()),
        ),
    ];
    rows.into_iter().for_each(run);
}

#[test]
fn unmapping_beyond_the_table() {
    let rows = vec![
        row(
            "closing a sheet returns the key to its window",
            over_sheet(),
            FocusIn::Unmapped(WindowId(6)),
            windows(&[], vec![]),
            start(),
            vec![
                FocusOut::Keyboard(Some(WindowId(1))),
                FocusOut::Emphasis(vec![(WindowId(1), Emphasis::Key)]),
            ],
        ),
        row(
            "closing the main window under a key utility chooses another document",
            focus(Some(a()), Some(8), Some(1), vec![a(), b()], &[8, 1, 2, 3]),
            FocusIn::Unmapped(WindowId(1)),
            windows(&[1], vec![utility(8)]),
            focus(Some(a()), Some(8), Some(2), vec![a(), b()], &[8, 2, 3]),
            vec![FocusOut::Emphasis(vec![(WindowId(2), Emphasis::Main)])],
        ),
        row(
            "closing a background window changes nothing the person sees",
            start(),
            FocusIn::Unmapped(WindowId(2)),
            windows(&[2], vec![]),
            focus(Some(a()), Some(1), Some(1), vec![a(), b()], &[1, 3]),
            nothing(),
        ),
        row(
            "closing the key window skips the app's windows on other Spaces",
            start(),
            FocusIn::Unmapped(WindowId(1)),
            windows(&[1], vec![on(2, &a(), "s-2")]),
            focus(Some(a()), None, None, vec![a(), b()], &[2, 3]),
            vec![FocusOut::Keyboard(None)],
        ),
    ];
    rows.into_iter().for_each(run);
}

/// CASC-14: a window with a modal sheet open is never keyed behind it, whether a token
/// activates it or the key window closes and its app picks the next one.
#[test]
fn a_window_with_an_open_sheet_is_keyed_through_the_sheet() {
    use casement_core::focus::{FocusView, step};
    let before = focus(Some(a()), Some(2), Some(2), vec![a(), b()], &[2, 1, 3]);
    let cases = [
        (
            "a valid token for the parent",
            FocusIn::Token {
                window: WindowId(1),
                token: TokenCheck::Valid,
            },
            windows(&[], vec![sheet(6, 1)]),
        ),
        (
            "the key window closing",
            FocusIn::Unmapped(WindowId(2)),
            windows(&[2], vec![sheet(6, 1)]),
        ),
    ];
    for (name, input, table) in cases {
        let apps = fixture::apps(&table);
        let spaces = rows::three_spaces("s-1");
        let view = FocusView {
            windows: &table,
            spaces: &spaces,
            apps: &apps,
        };
        let (got, _) = step(before.clone(), input, &view);
        assert_eq!(got.key, Some(WindowId(6)), "{name}: the sheet is key");
        assert_eq!(got.main, Some(WindowId(1)), "{name}: its window stays main");
    }
}
