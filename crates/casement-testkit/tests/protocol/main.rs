//! One file per protocol module (CONVENTIONS addendum 11, `scripts/check-protocol-coverage.sh`):
//! bind every version, happy paths, every protocol error, destruction order, a transcript
//! golden (11 §3.2). Every test drives the real `State` through the `Fixture`; each is
//! ignored with the lane that fills it.

mod alpha_modifier;
mod background_effect;
mod commit_timing;
mod compositor;
mod content_type;
mod cursor_shape;
mod data_device;
mod dmabuf;
mod ext_data_control;
mod fifo;
mod foreign_toplevel_list;
mod fractional_scale;
mod idle_inhibit;
mod idle_notify;
mod input_method;
mod layer_shell;
mod output;
mod pointer_constraints;
mod pointer_gestures;
mod presentation;
mod primary_selection;
mod quire_window;
mod relative_pointer;
mod seat;
mod security_context;
mod session_lock;
mod shm;
mod shoot;
mod shortcuts_inhibit;
mod single_pixel;
mod subcompositor;
mod system_bell;
mod tablet;
mod text_input;
mod viewporter;
mod virtual_keyboard;
mod wlr_data_control;
mod xdg_activation;
mod xdg_decoration;
mod xdg_dialog;
mod xdg_foreign;
mod xdg_grab;
mod xdg_shell;
mod xdg_toplevel_icon;
mod xdg_toplevel_tag;
