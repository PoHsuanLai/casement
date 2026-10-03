//! `wl_seat`: the one seat, its pointer, keyboard and touch. Input reaches clients only
//! through the `apply_*` functions here, after the model routed it; their focus targets are
//! surfaces the handler resolved from the model's ids.
//!
//! smithay's input handles take the whole compositor state (`&mut D`), so the `apply_*`
//! functions take the seat (a cheap handle) and the state instead of the part.

use crate::serial::SerialBook;
use crate::states::{GlobalFilter, WaylandFault};
use casement_core::geometry::{LogicalPoint, SubPoint};
use casement_core::input::{
    AxisFrame, ButtonCode, Edge, GestureStep, KeyCode, PinchDelta, TouchStep,
};
use casement_core::settings::KeyboardSettings;
use casement_core::time::Mono;
use casement_core::{Event, InputSerial};
use smithay::input::keyboard::{KeyboardHandle, XkbConfig};
use smithay::input::pointer::{CursorImageStatus, PointerHandle};
use smithay::input::touch::TouchHandle;
use smithay::input::{Seat, SeatHandler, SeatState};
use smithay::reexports::wayland_server::protocol::wl_seat::WlSeat;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{DisplayHandle, GlobalDispatch};
use smithay::wayland::seat::{SeatGlobalData, WaylandFocus};

/// The seat's name on the wire.
pub const SEAT_NAME: &str = "seat0";

/// `wl_seat` and its devices.
#[derive(Debug)]
pub struct SeatPart<D: SeatHandler + 'static> {
    /// smithay's state.
    pub state: SeatState<D>,
    /// The one seat.
    pub seat: Seat<D>,
    /// Its pointer.
    pub pointer: PointerHandle<D>,
    /// Its keyboard.
    pub keyboard: KeyboardHandle<D>,
    /// Its touch.
    pub touch: TouchHandle<D>,
    /// Which serials answer a real press.
    pub serials: SerialBook,
    /// The cursor the focused client asked for.
    pub cursor: CursorImageStatus,
}

impl<D: SeatHandler + 'static> SeatPart<D> {
    /// Creates `seat0` (version 9) with a pointer, a keyboard compiled from `keyboard`'s XKB
    /// names and repeat settings, and touch. Every connection sees it.
    pub fn new(
        display: &DisplayHandle,
        role_filter: GlobalFilter,
        keyboard: &KeyboardSettings,
    ) -> Result<SeatPart<D>, WaylandFault>
    where
        D: GlobalDispatch<WlSeat, SeatGlobalData<D>>,
        <D as SeatHandler>::PointerFocus: WaylandFocus,
        <D as SeatHandler>::KeyboardFocus: WaylandFocus,
    {
        let _ = role_filter;
        let mut state = SeatState::new();
        let mut seat = state.new_wl_seat(display, SEAT_NAME);
        let pointer = seat.add_pointer();
        let names = &keyboard.layout;
        let xkb = XkbConfig {
            rules: &names.rules,
            model: &names.model,
            layout: &names.layout,
            variant: &names.variant,
            options: Some(names.options.clone()).filter(|options| !options.is_empty()),
        };
        let delay = i32::try_from(keyboard.repeat_delay.as_millis()).unwrap_or(i32::MAX);
        let rate = i32::try_from(keyboard.repeat_rate.0).unwrap_or(i32::MAX);
        let keyboard = seat
            .add_keyboard(xkb, delay, rate)
            .map_err(|error| WaylandFault::Keymap(error.to_string()))?;
        let touch = seat.add_touch();
        Ok(SeatPart {
            state,
            seat,
            pointer,
            keyboard,
            touch,
            serials: SerialBook::empty(),
            cursor: CursorImageStatus::default_named(),
        })
    }
}

/// A surface and where its top-left is, in global logical coordinates: a pointer or touch
/// focus.
#[derive(Debug, Clone, PartialEq)]
pub struct SurfaceAt {
    /// The surface.
    pub surface: WlSurface,
    /// Its top-left.
    pub origin: LogicalPoint,
}

