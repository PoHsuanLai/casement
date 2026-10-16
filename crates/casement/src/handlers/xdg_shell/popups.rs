//! Popups: placed inside their output, grabbed on a press, repositioned on request.

use crate::core_loop;
use crate::state::State;
use casement_core::InputSerial;
use casement_core::geometry::{LogicalPoint, LogicalSize};
use casement_core::policy::ConnectionRole;
use casement_wayland::ClientState;
use casement_wayland::convert::{point_of, serial_of};
use casement_wayland::limits::{self, Limit, Standing};
use casement_wayland::xdg_shell::{
    PopupBounds, on_new_popup, on_popup_acked, on_popup_grab, on_reposition, on_room_changed,
    popups_of,
};
use smithay::desktop::{
    PopupKeyboardGrab, PopupKind, PopupManager, PopupPointerGrab, PopupUngrabStrategy,
    find_popup_root_surface, get_popup_toplevel_coords,
};
use smithay::input::pointer::Focus;
use smithay::reexports::wayland_server::Resource;
use smithay::reexports::wayland_server::protocol::wl_seat::WlSeat;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::Serial;
use smithay::wayland::shell::xdg::{PopupConfigure, PopupSurface, PositionerState};

/// The room an output leaves a popup of `surface`, seen from its parent: the whole output its
/// root window is arranged on, in the parent's coordinates. Unknown while the root has no
/// arrangement (it has not mapped).
fn bounds_of(state: &State, surface: &PopupSurface) -> PopupBounds {
    let kind = PopupKind::Xdg(surface.clone());
    let Ok(root) = find_popup_root_surface(&kind) else {
        return PopupBounds::Unbounded;
    };
    let Some(window) = state.wl.xdg_shell.window_of_surface(&root) else {
        return layer_bounds(state, &kind, &root);
    };
    let Some(target) = state
        .core
        .arrangement
        .windows
        .iter()
        .find(|target| target.window == window)
    else {
        return PopupBounds::Unbounded;
    };
    let Some(output) = state.core.model.outputs().0.get(&target.output) else {
        return PopupBounds::Unbounded;
    };
    let nested = point_of(get_popup_toplevel_coords(&kind));
    let parent_origin = LogicalPoint {
        x: target.rect.origin.x.saturating_add(nested.x),
        y: target.rect.origin.y.saturating_add(nested.y),
    };
    PopupBounds::relative_to(output.config.logical_rect(), parent_origin)
}

/// The room an output leaves a popup whose root is a layer surface: the output the layer is on,
/// seen from where the layer is arranged. Unknown while the layer is not arranged.
fn layer_bounds(state: &State, kind: &PopupKind, root: &WlSurface) -> PopupBounds {
    let model = &state.core.model;
    let (Some(layer), outputs) = (state.wl.layer_shell.key_of(root), model.outputs()) else {
        return PopupBounds::Unbounded;
    };
    let Some(spec) = model.layers().get(&layer) else {
        return PopupBounds::Unbounded;
    };
    let Some(output) = outputs.0.get(&spec.output) else {
        return PopupBounds::Unbounded;
    };
    let full = output.config.logical_rect();
    let committed = state
        .render
        .textures
        .layer(&layer)
        .map_or(LogicalSize::ZERO, |entry| entry.committed);
    let Some(at) = model
        .layers()
        .arrange(&spec.output, full)
        .place(&layer, committed)
    else {
        return PopupBounds::Unbounded;
    };
    let nested = point_of(get_popup_toplevel_coords(kind));
    let parent_origin = LogicalPoint {
        x: at.origin.x.saturating_add(nested.x),
        y: at.origin.y.saturating_add(nested.y),
    };
    PopupBounds::relative_to(full, parent_origin)
}

/// A popup was created: it gets a key and is placed.
pub(crate) fn new_popup(state: &mut State, surface: PopupSurface, positioner: PositionerState) {
    // A client past its cap is cut (CASC-03); no popup is made for the one it asked for.
    if let Some(client) = surface.wl_surface().client() {
        let held =
            ClientState::of(&client).map_or(0, |data| state.wl.xdg_shell.popups_held_by(data.key));
        if limits::admit(&state.display, &client, Limit::Popups, held) == Standing::Over {
            return;
        }
    }
    let key = state.wl.xdg_shell.mint_popup();
    let bounds = bounds_of(state, &surface);
    let events = on_new_popup(&mut state.wl.xdg_shell, &surface, &positioner, key, bounds);
    core_loop::feed(state, events);
}

/// A popup asked to be repositioned.
pub(super) fn reposition(
    state: &mut State,
    surface: PopupSurface,
    positioner: PositionerState,
    token: u32,
) {
    let bounds = bounds_of(state, &surface);
    let events = on_reposition(
        &mut state.wl.xdg_shell,
        &surface,
        &positioner,
        token,
        bounds,
    );
    core_loop::feed(state, events);
}

