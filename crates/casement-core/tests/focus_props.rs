//! Whatever the person and the apps do, in any order, the focus stays well formed: the key
//! window is a visible window of the active app, main needs a key, no list repeats, and the
//! MRU lists name only windows that exist.

#[path = "focus/fixture.rs"]
mod fixture;
#[path = "focus/rows.rs"]
mod rows;
mod support;

use casement_core::WindowId;
use casement_core::app::{AppKey, AppVisibility};
use casement_core::focus::{Focus, FocusIn, FocusView, HitPart, Press, TokenCheck, step};
use casement_core::input::{DeviceClass, Origin};
use casement_core::window::Window;
use casement_protocols::vocab::{ActorKind, MacMods};
use fixture::{a, apps, b};
use proptest::prelude::*;
use rows::three_spaces;
use std::collections::{BTreeMap, BTreeSet};
use support::{headless, space_id, window};

/// What happens next. Windows 1 to 6; 1, 2 and 5 belong to A and 3, 4 and 6 to B; 1 to 3 sit
/// on `s-1` and 4 to 6 on `s-2`.
#[derive(Debug, Clone)]
enum Op {
    Map(u64),
    Unmap(u64),
    Press(u64, bool),
    Token(u64),
    Activate(bool),
    Hide(bool),
    Unhide(bool),
    Show(bool),
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        (1u64..=6).prop_map(Op::Map),
        (1u64..=6).prop_map(Op::Unmap),
        ((1u64..=6), any::<bool>()).prop_map(|(n, command)| Op::Press(n, command)),
        (1u64..=6).prop_map(Op::Token),
        any::<bool>().prop_map(Op::Activate),
        any::<bool>().prop_map(Op::Hide),
        any::<bool>().prop_map(Op::Unhide),
        any::<bool>().prop_map(Op::Show),
    ]
}

fn app_of(second: bool) -> AppKey {
    if second { b() } else { a() }
}

fn window_of(n: u64) -> Window {
    let app = if matches!(n, 1 | 2 | 5) { a() } else { b() };
    window(n, &app, if n <= 3 { "s-1" } else { "s-2" })
}

/// The test's own picture of the world, which the machine reads only through the view.
struct World {
    present: BTreeSet<u64>,
    hidden: BTreeSet<AppKey>,
    shown: &'static str,
}

impl World {
    fn windows(&self) -> BTreeMap<WindowId, Window> {
        self.present
            .iter()
            .map(|n| (WindowId(*n), window_of(*n)))
            .collect()
    }

    fn is_visible(&self, n: u64) -> bool {
        let w = window_of(n);
        self.present.contains(&n)
            && !self.hidden.contains(&w.app)
            && (n <= 3) == (self.shown == "s-1")
    }
}

/// The input an op makes in `world` (changing the world as the event would), if it makes one.
fn apply(world: &mut World, op: &Op) -> Option<FocusIn> {
    match *op {
        Op::Map(n) => world
            .present
            .insert(n)
            .then_some(FocusIn::Mapped(WindowId(n))),
        Op::Unmap(n) => world
            .present
            .remove(&n)
            .then_some(FocusIn::Unmapped(WindowId(n))),
        Op::Press(n, command) => world.is_visible(n).then_some(FocusIn::Press(Press {
            window: WindowId(n),
            part: HitPart::Content,
            mods: if command {
                MacMods::COMMAND
            } else {
                MacMods::NONE
            },
            origin: Origin::Physical(DeviceClass::Pointer),
        })),
        Op::Token(n) => world.is_visible(n).then_some(FocusIn::Token {
            window: WindowId(n),
            token: TokenCheck::Valid,
        }),
        Op::Activate(second) => {
            world.hidden.remove(&app_of(second));
            Some(FocusIn::ActivateApp {
                app: app_of(second),
                by: ActorKind::User,
            })
        }
        Op::Hide(second) => {
            world.hidden.insert(app_of(second));
            Some(FocusIn::HideApp(app_of(second)))
        }
        Op::Unhide(second) => {
            world.hidden.remove(&app_of(second));
            Some(FocusIn::UnhideApp(app_of(second)))
        }
        Op::Show(second) => {
            world.shown = if second { "s-2" } else { "s-1" };
            Some(FocusIn::SpaceShown {
                output: headless("HEADLESS-1").key,
                space: space_id(world.shown),
            })
        }
    }
}

fn unique<T: Ord + Clone>(items: &[T]) -> bool {
    items.iter().cloned().collect::<BTreeSet<_>>().len() == items.len()
}

fn check(focus: &Focus, world: &World) -> Result<(), TestCaseError> {
    let windows = world.windows();
    prop_assert!(unique(&focus.window_mru) && unique(&focus.app_mru));
    prop_assert!(focus.window_mru.iter().all(|id| windows.contains_key(id)));
    prop_assert!(
        focus.main.is_none() || focus.key.is_some(),
        "main needs a key"
    );
    if let Some(key) = focus.key {
        prop_assert!(world.is_visible(key.0), "the key window {key:?} is visible");
        prop_assert_eq!(
            focus.active.as_ref(),
            windows.get(&key).map(|w| &w.app),
            "the key window belongs to the active app"
        );
    }
    if let Some(main) = focus.main {
        prop_assert!(windows.contains_key(&main), "main {main:?} exists");
    }
    if let Some(active) = &focus.active {
        prop_assert!(
            !world.hidden.contains(active),
            "the active app is not hidden"
        );
    }
    Ok(())
}

proptest! {
    #[test]
    fn focus_stays_well_formed(ops in proptest::collection::vec(op(), 1..60)) {
        let mut world = World {
            present: BTreeSet::new(),
            hidden: BTreeSet::new(),
            shown: "s-1",
        };
        let mut focus = Focus::none();
        for op in &ops {
            let Some(input) = apply(&mut world, op) else { continue };
            let windows = world.windows();
            let mut apps = apps(&windows);
            for app in &world.hidden {
                if let Some(record) = apps.get_mut(app) {
                    record.visibility = AppVisibility::Hidden { restore_order: Vec::new() };
                }
            }
            let spaces = three_spaces(world.shown);
            let view = FocusView { windows: &windows, spaces: &spaces, apps: &apps };
            let (next, _) = step(focus, input, &view);
            focus = next;
            check(&focus, &world)?;
        }
    }
}
