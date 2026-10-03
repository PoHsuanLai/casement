//! wlcs integration for casement: wlcs' server-integration ABI over casement's in-process
//! headless fixture (`casement_testkit::Fixture` with the `DummyRenderer`).
//!
//! Shape after smithay's `wlcs_anvil` example (MIT, Copyright (c) 2017 Victor Berger and
//! Victoria Brekenfeld; THIRD-PARTY.md): the ABI glue in [`ffi`], a channel from wlcs' thread to the
//! compositor's, and an adapter that turns each wlcs call into a fixture call. Nothing here is
//! linked into casement; wlcs (GPL-3.0) loads this cdylib in its own test process.
//!
//! Every body is the fill of lane P0-T-wlcs.

pub mod adapter;
mod ffi;

pub use adapter::WlcsEvent;

/// Runs the compositor thread: a [`casement_testkit::Fixture`] fed by `events` until
/// [`WlcsEvent::Exit`].
pub fn serve(events: std::sync::mpsc::Receiver<WlcsEvent>) {
    let _ = events;
    todo!("P0-T-wlcs: start the fixture and apply each WlcsEvent through the adapter")
}
