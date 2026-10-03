//! The fixture 03 §1.2's rows share: apps A and B; windows 1 and 2 of A and 3 of B on the shown
//! Space `s-1`; A active with 1 key and main.
#![allow(dead_code, reason = "each test binary uses a different part")]

use crate::support::{desktop, headless, proven, space_id, window};
use casement_core::WindowId;
use casement_core::app::{App, AppKey, AppLife, AppVisibility, PidSet};
use casement_core::focus::{Focus, FocusIn, FocusOut, HitPart, Press};
use casement_core::input::{DeviceClass, Origin};
use casement_core::space::SpaceBook;
use casement_core::window::{Parent, Window};
use casement_protocols::vocab::{Attention, MacMods, Modality, WindowRole};
use std::collections::BTreeMap;

pub fn a() -> AppKey {
    proven("org.example.A")
}

pub fn b() -> AppKey {
    proven("org.example.B")
}

pub fn sheet(n: u64, parent: u64) -> Window {
    Window {
        role: WindowRole::Dialog,
        modality: Modality::WindowModal,
        parent: Parent::Window(WindowId(parent)),
        ..window(n, &a(), "s-1")
    }
}

/// The fixture's windows, minus `gone`, plus `extra`.
pub fn windows(gone: &[u64], extra: Vec<Window>) -> BTreeMap<WindowId, Window> {
    [
        window(1, &a(), "s-1"),
        window(2, &a(), "s-1"),
        window(3, &b(), "s-1"),
    ]
    .into_iter()
    .chain(extra)
    .filter(|w| !gone.contains(&w.id.0))
    .map(|w| (w.id, w))
    .collect()
}

/// An app record for every app that owns a window, its windows in id order, shown.
pub fn apps(windows: &BTreeMap<WindowId, Window>) -> BTreeMap<AppKey, App> {
    windows
        .values()
        .fold(BTreeMap::<AppKey, App>::new(), |mut apps, window| {
            apps.entry(window.app.clone())
                .or_insert_with(|| App {
                    key: window.app.clone(),
                    pids: PidSet::default(),
                    windows: Vec::new(),
                    life: AppLife::Running,
                    visibility: AppVisibility::Shown,
                    attention: Attention::None,
                })
                .windows
                .push(window.id);
            apps
        })
}

pub fn spaces() -> SpaceBook {
    let output = headless("HEADLESS-1").key;
    SpaceBook::empty()
        .with_space(desktop("s-1", &output, 0))
        .with_shown(output, space_id("s-1"))
}

pub fn ids(ns: &[u64]) -> Vec<WindowId> {
    ns.iter().map(|n| WindowId(*n)).collect()
}

pub fn focus(
    active: Option<AppKey>,
    key: Option<u64>,
    main: Option<u64>,
    apps: Vec<AppKey>,
    mru: &[u64],
) -> Focus {
    Focus {
        active,
        key: key.map(WindowId),
        main: main.map(WindowId),
        app_mru: apps,
        window_mru: ids(mru),
    }
}

pub fn start() -> Focus {
    focus(Some(a()), Some(1), Some(1), vec![a(), b()], &[1, 2, 3])
}

pub fn press(n: u64, part: HitPart, mods: MacMods) -> FocusIn {
    FocusIn::Press(Press {
        window: WindowId(n),
        part,
        mods,
        origin: Origin::Physical(DeviceClass::Pointer),
    })
}

/// After activating B through window 3.
pub fn b_active() -> Focus {
    focus(Some(b()), Some(3), Some(3), vec![b(), a()], &[3, 1, 2])
}

pub enum Outs {
    /// The outputs, exactly.
    Exactly(Vec<FocusOut>),
    /// These among the outputs.
    Including(Vec<FocusOut>),
}
