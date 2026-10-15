//! The cursor session of `ext_image_copy_capture_manager_v1` as a client: where the pointer is
//! on an output, and its image as a session of its own (the frame requests of
//! [`OutputCapture`] then fetch the cursor's pixels).

use super::{Capturing, Live, OutputCapture, flush};
use crate::ClientError;
use wayland_client::protocol::wl_pointer::{self, WlPointer};
use wayland_client::protocol::wl_seat::{self, WlSeat};
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_cursor_session_v1::{
    self, ExtImageCopyCaptureCursorSessionV1,
};

/// What a cursor session said about the cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CursorEvent {
    /// The cursor's image met the captured frame.
    Entered,
    /// It left the frame.
    Left,
    /// The hotspot is at this pixel of the frame.
    Position(i32, i32),
    /// The hotspot is this far from the image's corner.
    Hotspot(i32, i32),
}

impl OutputCapture {
    /// Makes a source of the output named `connector` and a cursor session on it, and the
    /// session that captures the cursor's image. The seat's pointer is the one the cursor
    /// belongs to, so the seat must have one.
    ///
    /// ```no_run
    /// use casement_testclient::GlobalList;
    /// use casement_testclient::capture::OutputCapture;
    ///
    /// # fn demo(globals: &GlobalList) -> Result<(), casement_testclient::ClientError> {
    /// let mut capture = OutputCapture::start(globals)?;
    /// capture.open_cursor("HEADLESS-1")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn open_cursor(&mut self, connector: &str) -> Result<(), ClientError> {
        let output = self
            .state
            .outputs
            .iter()
            .find(|output| output.name.as_deref() == Some(connector))
            .map(|output| output.output.clone())
            .ok_or_else(|| ClientError::MissingGlobal(format!("wl_output {connector}")))?;
        let seat = self
            .seat
            .clone()
            .ok_or_else(|| ClientError::MissingGlobal("wl_seat".to_owned()))?;
        let qh = self.queue.handle();
        let pointer = seat.get_pointer(&qh, ());
        let source = self.sources.create_source(&output, &qh, ());
        let cursor = self
            .copies
            .create_pointer_cursor_session(&source, &pointer, &qh, ());
        let session = cursor.get_capture_session(&qh, ());
        self.live = Some(Live {
            source,
            cursor: Some(cursor),
            session,
            frame: None,
        });
        flush(&self.connection)
    }

    /// What the cursor session said since the last call, oldest first.
    pub fn take_cursor_events(&mut self) -> Vec<CursorEvent> {
        std::mem::take(&mut self.state.cursor_events)
    }
}

impl Dispatch<ExtImageCopyCaptureCursorSessionV1, ()> for Capturing {
    fn event(
        state: &mut Self,
        _: &ExtImageCopyCaptureCursorSessionV1,
        event: ext_image_copy_capture_cursor_session_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        use ext_image_copy_capture_cursor_session_v1::Event;
        match event {
            Event::Enter => state.cursor_events.push(CursorEvent::Entered),
            Event::Leave => state.cursor_events.push(CursorEvent::Left),
            Event::Position { x, y } => state.cursor_events.push(CursorEvent::Position(x, y)),
            Event::Hotspot { x, y } => state.cursor_events.push(CursorEvent::Hotspot(x, y)),
            _ => {}
        }
    }
}

impl Dispatch<WlSeat, ()> for Capturing {
    fn event(
        _: &mut Self,
        _: &WlSeat,
        _: wl_seat::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlPointer, ()> for Capturing {
    fn event(
        _: &mut Self,
        _: &WlPointer,
        _: wl_pointer::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
