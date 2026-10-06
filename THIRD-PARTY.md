# Third-party code

Code ported or shaped after another project, per the clean-room rule (CONVENTIONS addendum 1):
only smithay and its MIT examples (`anvil`, `smallvil`, `wlcs_anvil`) may be read as code.

| Path | After | Licence | What was taken |
| --- | --- | --- | --- |
| `wlcs-casement/src/lib.rs`, `wlcs-casement/src/ffi.rs` | smithay's `wlcs_anvil` (Copyright (c) 2017 Victor Berger and Victoria Brekenfeld) | MIT | the shape of a wlcs integration: the `wlcs_server_integration!` ABI, a compositor thread fed through a channel |
| `crates/casement/src/handlers/data_device.rs` (the drag grab in `dnd_requested`) | smithay's `anvil` (Copyright (c) 2017 Victor Berger and Victoria Brekenfeld) | MIT | how a client's `start_drag` becomes a `DnDGrab` on the pointer or touch with the seat's focus delivery; the surrounding handler traits are written for casement's `State` |
| `crates/casement/src/relax.rs` | smithay's `anvil` and `smallvil` (Copyright (c) 2017 Victor Berger and Victoria Brekenfeld) | MIT | only the commit-hook plumbing (`add_pre_commit_hook` on the surface's creation); the ack-before-attach relaxation itself (A37) is casement's own |
