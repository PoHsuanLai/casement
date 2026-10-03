//! `zwp_input_method_v2`: the client side of an input method (what fcitx5 is). Filled by
//! P0-H7. Self-contained like [`super::text_input`]: a client state implements
//! [`InputMethodSink`] and names [`InputMethodDelegate`] for the manager, the method, its
//! keyboard grab and its popup surface with `wayland_client::delegate_dispatch!`.

use crate::ClientError;
use wayland_client::globals::GlobalList;
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols_misc::zwp_input_method_v2::client::{
    zwp_input_method_keyboard_grab_v2::{self, ZwpInputMethodKeyboardGrabV2},
    zwp_input_method_manager_v2::ZwpInputMethodManagerV2,
    zwp_input_method_v2::{self, ZwpInputMethodV2},
    zwp_input_popup_surface_v2::{self, ZwpInputPopupSurfaceV2},
};

/// The user data of the method object.
#[derive(Debug, Clone, Copy, Default)]
pub struct MethodData;

/// The user data of its keyboard grab.
#[derive(Debug, Clone, Copy, Default)]
pub struct GrabData;

/// The user data of its popup surface.
#[derive(Debug, Clone, Copy, Default)]
pub struct PopupData;

/// One event the compositor sent an input method.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputMethodEvent {
    /// A field was enabled: the method may compose.
    Activate,
    /// The field is gone.
    Deactivate,
    /// The field's text around the cursor.
    SurroundingText {
        /// The text.
        text: String,
        /// Cursor byte offset.
        cursor: u32,
        /// Anchor byte offset.
        anchor: u32,
    },
    /// The end of one batch of state.
    Done,
    /// Another method holds the seat.
    Unavailable,
    /// The grabbed keyboard sent a key.
    GrabKey {
        /// The key (evdev code).
        key: u32,
        /// Pressed (`1`) or released (`0`).
        state: u32,
    },
    /// The grabbed keyboard's modifier state.
    GrabModifiers {
        /// Depressed mask.
        depressed: u32,
        /// Latched mask.
        latched: u32,
        /// Locked mask.
        locked: u32,
    },
    /// The grab began with a keymap of this many bytes.
    GrabKeymap {
        /// The keymap's size.
        size: u32,
    },
    /// The popup surface's text-input rectangle moved.
    PopupRectangle {
        /// X.
        x: i32,
        /// Y.
        y: i32,
        /// Width.
        width: i32,
        /// Height.
        height: i32,
    },
}

impl InputMethodEvent {
    /// The event's wire name.
    pub fn name(&self) -> &'static str {
        match self {
            InputMethodEvent::Activate => "activate",
            InputMethodEvent::Deactivate => "deactivate",
            InputMethodEvent::SurroundingText { .. } => "surrounding_text",
            InputMethodEvent::Done => "done",
            InputMethodEvent::Unavailable => "unavailable",
            InputMethodEvent::GrabKey { .. } => "key",
            InputMethodEvent::GrabModifiers { .. } => "modifiers",
            InputMethodEvent::GrabKeymap { .. } => "keymap",
            InputMethodEvent::PopupRectangle { .. } => "text_input_rectangle",
        }
    }
}

/// Where a client state collects its input method's events.
pub trait InputMethodSink {
    /// `event` arrived.
    fn input_method_event(&mut self, event: InputMethodEvent);
}

/// The `Dispatch` impls for the manager, the method, its grab and its popup surface.
#[derive(Debug, Clone, Copy)]
pub struct InputMethodDelegate;

impl<S> Dispatch<ZwpInputMethodManagerV2, (), S> for InputMethodDelegate
where
    S: Dispatch<ZwpInputMethodManagerV2, ()>,
{
    fn event(
        _state: &mut S,
        _proxy: &ZwpInputMethodManagerV2,
        _event: <ZwpInputMethodManagerV2 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<S>,
    ) {
    }
}

