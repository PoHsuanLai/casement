//! Popups: placed inside their output, grabbed on a press, repositioned on request.

use crate::core_loop;
use crate::state::State;
use casement_core::geometry::LogicalPoint;
use casement_wayland::convert::{point_of, serial_of};
use casement_wayland::xdg_shell::{
    GrabUse, PopupBounds, on_new_popup, on_popup_grab, on_reposition,
};
use smithay::desktop::{
    PopupKeyboardGrab, PopupKind, PopupPointerGrab, PopupUngrabStrategy, find_popup_root_surface,
    get_popup_toplevel_coords,
};
use smithay::input::pointer::Focus;
use smithay::reexports::wayland_server::protocol::wl_seat::WlSeat;
use smithay::utils::Serial;
use smithay::wayland::shell::xdg::{PopupSurface, PositionerState};

/// The room an output leaves a popup of `surface`, seen from its parent: the whole output its
/// root window is arranged on, in the parent's coordinates. Unknown while the root has no
/// arrangement (it has not mapped).
fn bounds_of(state: &State, surface: &PopupSurface) -> PopupBounds {
    let kind = PopupKind::Xdg(surface.clone());
    let Ok(root) = find_popup_root_surface(&kind) else {
        return PopupBounds::Unbounded;
    };
    let Some(window) = state.wl.xdg_shell.window_of_surface(&root) else {
        return PopupBounds::Unbounded;
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

/// A popup was created: it gets a key and is placed.
pub(super) fn new_popup(state: &mut State, surface: PopupSurface, positioner: PositionerState) {
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

/// A popup asked for an explicit grab: if the serial answers a press, it takes the seat's
/// keyboard and pointer (a menu), and a popup whose grab is refused is dismissed.
pub(super) fn grab(state: &mut State, surface: PopupSurface, _seat: WlSeat, serial: Serial) {
    let proven = state.serial_proves(surface.wl_surface(), serial_of(serial), GrabUse::AnyPress);
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
    if keyboard.is_grabbed()
        && !(keyboard.has_grab(serial)
            || keyboard.has_grab(grab.previous_serial().unwrap_or(serial)))
    {
        grab.ungrab(PopupUngrabStrategy::All);
        return;
    }
    keyboard.set_focus(state, grab.current_grab(), serial);
    keyboard.set_grab(state, PopupKeyboardGrab::new(&grab), serial);
    let pointer = state.wl.seat.pointer.clone();
    if pointer.is_grabbed()
        && !(pointer.has_grab(serial)
            || pointer.has_grab(grab.previous_serial().unwrap_or_else(|| grab.serial())))
    {
        grab.ungrab(PopupUngrabStrategy::All);
        return;
    }
    pointer.set_grab(state, PopupPointerGrab::new(&grab), serial, Focus::Keep);
}
