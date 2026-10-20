//! One handler per [`FocusIn`]: where focus goes, and what is asked that the move does not
//! imply. Each is a row (or a few) of 03 §1.2's table.

use super::super::{Focus, FocusOut, FocusView, Press, PressRoute, TokenCheck, route_press};
use super::keying::is_modeless_utility;
use super::outcome::Outcome;
use crate::app::AppKey;
use crate::ids::WindowId;
use crate::window::Window;
use casement_protocols::vocab::{ActorKind, Attention};

/// The window asks for the person's eye: a dock bounce, no focus.
fn attention(app: &AppKey) -> FocusOut {
    FocusOut::Attention(app.clone(), Attention::Informational)
}

/// The window that takes the key when `window` is to: its modal sheet while one is open (a
/// window with a sheet open is not keyed behind it, whatever asked for it).
fn through_sheet<'v>(window: &'v Window, view: &'v FocusView) -> &'v Window {
    view.sheet_of(window.id).unwrap_or(window)
}

/// A press: the route is the table's (`route_press`); an activating route moves focus to the
/// window (rows 1, 2 and the sheet row). A press on the key window while another window covers
/// it also raises it, as a click on any window brings it to the front.
pub(super) fn press(focus: Focus, press: &Press, view: &FocusView) -> Outcome {
    let route = route_press(press, &focus, view);
    let Some(window) = view.window(press.window) else {
        return Outcome::of(focus).routing(route);
    };
    match route {
        PressRoute::ActivateRaiseSwallow | PressRoute::ActivateRaiseDeliver => {
            Outcome::of(focus.keyed(window, view))
                .raising(vec![window.id])
                .routing(route)
        }
        PressRoute::SwallowWithBell => {
            Outcome::asking(focus, vec![FocusOut::Bell(window.app.clone())]).routing(route)
        }
        PressRoute::Deliver if focus.key == Some(window.id) && view.is_covered(window) => {
            Outcome::of(focus).raising(vec![window.id]).routing(route)
        }
        PressRoute::Deliver
        | PressRoute::DeliverWithoutActivation
        | PressRoute::MoveWithoutActivation => Outcome::of(focus).routing(route),
    }
}

/// An xdg-activation token: a valid one activates the window; a missing or stale one only
/// asks for attention (A7).
pub(super) fn token(
    focus: Focus,
    window: WindowId,
    token: TokenCheck,
    view: &FocusView,
) -> Outcome {
    let Some(window) = view.window(window) else {
        return Outcome::of(focus);
    };
    match token {
        TokenCheck::Valid => {
            Outcome::of(focus.keyed(through_sheet(window, view), view)).raising(vec![window.id])
        }
        TokenCheck::Stale | TokenCheck::Missing => {
            Outcome::asking(focus, vec![attention(&window.app)])
        }
    }
}

/// A window mapped: it takes the key when its app is active and the person can see it (unless
/// it is a modeless utility, which never does); otherwise its app asks for attention (A7).
pub(super) fn mapped(focus: Focus, window: WindowId, view: &FocusView) -> Outcome {
    let Some(window) = view.window(window) else {
        return Outcome::of(focus);
    };
    let wanted = focus.active.as_ref() == Some(&window.app) && view.is_visible(window);
    match (wanted, is_modeless_utility(window)) {
        (false, _) => Outcome::asking(focus, vec![attention(&window.app)]),
        (true, true) => Outcome::of(focus),
        (true, false) => Outcome::of(focus.keyed(window, view)).raising(vec![window.id]),
    }
}

/// A window unmapped (it is already gone from the view). If it was key, the app's next window
/// by MRU that the person can see takes the key; with none the app stays active and windowless
/// (A9). If it was main, main is chosen again. Windows the view no longer holds leave the MRU.
pub(super) fn unmapped(focus: Focus, window: WindowId, view: &FocusView) -> Outcome {
    let was_key = focus.key == Some(window);
    let was_main = focus.main == Some(window);
    let focus = Focus {
        window_mru: focus
            .window_mru
            .into_iter()
            .filter(|w| *w != window && view.window(*w).is_some())
            .collect(),
        ..focus
    };
    if was_key {
        return Outcome::of(rekeyed(focus, window, view));
    }
    if was_main {
        let main = focus
            .key
            .and_then(|key| view.window(key))
            .and_then(|key| focus.main_for(key, view));
        return Outcome::of(Focus { main, ..focus });
    }
    Outcome::of(focus)
}

/// The key `lost` window's app is active and its next window by MRU that the person can see takes
/// the key; with none the app stays active and windowless (A9). `lost` is never the next key.
fn rekeyed(focus: Focus, lost: WindowId, view: &FocusView) -> Focus {
    let next = focus.active.as_ref().and_then(|app| {
        let visible: Vec<&Window> = view
            .windows_of(app)
            .into_iter()
            .filter(|w| w.id != lost && view.is_visible(w))
            .collect();
        focus.latest(&visible)
    });
    match next {
        Some(next) => focus.keyed(through_sheet(next, view), view),
        None => Focus {
            key: None,
            main: None,
            ..focus
        },
    }
}

