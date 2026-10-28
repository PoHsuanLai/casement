//! Input: where each event came from (its [`Origin`], set once at entry, I8), what it is, which
//! targets it may reach, and the bindings it may fire.

mod binding;
mod chord;
mod event;
pub mod gesture_route;
mod key;
mod keysym_names;
mod lease;
pub mod lift;
mod maker;
mod origin;
mod reach;

pub use binding::{
    Argv, Binding, BindingAction, Bindings, DbusCall, DesktopCommand, SpaceStep, resolve,
};
pub use chord::Chord;
pub use event::{
    AxisFrame, AxisSource, GestureEnd, GestureStep, InputEvent, InputKind, PinchDelta, SwitchStep,
    TouchStep,
};
pub use key::{ButtonCode, KeyCode, Keysym};
pub use lease::Leases;
pub use maker::Maker;
pub use origin::{BUS_VIRTUAL, DeviceClass, FeedSource, KernelIds, Origin, PortalSessionId};
pub use reach::{InputTarget, InputUse, ReachFacts, Refusal, reach};

#[cfg(test)]
mod tests;
