//! A text field and an input method on one connection, read on a side queue: what fcitx5 and a
//! GTK entry do together. The session binds `wl_seat`, a `wl_keyboard` (the document's own
//! keyboard: the keys it sees are the ones the input method did not take), a text input and an
//! input method; the test drives the steps and the compositor answers at the fixture's pumps.
//!
//! Every event is stamped with the virtual instant [`ImeSession::pump`] was given.

use super::input_method::{
    GrabData, InputMethodDelegate, InputMethodEvent, InputMethodSink, MethodData, PopupData,
    bind_input_method,
};
use super::text_input::{
    TextInputData, TextInputDelegate, TextInputEvent, TextInputSink, bind_text_input,
};
use crate::misc::idle::{pump_queue, side_queue};
use crate::{ClientError, GlobalList, Pumped};
use casement_core::time::Mono;
use std::time::Duration;
use wayland_client::protocol::wl_keyboard::{self, WlKeyboard};
use wayland_client::protocol::wl_seat::{self, WlSeat};
use wayland_client::{Connection, Dispatch, EventQueue, QueueHandle, delegate_dispatch};
use wayland_protocols::wp::text_input::zv3::client::{
    zwp_text_input_manager_v3::ZwpTextInputManagerV3, zwp_text_input_v3::ZwpTextInputV3,
};
use wayland_protocols_misc::zwp_input_method_v2::client::{
    zwp_input_method_keyboard_grab_v2::ZwpInputMethodKeyboardGrabV2,
    zwp_input_method_manager_v2::ZwpInputMethodManagerV2, zwp_input_method_v2::ZwpInputMethodV2,
    zwp_input_popup_surface_v2::ZwpInputPopupSurfaceV2,
};

/// Something received, and the virtual instant it was read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stamped<T> {
    /// The instant of the pump that read it.
    pub at: Mono,
    /// What it was.
    pub event: T,
}

/// A key the document's own `wl_keyboard` was sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocumentKey {
    /// The evdev key.
    pub key: u32,
    /// Pressed (`1`) or released (`0`).
    pub state: u32,
}

/// What the dispatch collects.
#[derive(Debug)]
struct Seen {
    now: Mono,
    field: Vec<Stamped<TextInputEvent>>,
    method: Vec<Stamped<InputMethodEvent>>,
    document_keys: Vec<DocumentKey>,
}

impl TextInputSink for Seen {
    fn text_input_event(&mut self, event: TextInputEvent) {
        self.field.push(Stamped {
            at: self.now,
            event,
        });
    }
}

impl InputMethodSink for Seen {
    fn input_method_event(&mut self, event: InputMethodEvent) {
        self.method.push(Stamped {
            at: self.now,
            event,
        });
    }
}

delegate_dispatch!(Seen: [ZwpTextInputManagerV3: ()] => TextInputDelegate);
delegate_dispatch!(Seen: [ZwpTextInputV3: TextInputData] => TextInputDelegate);
delegate_dispatch!(Seen: [ZwpInputMethodManagerV2: ()] => InputMethodDelegate);
delegate_dispatch!(Seen: [ZwpInputMethodV2: MethodData] => InputMethodDelegate);
delegate_dispatch!(Seen: [ZwpInputMethodKeyboardGrabV2: GrabData] => InputMethodDelegate);
delegate_dispatch!(Seen: [ZwpInputPopupSurfaceV2: PopupData] => InputMethodDelegate);

impl Dispatch<WlSeat, ()> for Seen {
    fn event(
        _state: &mut Seen,
        _proxy: &WlSeat,
        _event: wl_seat::Event,
        _data: &(),
        _connection: &Connection,
        _qh: &QueueHandle<Seen>,
    ) {
    }
}

impl Dispatch<WlKeyboard, ()> for Seen {
    fn event(
        state: &mut Seen,
        _proxy: &WlKeyboard,
        event: wl_keyboard::Event,
        _data: &(),
        _connection: &Connection,
        _qh: &QueueHandle<Seen>,
    ) {
        if let wl_keyboard::Event::Key {
            key, state: held, ..
        } = event
        {
            state.document_keys.push(DocumentKey {
                key,
                state: u32::from(held),
            });
        }
    }
}

