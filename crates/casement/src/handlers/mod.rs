//! `impl <Smithay>Handler for State`, one file per protocol module (C28): each method converts
//! its arguments and calls `casement_wayland::<protocol>::on_*`, then `core_loop::feed`. The
//! handler lanes own their files; this list changes only in a freeze.

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
mod gamma;
mod idle_inhibit;
mod idle_notify;
mod image_capture;
mod input_method;
mod layer_shell;
mod output;
mod output_management;
mod output_power;
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
mod shortcuts_inhibit;
mod single_pixel;
mod subcompositor;
mod system_bell;
mod tablet;
mod text_input;
mod viewporter;
mod virtual_keyboard;
mod wlr_data_control;
mod wlr_toplevel;
mod workspace;
mod xdg_activation;
mod xdg_decoration;
mod xdg_dialog;
mod xdg_foreign;
mod xdg_shell;
pub(crate) use xdg_shell::popups::{
    dismiss_for_focus as dismiss_popups_for_focus, refit_reactive as refit_reactive_popups,
};
mod xdg_toplevel_icon;
mod xdg_toplevel_tag;
