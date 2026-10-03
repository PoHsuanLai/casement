//! `text_input` (P0-H7): smithay has no handler trait for it, and text-input focus follows
//! keyboard focus inside smithay's own `KeyboardTarget` for surfaces; the reconciliation and
//! the activation gate's functions are `casement_wayland::text_input`'s, called by `exec`.
