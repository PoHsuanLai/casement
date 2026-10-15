//! `wl_seat`: the one seat, its pointer, keyboard and touch. Input reaches clients only
//! through the `apply_*` functions here, after the model routed it; their focus targets are
//! surfaces the handler resolved from the model's ids.
//!
//! smithay's input handles take the whole compositor state (`&mut D`), so the `apply_*`
//! functions take the seat (a cheap handle) and the state instead of the part.

mod edge;
mod focus_origin;
mod grab;
mod hold;
mod live;
mod pointer;
mod taken;
mod touch;
mod under;

pub use grab::Drag;
pub use live::Live;
pub use pointer::{apply_axis, apply_gesture, apply_pointer, apply_refocus, end_grab, forget_dead};
pub use taken::{PressFate, Taken};
pub use touch::{apply_touch, release_dead};
pub use under::{Candidate, surface_under};

use crate::convert::serial_to;
use crate::seat_keymap::{KeymapBook, xkb_config};
use crate::serial::{PressRecord, SerialBook};
use crate::states::{GlobalFilter, WaylandFault};
use casement_core::geometry::{LogicalPoint, SubPoint};
use casement_core::input::{ButtonCode, Edge, GestureStep, KeyCode, Origin, PinchDelta};
use casement_core::settings::KeyboardSettings;
use casement_core::time::Mono;
use casement_core::{Event, InputSerial};
use smithay::input::keyboard::{FilterResult, KeyboardHandle, KeysymHandle, ModifiersState};
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
    /// What its keymap was compiled from, and what it refused.
    pub keymap: KeymapBook,
    /// Its touch.
    pub touch: TouchHandle<D>,
    /// Which serials answer a real press.
    pub serials: SerialBook,
    /// The presses taken before a client saw them (their releases are taken too).
    pub taken: Taken,
    /// The interactive move or resize in progress, if any.
    pub grab: Option<Drag>,
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
        let xkb = xkb_config(&keyboard.layout);
        let running = keyboard.layout.clone();
        let (rate, delay) = repeat_wire(keyboard);
        let keyboard = seat
            .add_keyboard(xkb, delay, rate)
            .map_err(|error| WaylandFault::Keymap(error.to_string()))?;
        let touch = seat.add_touch();
        Ok(SeatPart {
            state,
            seat,
            pointer,
            keyboard,
            keymap: KeymapBook::new(running),
            touch,
            serials: SerialBook::empty(),
            taken: Taken::none(),
            grab: None,
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

/// One key to deliver: it waits at the text-input gate (`text_input::admit_key`) and is then
/// carried to the focused client by [`apply_key`], which takes the bindings' filter.
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
    /// Where the key came from (a held key keeps it while it waits).
    pub origin: Origin,
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

/// The serial a press was delivered with is remembered, so a later move, resize, popup grab or
/// activation request can prove it answers a real press (03 section 1.2, A7).
impl<D: SeatHandler + 'static> SeatPart<D> {
    /// Remembers `press` under `serial`.
    pub fn record_press(&mut self, serial: InputSerial, press: PressRecord) {
        self.serials = std::mem::take(&mut self.serials).record(serial, press);
    }
}

/// The focused client asked for a cursor (stored in the part; the renderer's chrome reads it).
pub fn on_cursor_image(
    part: &mut SeatPart<impl SeatHandler>,
    image: CursorImageStatus,
) -> Vec<Event> {
    part.cursor = image;
    Vec::new()
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
    let keyboard = seat.get_keyboard().ok_or(WaylandFault::SurfaceGone)?;
    keyboard.set_focus(state, focus, serial_to(serial));
    Ok(())
}

/// Delivers a key to the focused client through smithay's keyboard `input`, which honours the
/// input method's grab and sends the modifiers first when the key changed them (so the order on
/// the wire is `enter`, `modifiers`, `key`). `filter` sees the modifiers and the key's symbol
/// first and may intercept it (a binding): its answer is returned, `None` when the key was
/// forwarded.
pub fn apply_key<D, T>(
    seat: &Seat<D>,
    state: &mut D,
    delivery: KeyDelivery,
    filter: impl FnOnce(&mut D, &ModifiersState, KeysymHandle<'_>) -> FilterResult<T>,
) -> Result<Option<T>, WaylandFault>
where
    D: SeatHandler + 'static,
{
    let keyboard = seat.get_keyboard().ok_or(WaylandFault::SurfaceGone)?;
    Ok(keyboard.input(
        state,
        edge::keycode_of(delivery.code),
        edge::key_state_of(delivery.edge),
        serial_to(delivery.serial),
        edge::time_of(delivery.time),
        filter,
    ))
}

/// Changes the repeat rate and delay every keyboard of the seat advertises (a settings
/// change reaches clients that are already bound).
pub fn apply_repeat_info<D>(seat: &Seat<D>, keyboard: &KeyboardSettings) -> Result<(), WaylandFault>
where
    D: SeatHandler + 'static,
{
    let handle = seat.get_keyboard().ok_or(WaylandFault::SurfaceGone)?;
    let (rate, delay) = repeat_wire(keyboard);
    handle.change_repeat_info(rate, delay);
    Ok(())
}

/// The `repeat_info` arguments the settings say: keys per second, then milliseconds before
/// the first repeat.
pub fn repeat_wire(keyboard: &KeyboardSettings) -> (i32, i32) {
    (
        i32::try_from(keyboard.repeat_rate.0).unwrap_or(i32::MAX),
        i32::try_from(keyboard.repeat_delay.as_millis()).unwrap_or(i32::MAX),
    )
}
