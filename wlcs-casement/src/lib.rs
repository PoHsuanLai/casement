//! wlcs integration for casement: wlcs' server-integration ABI over a [`Compositor`] seam.
//!
//! Shape after smithay's `wlcs_anvil` example (MIT, Copyright (c) 2017 Victor Berger and
//! Victoria Brekenfeld; THIRD-PARTY.md): the ABI glue in `ffi`, a channel from wlcs' thread to
//! the compositor's, and a seam that turns each wlcs call into a compositor call. Nothing here is
//! linked into casement; wlcs (GPL-3.0) loads this cdylib in its own test process.
//!
//! The compositor behind the seam is [`stand_in`], a small smithay compositor, until casement's
//! `Fixture` can adopt a raw client socket and a model-level pointer (interface asks in the lane
//! report); the stand-in proves the shim, the wlcs build and the subsets today.

pub mod adapter;
mod descriptor;
mod ffi;
pub mod stand_in;

pub use adapter::{Compositor, Flow, WlcsEvent};
use smithay::reexports::calloop::channel::Channel;

/// Runs the compositor thread: the stand-in fed by `events` until [`WlcsEvent::Exit`].
pub fn serve(events: Channel<WlcsEvent>) {
    stand_in::run(events);
}
