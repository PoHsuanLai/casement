//! casement's pure model: what the compositor knows and decides, as values.
//!
//! Every input arrives as an [`Event`]; [`Model::step`] returns the next model and the
//! [`Effect`]s the composition root carries out. Nothing here reads a clock, a device, a socket
//! or the environment: time is a [`time::Mono`] argument, hardware arrives as facts
//! ([`output::OutputFacts`]), and randomness as a seed ([`space::SpaceSeed`]).
//!
//! ```
//! use casement_core::settings::CasementSettings;
//! use casement_core::space::SpaceSeed;
//! use casement_core::time::Mono;
//! use casement_core::{Event, Model};
//! use std::time::Duration;
//!
//! let epoch = Mono(Duration::ZERO);
//! let seed = SpaceSeed::from_entropy([7; 16]);
//! let model = Model::new(CasementSettings::defaults(), epoch, seed);
//! let event = Event::SettingsChanged(Box::new(CasementSettings::defaults()));
//! let (model, _effects) = model.step(event, epoch);
//! assert_eq!(model.check_invariants(), Ok(()));
//! ```

mod cause;
mod effect;
mod error;
mod event;
mod ids;
mod invariants;
mod model;

pub mod app;
pub mod desktop;
pub mod focus;
pub mod frame;
pub mod geometry;
pub mod gpu;
pub mod input;
pub mod layer;
pub mod lock;
pub mod output;
pub mod overview;
pub mod overview_drag;
pub mod policy;
pub mod settings;
pub mod shell;
pub mod space;
pub mod time;
pub mod window;

pub use cause::{Cause, MinimiseTarget, SwipeRelease};
pub use ds_behaviour::space_swipe::PageMilli;
pub use effect::{
    Activation, ConfigureSize, ConfigureSpec, CoreNote, Deferred, Effect, GrabSpec, Held,
    KeyboardTarget, ProtocolFault, TiledEdges, ToplevelStates,
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
pub use model::{
    Model, OpeningRoom, OutputBook, OutputState, OverviewPick, SpaceStrip, SwipeOwner,
};
