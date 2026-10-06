//! `wl_seat` (P0-H4). Focus targets are plain surfaces at P0 (no compositor-drawn focusable
//! chrome until P3).

use crate::core_loop;
use crate::state::State;
use casement_render::{CursorDraw, CursorShape};
use casement_wayland::convert::point_of;
use casement_wayland::{data_device, seat, shortcuts_inhibit};
use smithay::input::pointer::{CursorIcon, CursorImageStatus, CursorImageSurfaceData};
use smithay::input::{Seat, SeatHandler, SeatState};
use smithay::reexports::wayland_server::Resource;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::wayland::compositor;

/// The compositor's cursor shape for a named cursor; the ones it has no sprite for draw as the
/// arrow.
fn shape_of(icon: CursorIcon) -> CursorShape {
    match icon {
        CursorIcon::Text | CursorIcon::VerticalText => CursorShape::Text,
        CursorIcon::Pointer => CursorShape::Pointer,
        CursorIcon::Grab => CursorShape::Grab,
        CursorIcon::Grabbing => CursorShape::Grabbing,
        CursorIcon::NResize
        | CursorIcon::SResize
        | CursorIcon::NsResize
        | CursorIcon::RowResize => CursorShape::ResizeNs,
        CursorIcon::EResize
        | CursorIcon::WResize
        | CursorIcon::EwResize
        | CursorIcon::ColResize => CursorShape::ResizeEw,
        CursorIcon::NeResize | CursorIcon::SwResize | CursorIcon::NeswResize => {
            CursorShape::ResizeNesw
        }
        CursorIcon::NwResize | CursorIcon::SeResize | CursorIcon::NwseResize => {
            CursorShape::ResizeNwse
        }
        CursorIcon::Wait | CursorIcon::Progress => CursorShape::Wait,
        CursorIcon::NotAllowed | CursorIcon::NoDrop => CursorShape::NotAllowed,
        _ => CursorShape::Default,
    }
}

/// How a client's cursor request is drawn, and the surface the renderer takes it from.
fn draw_of(image: &CursorImageStatus) -> (CursorDraw, Option<WlSurface>) {
    match image {
        CursorImageStatus::Hidden => (CursorDraw::Hidden, None),
        CursorImageStatus::Named(icon) => (CursorDraw::Named(shape_of(*icon)), None),
        CursorImageStatus::Surface(surface) => {
            let hotspot = compositor::with_states(surface, |states| {
                states.data_map.get::<CursorImageSurfaceData>().map_or_else(
                    Default::default,
                    |data| match data.lock() {
                        Ok(attributes) => attributes.hotspot,
                        Err(poisoned) => poisoned.into_inner().hotspot,
                    },
                )
            });
            (
                CursorDraw::ClientSurface {
                    hotspot: point_of(hotspot),
                },
                Some(surface.clone()),
            )
        }
    }
}

impl SeatHandler for State {
    type KeyboardFocus = WlSurface;
    type PointerFocus = seat::Live;
    type TouchFocus = seat::Live;

    fn seat_state(&mut self) -> &mut SeatState<State> {
        &mut self.wl.seat.state
    }

    /// Keyboard focus moved: the selections and the shortcut inhibitors follow it.
    fn focus_changed(&mut self, seat: &Seat<State>, focused: Option<&WlSurface>) {
        shortcuts_inhibit::sync_focus(&self.wl.shortcuts_inhibit, focused);
        let client = focused.and_then(Resource::client);
        let display = self.display.clone();
        if let Err(error) = data_device::apply_selection_focus(&display, seat, client) {
            tracing::warn!(%error, "selection focus did not follow the keyboard");
        }
    }

    /// The focused client asked for a cursor: the part stores it and the chrome draws it.
    fn cursor_image(&mut self, _seat: &Seat<State>, image: CursorImageStatus) {
        let (draw, surface) = draw_of(&image);
        let events = seat::on_cursor_image(&mut self.wl.seat, image);
        self.render.chrome.cursor = draw;
        let textures = std::mem::take(&mut self.render.textures);
        self.render.textures = textures.with_cursor(surface);
        core_loop::feed(self, events);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_cursors_map_to_the_compositors_shapes() {
        assert_eq!(shape_of(CursorIcon::Text), CursorShape::Text);
        assert_eq!(shape_of(CursorIcon::EwResize), CursorShape::ResizeEw);
        assert_eq!(shape_of(CursorIcon::SwResize), CursorShape::ResizeNesw);
        assert_eq!(shape_of(CursorIcon::SeResize), CursorShape::ResizeNwse);
        assert_eq!(shape_of(CursorIcon::Progress), CursorShape::Wait);
        assert_eq!(shape_of(CursorIcon::NoDrop), CursorShape::NotAllowed);
        assert_eq!(shape_of(CursorIcon::Crosshair), CursorShape::Default);
        assert_eq!(shape_of(CursorIcon::Default), CursorShape::Default);
    }

    #[test]
    fn a_hidden_or_named_request_needs_no_surface() {
        let (draw, surface) = draw_of(&CursorImageStatus::Hidden);
        assert_eq!((draw, surface), (CursorDraw::Hidden, None));
        let (draw, surface) = draw_of(&CursorImageStatus::Named(CursorIcon::Pointer));
        assert_eq!(
            (draw, surface),
            (CursorDraw::Named(CursorShape::Pointer), None)
        );
    }
}