/// The switcher, the dock, the launcher or an intent activated an app: the app's windows on
/// the shown Spaces come forward in its own order and its most recent one takes the key. An
/// app, a third party or an unknown actor cannot move focus (A7): it asks for attention. An
/// app with no window to show becomes active and windowless; the Space switch of rule S5 is
/// the model's, and `SpaceShown` then keys the app's window.
pub(super) fn activate_app(focus: Focus, app: AppKey, by: ActorKind, view: &FocusView) -> Outcome {
    if matches!(
        by,
        ActorKind::App | ActorKind::ThirdParty | ActorKind::Unknown
    ) {
        return Outcome::asking(focus, vec![attention(&app)]);
    }
    let shown: Vec<&Window> = view
        .windows_of(&app)
        .into_iter()
        .filter(|w| view.is_shown(w))
        .collect();
    let key = focus
        .latest(&shown)
        .map(|window| through_sheet(window, view));
    let raise = shown.iter().map(|w| w.id).collect();
    Outcome::of(focus.activated(&app, key, view)).raising(raise)
}

/// A live lease keys the window it holds (a run about to send keys): the window the person can
/// see takes the key and comes to the front, its app active. Any other window (on a Space not
/// shown, in the dock, of a hidden app) moves nothing; the model brings the Space and the app
/// back first. This is not [`activate_app`]: A7 is for apps, and a lease is the person's run.
pub(super) fn lease_focus(focus: Focus, window: WindowId, view: &FocusView) -> Outcome {
    let Some(window) = view.window(window).filter(|w| view.is_visible(w)) else {
        return Outcome::of(focus);
    };
    Outcome::of(focus.keyed(through_sheet(window, view), view)).raising(vec![window.id])
}

/// An app was hidden. If it was the active app, the previous app in MRU with a window the
/// person can see becomes active and takes its most recent window as key; with none, nothing
/// is active. Hiding any other app moves nothing.
pub(super) fn hide_app(focus: Focus, app: &AppKey, view: &FocusView) -> Outcome {
    if focus.active.as_ref() != Some(app) {
        return Outcome::of(focus);
    }
    let previous = focus
        .app_mru
        .iter()
        .filter(|other| *other != app)
        .find_map(|other| {
            let visible: Vec<&Window> = view
                .windows_of(other)
                .into_iter()
                .filter(|w| view.is_visible(w))
                .collect();
            focus.latest(&visible).map(|window| (other.clone(), window))
        });
    match previous {
        Some((other, window)) => Outcome::of(focus.activated(&other, Some(window), view)),
        None => Outcome::of(focus.idle()),
    }
}

/// An output began showing a Space. A key window the person can still see stays key.
/// Otherwise the active app's most recent window on the shown Spaces takes the key, or, when
/// the active app has none there, the most recent window of any app does and its app becomes
/// active (the Mac follows the Space). With no app active, or no window to take it, the key
/// goes to nothing and the active app stays.
pub(super) fn space_shown(focus: Focus, view: &FocusView) -> Outcome {
    let key_visible = focus
        .key
        .and_then(|key| view.window(key))
        .is_some_and(|w| view.is_visible(w));
    let Some(active) = focus.active.clone().filter(|_| !key_visible) else {
        return Outcome::of(focus);
    };
    let own: Vec<&Window> = view
        .windows_of(&active)
        .into_iter()
        .filter(|w| view.is_visible(w))
        .collect();
    let any: Vec<&Window> = focus
        .window_mru
        .iter()
        .filter_map(|id| view.window(*id))
        .filter(|w| view.is_visible(w))
        .collect();
    let next = focus.latest(&own).or_else(|| any.first().copied());
    Outcome::of(match next {
        Some(window) => focus.keyed(window, view),
        None => Focus {
            key: None,
            main: None,
            ..focus
        },
    })
}

/// A sheet opened on `parent`: when the parent is key the sheet takes the key and the parent
/// stays main; a sheet on a window that is not key waits for a press.
pub(super) fn sheet_opened(
    focus: Focus,
    parent: WindowId,
    sheet: WindowId,
    view: &FocusView,
) -> Outcome {
    match (view.window(parent), view.window(sheet)) {
        (Some(parent), Some(sheet)) if focus.key == Some(parent.id) => {
            Outcome::of(focus.keyed_over(sheet, Some(parent.id)))
        }
        _ => Outcome::of(focus),
    }
}

/// A sheet closed: the key goes back to its window when the person can see it, unless it
/// already has (the sheet unmapped first) or the person has moved on to another window. A
/// window that is minimised, on a Space no output shows or of a hidden app cannot take the key:
/// when the closing sheet held it, the key is re-chosen as for any key lost (A39).
pub(super) fn sheet_closed(focus: Focus, parent: WindowId, view: &FocusView) -> Outcome {
    let Some(parent) = view.window(parent) else {
        return Outcome::of(focus);
    };
    let sheet_key = focus.key.filter(|key| {
        view.window(*key)
            .is_some_and(|key| key.parent == crate::window::Parent::Window(parent.id))
    });
    let key_gone_to_nothing = focus.key.is_none() && focus.active.as_ref() == Some(&parent.app);
    match (view.is_visible(parent), sheet_key) {
        (true, Some(_)) => Outcome::of(focus.keyed(parent, view)),
        (true, None) if key_gone_to_nothing => Outcome::of(focus.keyed(parent, view)),
        (false, Some(sheet)) => Outcome::of(rekeyed(focus, sheet, view)),
        _ => Outcome::of(focus),
    }
}
