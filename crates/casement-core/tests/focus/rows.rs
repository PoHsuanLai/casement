//! Rows that check the focus and the outputs exactly, over a fixture of three Spaces.
#![allow(dead_code, reason = "each test binary uses a different part")]

use crate::fixture::{a, apps, b, focus, press};
use crate::support::{desktop, headless, proven, space_id, window};
use casement_core::WindowId;
use casement_core::app::{AppKey, AppVisibility};
use casement_core::focus::{Focus, FocusIn, FocusOut, FocusView, HitPart, step};
use casement_core::space::SpaceBook;
use casement_core::window::Window;
use casement_protocols::vocab::{Attention, MacMods, WindowRole};
use std::collections::BTreeMap;

pub fn c() -> AppKey {
    proven("org.example.C")
}

/// A document window of `app` on Space `space`.
pub fn on(n: u64, app: &AppKey, space: &str) -> Window {
    window(n, app, space)
}

/// A modeless utility panel of A on `s-1`.
pub fn utility(n: u64) -> Window {
    Window {
        role: WindowRole::Utility,
        ..window(n, &a(), "s-1")
    }
}

/// `s-1`, `s-2` and `s-3` on one output, the first or `shown` showing.
pub fn three_spaces(shown: &str) -> SpaceBook {
    let output = headless("HEADLESS-1").key;
    ["s-1", "s-2", "s-3"]
        .iter()
        .zip(0..)
        .fold(SpaceBook::empty(), |book, (id, position)| {
            book.with_space(desktop(id, &output, position))
        })
        .with_shown(output, space_id(shown))
}

/// One row: the focus before, the input, the windows after the event's own change, which
/// apps are hidden, which Space is shown, and what must come out.
pub struct Row {
    pub name: &'static str,
    pub before: Focus,
    pub input: FocusIn,
    pub windows: BTreeMap<WindowId, Window>,
    pub hidden: Vec<AppKey>,
    pub shown: &'static str,
    pub focus: Focus,
    pub out: Vec<FocusOut>,
}

pub fn row(
    name: &'static str,
    before: Focus,
    input: FocusIn,
    windows: BTreeMap<WindowId, Window>,
    focus: Focus,
    out: Vec<FocusOut>,
) -> Row {
    Row {
        name,
        before,
        input,
        windows,
        hidden: Vec::new(),
        shown: "s-1",
        focus,
        out,
    }
}

pub fn run(row: Row) {
    let mut apps = apps(&row.windows);
    for app in &row.hidden {
        if let Some(record) = apps.get_mut(app) {
            record.visibility = AppVisibility::Hidden {
                restore_order: Vec::new(),
            };
        }
    }
    let spaces = three_spaces(row.shown);
    let view = FocusView {
        windows: &row.windows,
        spaces: &spaces,
        apps: &apps,
    };
    let (got, out) = step(row.before, row.input, &view);
    assert_eq!(got, row.focus, "{}: focus", row.name);
    assert_eq!(out, row.out, "{}: outputs", row.name);
}

pub fn nothing() -> Vec<FocusOut> {
    Vec::new()
}

pub fn attention(app: AppKey) -> Vec<FocusOut> {
    vec![FocusOut::Attention(app, Attention::Informational)]
}

pub fn over_sheet() -> Focus {
    focus(Some(a()), Some(6), Some(1), vec![a(), b()], &[6, 1, 2, 3])
}

pub fn press_on(n: u64, part: HitPart) -> FocusIn {
    press(n, part, MacMods::NONE)
}
