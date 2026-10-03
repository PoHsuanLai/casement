//! Helpers for the `misc` protocols. One module per protocol; the lane named on each fills it.

pub mod activation;
pub mod background_effect;
pub mod commit_timing;
pub mod content_type;
pub mod decoration;
pub mod dmabuf_feedback;
pub mod fifo;
pub mod fractional;
pub mod idle;
pub mod idle_inhibit;
pub mod presentation;
pub mod security_context;
pub mod syncobj;
pub mod viewporter;
