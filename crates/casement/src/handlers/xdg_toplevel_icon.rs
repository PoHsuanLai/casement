//! `xdg_toplevel_icon_manager_v1` (P0-H2): served by `casement_wayland::xdg_toplevel_icon`
//! alone, with no smithay handler. The one callback it needs, a buffer's destruction, arrives
//! through `BufferHandler` (`handlers/shm.rs`).
