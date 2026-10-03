//! Windows: role, place, state, parent, emphasis, limits, title and hints (03 §1.1).

mod hints;
mod limits;
mod state;
mod title;
#[allow(
    clippy::module_inception,
    reason = "the freeze plan names the record file after its concept"
)]
mod window;

pub use hints::{A11yRoot, FirstMouse, FrameHint, WindowHints};
pub use limits::{MaxSize, SizeLimits};
pub use state::{Restore, WindowState};
pub use title::WindowTitle;
pub use window::{Parent, SpacePlace, Window};
