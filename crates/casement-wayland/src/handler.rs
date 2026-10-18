//! The two things most handler traits ask of the composition root alike: feed the events a
//! request produced into the model, and name the model's output for a client's `wl_output`.
//! A protocol's handler trait extends the one it needs instead of declaring its own copy.

use casement_core::{Event, OutputKey};
use smithay::reexports::wayland_server::protocol::wl_output::WlOutput;

/// Where the events a request produced go: into the model (`core_loop::feed`). The model may
/// resolve a use before this returns.
pub trait EventSink {
    /// Feeds `events` into the model.
    fn feed(&mut self, events: Vec<Event>);
}

/// The model's key for an output a client names.
pub trait OutputLookup {
    /// The output a client's `wl_output` is, when it is one casement advertises.
    fn output_key_of(&self, output: &WlOutput) -> Option<OutputKey>;
}