/// A text field and an input method on one connection.
#[derive(Debug)]
pub struct ImeSession {
    connection: Connection,
    queue: EventQueue<Seen>,
    seen: Seen,
    field: ZwpTextInputV3,
    method: ZwpInputMethodV2,
    grab: Option<ZwpInputMethodKeyboardGrabV2>,
    _keyboard: WlKeyboard,
}

impl ImeSession {
    /// Binds the seat, its keyboard, a text input and an input method. The requests are
    /// flushed; the compositor sees them at the fixture's next pump.
    pub fn start(globals: &GlobalList) -> Result<ImeSession, ClientError> {
        let (connection, queue) = side_queue::<Seen>(globals)?;
        let qh = queue.handle();
        let seat = globals
            .bind::<WlSeat, _, _>(&qh, 1..=1, ())
            .map_err(|_| ClientError::MissingGlobal("wl_seat".to_owned()))?;
        let keyboard = seat.get_keyboard(&qh, ());
        let field = bind_text_input(globals, &qh, &seat)?;
        let method = bind_input_method(globals, &qh, &seat)?;
        let session = ImeSession {
            connection,
            queue,
            seen: Seen {
                now: Mono(Duration::ZERO),
                field: Vec::new(),
                method: Vec::new(),
                document_keys: Vec::new(),
            },
            field,
            method,
            grab: None,
            _keyboard: keyboard,
        };
        session.flush()?;
        Ok(session)
    }

    fn flush(&self) -> Result<(), ClientError> {
        self.connection
            .flush()
            .map_err(|error| ClientError::Dispatch(error.to_string()))
    }

    /// Reads what arrived, stamping it with `now` (the fixture's virtual instant).
    pub fn pump(&mut self, now: Mono) -> Result<Pumped, ClientError> {
        self.seen.now = now;
        pump_queue(&self.connection, &mut self.queue, &mut self.seen)
    }

    /// The field enables text input and commits the state (the compositor then activates the
    /// input method).
    pub fn enable(&self) -> Result<(), ClientError> {
        self.field.enable();
        self.field.commit();
        self.flush()
    }

    /// The input method grabs the keyboard.
    pub fn grab_keyboard(&mut self) -> Result<(), ClientError> {
        let qh = self.queue.handle();
        self.grab = Some(self.method.grab_keyboard(&qh, GrabData));
        self.flush()
    }

    /// The input method sets the preedit text.
    pub fn set_preedit(&self, text: &str, begin: i32, end: i32) -> Result<(), ClientError> {
        self.method.set_preedit_string(text.to_owned(), begin, end);
        self.flush()
    }

    /// The input method commits text.
    pub fn commit_string(&self, text: &str) -> Result<(), ClientError> {
        self.method.commit_string(text.to_owned());
        self.flush()
    }

    /// The input method commits its batch, naming the last `done` it has read (the protocol's
    /// serial: how many `done` events the method has received).
    pub fn commit_batch(&self) -> Result<(), ClientError> {
        let dones = self
            .seen
            .method
            .iter()
            .filter(|line| line.event == InputMethodEvent::Done)
            .count();
        self.method.commit(u32::try_from(dones).unwrap_or(u32::MAX));
        self.flush()
    }

    /// Everything the text field was sent, in order.
    pub fn field_events(&self) -> &[Stamped<TextInputEvent>] {
        &self.seen.field
    }

    /// Everything the input method was sent, in order.
    pub fn method_events(&self) -> &[Stamped<InputMethodEvent>] {
        &self.seen.method
    }

    /// The keys the document's own `wl_keyboard` was sent.
    pub fn document_keys(&self) -> &[DocumentKey] {
        &self.seen.document_keys
    }

    /// The field's events as transcript lines: `<seconds>.<millis> zwp_text_input_v3.<event>(<args>)`.
    pub fn field_lines(&self) -> String {
        self.seen
            .field
            .iter()
            .map(|line| {
                format!(
                    "{}.{:03} zwp_text_input_v3.{}({})\n",
                    line.at.0.as_secs(),
                    line.at.0.subsec_millis(),
                    line.event.name(),
                    line.event.args().join(", ")
                )
            })
            .collect()
    }
}
