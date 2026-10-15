//! Output capture as a client: `ext_output_image_capture_source_manager_v1` makes a source from
//! a `wl_output`, `ext_image_copy_capture_manager_v1` a session on it and a frame into a shared
//! memory buffer. One [`OutputCapture`] is one session, on its own queue next to the client's.
//!
//! The events are folded by [`desktop_client`]'s reducer and the frame is its [`Frame`], so the
//! test client and `casementctl shot` read a capture the same way (`Frame::to_rgba8`). The
//! wire is not `desktop_client::client::DesktopClient`: that client waits for the compositor
//! with blocking round trips, which a fixture serving the compositor on the same thread can
//! never answer (FINDINGS, p1-c1).

pub mod copy_session;
pub mod output_source;
pub mod screencopy;
pub mod toplevel_source;

pub use copy_session::{BufferAsk, CursorEvent, DmabufFormat, OutputCapture};
pub use desktop_client::capture::{CaptureFailure, CursorMode};
pub use desktop_client::{Frame, Size};
pub use screencopy::{Copied, CopyArea, CopyFailure, CopyKind, DmabufOffer, Offer, ScreenCopy};
