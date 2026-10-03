//! Input: where each event came from (its [`Origin`], set once at entry, I8), what it is, which
//! targets it may reach, and the bindings it may fire.

mod binding;
mod chord;
mod event;
mod key;
mod keysym_names;
mod origin;
mod reach;

pub use binding::{
    Argv, Binding, BindingAction, Bindings, DbusCall, DesktopCommand, Resolved, SpaceStep, resolve,
};
pub use chord::Chord;
pub use event::{
    AxisFrame, AxisSource, AxisStop, Edge, GestureEnd, GestureStep, InputEvent, InputKind,
    LidState, PinchDelta, SwitchStep, TabletState, TouchStep,
};
pub use key::{ButtonCode, KeyCode, Keysym};
pub use origin::{DeviceClass, FeedSource, Origin, PortalSessionId};
pub use reach::{InputTarget, InputUse, Reach, ReachFacts, Refusal, reach};

#[cfg(test)]
mod tests;
