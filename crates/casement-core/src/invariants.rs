//! What must hold after every step ([`crate::Model::check_invariants`]).

use crate::app::{AppKey, AppVisibility};
use crate::ids::{LayerKey, OutputKey, WindowId};
use crate::lock::{LockState, LockSurfaces};
use crate::model::Model;
use crate::space::{Space, SpaceContent};
use crate::window::{Parent, SpacePlace, Window, WindowState};
use casement_protocols::vocab::Emphasis;
use porter_core::SpaceId;
use std::collections::BTreeSet;

/// The first invariant found broken.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvariantBreak {
    /// The key window is not a mapped window.
    KeyWindowNotMapped(WindowId),
    /// A window is on a Space that does not exist.
    WindowWithoutSpace(WindowId),
    /// A Space belongs to an output that is not plugged.
    SpaceOnMissingOutput(SpaceId),
    /// A window's app is not in the app table.
    AppWithoutKey(WindowId),
    /// The active app is hidden.
    FocusOnHiddenApp(AppKey),
    /// Two Spaces of one output share a position.
    DuplicateSpaceIndex(OutputKey),
    /// The main window is not a mapped window.
    MainWindowNotMapped(WindowId),
    /// The key window is not its app's, or the active app is not the key window's.
    KeyOutsideActiveApp(WindowId),
    /// The key window cannot be seen: it is in the dock, on a Space no output shows, or its
    /// app is hidden.
    KeyWindowNotVisible(WindowId),
    /// A window's emphasis does not follow from who is key and main.
    EmphasisOutOfStep(WindowId),
    /// A window's parent is not a mapped window.
    ParentMissing(WindowId),
    /// A window on every Space has no plugged home output while an output is plugged, or a
    /// home that is not plugged.
    HomeOutputMissing(WindowId),
    /// Two windows share a place in the stacking order.
    DuplicateStackRank(WindowId),
    /// An app's own order of windows is not its windows.
    AppWindowsOutOfStep(AppKey),
    /// An output shows no Space, or a Space that is not its own.
    OutputShowsNothing(OutputKey),
    /// A full-screen Space and its window do not agree.
    FullScreenOutOfStep(SpaceId),
    /// A Split View names a window that is not mapped.
    SplitViewWindowMissing(SpaceId),
    /// A layer surface is on an output that is not plugged.
    LayerOnMissingOutput(LayerKey),
    /// The lock says a surface covers an output that is not plugged.
    LockCoversMissingOutput(OutputKey),
    /// The consent machine's questions, clients and book disagree (`Privilege::in_step`).
    PrivilegeOutOfStep,
}

/// The first invariant `model` breaks, checking places, then Spaces, then apps, then focus,
/// then layers and the lock.
pub(crate) fn check(model: &Model) -> Result<(), InvariantBreak> {
    let checks: [fn(&Model) -> Option<InvariantBreak>; 16] = [
        windows_have_places,
        spaces_have_outputs,
        space_rows_are_numbered_once,
        outputs_show_their_own_space,
        full_screen_spaces_agree,
        windows_have_apps,
        apps_hold_their_windows,
        parents_exist,
        stacking_is_total,
        focus_is_on_mapped_windows,
        key_window_is_seen,
        emphasis_follows_focus,
        layers_have_outputs,
        lock_covers_plugged_outputs,
        active_app_is_shown,
        privilege_is_in_step,
    ];
    checks
        .into_iter()
        .find_map(|check| check(model))
        .map_or(Ok(()), Err)
}

fn privilege_is_in_step(model: &Model) -> Option<InvariantBreak> {
    (!model.privilege().in_step()).then_some(InvariantBreak::PrivilegeOutOfStep)
}

fn windows_have_places(model: &Model) -> Option<InvariantBreak> {
    model
        .windows()
        .values()
        .find_map(|window| match &window.place {
            SpacePlace::On(id) => model
                .spaces()
                .get(id)
                .is_none()
                .then_some(InvariantBreak::WindowWithoutSpace(window.id)),
            SpacePlace::AllSpaces => homeless_or_misplaced(model, window),
        })
}

/// An every-Space window sits on a plugged output; with none plugged it has no home.
fn homeless_or_misplaced(model: &Model, window: &Window) -> Option<InvariantBreak> {
    let plugged = !model.outputs().0.is_empty();
    let well = match &window.home {
        Some(home) => model.outputs().0.contains_key(home),
        None => !plugged,
    };
    (!well).then_some(InvariantBreak::HomeOutputMissing(window.id))
}

fn spaces_have_outputs(model: &Model) -> Option<InvariantBreak> {
    model
        .spaces()
        .iter()
        .find(|space| !model.outputs().0.contains_key(&space.output))
        .map(|space| InvariantBreak::SpaceOnMissingOutput(space.id.clone()))
}

fn space_rows_are_numbered_once(model: &Model) -> Option<InvariantBreak> {
    let mut seen = BTreeSet::new();
    model
        .spaces()
        .iter()
        .find(|space| !seen.insert((space.output.clone(), space.position)))
        .map(|space| InvariantBreak::DuplicateSpaceIndex(space.output.clone()))
}

fn outputs_show_their_own_space(model: &Model) -> Option<InvariantBreak> {
    model.outputs().0.keys().find_map(|output| {
        let shown = model.spaces().shown_on(output);
        let own = shown.is_some_and(|space| space.output == *output);
        (!own).then(|| InvariantBreak::OutputShowsNothing(output.clone()))
    })
}

