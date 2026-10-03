//! casement's pure model: what the compositor knows and decides, as values.
//!
//! Every input arrives as an [`Event`]; [`Model::step`] returns the next model and the
//! [`Effect`]s the composition root carries out. Nothing here reads a clock, a device, a socket
//! or the environment: time is a [`time::Mono`] argument, hardware arrives as facts
//! ([`output::OutputFacts`]), and randomness as a seed ([`space::SpaceSeed`]).

mod cause;
mod effect;
mod error;
mod event;
mod ids;
mod invariants;
mod model;

pub mod app;
pub mod focus;
pub mod geometry;
pub mod input;
pub mod layer;
pub mod lock;
pub mod output;
pub mod policy;
pub mod settings;
pub mod space;
pub mod time;
pub mod window;

pub use cause::{Cause, MinimiseTarget, SwipeRelease};
pub use effect::{
    Activation, ConfigureSize, ConfigureSpec, CoreNote, Effect, GrabSpec, Held, KeyboardTarget,
    ProtocolFault, TiledEdges, ToplevelStates,
};
pub use error::CoreError;
pub use event::{
    Event, MappedToplevel, ResizeEdges, ToplevelMeta, ToplevelRequest, WindowHintChange,
};
pub use ids::{
    ClientKey, DeviceId, IdMint, InputSerial, LayerKey, LeaseId, OutputKey, OutputName, Pid,
    PopupKey, WindowId,
};
pub use invariants::InvariantBreak;
pub use model::{Model, OutputBook, OutputState};
