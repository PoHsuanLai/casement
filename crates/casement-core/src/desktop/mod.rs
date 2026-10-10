//! What `quire_desktop_manager_v1` asks of the desktop and what the model says back: the
//! requests a shell makes of apps, windows and Spaces, and the notes that tell the protocol
//! layer which object to publish again (P2-Z).

mod note;
mod request;

pub use note::{DesktopNote, DesktopRef};
pub use request::{AppAsk, DesktopRequest, SpaceAsk, WindowAsk};