impl<S> Dispatch<ZwpInputMethodV2, MethodData, S> for InputMethodDelegate
where
    S: InputMethodSink + Dispatch<ZwpInputMethodV2, MethodData>,
{
    fn event(
        state: &mut S,
        _proxy: &ZwpInputMethodV2,
        event: zwp_input_method_v2::Event,
        _data: &MethodData,
        _conn: &Connection,
        _qh: &QueueHandle<S>,
    ) {
        let event = match event {
            zwp_input_method_v2::Event::Activate => InputMethodEvent::Activate,
            zwp_input_method_v2::Event::Deactivate => InputMethodEvent::Deactivate,
            zwp_input_method_v2::Event::SurroundingText {
                text,
                cursor,
                anchor,
            } => InputMethodEvent::SurroundingText {
                text,
                cursor,
                anchor,
            },
            zwp_input_method_v2::Event::Done => InputMethodEvent::Done,
            zwp_input_method_v2::Event::Unavailable => InputMethodEvent::Unavailable,
            _ => return,
        };
        state.input_method_event(event);
    }
}

impl<S> Dispatch<ZwpInputMethodKeyboardGrabV2, GrabData, S> for InputMethodDelegate
where
    S: InputMethodSink + Dispatch<ZwpInputMethodKeyboardGrabV2, GrabData>,
{
    fn event(
        state: &mut S,
        _proxy: &ZwpInputMethodKeyboardGrabV2,
        event: zwp_input_method_keyboard_grab_v2::Event,
        _data: &GrabData,
        _conn: &Connection,
        _qh: &QueueHandle<S>,
    ) {
        let event = match event {
            zwp_input_method_keyboard_grab_v2::Event::Keymap { size, .. } => {
                InputMethodEvent::GrabKeymap { size }
            }
            zwp_input_method_keyboard_grab_v2::Event::Key { key, state, .. } => {
                InputMethodEvent::GrabKey {
                    key,
                    state: u32::from(state),
                }
            }
            zwp_input_method_keyboard_grab_v2::Event::Modifiers {
                mods_depressed,
                mods_latched,
                mods_locked,
                ..
            } => InputMethodEvent::GrabModifiers {
                depressed: mods_depressed,
                latched: mods_latched,
                locked: mods_locked,
            },
            _ => return,
        };
        state.input_method_event(event);
    }
}

impl<S> Dispatch<ZwpInputPopupSurfaceV2, PopupData, S> for InputMethodDelegate
where
    S: InputMethodSink + Dispatch<ZwpInputPopupSurfaceV2, PopupData>,
{
    fn event(
        state: &mut S,
        _proxy: &ZwpInputPopupSurfaceV2,
        event: zwp_input_popup_surface_v2::Event,
        _data: &PopupData,
        _conn: &Connection,
        _qh: &QueueHandle<S>,
    ) {
        let zwp_input_popup_surface_v2::Event::TextInputRectangle {
            x,
            y,
            width,
            height,
        } = event
        else {
            return;
        };
        state.input_method_event(InputMethodEvent::PopupRectangle {
            x,
            y,
            width,
            height,
        });
    }
}

/// Binds the input-method manager and asks for the method of `seat`.
pub fn bind_input_method<S>(
    globals: &GlobalList,
    qh: &QueueHandle<S>,
    seat: &WlSeat,
) -> Result<ZwpInputMethodV2, ClientError>
where
    S: Dispatch<ZwpInputMethodManagerV2, ()> + Dispatch<ZwpInputMethodV2, MethodData> + 'static,
{
    let manager = globals
        .bind::<ZwpInputMethodManagerV2, S, ()>(qh, 1..=1, ())
        .map_err(|_| ClientError::MissingGlobal("zwp_input_method_manager_v2".to_owned()))?;
    Ok(manager.get_input_method(seat, qh, MethodData))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_name_themselves_like_the_wire() {
        assert_eq!(InputMethodEvent::Activate.name(), "activate");
        assert_eq!(
            InputMethodEvent::GrabKey { key: 30, state: 1 }.name(),
            "key"
        );
    }
}