/// Whether `surface` belongs to the shell's own connection. The shell opens the window menu
/// when the compositor asks (the dwell on the green light), with no input of its own to
/// name, so its popup grab is taken on trust instead of a serial.
fn is_shell(surface: &WlSurface) -> bool {
    surface.client().is_some_and(|client| {
        ClientState::of(&client).is_some_and(|data| data.role == ConnectionRole::Shell)
    })
}

/// A popup asked for an explicit grab: if the serial answers a press (or the shell asks), it
/// takes the seat's keyboard and pointer (a menu), and a popup whose grab is refused is
/// dismissed.
pub(super) fn grab(state: &mut State, surface: PopupSurface, _seat: WlSeat, serial: Serial) {
    let proven = is_shell(surface.wl_surface())
        || state.grab_serial_proves(surface.wl_surface(), serial_of(serial));
    let events = on_popup_grab(&mut state.wl.xdg_shell, &surface, serial_of(serial));
    core_loop::feed(state, events);
    if !proven {
        surface.send_popup_done();
        return;
    }
    let kind = PopupKind::Xdg(surface.clone());
    let Ok(root) = find_popup_root_surface(&kind) else {
        return;
    };
    let seat = state.wl.seat.seat.clone();
    let Ok(mut grab) = state
        .wl
        .xdg_shell
        .popups
        .grab_popup::<State>(root, kind, &seat, serial)
    else {
        return;
    };
    let keyboard = state.wl.seat.keyboard.clone();
    let pointer = state.wl.seat.pointer.clone();
    let previous = grab.previous_serial().unwrap_or_else(|| grab.serial());
    let free_or_ours = |held: bool, has: bool| !held || has;
    if !free_or_ours(
        keyboard.is_grabbed(),
        keyboard.has_grab(serial) || keyboard.has_grab(previous),
    ) || !free_or_ours(
        pointer.is_grabbed(),
        pointer.has_grab(serial) || pointer.has_grab(previous),
    ) {
        grab.ungrab(PopupUngrabStrategy::All);
        return;
    }
    // A nested grab replaces its parent's: the keyboard grab goes first, so the parent's pointer
    // grab, when it goes, finds the keyboard grab is another popup grab's and leaves it (the
    // fork's rule; a submenu opened with its parent's serial keeps the keyboard).
    keyboard.set_focus(state, grab.current_grab(), serial);
    keyboard.set_grab(state, PopupKeyboardGrab::new(&grab), serial);
    pointer.set_grab(state, PopupPointerGrab::new(&grab), serial, Focus::Keep);
}

/// The keyboard is about to go to `next` while a popup holds the grab: when `next` is not the
/// toplevel (or layer) the popup hangs from, the user has moved on (another window was
/// activated, the screen locked), and the grab is dismissed with its popups, the topmost first
/// (xdg_popup.grab: "dismissed when the user dismisses the popup"). The seat's grabs end here, so
/// the focus change that follows reaches the new target.
pub(crate) fn dismiss_for_focus(state: &mut State, next: Option<&WlSurface>) {
    let keyboard = state.wl.seat.keyboard.clone();
    if !keyboard.is_grabbed() {
        return;
    }
    let Some(held) = keyboard.current_focus() else {
        return;
    };
    let Some(popup) = state.wl.xdg_shell.popups.find_popup(&held) else {
        return;
    };
    let Ok(root) = find_popup_root_surface(&popup) else {
        return;
    };
    if next == Some(&root) {
        return;
    }
    let first = first_popup_under(&root, popup);
    let _ = PopupManager::dismiss_popup(&root, &first);
    keyboard.unset_grab(state);
    let pointer = state.wl.seat.pointer.clone();
    if pointer.is_grabbed() {
        let serial = InputSerial(smithay::utils::SERIAL_COUNTER.next_serial().into());
        let seat = state.wl.seat.seat.clone();
        casement_wayland::seat::end_grab(&seat, state, serial, state.clock.now());
    }
}

/// The popup of `root` that `popup` hangs from, at the bottom of its chain.
fn first_popup_under(root: &WlSurface, popup: PopupKind) -> PopupKind {
    let mut first = popup;
    while let Some(parent) = parent_of(&first).filter(|parent| parent != root) {
        let Some(next) = PopupManager::popups_for_surface(root)
            .map(|(kind, _)| kind)
            .find(|kind| *kind.wl_surface() == parent)
        else {
            break;
        };
        first = next;
    }
    first
}

fn parent_of(popup: &PopupKind) -> Option<WlSurface> {
    match popup {
        PopupKind::Xdg(surface) => surface.get_parent_surface(),
        PopupKind::InputMethod(_) => None,
    }
}

/// A popup's configure was acknowledged: it takes effect now, and what is under the pointer may
/// have changed with it.
pub(super) fn acked(state: &mut State, surface: &WlSurface, configure: PopupConfigure) {
    on_popup_acked(surface, configure);
    core_loop::refocus_pointer(state);
}

/// Windows moved or resized: the popups that asked to follow their parent are placed again.
pub(crate) fn refit_reactive(state: &mut State) {
    for popup in popups_of(&state.wl.xdg_shell) {
        let bounds = bounds_of(state, &popup);
        on_room_changed(&popup, bounds);
    }
}
