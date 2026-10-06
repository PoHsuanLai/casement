# Findings

Open items and standing facts. An entry names the condition that closes it.

## Stubs behind frozen interfaces

Every `todo!()` in the workspace's sources (270, all from P0-Z2 below), its fill lane and the
ignored test that proves it. Un-ignoring the test is part of the fill. The 27 the P0-Z1
freeze left in casement-core, casement-layout and casement-motion are filled (the P0-F1 lanes
and P0-F1b-model).

### P0-Z1: the pure crates

No stub is left. Ignored tests by lane: P3-Z 1 (`crates/casement-layout/tests/spread.rs`, the
Mission Control spread's properties from SP8, for the P3 freeze to inherit).

P0-F1-ids had no stub: ids, geometry and time are implemented at the freeze, with their
tables (the clock's ds-core conformance cases, stop order, chunk invariance, ten minutes
without drift, `buffer_size` at 1, 1.25, 1.5 and 2).

### P0-Z2: the smithay-facing crates, the root, the test kit

`todo!()` by lane (267): P0-R-scene 23, P0-H1 18, P0-H2 51, P0-H3 15, P0-H4 11, P0-H5 6, P0-H6 3, P0-H7 12, P0-H8 12, P0-B-headless 79, P0-B-loop 14, P0-T-client 6, P0-T-dev 11, P1-D1 1, P1-F2 2, P2-Q2 2, P3-Z 1. Seventy-six more sit in ignored test bodies (the protocol
tests' client steps, the scene builder, the fuzz box), each naming the same lane: the fixture
setup and the assertion are written, the client steps wait for the helper the lane writes in
`casement-testclient`. The excluded crates add 20 (`wlcs-casement`: 16 for P0-T-wlcs in
`src/lib.rs`, `src/adapter.rs` and `src/ffi.rs`, checked to compile; `casement-scene-css`: 4 for
P3-CSS, three in `src/` and its ignored parity test, never built at P0-Z2).

| File | Items | Lane | Proven by |
| --- | --- | --- | --- |
| `crates/casement-wayland/src/commit_timing.rs` | on_barrier, apply_presented (2) | P0-H1 | `crates/casement-testkit/tests/protocol/commit_timing.rs` |
| `crates/casement-wayland/src/compositor.rs` | on_commit, on_destroyed (2) | P0-H1 | `crates/casement-testkit/tests/protocol/compositor.rs` |
| `crates/casement-wayland/src/content_type.rs` | on_content_type (1) | P0-H1 | `crates/casement-testkit/tests/protocol/content_type.rs` |
| `crates/casement-wayland/src/fifo.rs` | on_barrier, apply_presented (2) | P0-H1 | `crates/casement-testkit/tests/protocol/fifo.rs` |
| `crates/casement-wayland/src/fractional_scale.rs` | on_preferred, apply_scale (2) | P0-H1 | `crates/casement-testkit/tests/protocol/fractional_scale.rs` |
| `crates/casement-wayland/src/output.rs` | apply_plug, apply_change, apply_unplug (3) | P0-H1 | `crates/casement-testkit/tests/protocol/output.rs` |
| `crates/casement-wayland/src/presentation.rs` | on_feedback, apply_presented (2) | P0-H1 | `crates/casement-testkit/tests/protocol/presentation.rs` |
| `crates/casement-wayland/src/shm.rs` | on_buffer_destroyed (1) | P0-H1 | `crates/casement-testkit/tests/protocol/shm.rs` |
| `crates/casement-wayland/src/subcompositor.rs` | on_new_subsurface (1) | P0-H1 | `crates/casement-testkit/tests/protocol/subcompositor.rs` |
| `crates/casement/src/handlers/compositor.rs` | CompositorHandler::commit (1) | P0-H1 | `crates/casement-testkit/tests/protocol/compositor.rs` |
| `crates/casement/src/handlers/shm.rs` | BufferHandler::buffer_destroyed (1) | P0-H1 | `crates/casement-testkit/tests/protocol/shm.rs` |
| `crates/casement-wayland/src/quire_window.rs` | Dispatch2::request, Dispatch2::destroyed, on_get_window, on_set_first_mouse, on_set_frame, on_set_role, on_set_a11y_root, on_set_edited, on_request_placement, apply_emphasis, apply_placement (12) | P0-H2 | `crates/casement-testkit/tests/protocol/quire_window.rs` |
| `crates/casement-wayland/src/system_bell.rs` | on_bell (1) | P0-H2 | `crates/casement-testkit/tests/protocol/system_bell.rs` |
| `crates/casement-wayland/src/xdg_activation.rs` | on_token_request, on_activate (2) | P0-H2 | `crates/casement-testkit/tests/protocol/xdg_activation.rs` |
| `crates/casement-wayland/src/xdg_decoration.rs` | on_decoration_mode, apply_mode (2) | P0-H2 | `crates/casement-testkit/tests/protocol/xdg_decoration.rs` |
| `crates/casement-wayland/src/xdg_dialog.rs` | on_modal (1) | P0-H2 | `crates/casement-testkit/tests/protocol/xdg_dialog.rs` |
| `crates/casement-wayland/src/xdg_shell.rs` | on_new_toplevel, on_toplevel_commit, on_move, on_resize, on_maximise, on_minimise, on_full_screen, on_window_menu, on_title, on_app_id, on_parent, on_limits, on_toplevel_destroyed, on_new_popup, on_popup_grab, on_reposition, on_pong, apply_configure, apply_close, apply_ping, apply_popup_done (21) | P0-H2 | `crates/casement-testkit/tests/protocol/xdg_shell.rs` |
| `crates/casement-wayland/src/xdg_toplevel_icon.rs` | on_icon (1) | P0-H2 | `crates/casement-testkit/tests/protocol/xdg_toplevel_icon.rs` |
| `crates/casement-wayland/src/xdg_toplevel_tag.rs` | on_tag (1) | P0-H2 | `crates/casement-testkit/tests/protocol/xdg_toplevel_tag.rs` |
| `crates/casement/src/handlers/quire_window.rs` | QuireWindowHandler::window_of_toplevel (1) | P0-H2 | `crates/casement-testkit/tests/protocol/quire_window.rs` |
| `crates/casement/src/handlers/system_bell.rs` | XdgSystemBellHandler::ring (1) | P0-H2 | `crates/casement-testkit/tests/protocol/system_bell.rs` |
| `crates/casement/src/handlers/xdg_activation.rs` | XdgActivationHandler::request_activation (1) | P0-H2 | `crates/casement-testkit/tests/protocol/xdg_activation.rs` |
| `crates/casement/src/handlers/xdg_decoration.rs` | XdgDecorationHandler::new_decoration, XdgDecorationHandler::request_mode, XdgDecorationHandler::unset_mode (3) | P0-H2 | `crates/casement-testkit/tests/protocol/xdg_decoration.rs` |
| `crates/casement/src/handlers/xdg_shell.rs` | XdgShellHandler::new_toplevel, XdgShellHandler::new_popup, XdgShellHandler::grab, XdgShellHandler::reposition_request (4) | P0-H2 | `crates/casement-testkit/tests/protocol/xdg_shell.rs` |
| `crates/casement-wayland/src/layer_shell.rs` | on_new_layer, on_layer_commit, on_layer_destroyed, on_layer_popup, apply_layer_configure (5) | P0-H3 | `crates/casement-testkit/tests/protocol/layer_shell.rs` |
| `crates/casement-wayland/src/session_lock.rs` | on_lock, on_lock_surface, on_unlock, on_locker_gone, apply_locked, apply_lock_configure (6) | P0-H3 | `crates/casement-testkit/tests/protocol/session_lock.rs` |
| `crates/casement/src/handlers/layer_shell.rs` | WlrLayerShellHandler::new_layer_surface (1) | P0-H3 | `crates/casement-testkit/tests/protocol/layer_shell.rs` |
| `crates/casement/src/handlers/session_lock.rs` | SessionLockHandler::lock, SessionLockHandler::unlock, SessionLockHandler::new_surface (3) | P0-H3 | `crates/casement-testkit/tests/protocol/session_lock.rs` |
| `crates/casement-wayland/src/pointer_constraints.rs` | on_constraint (1) | P0-H4 | `crates/casement-testkit/tests/protocol/pointer_constraints.rs` |
| `crates/casement-wayland/src/seat.rs` | on_cursor_image, apply_keyboard_focus, apply_pointer, apply_key, apply_axis, apply_gesture, apply_touch (7) | P0-H4 | `crates/casement-testkit/tests/protocol/seat.rs` |
| `crates/casement-wayland/src/serial.rs` | SerialBook::record, SerialBook::answers (2) | P0-H4 | `crates/casement-testkit/tests/protocol/seat.rs` |
| `crates/casement-wayland/src/shortcuts_inhibit.rs` | on_inhibit (1) | P0-H4 | `crates/casement-testkit/tests/protocol/shortcuts_inhibit.rs` |
| `crates/casement-wayland/src/data_device.rs` | on_selection, on_drag_start, on_drop, apply_selection_focus (4) | P0-H5 | `crates/casement-testkit/tests/protocol/data_device.rs` |
| `crates/casement-wayland/src/ext_data_control.rs` | on_control_selection (1) | P0-H5 | `crates/casement-testkit/tests/protocol/ext_data_control.rs` |
| `crates/casement-wayland/src/wlr_data_control.rs` | on_control_selection (1) | P0-H5 | `crates/casement-testkit/tests/protocol/wlr_data_control.rs` |
| `crates/casement-wayland/src/dmabuf.rs` | on_import, apply_offer (2) | P0-H6 | `crates/casement-testkit/tests/protocol/dmabuf.rs` |
| `crates/casement/src/handlers/dmabuf.rs` | DmabufHandler::dmabuf_imported (1) | P0-H6 | `crates/casement-testkit/tests/protocol/dmabuf.rs` |
| `crates/casement-wayland/src/input_method.rs` | on_im_commit, on_im_popup (2) | P0-H7 | `crates/casement-testkit/tests/protocol/input_method.rs` |
| `crates/casement-wayland/src/text_input.rs` | on_enable, on_commit_state, apply_text_focus, apply_preedit, apply_commit_string (5) | P0-H7 | `crates/casement-testkit/tests/protocol/text_input.rs` |
| `crates/casement-wayland/src/virtual_keyboard.rs` | on_vk_key (1) | P0-H7 | `crates/casement-testkit/tests/protocol/virtual_keyboard.rs` |
| `crates/casement/src/handlers/input_method.rs` | InputMethodHandler::new_popup, InputMethodHandler::dismiss_popup, InputMethodHandler::popup_repositioned, InputMethodHandler::parent_geometry (4) | P0-H7 | `crates/casement-testkit/tests/protocol/input_method.rs` |
| `crates/casement-wayland/src/background_effect.rs` | on_blur_region (1) | P0-H8 | `crates/casement-testkit/tests/protocol/background_effect.rs` |
| `crates/casement-wayland/src/foreign_toplevel_list.rs` | apply_toplevel, apply_closed (2) | P0-H8 | `crates/casement-testkit/tests/protocol/foreign_toplevel_list.rs` |
| `crates/casement-wayland/src/idle_inhibit.rs` | on_inhibit, on_uninhibit (2) | P0-H8 | `crates/casement-testkit/tests/protocol/idle_inhibit.rs` |
| `crates/casement-wayland/src/idle_notify.rs` | on_notification, apply_idle, apply_resumed (3) | P0-H8 | `crates/casement-testkit/tests/protocol/idle_notify.rs` |
| `crates/casement-wayland/src/security_context.rs` | on_create_listener (1) | P0-H8 | `crates/casement-testkit/tests/protocol/security_context.rs` |
| `crates/casement/src/handlers/idle_inhibit.rs` | IdleInhibitHandler::inhibit, IdleInhibitHandler::uninhibit (2) | P0-H8 | `crates/casement-testkit/tests/protocol/idle_inhibit.rs` |
| `crates/casement/src/handlers/security_context.rs` | SecurityContextHandler::context_created (1) | P0-H8 | `crates/casement-testkit/tests/protocol/security_context.rs` |
| `crates/casement-backend/src/headless/cost.rs` | RenderCostModel::cost_of (1) | P0-B-headless | `crates/casement-testkit/tests/fixture.rs`; input through `tests/protocol/seat.rs` |
| `crates/casement-backend/src/headless/input.rs` | HeadlessInput::drain (1) | P0-B-headless | `crates/casement-testkit/tests/fixture.rs`; input through `tests/protocol/seat.rs` |
| `crates/casement-backend/src/headless/mod.rs` | HeadlessBackend::start, HeadlessBackend::plug, HeadlessBackend::unplug, HeadlessBackend::set, HeadlessBackend::last_frame, HeadlessBackend::input (6) | P0-B-headless | `crates/casement-testkit/tests/fixture.rs`; input through `tests/protocol/seat.rs` |
| `crates/casement-backend/src/headless/renderer.rs` | HeadlessRenderer::start (1) | P0-B-headless | `crates/casement-testkit/tests/fixture.rs`; input through `tests/protocol/seat.rs` |
| `crates/casement-backend/src/input/virtual_events.rs` | Event::time, Event::device, KeyboardKeyEvent::key_code, KeyboardKeyEvent::state, KeyboardKeyEvent::count, PointerMotionEvent::delta_x, PointerMotionEvent::delta_y, PointerMotionEvent::delta_x_unaccel, PointerMotionEvent::delta_y_unaccel, AbsolutePositionEvent::x, AbsolutePositionEvent::y, AbsolutePositionEvent::x_transformed, AbsolutePositionEvent::y_transformed, PointerButtonEvent::button_code, PointerButtonEvent::state, PointerAxisEvent::amount, PointerAxisEvent::amount_v120, PointerAxisEvent::source, PointerAxisEvent::relative_direction, GestureBeginEvent::fingers, GestureEndEvent::cancelled, GestureSwipeUpdateEvent::delta_x, GestureSwipeUpdateEvent::delta_y, GesturePinchUpdateEvent::delta_x, GesturePinchUpdateEvent::delta_y, GesturePinchUpdateEvent::scale, GesturePinchUpdateEvent::rotation, TouchEvent::slot, SwitchToggleEvent::switch, SwitchToggleEvent::state (34) | P0-B-headless | `crates/casement-testkit/tests/fixture.rs`; input through `tests/protocol/seat.rs` |
| `crates/casement-backend/src/input/virtual_input.rs` | VirtualInput::plug, VirtualInput::unplug, VirtualInput::push, Device::id, Device::name, Device::has_capability, Device::usb_id, Device::syspath (8) | P0-B-headless | `crates/casement-testkit/tests/fixture.rs`; input through `tests/protocol/seat.rs` |
| `crates/casement-testkit/src/fakes/session.rs` | Session::open, Session::close, Session::change_vt, Session::is_active, Session::seat (5) | P0-B-headless | `crates/casement-testkit/tests/fixture.rs`; input through `tests/protocol/seat.rs` |
| `crates/casement-testkit/src/fakes/udev.rs` | FakeUdev::next_event (1) | P0-B-headless | `crates/casement-testkit/tests/fixture.rs`; input through `tests/protocol/seat.rs` |
| `crates/casement-testkit/src/fixture.rs` | Fixture::start, Fixture::connect, Fixture::client, Fixture::pump, Fixture::advance, Fixture::frames, Fixture::run_until, Fixture::shot, Fixture::scene, Fixture::model (10) | P0-B-headless | `crates/casement-testkit/tests/fixture.rs`; input through `tests/protocol/seat.rs` |
| `crates/casement-testkit/src/input.rs` | FixtureInput::plug, FixtureInput::key, FixtureInput::pointer_to, FixtureInput::button, FixtureInput::axis, FixtureInput::swipe, FixtureInput::pinch, FixtureInput::hold, FixtureInput::touch, FixtureInput::switch (10) | P0-B-headless | `crates/casement-testkit/tests/fixture.rs`; input through `tests/protocol/seat.rs` |
| `crates/casement-testkit/src/output.rs` | Fixture::plug, Fixture::unplug (2) | P0-B-headless | `crates/casement-testkit/tests/fixture.rs`; input through `tests/protocol/seat.rs` |
| `crates/casement-backend/src/winit.rs` | WinitBackend::start (1) | P0-B-loop | builds only (never run by an agent; the user's nested-winit check) |
| `crates/casement/src/core_loop.rs` | feed, frame (2) | P0-B-loop | `crates/casement/tests/time.rs`; frames through `crates/casement-testkit/tests/fixture.rs` |
| `crates/casement/src/display.rs` | attach_display, flush (2) | P0-B-loop | `crates/casement/tests/time.rs`; frames through `crates/casement-testkit/tests/fixture.rs` |
| `crates/casement/src/exec.rs` | exec (1) | P0-B-loop | `crates/casement/tests/time.rs`; frames through `crates/casement-testkit/tests/fixture.rs` |
| `crates/casement/src/main.rs` | run (1) | P0-B-loop | `crates/casement/tests/time.rs`; frames through `crates/casement-testkit/tests/fixture.rs` |
| `crates/casement/src/time/calloop.rs` | arm (1) | P0-B-loop | `crates/casement/tests/time.rs`; frames through `crates/casement-testkit/tests/fixture.rs` |
| `crates/casement/src/time/source.rs` | ClockSource::start, ClockSource::now (2) | P0-B-loop | `crates/casement/tests/time.rs`; frames through `crates/casement-testkit/tests/fixture.rs` |
| `crates/casement/src/time/virtual_clock.rs` | SharedClock::update, SharedClock::advance_to, EventSource::process_events, EventSource::before_sleep (4) | P0-B-loop | `crates/casement/tests/time.rs`; frames through `crates/casement-testkit/tests/fixture.rs` |
| `crates/casement-testclient/src/bin/probe.rs` | main (1) | P0-T-client | `crates/casement-testclient/tests/infra.rs`; `crates/casement-testkit/tests/globals.rs`; `crates/casement-testkit/tests/fuzz/main.rs` |
| `crates/casement-testclient/src/conn.rs` | TestClient::over, TestClient::connect_env, TestClient::pump (3) | P0-T-client | `crates/casement-testclient/tests/infra.rs`; `crates/casement-testkit/tests/globals.rs`; `crates/casement-testkit/tests/fuzz/main.rs` |
| `crates/casement-testclient/src/shm.rs` | shm_buffer (1) | P0-T-client | `crates/casement-testclient/tests/infra.rs`; `crates/casement-testkit/tests/globals.rs`; `crates/casement-testkit/tests/fuzz/main.rs` |
| `crates/casement-testkit/src/fuzz/ops.rs` | run_ops (1) | P0-T-client | `crates/casement-testclient/tests/infra.rs`; `crates/casement-testkit/tests/globals.rs`; `crates/casement-testkit/tests/fuzz/main.rs` |
| `crates/casement-ipc/src/dbus/debug1.rs` | Debug1::inject, Debug1::wait, Debug1::clock, Debug1::output, Debug1::windows, Debug1::scene, Debug1::fault (7) | P0-T-dev | `crates/casement-ipc/src/inject.rs`, `wait.rs` tests; `crates/casementctl/src/command.rs` test; `dev/accept.sh --jobs 8` |
| `crates/casement-ipc/src/inject.rs` | parse_inject (1) | P0-T-dev | `crates/casement-ipc/src/inject.rs`, `wait.rs` tests; `crates/casementctl/src/command.rs` test; `dev/accept.sh --jobs 8` |
| `crates/casement-ipc/src/wait.rs` | parse_wait (1) | P0-T-dev | `crates/casement-ipc/src/inject.rs`, `wait.rs` tests; `crates/casementctl/src/command.rs` test; `dev/accept.sh --jobs 8` |
| `crates/casementctl/src/command.rs` | parse, run (2) | P0-T-dev | `crates/casement-ipc/src/inject.rs`, `wait.rs` tests; `crates/casementctl/src/command.rs` test; `dev/accept.sh --jobs 8` |
| `crates/casement-testkit/src/fixture.rs` | Fixture::session (1) | P1-D1 | P1-Z's session suite |
| `crates/casement-testkit/src/bin/fake_logind.rs` | main: no longer a `todo!()` (P0-accept): it prints "P1-F2: not built" and exits 0, so the headless session starts without it | P1-F2 | P1-Z's logind tests |
| `crates/casement-testkit/src/fakes/logind.rs` | FakeLogind::serve (1) | P1-F2 | P1-Z's logind tests |
| `crates/casement-agent/src/socket.rs` | hand_out, role_for (2) | P2-Q2 | `crates/casement-agent/src/socket.rs` `only_the_shell_and_the_agent_tier_get_privileged_roles` |
| `crates/casement-testkit/src/fixture.rs` | Fixture::palmrest (1) | P3-Z | P3-Z's palmrest tests |

Ignored tests by lane (P0-Z2): P0-H1 17, P0-H2 16, P0-H3 7, P0-H4 10, P0-H5 7, P0-H6 3,
P0-H7 5, P0-H8 7, P0-B-headless 7, P0-B-loop 4, P0-R-scene 2, P0-T-client 5, P0-T-dev 6,
P0-F1-output 2, P2-Q2 1.

Implemented at the freeze (with their tests): every `Part::new`, `WaylandStates`,
`GlobalFilter`, `ClientState`, `ClientMint`, `convert`; `State::new`, `delegates.rs`,
`clients::insert_client`, `cli` (parse, defaults, safe mode), `jail_check`, `log`;
`VirtualOutputSpec::parse` and `text`; the golden comparison; `surfaceless_gles` (the one unsafe
module); `Rgba8`; `TextureBook`; `Transcript`, `Pattern`; `FixtureSpec`, `TestOrigin`, `Env`,
`fabricated_nodes`; `SoftPixman::new`, `FixedUdmabuf::new`; the session seam's types.

## Open items

| Item | Closes when |
| --- | --- |
| `taffy` (git rev `4863877b`, A14/A24) is declared here outside the block copied from quire's `docs/workspace-deps.toml` | quire's pinned block carries the line and this workspace copies it verbatim |
| ds-core's `Scale` has no serde, so stored types write it through `geometry::scale_serde` | ds-core derives `Serialize, Deserialize` (transparent) on `Scale` (interface ask to quire); the helper module is deleted |
| ds-style without dioxus still pulls `image` (png, jpeg, moxcms) into casement-core's graph (A22, SP8) | quire makes `image` an optional ds-style feature |
| ds-behaviour is a dependency of casement-core with no caller yet | the switcher, hot-corner and Space-swipe machines run in casement (C41, P3-Z), fed through `Mono::stamp`, with their `*Params` in `CasementSettings` |
| `CasementSettings::defaults` carries no bindings (C30 says the settings document holds the defaults) | P1-I1 writes the default table (Space steps, placement chords, hide and quit) into `settings/bindings.rs` (A26) |
| The smithay fork `PoHsuanLai/smithay` (branch `casement`, C2) exists (P0-fork) and the pin names its commit; it carries two patches, listed below | a further patch is one more commit on the fork and a new rev in the root Cargo.toml and `wlcs-casement/Cargo.toml` |
| Workspace pins outside quire's pinned block: smithay, wayland-protocols-wlr, wayland-protocols-misc, xkbcommon 0.9 (quire's block says 0.8 through sctk 0.20, A14), libseat, tracing, tracing-subscriber | quire's `docs/workspace-deps.toml` carries them (with the xkbcommon note) and this workspace copies the block verbatim |
| `reis` and `pipewire` are pinned (A14) and named by no crate | P2-Z (capture source) and P4-Z (EIS) name them |
| wlcs attaches buffers before acking the first configure; smithay refuses (about 260 failures, A6) | the orchestrator decides: a wlcs-only relaxation under a debug feature, or a newer wlcs pin |
| smithay's `IdleNotifierState` arms calloop timers on the wall clock, which the virtual clock cannot stop | P0-H8 times idleness with the model's `TimerKey::IdleNotify` and drives smithay's notifier objects only (test `idle_is_notified_at_virtual_time`) |
| smithay's virtual keyboard hands keys straight to the focused client, past the model (I8 wants `Origin::Virtual` and idle activity) | P0-H7 routes them through `virtual_keyboard::on_vk_key` (test `virtual_keys_are_origin_virtual_and_count_as_activity`), or files an interface ask if smithay offers no hook |
| smithay stamps xdg-activation tokens with `Instant::now` | P0-H2 judges token age by `XdgActivationPart::issued` (session `Mono`) only |
| The model has no tablet input kind, so `VirtualInput` maps tablet events to smithay's `UnusedEvent` | a delta freeze adds tablet input to `casement_core::input::InputKind` |
| `SoftPixman` and `FixedUdmabuf` (A4) have constructors only; their renderer and allocator trait impls (pure delegation) and the `GraphicsApi` over the jail's fabricated nodes are P1-D4's | P1-D4's soft topology suite ("display on A, client buffers from B", counters asserted, A5) |
| `casement-scene-css` was written, never built (Blitz is heavy and the oracle is P3's) | P3-CSS builds and fills it |
| Protocol tests carry `todo!()` client steps: the fixture setup and the scenario are written, the testclient helpers are not | each H lane writes its helper module and the steps in the same change |
| `taffy_bridge` (`new_tree`, `solve`, `rect_of`, `fixed_size`) has no caller outside its test | P3-Z's Split View and Exposé call it; the `expect(dead_code, unused_imports)` on the module then fails and is removed |
| Mission Control spread quality: scale about 0.30 at 20 windows, 0.19 to 0.22 at 30 to 50 (A21) | the user decides at P3 whether to enable refinement trials or a better packer |
| `Event` has no variant for a window press, an app activation or a desktop command, so `Model::press`, `Model::activate_app` and `Model::command` are called directly | the edge's handler lanes (P0-H2, P0-B-loop) say whether `Event::Press`, `Event::Activate` and `Event::Command` join the enum, each routed to those methods in `Model::step` |
| The model never arms a timer: `Ping`, `QuitGrace` and the other model-owned `TimerKey`s have handlers (`Model::step` on `TimerFired` and `Pong`) but nothing asks for them | P3-Z's quit machine and the not-responding policy arm them through `Effect::ArmTimer` |
| `QuitActive` closes the active app's windows and stops there; `ForceQuitPanel` does nothing | P3-Z's `quit.rs` |
| A minimised window travels to the bottom centre of its output (`Cause::Minimise` needs a target and the model does not know the dock) | the dock's tile arrives as an event or a layer fact (P3-Z) |
| `focus::step` on `SheetClosed` keys the parent even when the parent is in the dock or on a Space nobody sees (found by the T0 proptest); `Model::step` skips the feed for a parent it cannot see | the focus machine checks `is_visible` in `sheet_closed`; the model's guard goes |
| A window moved by a grab has no event to report its place, so a placed window stays `WindowState::New` until a placement request moves it | P0-H2's grabs report geometry (an `Event`) or P3-Z's `Drag` cause does |
| ds-motion's `Contact::pressed` is public without dioxus although its doc says crate-private (A23) | decided in quire (S0-QM1 follow-up): intended for compositor swipe releases, or closed |

## Departures from 20-FREEZE-PLAN §4.2

- `TimerBook` keeps an issue counter beside the map, so same-instant timers fire in arming
  order (SP3, A1); the private field set differs from the sketch.
- `VirtualClock` mirrors ds-core's API (A1): `elapsed`, `next_due`, `schedule`, plus
  `fire_due` and `timers`, beside the frozen `new`, `now`, `with_timers`, `with_output`,
  `without_output`, `next_stop`, `advance_to`. The calloop event source over it is P0-Z2's.
- `Model::new` takes a `SpaceSeed`: `Model::step` mints Space ids and must stay pure.
  `space/mint.rs` gains `SpaceSeed { from_entropy, next }`. `Model` drops the sketch's `mint`
  (window, layer and lease ids are minted by the edge that sees them, with `IdMint`) and gains
  `now()`.
- `route_press` takes `&Focus` (the press table depends on which window is key).
- `place_outputs` takes the `ScalePolicy` (a new display's scale needs it).
- `Chord::text` added: the inverse of `parse`, the stored form.
- Builders added beside frozen books: `SpaceBook::with_space`, `with_shown`;
  `LayerBook::empty`, `with_layer`, `without_layer`; `Focus::none`; `WindowHints::unset`.
- `GlobalKind` is filled with the P0 globals now (the sketch left it to P0-Z2) so the I7 table
  can be written.
- casement-layout follows SP8 (A20): `WindowSpec.app`, `SceneMode::Overview { shown, kind }`,
  `Arrangement.space_thumbs` and `reports` (`LayoutReport`), `OverviewSlot.kind`
  (`SlotKind`), `WindowPlacement::Minimised` without a target (motion takes the tile from
  `Cause::Minimise`). `WindowTarget` and `SpaceTarget` carry their `output`, which motion and
  the renderer group by.
- P0-F1b-model (A28): `WindowState::New(LogicalSize)` and `Restore::New` (a window mapped and
  not yet placed; `spec_of` emits `WindowPlacement::New`), `Window.stack: StackRank` (a rank
  distinct across the model; `spec_of` numbers each Space's windows from it) and
  `Window.home: Option<OutputKey>` (the output a window on every Space sits on).
  `WindowState::restore` and `Restore::into_state` convert between them.
- P0-F1b-model: `InvariantBreak` grows from 6 to 19 variants (main window, key outside the
  active app, key not visible, emphasis, parent, home output, stack rank, app window order,
  output shows nothing, full-screen and Split View Spaces, layer output, lock coverage).
- P0-F1b-model: `Model` is stepped through `step` and three more public entry points for what
  `Event` has no variant for (`press`, `activate_app`, `command`), and answers `reach`,
  `states_of` and `keyboard_target`. Keyboard focus is an effect derived once per step from the
  lock, an exclusive layer and the key window, never moved by a handler.
- P0-F1b-model: read and write accessors added beside frozen books: `SpaceBook::{without_space,
  without_shown, iter, shown}`, `LayerBook::{get, iter}`, `OutputConfig::logical_rect` (the one
  logical rectangle of an output, replacing layout's private copy), `FocusView::is_visible`
  public.
- casement-motion's `sample` takes `WindowLooks` (emphasis and frame per window): emphasis,
  corner radius and shadow had no source in the sketch's inputs. `FrameScene::text` is the
  golden form.

## Departures from 20-FREEZE-PLAN §4.3 (P0-Z2)

- **Dispatch at smithay 118e34ff.** The per-protocol `delegate_*!` macros are gone: every
  global's and object's user data implements `GlobalDispatch2`/`Dispatch2`, and
  `delegates.rs` is the one line `delegate_dispatch2!(State)`. `quire_window` carries its own
  `GlobalDispatch2`/`Dispatch2` impls and reaches `State` through `QuireWindowHandler`.
- **casement-wayland.** `Part::new` is generic (`new::<D>`) with smithay's constructor bounds;
  filtered globals (decoration, layer shell, session lock, primary selection, data control,
  input method, virtual keyboard, foreign toplevel list, security context, quire_window) pass
  `GlobalFilter::for_global`, the rest ignore the filter (I7 shows them to everyone).
  `WaylandStates<D>` is generic because smithay's seat and idle notifier are, and gains
  `clients: ClientMint`. New modules: `client.rs` (`ClientState`: key, role, peer, smithay's
  per-client state) and `output.rs` (`GlobalKind::Output` had no home; owned by P0-H1).
  `SeatPart::new` takes the keyboard settings and returns `Result` (the keymap may not compile);
  `IdleNotifyPart::new` takes the loop handle; the data-control parts take the primary-selection
  part; `DmabufPart::new` creates no global (`apply_offer(DmabufOffer)` does, until P1's
  `FeedbackPlan`). `compositor::on_commit` returns a `CommitRoute` (routing needs other parts);
  `xdg_foreign` has no request functions (smithay serves export and import); `xdg_activation`
  has no `apply_token`, and `on_token_request` answers a `TokenVerdict`; the seat's `apply_*`
  take the seat and the whole state (smithay's input handles need `&mut D`); no
  `apply_modifiers` (smithay sends modifiers with enter and keys); `on_create_listener` returns
  the listener's `SandboxFacts` with its events.
- **casement-render.** `Compositor` has no `ExportMem` supertrait (smithay's `DummyRenderer`
  has none); `output_elements` takes an `OutputView` (the frame has no output geometry);
  `CursorDraw::ClientSurface { hotspot }` replaces `Surface(WindowId)` and `ChromeLayer` gains
  `cursor_at`; `TextureBook` has builders; `Rgba8` is the quantised colour (A18); the unsafe
  module exists (`gl_raw.rs`, A17: `surfaceless_gles`).
- **casement-backend.** `BackendKind` has no `Tty` variant until P1-Z; `FramePixels` is
  renderer-neutral (`Rgba(RgbaFrame) | Dummy`, Abgr8888 read-back, SP2); the virtual output
  grammar gains optional `transform=`, `gpu=`, `vrr=`, `edid=` fields and `text()`;
  `VirtualInput::{plug, unplug, push}` exist only under `testing`/`debug-inject`; event types
  are one struct per family (`VirtualKey`, ..., `VirtualTouch` for every touch step);
  `HeadlessRenderer` boxes its renderers; `gpu/soft.rs` holds `SoftPixman` and `FixedUdmabuf`.
- **casement.** `State` gains `display` and `event_loop`; `CoreState` gains `mint: IdMint`;
  `IpcState` holds `casement_ipc::IpcHandles` (so Debug1's lane changes casement-ipc, not
  `state.rs`); `ClockSource::{Virtual, Stepped}` hold a `SharedClock` (the calloop source shares
  it, A1) and `time/virtual_clock.rs` holds `VirtualTimerSource`. `StartConfig` gains `renderer`,
  `cost` and `entropy`; `ConfigHome` is `Xdg | At(path)`; `ClockChoice` gains `Virtual` (the
  fixture's, never on the command line); `Command` gains `Help`; `JailCheck::Outside` names the
  first breach found. New files: `clients.rs` (`insert_client`) and `display.rs` (the display on
  the loop without `unsafe`, SP3).
- **casement-ipc, casementctl, test kit.** `IpcHandles` and `DebugCall`/`DebugRequest`/
  `DebugReply` added; casementctl is a library plus binary (its table is tested);
  `FixtureSpec::{desktop, with_outputs, with_renderer}`, `Env`, `SessionFixture`, `UdevEvent`,
  `LogindCall`, `gpu::fabricated_nodes` added; testclient's `GlobalList` is wayland-client's.
- **`dev/gate.sh`.** §6.2's `jail` was a shell function, which `step.sh` (its own process)
  cannot call; it is an array. `scripts/check-boundary.sh` skips comment lines in its "names"
  rows (B20 still reads comments), matches the libinput crate by its items (casement has its own
  `input` modules), and lets the test kit reach ds-settings through `casement` while naming none
  of it.

## Departures in the P0-R-scene fill

P0-R-scene filled its 23 stubs with no `todo!()` left in `crates/casement-render`. Renderer-only
goldens in `crates/casement-render/tests/golden/` (ten tier P, `blur-v0-1x` tier L) and a tier X
check cover what draws without a client surface; the fixture scenes are P0-shoot's (below).

1. **`Compositor` grows, additively.** Its textures are bounded `Clone + Send + 'static`
   (`RendererSuper<TextureId: …>`; smithay's surface and memory elements need it), and it
   gains two default hooks, `capture_blur_v0` and `draw_blur_v0` (stand-in by default, GLES
   overrides). `render_output` wraps each element in a private `Routed` so the `Blur` variant
   reaches them: a generic `render_elements!` enum cannot name a renderer per variant. P1-D4's
   `SoftPixman` needs a `Compositor` impl (one method).
2. **Blur v0 on GLES** is a quarter-size linear `blit_to` of the framebuffer under the element
   into a cached `GlesTexture`, drawn back through a 5×5 binomial texture shader. It reads raw
   framebuffer coordinates, so it assumes a `Transform::Normal` output (true headless; P1 for
   rotated outputs).
3. **Windows draw at their committed size.** The closed element set has no rescaling wrapper,
   so a sampled size that differs from the committed one shows once the client commits it;
   P3-Z adds the wrapper with its effect elements.
4. **`OutputFrame.strip` is not applied.** A `PlacedWindow` does not say which Space it is on,
   so the renderer cannot shift windows per Space; P3-Z.
5. **Layer placement has two homes.** `crates/casement-render/src/scene/place.rs` places layer
   surfaces from their committed layer-shell state (anchors, margins and exclusive zones:
   exclusive surfaces first, overlay level down, then the rest inside what they leave), because
   `output_elements` receives no layer geometry. P0-H3's
   `casement_wayland::layer_shell::layer_area` uses the same axis rules but places in the whole
   output and applies no zones, so the two differ for a zone-respecting surface beside a bar.
   Closes when one `casement_core::layer` function places layers for both (P1 ask).
6. **Named cursors are casement's own rasters** (signed-distance shapes, 16 samples a pixel,
   at the output's native scale, cached per shape and scale); no Xcursor theme is read yet.
   A locked output draws only the cursor, its lock surface and an opaque cover under it. The
   cover's id is thread-local and the cover's and blur's commits are content fingerprints, so a
   still frame has empty damage. The renderer tests read `CASEMENT_BLESS` with `Env`'s rule in
   `tests/support/mod.rs`, since casement-render cannot depend on the test kit.

## Departures in the P0-shoot fill

P0-shoot wrote the fixture scenes (`crates/casement-testkit/tests/scene/`: `the_ten_base_scenes_match_their_pixman_goldens`,
tier P; `blur_v0_matches_its_llvmpipe_golden`, tier L), the three surface-adjunct pixel tests
(viewporter crop, single pixel, alpha multiplier) and the fractional-scale ordering test. Real
code the scenes found missing or wrong, fixed here:

1. **No window was ever drawn.** Nothing filled `TextureBook`'s window map, so a mapped toplevel
   showed as black. `casement::core_loop::windows` follows `ToplevelMapped`, `ToplevelCommitted`
   and `ToplevelUnmapped`.
2. **A layer surface's committed size was stale.** `LayerMapped` fires on the first commit (no
   buffer yet) and the buffer's commit changes nothing the model hears, so the book kept 0 x 0
   and a full-width bar drew from the output's middle. `core_loop::frame` now reads every layer
   tree's size again (`TextureBook::with_layers_sized`).
3. **GLES frames read back upside down.** `GlesMapping::flipped()` is a constant `true`, but the
   damage tracker's framebuffer is already y-inverted, so reversing the rows turned the frame
   over (the tier X check in casement-render never read through the backend). The headless
   backend no longer reverses rows; pixman and llvmpipe agree.
4. **Read-back was a per-byte iterator** (about 4 s a 4K scene in a debug build); it copies rows.
5. **The scenes settle two virtual seconds** before the shot (a window opens with motion); the
   desktop has no wallpaper (a layer client draws it), so an empty scene is black with the arrow
   at the pointer.

Test-support notes: `tests/scene.rs` became `tests/scene/main.rs`; the layer-shell and
session-lock client helpers (`casement-testclient/src/surface/{layer,lock}.rs`) are the P0-H3
helpers verbatim from `~/rs-wt/p0-h3/pending/p0-h3-protocol-tests.patch` (apply that patch
without those two files); `BlurAsk::surface()` is a new accessor so a blur region can sit on a
toplevel's own surface; `tests/protocol/shoot.rs` holds the helper the adjunct tests share.
Goldens: `tests/golden/scenes/*.png` (the ten, `blur-v0-1x`, `viewport-crop`, `alpha-multiplier`),
all blessed once on the first real run (nothing existed to differ from); the compare is
byte-exact for tier P and 2 per channel on at most 1000 ppm of pixels for tier L (11 §2.8).

## Standing facts

- **The virtual clock is a value, not a thread global** (SP3): calloop's `Timer` and smithay's
  `Clock` read the wall clock and cannot be virtualised, so the composition root wraps
  `VirtualClock` in its own event source. Stops: timers before vblanks at one instant, timers
  by arming order, vblanks by output key; a timer already due stops at now; vblanks are
  computed from their phase (no drift: vblank 36,000 at 60 Hz is exactly 600 s).
- **`OutputKey` text**: `edid:<make>/<model>/<serial>` when the EDID has a serial, else
  `connector:<name>`.
- **`to_physical` rounds edges, not sizes**, so touching rectangles touch at every scale.
- **The first-sight scale is sill's PreferWhole rule** (`sill-model` `displays/scale.rs`): 109
  points per inch, whole scale from an ideal of 1.5 while at least 1280 points wide, capped at
  2x; quarter steps otherwise.
- **`cargo fmt --all` reaches sibling workspaces** through path dependencies (CONVENTIONS 15);
  `--check` needs `../stoker` beside porter.
- **Motion goldens rest on the platform's `f64::exp`** (A25); quantising to `SubPx` keeps the
  chance of a one-ulp flip moving a value tiny, not zero.
- **smithay 118e34ff, as P0-Z2 found it:** every global's dispatch goes through
  `delegate_dispatch2!`; handler traits a `State` must implement for the P0 globals: Compositor,
  Buffer, Shm, Output, FractionalScale, XdgShell, XdgDecoration, XdgActivation, XdgDialog,
  XdgToplevelIcon, XdgToplevelTag, XdgForeign, XdgSystemBell, WlrLayerShell, SessionLock, Seat
  (focus targets are `WlSurface` at P0), TabletSeat, PointerConstraints,
  KeyboardShortcutsInhibit, Selection, DataDevice, WaylandDndGrab, DndGrab, PrimarySelection,
  ext and wlr DataControl, Dmabuf, InputMethod, IdleNotifier, IdleInhibit,
  ForeignToplevelList, SecurityContext, ExtBackgroundEffect. Global versions are in
  `crates/casement-testkit/tests/golden/globals.txt`.
- **smithay reads the wall clock in three places casement must not depend on**: the idle
  notifier's calloop timers, xdg-activation token stamps, and the times it invents for
  synthesised touch and tablet events (FINDINGS open items above).
- **The jail's PID 1 is `bwrap`** (`--unshare-pid` with `--proc /proc`), which both
  `require_jail` checks (shell and `casement::jail_check`) rely on.
- **Layout reads the usable area, not the output** (A26): `spec_of` fills `OutputSpec.usable`
  from `LayerBook::usable_area`, so maximised and filled windows respect exclusive zones.
  `pick_mode` returns a 0x0 mode for a display with no modes; the model leaves such an output
  dark (`OutputPower::Off`) and still gives it a desktop.
- **A new window is unplaced** (A28): it maps as `WindowState::New(first_size)` on the shown
  desktop of the output the key window is on (else the leftmost lit output), and layout
  places it (centre, then cascade). A window on every Space (shell role) is arranged on the
  shown Space of its home output only. With no output plugged every window waits on every Space
  with no home, and the next output takes them in.
- **Spec stacking is the model's** (A28): a Space's windows are ordered by `Window.stack`, which
  a raise (activation, a press, an unhide) moves to the top; a hidden app's windows are
  arranged as minimised.
- **P3-Z owes two fields** (A29): `OverviewSlot` gains `output: OutputKey` and `FrameScene` an
  overview progress field (`MotionKey::Overview` is unbuilt).
- **Benches live outside `crates/`** (A30): B8 bans `Instant::now` there; P0-Z2 hosts a perf
  project under dev, and until then the layout and motion benches sit in their lane worktrees,
  beside the checkout.
- **Layout reads a chrome token** (A31): the cascade step is ds-style's
  `ChromeToken::TitlebarHeight` (28).
- **A timer and vblanks that share an instant arrive together** (A32): `VirtualClock::arrive(t)`
  reports timers in arming order, then vblanks by output key; the calloop source steps with
  `next_stop` then `arrive`, never `advance_to` alone.
- **The focus machine's view carries apps, and rule S5 is split** (A33): `FocusView` has
  `apps` (the hide row, and an app's windows bottom first); when the activated app has no
  window on a shown Space `step` leaves it active with no key, and the model switches Space
  then feeds `SpaceShown` (unless `spaces.switch_on_activate` is off); the model unhides an app
  on activation; a layer-surface press never reaches `route_press`; an xdg-activation
  `Missing` token is handled as `Stale`.
- **Space ids** (A34): `mint_space_id` is `s-` and 32 lowercase hex digits (34 bytes), from a
  splitmix64 sequence over the seed's counter.
- **A parent is its own app's window**: a sheet or a dialog names a mapped window of the same
  app; any other parent is dropped to none, so a sheet cannot pull focus into another app.
- **`ConfigHome::At(DIR)` is a config home** (P0-F1-output): it stands in for `$XDG_CONFIG_HOME`,
  so the document is `DIR/casement/settings.toml`, not `DIR/settings.toml`.
- **Bindings are word lists** (P0-F1-output): a `[bindings]` entry is chord text to a list of
  words (`"cmd+q" = ["desktop", "quit_active"]`, `"ctrl+option+t" = ["spawn", "foot", ...]`),
  not a nested table; an entry whose chord or words do not parse is dropped with a warning.
- **casement-core's settings enums derive `Word`** (P0-F1-output): the ds-settings derive gives
  each its document spelling; the serde derives stay for the model's own stores.
- **casement takes a `toml` dev-dependency** (P0-F1-output): `tests/config.rs` writes and reads
  documents through it.
- **`serve_debug` takes Debug1's name with replacement off** (P0-debug-dbus): a second casement
  on the same bus fails with `IpcError::Bus` and the first keeps the name. The bus thread is
  not joinable: it lives as long as the process.

### P0-asks: what sill's acceptance scripts found

- **A zone-0 layer lives in the usable area** (wlr-layer-shell): `LayerBook::room_for` gives the
  rectangle a surface is sized and placed in: the whole output for `-1` and for a surface that
  reserves a zone along its own edge, the usable area for `0` and for a positive zone its anchoring
  makes meaningless (the protocol says "treated the same as zero"). Surfaces that avoid zones are
  configured again when another layer's zone appears, changes or goes (`Model::reconfigured`).
  Hit-testing places layers with the same rectangle as the renderer, and orders them by level
  (overlay, top, bottom, background) instead of by key across levels.
- **A full-screen window covers the output**: `OutputSpec.full`, `WindowPlacement::FullScreen`;
  `RustScene` gives it the whole output with a `Resize`, whatever its limits. Windows in a
  full-screen Space are arranged (Split View Spaces still are not, P3-Z). It is drawn and hit above
  the top layers and below the overlay layers (`LayerDraw::full_screen`).
- **Pointer start**: the centre of the primary output (`Model::pointer_start`), set once when the
  first output is plugged (`PointerStart`). No document names a rule; this avoids a hot corner
  at start-up.
- **A client that leaves with its grabbing popup open**: smithay drops the grab before
  `popup_destroyed`, so after `ClientGone` the loop puts the keyboard where the model says when the
  seat holds a dead surface or is ungrabbed and out of step (`exec::resync_keyboard`).
- **ext-session-lock waits at most `LOCK_WAIT` (1 s) for a lock surface**: the protocol lets the
  compositor wait for the first surface before `locked` but says it "must impose a reasonable time
  limit"; casement had none. `TimerKey::LockWait`; a surface that arrives later is still taken in.
- **`casementctl output ...` is `casementctl debug output ...`** (the Debug1 verb table holds
  `output`; the plan spells it both ways).
- **`casement-probe` rejects unknown flags** (exit 64) and `toplevel --maximised` prints
  `maximised` in its configure line. `Debug1::Inject` answers `ok <line>` as written.
- **Popups get frame callbacks**: `drawn_roots` lists the popups of every drawn root (windows, layers,
  lock surface), so a menu that animates is told `done` on each frame.
- Not done: a Reserve bar's own placement is still against the whole output in hit-testing while
  the renderer shrinks it by earlier bars on other edges (two bars on one edge overlap in hit-tests).

### P0-fork: the smithay fork

`PoHsuanLai/smithay` branch `casement` starts at upstream `118e34ff` (the rev casement had pinned) and carries
two commits, one per patch, each local to its bug; patches live only there and nothing goes upstream (A13).

- `87c4f4df` xdg_shell: destroying an `xdg_surface` before its role object posts `xdg_surface.defunct_role_object`
  (was `xdg_wm_base.role`); a `get_toplevel` or `get_popup` on an `xdg_surface` that already has a role object
  posts `xdg_surface.already_constructed` (was accepted). Test: `xdg_shell::destruction_order_and_double_roles_are_errors`,
  no longer ignored.
- `576cfd95` wl_region: `add`/`subtract` with a negative width or height is ignored (treated as empty; wl_region's XML
  defines no error) instead of reaching `Size::new`'s `debug_assert`.

The root and `wlcs-casement` pins (and both `Cargo.lock`s) name the fork's tip `576cfd954b6bd5badbd5f15d1708cafc941a0522`. The
`[profile.dev.package.smithay] debug-assertions = false` override is removed: smithay's debug assertions are on in
every dev build and the fuzz box (24 cases) passes with them.

### P0-accept: the seven acceptance scripts

`dev/accept/*.sh` are all real (map-resize, scale-crisp, popup-grab, lock-blocks-input, ime-commit,
drag-selection, exclusive-layer-focus; globals was already). Their client is `casement-probe session`,
a test client driven from stdin (`probe/command.rs`); frames come from `casementctl shot`, which
until P1-C1 asks Debug1 (`Shot`) for the PNG of the last presented frame. Found by them and fixed:
an interactive resize changed the frame but never configured the client (floating layout asks
nothing when the rect is its own size): `core_loop::configure_size`, test
`a_resize_configures_the_client_to_the_dragged_size`; `debug wait settled` did not step the stepped
clock. Open: `scale-crisp` uses casement's own test client, not a quire or shell-host window (those
are the consumer scripts'); after a drop casement sent `wl_data_device.leave` at once (fixed by
P0-dead-focus); pointer motion while a dead locker's cover stands did not move the
cursor (not asserted).

### P0-dead-focus: a surface that dies stops being the seat's focus

- **The crash** (`State::toplevel_destroyed` -> `refocus` -> smithay `pointer.motion` -> `wl_pointer.leave`,
  `send_event` InvalidId, wlcs `Layer/LayerSurfaceLayerTest.surface_can_be_moved_to_layer/7`) happens while
  libwayland tears a client down: the backend has already forgotten the client, so any event that names one
  of its objects (leave and enter carry the surface) panics. The cause is not a stale focus alone, so
  the guard lives where the event is sent: `casement_wayland::seat::Live`, the type of `PointerFocus` and
  `TouchFocus` (was a bare `WlSurface`), forwards every smithay target call but sends `leave` and `enter`
  only when the surface is alive and its client is known (`Live::reachable`). The keyboard needs no guard
  (smithay's focus-destroy hook clears it). The in-process Fixture cannot reproduce the C-level panic (a
  `disconnect` does not run libwayland's teardown order), so the regression tests assert the visible half:
  the seat holds no pointer focus on a surface that is gone (`CompositorHandler::destroyed` ->
  `seat::forget_dead`), checked between the surface's death and its role's. wlcs is the proof of the
  panic: the whole `LayerSurfaceLayerTest` group now runs to its verdicts instead of aborting the run.
- **W6**: `seat::release_dead` lifts every touch point that went down on a dying surface (smithay sends
  nothing to a dead surface, so the client's `wl_touch` is told directly: `up` then `frame`); the seat
  remembers each point's surface and client from its `down`. The four
  `sends_touch_up_on_surface_destroy` wlcs tests pass and are off the expected-failures list.
- **Drop then leave**: smithay's `DnDGrab` sends `wl_data_device.leave` straight after `drop`. `Live`'s
  `DndFocus` impl remembers that a validated drop was delivered and does not send that `leave`; the
  offer stays the client's to read until it finishes it. A drag that ends without a drop still gets its
  `leave`. The probe's workaround (a clone of the offer taken at `drop`) is gone.
- Tests: `tests/protocol/dead_focus.rs` (toplevel, layer surface, popup, hung-up client, touch),
  `data_device::a_drop_is_not_followed_by_leave`. Testkit: `Fixture::pointer_focus`; test client:
  `seat::touch::Touch`, `Toplevel::destroy_roles`, `LayerSurface::destroy_surface_first`.
- wlcs: 302 entries before, 298 after (the 4 W6 removed); `--phase` runs 927 tests, 596 pass, 331 skipped,
  none fail. The 25 `LayerSurfaceLayerTest` entries stay listed: they no longer crash, they fail on
  `window_under_cursor()` (NULL or the wrong surface), a layer stacking and hit-test gap (W1).


### P0-output-enter: wl_surface.enter and leave, preferred buffer scale

- **W5**: `casement/src/core_loop/presence.rs` (new). After every `feed` and at the start of every
  `frame`, it walks the hit-test stack (`scene::stack`: lock surfaces, layer surfaces, windows, each
  with its popups, at the origins the stack gives) and tells each tree which outputs it is on. A root
  is on an output when a surface of its tree has a view that overlaps the output's logical rectangle
  (a local copy of smithay's `output_update`, which `desktop::space` keeps `pub(crate)`; it enters
  and leaves subsurfaces one by one). A popup is on every output its parent's drawn bounding box
  touches and on those its own touches. A surface that left the stack (unmapped, destroyed) is told
  it left every output; an unplugged output is left by the surfaces (`apply_unplug` also calls
  `leave_all`). State: `RenderState.presence` holds the roots told last time.
- **Preferred buffer scale**: `wl_compositor` is now version 6 (`CompositorState::new_v6`). The same
  pass sends each tree the scale of its home output (the one with the largest overlap) through
  `fractional_scale::apply_scale`, which sends `wl_surface.preferred_buffer_scale` (v6) and the
  fractional preferred scale; so every surface hears its scale, not only those with a
  `wp_fractional_scale_v1`. `preferred_buffer_transform` stays Normal (the renderer assumes
  `Transform::Normal`, P1 for rotated outputs).
- **Model fact the tests lean on**: a toplevel never straddles two outputs (its rect is held inside
  its Space's output), so a window changes output only by `set_fullscreen(output)` or by its output
  going; a layer surface is arranged inside its output. Only a subsurface or a popup can reach over
  a seam, and neither is tested to straddle yet.
- **Gap**: cursor surfaces and drag icons get no enter/leave (they are not in the stack).
- Tests: `tests/protocol/compositor.rs::outputs_of_a_surface` (map, change of output both ways,
  unplug, layer surface on its output, popup follows its parent); each failed with the
  `presence::update` calls removed. Test client: `LayerSurface::create_on`,
  `Toplevel::set_fullscreen_on`, `unset_fullscreen`. wl_compositor test binds 1 to 6.
- wlcs: `ClientSurfaceEventsTest.surface_enters_output` passes and is off the list: 298 entries
  before, 297 after; `--phase` runs 928 tests, 597 pass, none fail.

### P0-layer-geom: where things are drawn and hit (W1, W2)

- **W2, window geometry.** A toplevel's place and size are its `xdg_surface.set_window_geometry` rect's
  (the model already took the size from it); the buffer is drawn offset so the geometry origin lands at
  the window's place, and the hit test and the pointer-local coordinates use the same origin.
  `casement_render::surface_origin` / `surface_origin_logical` are the one conversion (the renderer's
  `window_elements` and `output_elements`, and `core_loop::scene::stack`, call them); popups already
  added the root's geometry offset, so they now hang off the same origin. A window's committed size
  in the surface book is the geometry's when it declared one (`windows::tree_size`).
- **W1, layer-shell.** Four causes, none a wlcs expectation that is wrong (checked against
  wlr-layer-shell-unstable-v1):
  1. `set_size` was dropped: `LayerSpec` had no size and the configure said 0 on every axis not held to
     both ends. The spec now carries `size`; `Arranged::configure_size` is what a surface is told
     (what it asked for on an axis, else the room less margins when held to both ends, else 0), and
     `Arranged::place` places the configured box, not the buffer (a smaller buffer sits at the box's
     corner). The model's `layer_size` is gone: the configure and the placement read `Arranged`.
  2. A maximised or full-screen request made before the first commit was dropped (the model knew no
     window yet), so the initial configure had no state and no size; wlcs' client maps only from a
     size it is told, so the "exclusive zone does not shrink the toplevel" tests timed out. Now the
     edge keeps the ask (`ToplevelRecord::opening`), writes the state and `Model::opening_room`'s size
     into the pending configure, and replays the request to the model right after the map.
     (The doc line "a request for a window that has not mapped is dropped" in `xdg_shell/requests.rs`
     is amended.) The existing test `a_dock_mapped_after_a_bar_is_hit_below_it` asserted the old width
     0 for a dock that asked for 64; it now expects 64.
  3. wlcs' `move_surface_to` on a layer surface was a no-op in the shim (it moved toplevels only), so
     the 25 `LayerSurfaceLayerTest` x layer-pair tests had both surfaces centred on each other. A
     layer put by hand is `Event::LayerPlaced` (a pin in `LayerBook`, read by `Arranged::place`, so
     the draw, the hit test and the sizes agree); no protocol sends it, only `Fixture::place_layer`
     (the shim). Stacking by `set_layer` itself already worked (test added).
  4. Layer popups were never placed inside the output (`wlr_layer_surface.get_popup` only tracked
     them): they now go through `popups::new_popup`, and `bounds_of` knows a layer root.
  Also: `zwlr_layer_surface_v1` keyboard interactivity `on_demand` was not implemented. A surface that
  asks for it takes the keyboard when it maps with no key window, or when it is pressed
  (`Model::press_layer`, called from the pointer press path); a window that is pressed or becomes key
  takes it back (`Model::demand`, cleared in `run` and `press`). 03 §1.2's line "layer surfaces never
  take the keyboard" holds for `none` only.
- **Not fixed, diagnosed.** The four `LayerShellPopup/XdgPopupTest` grab tests (now tagged W8 with the
  four `XdgPopupStable` twins): `xdg_popup.grab` is refused because wlcs passes the client's latest
  serial of any event and the seat's `SerialBook` proves presses only, so the popup is dismissed
  at once (with the proof bypassed `grabbed_popup_gets_keyboard_focus` and, on layers,
  `grabbed_popups_get_done_events_in_correct_order` pass; `popup_gives_up_pointer_focus_when_gone` and
  `grabbed_popup_gets_done_event_when_new_toplevel_created` still fail for another reason). The four
  `input_seen_*_after_*_unmapped_and_remapped/6,7` tests were tagged W2 but are W13 (remap after a null
  buffer; `/4,5` are the same on the other builder).
- Tests: `tests/protocol/geometry.rs` (pointer-local at a geometry offset, drawn pixel at a geometry
  offset, configure sizes, placement by the configured box, open maximised / full screen, a layer put by
  hand, `set_layer` stacking, a layer popup kept inside the output), `tests/protocol/layer_keyboard.rs`,
  `layer/arrange_size_tests.rs`. Test kit: `Fixture::layer_of`, `Fixture::place_layer`; test client:
  `LayerSurface::set_layer`.

### P0-popups: popup grabs, popup motion, interactive move focus, constraints (W8, W10, W13, part of W9 and W12)

wlcs `--phase`, after: 857 pass, 37 fail (the listed ones), 331 skip; before: 829 pass, 65 fail.
Per row, before to after: W3 4 to 9 (five re-tagged), W4 1 to 5 (four re-tagged), W7 6 to 6, W8 12 to 0
(11 fixed, 1 re-tagged W3), W9 1 to 1 (diagnosed), W10 4 to 0, W12 29 to 16 (5 fixed, 8 re-tagged W3/W4),
W13 8 to 0.

- **W8, the grab serial.** `SerialBook` now records a pointer button release too (`PressKind::Release`),
  and keeps each client's latest recorded input. `xdg_popup.grab` is accepted when its serial is that
  client's latest input (a press, a release, a key press, a touch down on a surface of the client), or
  the press that the latest release ended (a menu opened on the press by a client that has not yet read
  the release: GTK). A stale serial, a forged one and another client's are refused. Qt opens its context
  menus on the release. A release proves no move or resize (`grab_proven`, `GrabUse::Drag`).
- **W8, the second cause behind `popup_gives_up_pointer_focus_when_gone`.** The client destroys its
  `xdg_popup`, then the `wl_surface`. casement refocused only when the surface was gone, and smithay sends
  no `leave` to a dead surface, so the client was told of an `enter` with no `leave` before it. The
  handler now refocuses the pointer when the popup is destroyed (`popup_destroyed`), while the surface
  still lives.
- **W8, `grabbed_popup_gets_done_event_when_new_toplevel_created` and the done order.** smithay's popup
  keyboard grab ignores a focus change while the grab holds, and the swallowed activating press never
  reached the pointer grab, so nothing dismissed the popups when the user moved on. `exec::keyboard`
  now asks `popups::dismiss_for_focus` first: a keyboard target other than the popup's root dismisses the
  grab's popups, topmost first (smithay's `send_done` order), and ends both grabs. Layer roots too.
- **W8, a nested grab with its parent's serial** (a submenu a timer opens, no new input). smithay's old
  `PopupPointerGrab::unset` unsets a keyboard grab that has the same serial, so the submenu's own
  keyboard grab was dropped and the keyboard went to the root (a bar that never asked for it). `grab` now
  lifts the keyboard grab first (no focus change), replaces the pointer grab, then sets the keyboard grab.
- **W8, reactive popups and reposition.** A popup whose positioner is `reactive` is placed again and
  configured when the arrangement changes (`relayout` calls `refit_reactive`); an unmapped one is left to
  its first buffer. A mapped popup's acked configure takes effect at once, not at the next commit
  (`xdg_popup.repositioned`: "must acknowledge ... for the new position to take effect"): `ack_configure`
  copies the configure into smithay's committed popup state (`PopupCachedState`, public) when the popup
  has mapped. wlcs acks and never commits.
- **W10.** xdg_toplevel.move/resize: "the surface will lose the focus of the device used for the move".
  `grab::start` ends the press's implicit pointer grab and leaves the surface; `refocus` does nothing
  while a grab holds the pointer; the release gives the pointer back (after the release is delivered).
  The shim also lets a dropped window glide to rest (1 s at each release): casement springs a dragged
  window after the pointer by design.
- **W13.** casement was right (a null buffer returns the toplevel to its initial state, xdg-shell's
  "Unmapping"; the lenient policy already maps a buffer without an ack). The test
  (`tests/relax.rs`) passes without a casement change. A toplevel that maps again keeps its id and is a
  fresh window (`WindowState::New`) where casement puts it; wlcs takes it to be where it was. The shim
  remembers where wlcs put each toplevel and puts a window that maps again there.
- **W12, pointer constraints (5 fixed).** A confined pointer is pinned to the last pixel of its room
  (the surface tree's extent cut to the region's added rectangles) instead of not moving when the
  motion would leave it, and keeps its surface as the focus (`seat/hold.rs`). A constraint is released
  when its surface loses the keyboard and holds again when it has it back and the pointer is on it
  (`pointer_constraints::on_keyboard_focus`).
- **The flaky `CopyCutPaste.given_source_has_offered_when_sink_gets_focus_it_sees_offer`.** The shim
  activated a mapped window a pass after the commit that mapped it; a client that sets the selection
  in its next request, right after its roundtrip, can be ahead of that, and smithay ignores a
  selection from a client without the keyboard. 11 failures in 300 runs under 12-way load before;
  0 in 300 after. Now the lenient policy activates the window in the commit that maps it.
- **Diagnosed, not casement bugs** (rows W3, W4, W9, W12 in `dev/wlcs/expected-failures.txt`): the
  zero-size anchor rectangle (smithay rejects 0, the XML only negative), the error objects of
  fractional-scale and viewporter, the viewporter's missing `bad_size` hook and `out_of_buffer` check,
  the decoration configure and orphaned tests (wlcs-side), set_selection from an unfocused source
  (smithay policy), text-input `enter` without an input method (smithay's rule, and wlcs' unflushed
  `get_input_method`), and the touch at y = -5 (a placement rule).
- Interface asks for the smithay fork (`PoHsuanLai/smithay`, branch `casement`): set_anchor_rect to accept
  a zero size; `fractional_scale_exists` and `viewport_exists` raised on the manager object; the
  viewporter pre-commit hook added when `ViewporterSurfaceState` exists but is `None`; `out_of_buffer`
  checked when only `set_source` changes; `PopupPointerGrab::unset` not to unset a keyboard grab that a
  nested popup took with the same serial. Nothing was pushed.
- `dead_focus::a_popup_destroyed_under_the_pointer_gets_no_leave` (P0-dead-focus) asserted no `leave`;
  now the client is told once, at the `xdg_popup`'s destruction, and nothing follows the surface's death.
- Tests: `tests/protocol/popup_grab.rs` (a release or the press it ended grabs; a stale, forged or
  foreign serial does not; a nested grab with the same serial keeps the keyboard; done when another
  window is activated; leave when a popup is destroyed under the pointer; a reactive popup follows and
  another does not; a reposition at the ack; a window in a move loses the pointer and gets it back),
  `tests/protocol/pointer_constraints.rs` (stops at the edge; released and held again with the
  keyboard), `tests/relax.rs` (remap), unit tests in `serial.rs`, `xdg_shell/requests.rs`,
  `seat/hold.rs`. Each failed with the fix disabled, except the remap test (casement needed no change).