/// One pointer delivery the model routed.
#[derive(Debug, Clone, PartialEq)]
pub enum PointerDelivery {
    /// The pointer is at `at` over `focus`.
    Motion {
        /// The surface under it, if any.
        focus: Option<SurfaceAt>,
        /// Where, in global sub-pixels.
        at: SubPoint,
        /// The relative movement that got it there (relative-pointer clients).
        delta: SubPoint,
        /// When.
        time: Mono,
    },
    /// A button.
    Button {
        /// Which.
        button: ButtonCode,
        /// Down or up.
        edge: Edge,
        /// The serial the press is known by.
        serial: InputSerial,
        /// When.
        time: Mono,
    },
}

/// One key delivery the model routed (bindings already taken out).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyDelivery {
    /// The key.
    pub code: KeyCode,
    /// Down or up.
    pub edge: Edge,
    /// The serial.
    pub serial: InputSerial,
    /// When.
    pub time: Mono,
}

/// One gesture step the model routed to the focused client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GestureDelivery {
    /// A swipe step.
    Swipe(GestureStep<SubPoint>),
    /// A pinch step.
    Pinch(GestureStep<PinchDelta>),
    /// A hold step.
    Hold(GestureStep<()>),
}

/// The focused client asked for a cursor (stored in the part; the renderer's chrome reads it).
pub fn on_cursor_image(
    part: &mut SeatPart<impl SeatHandler>,
    image: CursorImageStatus,
) -> Vec<Event> {
    let _ = (part, image);
    todo!("P0-H4: seat::on_cursor_image")
}

/// Moves keyboard focus to `focus` (nothing when `None`), sending the current modifiers with
/// the enter.
pub fn apply_keyboard_focus<D>(
    seat: &Seat<D>,
    state: &mut D,
    focus: Option<D::KeyboardFocus>,
    serial: InputSerial,
) -> Result<(), WaylandFault>
where
    D: SeatHandler + 'static,
{
    let _ = (seat, state, focus, serial);
    todo!("P0-H4: seat::apply_keyboard_focus")
}

/// Delivers a pointer motion or button.
pub fn apply_pointer<D>(
    seat: &Seat<D>,
    state: &mut D,
    delivery: PointerDelivery,
) -> Result<(), WaylandFault>
where
    D: SeatHandler + 'static,
{
    let _ = (seat, state, delivery);
    todo!("P0-H4: seat::apply_pointer")
}

/// Delivers a key to the focused client.
pub fn apply_key<D>(
    seat: &Seat<D>,
    state: &mut D,
    delivery: KeyDelivery,
) -> Result<(), WaylandFault>
where
    D: SeatHandler + 'static,
{
    let _ = (seat, state, delivery);
    todo!("P0-H4: seat::apply_key")
}

/// Delivers one scroll frame (`axis_source` and `axis_value120` always sent: SP1 B1).
pub fn apply_axis<D>(
    seat: &Seat<D>,
    state: &mut D,
    frame: AxisFrame,
    time: Mono,
) -> Result<(), WaylandFault>
where
    D: SeatHandler + 'static,
{
    let _ = (seat, state, frame, time);
    todo!("P0-H4: seat::apply_axis")
}

/// Delivers a gesture step to the client under the pointer.
pub fn apply_gesture<D>(
    seat: &Seat<D>,
    state: &mut D,
    gesture: GestureDelivery,
    serial: InputSerial,
    time: Mono,
) -> Result<(), WaylandFault>
where
    D: SeatHandler + 'static,
{
    let _ = (seat, state, gesture, serial, time);
    todo!("P0-H4: seat::apply_gesture")
}

/// Delivers a touch step over `focus`.
pub fn apply_touch<D>(
    seat: &Seat<D>,
    state: &mut D,
    step: TouchStep,
    focus: Option<SurfaceAt>,
    serial: InputSerial,
    time: Mono,
) -> Result<(), WaylandFault>
where
    D: SeatHandler + 'static,
{
    let _ = (seat, state, step, focus, serial, time);
    todo!("P0-H4: seat::apply_touch")
}
