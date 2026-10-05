# Quirks

Behaviour facts about hardware, drivers and toolkits that casement works around, in our own
words (CONVENTIONS addendum 1). One row per fact; a fact without a test or a fixture is a
rumour and does not go here.

| Fact (keyed by driver, PCI class, connector type or toolkit) | Behaviour | Link | Test or fixture |
| --- | --- | --- | --- |
| smithay 118e34ff `renderer_pixman` with `renderer_multi` | every `PixmanRenderer` shares one static `ContextId`, so two pixman devices collapse into one `MultiTexture` slot and the cross-device copy unwraps `None` on the first frame | not filed upstream (A13); spike SP4 (`~/rs-wt/spikes/casement-sp4`, `cross plain direct`) | `casement_backend::gpu::SoftPixman` (own `ContextId`); P1-D4's soft topology suite |
| smithay 118e34ff `UdmabufAllocator::create_buffer` | the stride is width x bits per pixel (8x over-allocation): above about 1920x1080 the kernel's 64 MB `udmabuf.size_limit_mb` refuses with EINVAL and smithay silently falls back from the dma shadow copy to a CPU copy | not filed upstream (A13); SP4 `alloc` | `casement_backend::gpu::FixedUdmabuf`; boundary row A4 bans the call |
| smithay 118e34ff `ShmState` | announces `wl_shm.format` in HashSet order, so the order differs between runs and transcripts are not deterministic | not filed upstream (A13); P0-proto | `casement_wayland::shm::ShmGlobal` announces `[Argb8888, Xrgb8888]` in order; `shm::argb_and_xrgb_are_advertised` |
| smithay 118e34ff `xdg_toplevel_icon_v1` | after posting `immutable` the request handler keeps mutating the icon and trips its own `debug_assert` (`xdg_toplevel_icon.rs:149`), aborting a debug build | not filed upstream (A13); P0-proto | served in `casement_wayland::xdg_toplevel_icon`; `xdg_toplevel_icon::the_documented_icon_errors` |
| smithay 118e34ff `xdg_foreign` | never sends `zxdg_imported_v2.destroyed` when the export goes away, and draws handles from a thread rng | not filed upstream (A13); P0-proto | served in `casement_wayland::xdg_foreign`; `xdg_foreign::a_destroyed_exported_toplevel_destroys_its_imports` |
| smithay 118e34ff `xdg_shell` | destroying an `xdg_surface` before its role object posts `xdg_wm_base.role` instead of `xdg_surface.defunct_role_object`, and a second `get_toplevel` on one surface is not refused; both are decided inside smithay's request handler, with no hook | not filed upstream (A13); needs a fork patch | `xdg_shell::destruction_order_and_double_roles_are_errors` (ignored until the fork); `destroying_the_surface_before_its_role_disconnects_only_that_client` |
| smithay 118e34ff `wl_region` | `add`/`subtract` with a negative size trips a `debug_assert` in `Size::new` that a client can trigger; release builds are unaffected | not filed upstream (A13); P0-proto fuzz seeds 4 and 5 | `[profile.dev.package.smithay] debug-assertions = false` in Cargo.toml until a fork patch; the fuzz box at 24 cases |
| smithay 118e34ff `xdg_toplevel` | sends `configure_bounds(0, 0)` with the first configure (the XML allows 0x0 as unknown) | P0-proto | `tests/golden/xdg_shell-map.txt` |
