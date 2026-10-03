//! casement's protocol edge: one module per Wayland protocol.
//!
//! Every module has the same shape (ARCHITECTURE.md, "Add a protocol"): a `<Protocol>Part`
//! holding smithay's state and our own tables (BTreeMaps only, row B17); `Part::new`, which
//! creates the global behind the I7 filter; `on_<request>` functions turning a request into
//! `casement_core::Event`s; and `apply_<effect>` functions carrying an `Effect` onto the wire.
//! The composition root's `handlers/<protocol>.rs` implements smithay's handler trait and calls
//! them (C28). smithay's `bool`s and loose integers become our types here or in the handler,
//! never deeper (`convert`).
//!
//! Nothing here names a device backend (row B6).

pub mod client;
pub mod convert;
pub mod serial;
pub mod states;

pub mod alpha_modifier;
pub mod background_effect;
pub mod commit_timing;
pub mod compositor;
pub mod content_type;
pub mod cursor_shape;
pub mod data_device;
pub mod dmabuf;
pub mod ext_data_control;
pub mod fifo;
pub mod foreign_toplevel_list;
pub mod fractional_scale;
pub mod idle_inhibit;
pub mod idle_notify;
pub mod input_method;
pub mod layer_shell;
pub mod output;
pub mod pointer_constraints;
pub mod pointer_gestures;
pub mod presentation;
pub mod primary_selection;
pub mod quire_window;
pub mod relative_pointer;
pub mod seat;
pub mod security_context;
pub mod session_lock;
pub mod shm;
pub mod shortcuts_inhibit;
pub mod single_pixel;
pub mod subcompositor;
pub mod system_bell;
pub mod tablet;
pub mod text_input;
pub mod viewporter;
pub mod virtual_keyboard;
pub mod wlr_data_control;
pub mod xdg_activation;
pub mod xdg_decoration;
pub mod xdg_dialog;
pub mod xdg_foreign;
pub mod xdg_shell;
pub mod xdg_toplevel_icon;
pub mod xdg_toplevel_tag;

pub use client::{ClientMint, ClientState};
pub use states::{GlobalFilter, WaylandFault, WaylandStates};
