//! The stand-in's side of the [`Compositor`] seam: clients, window positions, pointer and touch.

use super::StandIn;
use crate::adapter::{
    Compositor, Edge, FixedPoint, Flow, WlcsClient, WlcsEvent, WlcsSurface, WlcsTouchId,
};
use smithay::backend::input::{ButtonState, InputTime};
use smithay::desktop::{Window, WindowSurfaceType};
use smithay::input::pointer::{ButtonEvent, MotionEvent, RelativeMotionEvent};
use smithay::input::touch::{DownEvent, MotionEvent as TouchMotion, UpEvent};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{Client, Resource};
use smithay::utils::{Logical, Point, SERIAL_COUNTER};
use smithay::wayland::seat::WaylandFocus;
use std::sync::Arc;

use super::handlers::ClientState;

fn logical(at: FixedPoint) -> Point<f64, Logical> {
    at.to_f64().into()
}

impl Compositor for StandIn {
    fn apply(&mut self, event: WlcsEvent) -> Flow {
        match event {
            WlcsEvent::Exit => return Flow::Stop,
            WlcsEvent::NewClient { stream, client } => {
                if let Ok(inserted) = self
                    .dh
                    .insert_client(stream, Arc::new(ClientState::default()))
                {
                    self.clients.insert(client.0, inserted);
                }
            }
            WlcsEvent::PositionWindow {
                client,
                surface,
                at,
            } => {
                if let Some(window) = self.window_of(client, surface) {
                    self.space.map_element(window, (at.x, at.y), false);
                }
            }
            WlcsEvent::PointerTo(at) => self.pointer_to(logical(at)),
            WlcsEvent::PointerBy(delta) => self.pointer_by(logical(delta)),
            WlcsEvent::Button { button, edge } => self.button(button.0, edge),
            WlcsEvent::TouchDown { id, at } => self.touch_down(id, logical(at)),
            WlcsEvent::TouchMove { id, at } => self.touch_move(id, logical(at)),
            WlcsEvent::TouchUp { id } => self.touch_up(id),
        }
        Flow::Continue
    }
}

impl StandIn {
    /// One request, applied; the new flow.
    pub(super) fn serve_one(&mut self, event: WlcsEvent) -> Flow {
        self.apply(event)
    }

    fn client_of(&self, surface: &WlSurface) -> Option<Client> {
        self.dh.get_client(surface.id()).ok()
    }

    fn window_of(&self, client: WlcsClient, surface: WlcsSurface) -> Option<Window> {
        let client = self.clients.get(&client.0)?;
        self.space
            .elements()
            .find(|window| {
                window.wl_surface().is_some_and(|s| {
                    self.client_of(&s).as_ref() == Some(client) && s.id().protocol_id() == surface.0
                })
            })
            .cloned()
    }

    fn surface_under(&self, at: Point<f64, Logical>) -> Option<(WlSurface, Point<f64, Logical>)> {
        self.space.element_under(at).and_then(|(window, origin)| {
            window
                .surface_under(at - origin.to_f64(), WindowSurfaceType::ALL)
                .map(|(surface, within)| (surface, (within + origin).to_f64()))
        })
    }

    fn pointer_to(&mut self, location: Point<f64, Logical>) {
        let under = self.surface_under(location);
        let Some(pointer) = self.seat.get_pointer() else {
            return;
        };
        let (serial, time) = (SERIAL_COUNTER.next_serial(), InputTime::now());
        pointer.motion(
            self,
            under,
            &MotionEvent {
                location,
                serial,
                time,
            },
        );
        pointer.frame(self);
    }

    fn pointer_by(&mut self, delta: Point<f64, Logical>) {
        let Some(pointer) = self.seat.get_pointer() else {
            return;
        };
        let location = pointer.current_location() + delta;
        let under = self.surface_under(location);
        let (serial, time) = (SERIAL_COUNTER.next_serial(), InputTime::now());
        pointer.motion(
            self,
            under.clone(),
            &MotionEvent {
                location,
                serial,
                time,
            },
        );
        pointer.relative_motion(
            self,
            under,
            &RelativeMotionEvent {
                delta,
                delta_unaccel: delta,
                time,
            },
        );
        pointer.frame(self);
    }

    /// Click-to-focus and raise on a press, then the button.
    fn button(&mut self, button: u32, edge: Edge) {
        let Some(pointer) = self.seat.get_pointer() else {
            return;
        };
        let (serial, time) = (SERIAL_COUNTER.next_serial(), InputTime::now());
        if edge == Edge::Press && !pointer.is_grabbed() {
            let under = self
                .space
                .element_under(pointer.current_location())
                .map(|(w, _)| w.clone());
            if let Some(window) = under.as_ref() {
                self.space.raise_element(window, true);
            }
            if let Some(keyboard) = self.seat.get_keyboard() {
                let focus = under.and_then(|w| w.wl_surface().map(|s| s.into_owned()));
                keyboard.set_focus(self, focus, serial);
            }
        }
        let state = match edge {
            Edge::Press => ButtonState::Pressed,
            Edge::Release => ButtonState::Released,
        };
        pointer.button(
            self,
            &ButtonEvent {
                button,
                state,
                serial,
                time,
            },
        );
        pointer.frame(self);
    }

    fn touch_down(&mut self, id: WlcsTouchId, at: Point<f64, Logical>) {
        let under = self.surface_under(at);
        let Some(touch) = self.seat.get_touch() else {
            return;
        };
        let (serial, time) = (SERIAL_COUNTER.next_serial(), InputTime::now());
        touch.down(
            self,
            under,
            &DownEvent {
                slot: Some(id.0).into(),
                location: at,
                serial,
                time,
            },
        );
        touch.frame(self);
    }

    fn touch_move(&mut self, id: WlcsTouchId, at: Point<f64, Logical>) {
        let under = self.surface_under(at);
        let Some(touch) = self.seat.get_touch() else {
            return;
        };
        let time = InputTime::now();
        touch.motion(
            self,
            under,
            &TouchMotion {
                slot: Some(id.0).into(),
                location: at,
                time,
            },
        );
        touch.frame(self);
    }

    fn touch_up(&mut self, id: WlcsTouchId) {
        let Some(touch) = self.seat.get_touch() else {
            return;
        };
        let (serial, time) = (SERIAL_COUNTER.next_serial(), InputTime::now());
        touch.up(
            self,
            &UpEvent {
                slot: Some(id.0).into(),
                serial,
                time,
            },
        );
        touch.frame(self);
    }
}
