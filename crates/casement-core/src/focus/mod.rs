//! Activation, focus and raise (03 §1.2): one active app, its key and main windows, and how a
//! press is routed before any client sees it.

#[allow(
    clippy::module_inception,
    reason = "the freeze plan names the record file after its concept"
)]
mod focus;
mod press;
mod step;
mod view;

pub use focus::{Focus, FocusIn, FocusOut, TokenCheck};
pub use press::{HitPart, Press, PressRoute, route_press};
pub use step::step;
pub use view::FocusView;
