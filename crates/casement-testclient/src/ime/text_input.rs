//! `zwp_text_input_v3`: the client side of a text field. Filled by P0-H7.
//!
//! Self-contained so it works with whatever event-queue state the shared core (P0-T-client)
//! settles on: a client state implements [`TextInputSink`], names this delegate once with
//! `wayland_client::delegate_dispatch!(State: [ZwpTextInputManagerV3: ()] => TextInputDelegate)`
//! and the same for `ZwpTextInputV3: TextInputData`, and [`bind_text_input`] creates the
//! field. Events arrive as [`TextInputEvent`]s, which print as transcript lines.

use crate::ClientError;
use crate::GlobalList;
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols::wp::text_input::zv3::client::{
    zwp_text_input_manager_v3::ZwpTextInputManagerV3,
    zwp_text_input_v3::{self, ZwpTextInputV3},
};

/// The user data of a text-input object.
#[derive(Debug, Clone, Copy, Default)]
pub struct TextInputData;

/// One event the compositor sent a text field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextInputEvent {
    /// The field's surface gained text-input focus.
    Enter,
    /// It lost it.
    Leave,
    /// The preedit text and its cursor (`begin`, `end` in bytes; `-1` hides the cursor).
    Preedit {
        /// The text, or none to clear it.
        text: Option<String>,
        /// Cursor start.
        begin: i32,
        /// Cursor end.
        end: i32,
    },
    /// Text to insert.
    CommitString(Option<String>),
    /// Surrounding text to delete, in bytes.
    DeleteSurrounding {
        /// Before the cursor.
        before: u32,
        /// After it.
        after: u32,
    },
    /// The end of one batch, with the serial of the last commit it answers.
    Done(u32),
}

impl TextInputEvent {
    /// The event's wire name.
    pub fn name(&self) -> &'static str {
        match self {
            TextInputEvent::Enter => "enter",
            TextInputEvent::Leave => "leave",
            TextInputEvent::Preedit { .. } => "preedit_string",
            TextInputEvent::CommitString(_) => "commit_string",
            TextInputEvent::DeleteSurrounding { .. } => "delete_surrounding_text",
            TextInputEvent::Done(_) => "done",
        }
    }

    /// The event's arguments as transcript text.
    pub fn args(&self) -> Vec<String> {
        let text = |text: &Option<String>| format!("{:?}", text.as_deref().unwrap_or(""));
        match self {
            TextInputEvent::Enter | TextInputEvent::Leave => Vec::new(),
            TextInputEvent::Preedit {
                text: shown,
                begin,
                end,
            } => {
                vec![text(shown), begin.to_string(), end.to_string()]
            }
            TextInputEvent::CommitString(committed) => vec![text(committed)],
            TextInputEvent::DeleteSurrounding { before, after } => {
                vec![before.to_string(), after.to_string()]
            }
            TextInputEvent::Done(serial) => vec![serial.to_string()],
        }
    }
}

/// Where a client state collects its text field's events.
pub trait TextInputSink {
    /// `event` arrived.
    fn text_input_event(&mut self, event: TextInputEvent);
}

/// The `Dispatch` impls for the manager (no events) and the field.
#[derive(Debug, Clone, Copy)]
pub struct TextInputDelegate;

impl<S> Dispatch<ZwpTextInputManagerV3, (), S> for TextInputDelegate
where
    S: Dispatch<ZwpTextInputManagerV3, ()>,
{
    fn event(
        _state: &mut S,
        _proxy: &ZwpTextInputManagerV3,
        _event: <ZwpTextInputManagerV3 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<S>,
    ) {
    }
}

impl<S> Dispatch<ZwpTextInputV3, TextInputData, S> for TextInputDelegate
where
    S: TextInputSink + Dispatch<ZwpTextInputV3, TextInputData>,
{
    fn event(
        state: &mut S,
        _proxy: &ZwpTextInputV3,
        event: zwp_text_input_v3::Event,
        _data: &TextInputData,
        _conn: &Connection,
        _qh: &QueueHandle<S>,
    ) {
        let event = match event {
            zwp_text_input_v3::Event::Enter { .. } => TextInputEvent::Enter,
            zwp_text_input_v3::Event::Leave { .. } => TextInputEvent::Leave,
            zwp_text_input_v3::Event::PreeditString {
                text,
                cursor_begin,
                cursor_end,
            } => TextInputEvent::Preedit {
                text,
                begin: cursor_begin,
                end: cursor_end,
            },
            zwp_text_input_v3::Event::CommitString { text } => TextInputEvent::CommitString(text),
            zwp_text_input_v3::Event::DeleteSurroundingText {
                before_length,
                after_length,
            } => TextInputEvent::DeleteSurrounding {
                before: before_length,
                after: after_length,
            },
            zwp_text_input_v3::Event::Done { serial } => TextInputEvent::Done(serial),
            _ => return,
        };
        state.text_input_event(event);
    }
}

/// Binds the text-input manager and creates a field for `seat`.
pub fn bind_text_input<S>(
    globals: &GlobalList,
    qh: &QueueHandle<S>,
    seat: &WlSeat,
) -> Result<ZwpTextInputV3, ClientError>
where
    S: Dispatch<ZwpTextInputManagerV3, ()> + Dispatch<ZwpTextInputV3, TextInputData> + 'static,
{
    let manager = globals
        .bind::<ZwpTextInputManagerV3, S, ()>(qh, 1..=1, ())
        .map_err(|_| ClientError::MissingGlobal("zwp_text_input_manager_v3".to_owned()))?;
    Ok(manager.get_text_input(seat, qh, TextInputData))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_print_as_transcript_arguments() {
        let preedit = TextInputEvent::Preedit {
            text: Some("ni".to_owned()),
            begin: 2,
            end: 2,
        };
        assert_eq!(preedit.name(), "preedit_string");
        assert_eq!(preedit.args(), vec!["\"ni\"", "2", "2"]);
        assert_eq!(TextInputEvent::Done(3).args(), vec!["3"]);
        assert_eq!(TextInputEvent::CommitString(None).args(), vec!["\"\""]);
    }
}
