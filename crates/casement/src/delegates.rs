//! smithay's dispatch for every global (FROZEN; each delta freeze adds what its globals need).
//!
//! At the pinned smithay (118e34ff) one `delegate_dispatch2!` line replaces the per-protocol
//! `delegate_*!` macros: every global's and object's user data implements smithay's
//! `GlobalDispatch2`/`Dispatch2` (ours too: `casement_wayland::quire_window`), and this line
//! routes `State`'s `Dispatch`/`GlobalDispatch` to them. What each protocol needs from `State`
//! is its handler trait, implemented in `handlers/<protocol>.rs`.

smithay::delegate_dispatch2!(crate::state::State);
