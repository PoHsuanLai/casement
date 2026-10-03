# Third-party code

Code ported or shaped after another project, per the clean-room rule (CONVENTIONS addendum 1):
only smithay and its MIT examples (`anvil`, `smallvil`, `wlcs_anvil`) may be read as code.

| Path | After | Licence | What was taken |
| --- | --- | --- | --- |
| `wlcs-casement/src/lib.rs`, `wlcs-casement/src/ffi.rs` | smithay's `wlcs_anvil` (Copyright (c) 2017 Victor Berger and Victoria Brekenfeld) | MIT | the shape of a wlcs integration: the `wlcs_server_integration!` ABI, a compositor thread fed through a channel |
