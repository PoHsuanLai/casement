//! wlcs integration for casement: wlcs' server-integration ABI over a [`Compositor`] seam.
//!
//! Shape after smithay's `wlcs_anvil` example (MIT, Copyright (c) 2017 Victor Berger and
//! Victoria Brekenfeld; THIRD-PARTY.md): the ABI glue in `ffi`, a channel from wlcs' thread to
//! the compositor's, and a seam that turns each wlcs call into a compositor call. Nothing here is
//! linked into casement; wlcs (GPL-3.0) loads this cdylib in its own test process.
//!
//! The compositor behind the seam is casement itself: [`host::Host`] runs the test kit's
//! `Fixture` (the real `State` on the headless backend) on the compositor thread.

pub mod adapter;
mod descriptor;
mod ffi;
pub mod host;

pub use adapter::{Compositor, Flow, WlcsEvent};
