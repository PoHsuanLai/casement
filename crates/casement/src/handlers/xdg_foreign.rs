//! `zxdg_exporter_v2`, `zxdg_importer_v2` (P0-H2): served by `casement_wayland::xdg_foreign`
//! alone, with no smithay handler. The one callback it needs from the root, `parent_changed`,
//! is the `XdgShellHandler` one (`handlers/xdg_shell.rs`); a destroyed toplevel's exports are
//! revoked from its `toplevel_destroyed` there.