fn full_screen_spaces_agree(model: &Model) -> Option<InvariantBreak> {
    model
        .spaces()
        .iter()
        .find_map(|space| match &space.content {
            SpaceContent::FullScreen(window) => (!holds_full_screen(model, space, *window))
                .then(|| InvariantBreak::FullScreenOutOfStep(space.id.clone())),
            SpaceContent::SplitView(pair) => {
                let mapped = |id| model.windows().contains_key(&id);
                let holds = |id| holds_full_screen(model, space, id);
                match (
                    mapped(pair.leading) && mapped(pair.trailing),
                    holds(pair.leading) && holds(pair.trailing),
                ) {
                    (false, _) => Some(InvariantBreak::SplitViewWindowMissing(space.id.clone())),
                    (true, false) => Some(InvariantBreak::FullScreenOutOfStep(space.id.clone())),
                    (true, true) => None,
                }
            }
            SpaceContent::Desktop | SpaceContent::Agent(_) => None,
        })
}

/// The window is mapped, full screen on this Space, and on it.
fn holds_full_screen(model: &Model, space: &Space, window: WindowId) -> bool {
    model.windows().get(&window).is_some_and(|held| {
        held.place == SpacePlace::On(space.id.clone())
            && matches!(&held.state, WindowState::FullScreen { space: own, .. } if *own == space.id)
    })
}

fn windows_have_apps(model: &Model) -> Option<InvariantBreak> {
    model
        .windows()
        .values()
        .find(|window| !model.apps().contains_key(&window.app))
        .map(|window| InvariantBreak::AppWithoutKey(window.id))
}

fn apps_hold_their_windows(model: &Model) -> Option<InvariantBreak> {
    model.apps().values().find_map(|app| {
        let held: BTreeSet<WindowId> = app.windows.iter().copied().collect();
        let owned: BTreeSet<WindowId> = model
            .windows()
            .values()
            .filter(|window| window.app == app.key)
            .map(|window| window.id)
            .collect();
        let once = held.len() == app.windows.len();
        (!(once && held == owned)).then(|| InvariantBreak::AppWindowsOutOfStep(app.key.clone()))
    })
}

fn parents_exist(model: &Model) -> Option<InvariantBreak> {
    model
        .windows()
        .values()
        .find_map(|window| match window.parent {
            Parent::Window(parent) => (!model.windows().contains_key(&parent))
                .then_some(InvariantBreak::ParentMissing(window.id)),
            Parent::None => None,
        })
}

fn stacking_is_total(model: &Model) -> Option<InvariantBreak> {
    let mut seen = BTreeSet::new();
    model
        .windows()
        .values()
        .find(|window| !seen.insert(window.stack))
        .map(|window| InvariantBreak::DuplicateStackRank(window.id))
}

fn focus_is_on_mapped_windows(model: &Model) -> Option<InvariantBreak> {
    let focus = model.focus();
    let mapped = |id: &WindowId| model.windows().contains_key(id);
    let key = focus.key.filter(|id| !mapped(id));
    let main = focus.main.filter(|id| !mapped(id));
    let outside = focus
        .key
        .and_then(|id| model.windows().get(&id))
        .filter(|window| focus.active.as_ref() != Some(&window.app));
    key.map(InvariantBreak::KeyWindowNotMapped)
        .or_else(|| main.map(InvariantBreak::MainWindowNotMapped))
        .or_else(|| outside.map(|window| InvariantBreak::KeyOutsideActiveApp(window.id)))
}

fn key_window_is_seen(model: &Model) -> Option<InvariantBreak> {
    let key = model.focus().key.and_then(|id| model.windows().get(&id))?;
    (!is_seen(model, key)).then_some(InvariantBreak::KeyWindowNotVisible(key.id))
}

/// Whether the person can see the window: on a Space an output shows (or on every Space), not
/// in the dock, and its app not hidden.
fn is_seen(model: &Model, window: &Window) -> bool {
    let on_shown_space = match &window.place {
        SpacePlace::AllSpaces => true,
        SpacePlace::On(id) => model.spaces().get(id).is_some_and(|space| {
            model
                .spaces()
                .shown_on(&space.output)
                .is_some_and(|shown| shown.id == space.id)
        }),
    };
    let hidden = model
        .apps()
        .get(&window.app)
        .is_some_and(|app| matches!(app.visibility, AppVisibility::Hidden { .. }));
    on_shown_space && !matches!(window.state, WindowState::Minimised { .. }) && !hidden
}

fn emphasis_follows_focus(model: &Model) -> Option<InvariantBreak> {
    let focus = model.focus();
    model
        .windows()
        .values()
        .find(|window| {
            let want = if focus.key == Some(window.id) {
                Emphasis::Key
            } else if focus.main == Some(window.id) {
                Emphasis::Main
            } else {
                Emphasis::Inactive
            };
            window.emphasis != want
        })
        .map(|window| InvariantBreak::EmphasisOutOfStep(window.id))
}

fn layers_have_outputs(model: &Model) -> Option<InvariantBreak> {
    model
        .layers()
        .iter()
        .find(|(_, spec)| !model.outputs().0.contains_key(&spec.output))
        .map(|(key, _)| InvariantBreak::LayerOnMissingOutput(*key))
}

fn lock_covers_plugged_outputs(model: &Model) -> Option<InvariantBreak> {
    match model.lock() {
        LockState::Locked {
            surfaces: LockSurfaces::Covering(covered),
        } => covered
            .iter()
            .find(|output| !model.outputs().0.contains_key(*output))
            .cloned()
            .map(InvariantBreak::LockCoversMissingOutput),
        _ => None,
    }
}

fn active_app_is_shown(model: &Model) -> Option<InvariantBreak> {
    let active = model.focus().active.as_ref()?;
    let hidden = model
        .apps()
        .get(active)
        .is_some_and(|app| matches!(app.visibility, AppVisibility::Hidden { .. }));
    hidden.then(|| InvariantBreak::FocusOnHiddenApp(active.clone()))
}
