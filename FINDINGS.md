# Findings

Open items and standing facts. An entry names the condition that closes it.

## Stubs behind frozen interfaces

Every `todo!()` in the workspace's sources (270, all from P0-Z2 below), its fill lane and the
ignored test that proves it. Un-ignoring the test is part of the fill. The 27 the P0-Z1
freeze left in casement-core, casement-layout and casement-motion are filled (the P0-F1 lanes
and P0-F1b-model).

### P0-Z1: the pure crates

No stub is left. Ignored tests by lane: P3-Z 1 (`crates/casement-layout/tests/it/spread.rs`, the
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
| `crates/casement-wayland/src/commit_timing.rs` | on_barrier, apply_presented (2) | P0-H1 | `crates/casement-testkit/tests/it/protocol/commit_timing.rs` |
| `crates/casement-wayland/src/compositor.rs` | on_commit, on_destroyed (2) | P0-H1 | `crates/casement-testkit/tests/it/protocol/compositor.rs` |
| `crates/casement-wayland/src/content_type.rs` | on_content_type (1) | P0-H1 | `crates/casement-testkit/tests/it/protocol/content_type.rs` |
| `crates/casement-wayland/src/fifo.rs` | on_barrier, apply_presented (2) | P0-H1 | `crates/casement-testkit/tests/it/protocol/fifo.rs` |
| `crates/casement-wayland/src/fractional_scale.rs` | on_preferred, apply_scale (2) | P0-H1 | `crates/casement-testkit/tests/it/protocol/fractional_scale.rs` |
| `crates/casement-wayland/src/output.rs` | apply_plug, apply_change, apply_unplug (3) | P0-H1 | `crates/casement-testkit/tests/it/protocol/output.rs` |
| `crates/casement-wayland/src/presentation.rs` | on_feedback, apply_presented (2) | P0-H1 | `crates/casement-testkit/tests/it/protocol/presentation.rs` |
| `crates/casement-wayland/src/shm.rs` | on_buffer_destroyed (1) | P0-H1 | `crates/casement-testkit/tests/it/protocol/shm.rs` |
| `crates/casement-wayland/src/subcompositor.rs` | on_new_subsurface (1) | P0-H1 | `crates/casement-testkit/tests/it/protocol/subcompositor.rs` |
| `crates/casement/src/handlers/compositor.rs` | CompositorHandler::commit (1) | P0-H1 | `crates/casement-testkit/tests/it/protocol/compositor.rs` |
| `crates/casement/src/handlers/shm.rs` | BufferHandler::buffer_destroyed (1) | P0-H1 | `crates/casement-testkit/tests/it/protocol/shm.rs` |
| `crates/casement-wayland/src/quire_window.rs` | Dispatch2::request, Dispatch2::destroyed, on_get_window, on_set_first_mouse, on_set_frame, on_set_role, on_set_a11y_root, on_set_edited, on_request_placement, apply_emphasis, apply_placement (12) | P0-H2 | `crates/casement-testkit/tests/it/protocol/quire_window.rs` |
| `crates/casement-wayland/src/system_bell.rs` | on_bell (1) | P0-H2 | `crates/casement-testkit/tests/it/protocol/system_bell.rs` |
| `crates/casement-wayland/src/xdg_activation.rs` | on_token_request, on_activate (2) | P0-H2 | `crates/casement-testkit/tests/it/protocol/xdg_activation.rs` |
| `crates/casement-wayland/src/xdg_decoration.rs` | on_decoration_mode, apply_mode (2) | P0-H2 | `crates/casement-testkit/tests/it/protocol/xdg_decoration.rs` |
| `crates/casement-wayland/src/xdg_dialog.rs` | on_modal (1) | P0-H2 | `crates/casement-testkit/tests/it/protocol/xdg_dialog.rs` |
| `crates/casement-wayland/src/xdg_shell.rs` | on_new_toplevel, on_toplevel_commit, on_move, on_resize, on_maximise, on_minimise, on_full_screen, on_window_menu, on_title, on_app_id, on_parent, on_limits, on_toplevel_destroyed, on_new_popup, on_popup_grab, on_reposition, on_pong, apply_configure, apply_close, apply_ping, apply_popup_done (21) | P0-H2 | `crates/casement-testkit/tests/it/protocol/xdg_shell.rs` |
| `crates/casement-wayland/src/xdg_toplevel_icon.rs` | on_icon (1) | P0-H2 | `crates/casement-testkit/tests/it/protocol/xdg_toplevel_icon.rs` |
| `crates/casement-wayland/src/xdg_toplevel_tag.rs` | on_tag (1) | P0-H2 | `crates/casement-testkit/tests/it/protocol/xdg_toplevel_tag.rs` |
| `crates/casement/src/handlers/quire_window.rs` | QuireWindowHandler::window_of_toplevel (1) | P0-H2 | `crates/casement-testkit/tests/it/protocol/quire_window.rs` |
| `crates/casement/src/handlers/system_bell.rs` | XdgSystemBellHandler::ring (1) | P0-H2 | `crates/casement-testkit/tests/it/protocol/system_bell.rs` |
| `crates/casement/src/handlers/xdg_activation.rs` | XdgActivationHandler::request_activation (1) | P0-H2 | `crates/casement-testkit/tests/it/protocol/xdg_activation.rs` |
| `crates/casement/src/handlers/xdg_decoration.rs` | XdgDecorationHandler::new_decoration, XdgDecorationHandler::request_mode, XdgDecorationHandler::unset_mode (3) | P0-H2 | `crates/casement-testkit/tests/it/protocol/xdg_decoration.rs` |
| `crates/casement/src/handlers/xdg_shell.rs` | XdgShellHandler::new_toplevel, XdgShellHandler::new_popup, XdgShellHandler::grab, XdgShellHandler::reposition_request (4) | P0-H2 | `crates/casement-testkit/tests/it/protocol/xdg_shell.rs` |
| `crates/casement-wayland/src/layer_shell.rs` | on_new_layer, on_layer_commit, on_layer_destroyed, on_layer_popup, apply_layer_configure (5) | P0-H3 | `crates/casement-testkit/tests/it/protocol/layer_shell.rs` |
| `crates/casement-wayland/src/session_lock.rs` | on_lock, on_lock_surface, on_unlock, on_locker_gone, apply_locked, apply_lock_configure (6) | P0-H3 | `crates/casement-testkit/tests/it/protocol/session_lock.rs` |
| `crates/casement/src/handlers/layer_shell.rs` | WlrLayerShellHandler::new_layer_surface (1) | P0-H3 | `crates/casement-testkit/tests/it/protocol/layer_shell.rs` |
| `crates/casement/src/handlers/session_lock.rs` | SessionLockHandler::lock, SessionLockHandler::unlock, SessionLockHandler::new_surface (3) | P0-H3 | `crates/casement-testkit/tests/it/protocol/session_lock.rs` |
| `crates/casement-wayland/src/pointer_constraints.rs` | on_constraint (1) | P0-H4 | `crates/casement-testkit/tests/it/protocol/pointer_constraints.rs` |
| `crates/casement-wayland/src/seat.rs` | on_cursor_image, apply_keyboard_focus, apply_pointer, apply_key, apply_axis, apply_gesture, apply_touch (7) | P0-H4 | `crates/casement-testkit/tests/it/protocol/seat.rs` |
| `crates/casement-wayland/src/serial.rs` | SerialBook::record, SerialBook::answers (2) | P0-H4 | `crates/casement-testkit/tests/it/protocol/seat.rs` |
| `crates/casement-wayland/src/shortcuts_inhibit.rs` | on_inhibit (1) | P0-H4 | `crates/casement-testkit/tests/it/protocol/shortcuts_inhibit.rs` |
| `crates/casement-wayland/src/data_device.rs` | on_selection, on_drag_start, on_drop, apply_selection_focus (4) | P0-H5 | `crates/casement-testkit/tests/it/protocol/data_device.rs` |
| `crates/casement-wayland/src/ext_data_control.rs` | on_control_selection (1) | P0-H5 | `crates/casement-testkit/tests/it/protocol/ext_data_control.rs` |
| `crates/casement-wayland/src/wlr_data_control.rs` | on_control_selection (1) | P0-H5 | `crates/casement-testkit/tests/it/protocol/wlr_data_control.rs` |
| `crates/casement-wayland/src/dmabuf.rs` | on_import, apply_offer (2) | P0-H6 | `crates/casement-testkit/tests/it/protocol/dmabuf.rs` |
| `crates/casement/src/handlers/dmabuf.rs` | DmabufHandler::dmabuf_imported (1) | P0-H6 | `crates/casement-testkit/tests/it/protocol/dmabuf.rs` |
| `crates/casement-wayland/src/input_method.rs` | on_im_commit, on_im_popup (2) | P0-H7 | `crates/casement-testkit/tests/it/protocol/input_method.rs` |
| `crates/casement-wayland/src/text_input.rs` | on_enable, on_commit_state, apply_text_focus, apply_preedit, apply_commit_string (5) | P0-H7 | `crates/casement-testkit/tests/it/protocol/text_input.rs` |
| `crates/casement-wayland/src/virtual_keyboard.rs` | on_vk_key (1) | P0-H7 | `crates/casement-testkit/tests/it/protocol/virtual_keyboard.rs` |
| `crates/casement/src/handlers/input_method.rs` | InputMethodHandler::new_popup, InputMethodHandler::dismiss_popup, InputMethodHandler::popup_repositioned, InputMethodHandler::parent_geometry (4) | P0-H7 | `crates/casement-testkit/tests/it/protocol/input_method.rs` |
| `crates/casement-wayland/src/background_effect.rs` | on_blur_region (1) | P0-H8 | `crates/casement-testkit/tests/it/protocol/background_effect.rs` |
| `crates/casement-wayland/src/foreign_toplevel_list.rs` | apply_toplevel, apply_closed (2) | P0-H8 | `crates/casement-testkit/tests/it/protocol/foreign_toplevel_list.rs` |
| `crates/casement-wayland/src/idle_inhibit.rs` | on_inhibit, on_uninhibit (2) | P0-H8 | `crates/casement-testkit/tests/it/protocol/idle_inhibit.rs` |
| `crates/casement-wayland/src/idle_notify.rs` | on_notification, apply_idle, apply_resumed (3) | P0-H8 | `crates/casement-testkit/tests/it/protocol/idle_notify.rs` |
| `crates/casement-wayland/src/security_context.rs` | on_create_listener (1) | P0-H8 | `crates/casement-testkit/tests/it/protocol/security_context.rs` |
| `crates/casement/src/handlers/idle_inhibit.rs` | IdleInhibitHandler::inhibit, IdleInhibitHandler::uninhibit (2) | P0-H8 | `crates/casement-testkit/tests/it/protocol/idle_inhibit.rs` |
| `crates/casement/src/handlers/security_context.rs` | SecurityContextHandler::context_created (1) | P0-H8 | `crates/casement-testkit/tests/it/protocol/security_context.rs` |
| `crates/casement-backend/src/headless/cost.rs` | RenderCostModel::cost_of (1) | P0-B-headless | `crates/casement-testkit/tests/it/fixture.rs`; input through `tests/it/protocol/seat.rs` |
| `crates/casement-backend/src/headless/input.rs` | HeadlessInput::drain (1) | P0-B-headless | `crates/casement-testkit/tests/it/fixture.rs`; input through `tests/it/protocol/seat.rs` |
| `crates/casement-backend/src/headless/mod.rs` | HeadlessBackend::start, HeadlessBackend::plug, HeadlessBackend::unplug, HeadlessBackend::set, HeadlessBackend::last_frame, HeadlessBackend::input (6) | P0-B-headless | `crates/casement-testkit/tests/it/fixture.rs`; input through `tests/it/protocol/seat.rs` |
| `crates/casement-backend/src/headless/renderer.rs` | HeadlessRenderer::start (1) | P0-B-headless | `crates/casement-testkit/tests/it/fixture.rs`; input through `tests/it/protocol/seat.rs` |
| `crates/casement-backend/src/input/virtual_events.rs` | Event::time, Event::device, KeyboardKeyEvent::key_code, KeyboardKeyEvent::state, KeyboardKeyEvent::count, PointerMotionEvent::delta_x, PointerMotionEvent::delta_y, PointerMotionEvent::delta_x_unaccel, PointerMotionEvent::delta_y_unaccel, AbsolutePositionEvent::x, AbsolutePositionEvent::y, AbsolutePositionEvent::x_transformed, AbsolutePositionEvent::y_transformed, PointerButtonEvent::button_code, PointerButtonEvent::state, PointerAxisEvent::amount, PointerAxisEvent::amount_v120, PointerAxisEvent::source, PointerAxisEvent::relative_direction, GestureBeginEvent::fingers, GestureEndEvent::cancelled, GestureSwipeUpdateEvent::delta_x, GestureSwipeUpdateEvent::delta_y, GesturePinchUpdateEvent::delta_x, GesturePinchUpdateEvent::delta_y, GesturePinchUpdateEvent::scale, GesturePinchUpdateEvent::rotation, TouchEvent::slot, SwitchToggleEvent::switch, SwitchToggleEvent::state (34) | P0-B-headless | `crates/casement-testkit/tests/it/fixture.rs`; input through `tests/it/protocol/seat.rs` |
| `crates/casement-backend/src/input/virtual_input.rs` | VirtualInput::plug, VirtualInput::unplug, VirtualInput::push, Device::id, Device::name, Device::has_capability, Device::usb_id, Device::syspath (8) | P0-B-headless | `crates/casement-testkit/tests/it/fixture.rs`; input through `tests/it/protocol/seat.rs` |
| `crates/casement-testkit/src/fakes/session.rs` | Session::open, Session::close, Session::change_vt, Session::is_active, Session::seat (5) | P0-B-headless | `crates/casement-testkit/tests/it/fixture.rs`; input through `tests/it/protocol/seat.rs` |
| `crates/casement-testkit/src/fakes/udev.rs` | FakeUdev::next_event (1) | P0-B-headless | `crates/casement-testkit/tests/it/fixture.rs`; input through `tests/it/protocol/seat.rs` |
| `crates/casement-testkit/src/fixture.rs` | Fixture::start, Fixture::connect, Fixture::client, Fixture::pump, Fixture::advance, Fixture::frames, Fixture::run_until, Fixture::shot, Fixture::scene, Fixture::model (10) | P0-B-headless | `crates/casement-testkit/tests/it/fixture.rs`; input through `tests/it/protocol/seat.rs` |
| `crates/casement-testkit/src/input.rs` | FixtureInput::plug, FixtureInput::key, FixtureInput::pointer_to, FixtureInput::button, FixtureInput::axis, FixtureInput::swipe, FixtureInput::pinch, FixtureInput::hold, FixtureInput::touch, FixtureInput::switch (10) | P0-B-headless | `crates/casement-testkit/tests/it/fixture.rs`; input through `tests/it/protocol/seat.rs` |
| `crates/casement-testkit/src/output.rs` | Fixture::plug, Fixture::unplug (2) | P0-B-headless | `crates/casement-testkit/tests/it/fixture.rs`; input through `tests/it/protocol/seat.rs` |
| `crates/casement-backend/src/winit.rs` | WinitBackend::start (1) | P0-B-loop | builds only (never run by an agent; the user's nested-winit check) |
| `crates/casement/src/core_loop.rs` | feed, frame (2) | P0-B-loop | `crates/casement/tests/it/time.rs`; frames through `crates/casement-testkit/tests/it/fixture.rs` |
| `crates/casement/src/display.rs` | attach_display, flush (2) | P0-B-loop | `crates/casement/tests/it/time.rs`; frames through `crates/casement-testkit/tests/it/fixture.rs` |
| `crates/casement/src/exec.rs` | exec (1) | P0-B-loop | `crates/casement/tests/it/time.rs`; frames through `crates/casement-testkit/tests/it/fixture.rs` |
| `crates/casement/src/run.rs` | run (1) | P0-B-loop | `crates/casement/tests/it/time.rs`; frames through `crates/casement-testkit/tests/it/fixture.rs` |
| `crates/casement/src/time/calloop.rs` | arm (1) | P0-B-loop | `crates/casement/tests/it/time.rs`; frames through `crates/casement-testkit/tests/it/fixture.rs` |
| `crates/casement/src/time/source.rs` | ClockSource::start, ClockSource::now (2) | P0-B-loop | `crates/casement/tests/it/time.rs`; frames through `crates/casement-testkit/tests/it/fixture.rs` |
| `crates/casement/src/time/virtual_clock.rs` | SharedClock::update, SharedClock::advance_to, EventSource::process_events, EventSource::before_sleep (4) | P0-B-loop | `crates/casement/tests/it/time.rs`; frames through `crates/casement-testkit/tests/it/fixture.rs` |
| `crates/casement-testclient/src/bin/probe.rs` | main (1) | P0-T-client | `crates/casement-testclient/tests/infra.rs`; `crates/casement-testkit/tests/it/globals.rs`; `crates/casement-testkit/tests/fuzz/main.rs` |
| `crates/casement-testclient/src/conn.rs` | TestClient::over, TestClient::connect_env, TestClient::pump (3) | P0-T-client | `crates/casement-testclient/tests/infra.rs`; `crates/casement-testkit/tests/it/globals.rs`; `crates/casement-testkit/tests/fuzz/main.rs` |
| `crates/casement-testclient/src/shm.rs` | shm_buffer (1) | P0-T-client | `crates/casement-testclient/tests/infra.rs`; `crates/casement-testkit/tests/it/globals.rs`; `crates/casement-testkit/tests/fuzz/main.rs` |
| `crates/casement-testkit/src/fuzz/ops.rs` | run_ops (1) | P0-T-client | `crates/casement-testclient/tests/infra.rs`; `crates/casement-testkit/tests/it/globals.rs`; `crates/casement-testkit/tests/fuzz/main.rs` |
| `crates/casement-ipc/src/dbus/debug1.rs` | Debug1::inject, Debug1::wait, Debug1::clock, Debug1::output, Debug1::windows, Debug1::scene, Debug1::fault (7) | P0-T-dev | `crates/casement-ipc/src/inject.rs`, `wait.rs` tests; `crates/casementctl/src/command.rs` test; `dev/accept.sh --jobs 8` |
| `crates/casement-ipc/src/inject.rs` | parse_inject (1) | P0-T-dev | `crates/casement-ipc/src/inject.rs`, `wait.rs` tests; `crates/casementctl/src/command.rs` test; `dev/accept.sh --jobs 8` |
| `crates/casement-ipc/src/wait.rs` | parse_wait (1) | P0-T-dev | `crates/casement-ipc/src/inject.rs`, `wait.rs` tests; `crates/casementctl/src/command.rs` test; `dev/accept.sh --jobs 8` |
| `crates/casementctl/src/command.rs` | parse, run (2) | P0-T-dev | `crates/casement-ipc/src/inject.rs`, `wait.rs` tests; `crates/casementctl/src/command.rs` test; `dev/accept.sh --jobs 8` |
| `crates/casement-testkit/src/fixture.rs` | Fixture::session (1) | P1-D1 | P1-Z's session suite |
| `crates/casement-testkit/src/bin/fake_logind.rs` | main: no longer a `todo!()` (P0-accept): it prints "P1-F2: not built" and exits 0, so the headless session starts without it | P1-F2 | P1-Z's logind tests |
| `crates/casement-testkit/src/fakes/logind/mod.rs` | FakeLogind::serve (1) | P1-F2 | `crates/casement-testkit/tests/it/logind.rs` |
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
| `CasementSettings::defaults` carries no bindings (C30 says the settings document holds the defaults) | the shell's chords are not defaults of the settings document: c17 reads them from the desktop keymap (`casement::keymap`, see the c17 rows below); `settings/bindings.rs` stays empty and a `[bindings]` entry is only an addition or a win over the keymap for one chord |
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
5. **Layer placement has one home (closed by P0-reserve).** `casement_core::layer::LayerBook::arrange`
   (`crates/casement-core/src/layer/arrange.rs`) arranges layer surfaces once (exclusive zones,
   overlay down to background, then the rest inside what they leave); the renderer, the hit test
   and the configure sizes all read it.
6. **Named cursors are casement's own rasters** (signed-distance shapes, 16 samples a pixel,
   at the output's native scale, cached per shape and scale); no Xcursor theme is read yet.
   A locked output draws only the cursor, its lock surface and an opaque cover under it. The
   cover's id is thread-local and the cover's and blur's commits are content fingerprints, so a
   still frame has empty damage. The renderer tests read `CASEMENT_BLESS` with `Env`'s rule in
   `tests/support/mod.rs`, since casement-render cannot depend on the test kit.

## Departures in the P0-shoot fill

P0-shoot wrote the fixture scenes (`crates/casement-testkit/tests/it/scene/`: `the_ten_base_scenes_match_their_pixman_goldens`,
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
toplevel's own surface; `tests/it/protocol/shoot.rs` holds the helper the adjunct tests share.
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
- **`serve_debug_at` takes Debug1's name with replacement off** (P0-debug-dbus): a second casement
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

The root and `wlcs-casement` pins (and both `Cargo.lock`s) named the fork's tip `576cfd954b6bd5badbd5f15d1708cafc941a0522` here; P0-smithay moved them (see its section below). The
`[profile.dev.package.smithay] debug-assertions = false` override is removed: smithay's debug assertions are on in
every dev build and the fuzz box (24 cases) passes with them.

### P0-smithay: the fork's text-input, W3 and popup-grab patches

The fork branch is now `casement-textinput` (from `casement` at `576cfd95`; the `casement` branch is untouched). The pins (root
`Cargo.toml`, both `Cargo.lock`s) name its tip `dc7ca684e0a75b2c889718e5e97cc49bc94760ba`. Nine commits, each its own protocol rule:

- `579ac4b7` text_input: `enter`/`leave` follow keyboard focus with or without an input method; a text input created while its
  client holds the focus gets `enter` at once (on that object only).
- `690afd1f` input_method: `unavailable` goes to the newcomer, not the bound input method.
- `19d90eb2` text_input: requests are taken without an input method and the committed state (surrounding text, change cause, content
  type, cursor rectangle) is kept; an input method that binds later is activated for the enabled field with that state and a `done`;
  an input method that goes away leaves the field enabled (no event exists for it, `leave` would disable it); the active text input's
  destruction deactivates the input method and frees the active slot.
- `78abd7d6` xdg_positioner: `set_anchor_rect` accepts a zero size.
- `a0eda85e` fractional_scale and viewporter: the `*_exists` errors are raised on the manager.
- `b63854af` viewporter: the `bad_size` pre-commit hook is installed even when the surface state exists already.
- `999aac58` renderer utils: buffer scale and transform apply at every commit, not only with a new buffer.
- `66dd1e8e` viewporter: `ensure_viewport_valid` checks `out_of_buffer` and then `bad_size` at each commit (a source both fractional
  and outside the buffer is `out_of_buffer`, as wlcs expects); the hook covers commits before a buffer.
- `dc7ca684` popup grab: `PopupPointerGrab::unset` leaves a keyboard grab that another popup grab owns (grab ids). casement's
  workaround (lift the keyboard grab before replacing the pointer grab) is gone: `handlers/xdg_shell/popups.rs` now sets the
  keyboard grab, then the pointer grab.

casement side: `apply_text_focus` sends `leave`/`enter` without the input-method condition; the test kit's `ImeSession` can start
without an input method (`start_field`, `bind_method`, `destroy_method`, `enable_with_state`) and sees `content_type`. Tests added,
each failing against the old pin: `text_input::an_input_method_that_starts_after_the_field_is_enabled_gets_the_field`,
`text_input::a_field_stays_enabled_when_its_input_method_goes_away`, `popup_input::a_popup_may_anchor_to_a_point`,
`viewporter::a_viewport_made_after_the_first_commit_is_checked_for_bad_size`,
`viewporter::a_scale_change_without_a_new_buffer_is_checked_for_out_of_buffer`; `fractional_scale::a_second_scale_object_is_a_protocol_error`
and `viewporter::a_second_viewport_is_a_protocol_error` now expect the manager as the error object; the existing
`popup_grab::a_nested_grab_with_the_same_serial_keeps_the_keyboard` fails against the old pin without the workaround and passes with the fix.

wlcs `--phase`, before: 857 pass, 37 listed, 331 skip; after: 871 pass, 23 listed, 331 skip. Removed from the list: the zero-size
anchor, the fractional-scale and the three viewporter tests (W3), nine of the eleven text-input tests (W12). Left in W12:
`text_input_enters_grabbing_popup` and `text_input_enters_parent_surface_after_child_destroyed` run under gmock `InSequence` and expect
`enter` before the toplevel's first configure; the shim's lenient policy keys a window in the commit that maps it, after that configure.
The remaining W3 rows are the four `xdg_wm_base` error tests (smithay raises no error for `get_xdg_surface` on a surface with a role or
a buffer, and attaching to an unconfigured surface is the A37 relaxation).

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
- Tests: `tests/it/protocol/dead_focus.rs` (toplevel, layer surface, popup, hung-up client, touch),
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
- Tests: `tests/it/protocol/compositor.rs::outputs_of_a_surface` (map, change of output both ways,
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
- Tests: `tests/it/protocol/geometry.rs` (pointer-local at a geometry offset, drawn pixel at a geometry
  offset, configure sizes, placement by the configured box, open maximised / full screen, a layer put by
  hand, `set_layer` stacking, a layer popup kept inside the output), `tests/it/protocol/layer_keyboard.rs`,
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
- Tests: `tests/it/protocol/popup_grab.rs` (a release or the press it ended grabs; a stale, forged or
  foreign serial does not; a nested grab with the same serial keeps the keyboard; done when another
  window is activated; leave when a popup is destroyed under the pointer; a reactive popup follows and
  another does not; a reposition at the ack; a window in a move loses the pointer and gets it back),
  `tests/it/protocol/pointer_constraints.rs` (stops at the edge; released and held again with the
  keyboard), `tests/relax.rs` (remap), unit tests in `serial.rs`, `xdg_shell/requests.rs`,
  `seat/hold.rs`. Each failed with the fix disabled, except the remap test (casement needed no change).

## c17: shell bindings from the keymap (2026-10-11)
- **Bindings come from the keymap, not a file or a table** (AUDIT (e)3): before, `settings.bindings` was empty by default
  and held only what a `[bindings]` entry in `settings.toml` said, so a fresh casement bound nothing.
  `casement::keymap::attach` (called from `main.rs` after the outputs are announced) loads
  `KeycapSource::load_reporting(Linux/Ours)` (chordkit conventions, the desktop's own bindings, the machine
  keymap, the person's keymap), maps each standard action the compositor carries out (`keymap/shell.rs`
  `ROWS`: force quit, Space left/right/1-9,
  and sill's launcher, switcher, screenshots and power menu as `sill ...` spawns) to a `BindingAction`, converts
  each chord (`keymap/chord.rs`: Super is Command, Alt is Option; chordkit's `+` and `?` become shifted `=` and
  `/`) and appends the result after the document's own bindings. No `dist/casement/bindings.toml` ships:
  the action vocabulary is chordkit's `StandardAction`, so the contract with keycap is action names already.
- **Reload**: `KeycapSource::watch` (daemon message, or the keymap files polled) sends on a calloop channel; the
  handler recomputes the bindings and feeds `Event::SettingsChanged` only when they differ. The watch lives in
  the handler's closure. Under `ConfigHome::At(dir)` (tests, the jail) the source reads only inside `dir`
  (`keycap/keymap` for the person, `keycap-machine/keymap` for the machine, a socket path that does not exist).
- **Known gaps**: `Spawn` and `Call` bindings are still only logged by `carry_out` ("from P2"), so the launcher,
  switcher, screenshot and power-menu chords resolve and are swallowed but do nothing yet. LockScreen,
  MissionControl, AppWindows, NextWindow, NextInputSource and EmojiAndSymbols have no casement or sill command
  and are not bound. `ScreenshotTools` runs `sill screenshot area` (dist/cosmic's Screenshot).
- **Mac semantics (follow-up)**: Cmd+W, Cmd+M, Cmd+H, Cmd+Opt+H, Cmd+Q and Ctrl+Cmd+F are app menu commands on macOS,
  not window-server ones, so the focused app must receive them. Close, Minimise, Hide, HideOthers, Quit and FullScreen
  are not in `ROWS` (a test asserts none of those chords is bound), and `DesktopCommand::ToggleFullScreenKey`, added
  for full screen and used by nothing else, is dropped. ForceQuit (Cmd+Opt+Esc, system-level), Space switching and
  the sill hand-offs stay.
- **Dependencies**: `chordkit` (rev ba094c62) and `keycap-client` (rev 95f41dc4) are git dependencies of the
  workspace and of `casement`; `Cargo.lock` must be regenerated (`cargo metadata`) with network access.
- Caps Lock no longer blocks bindings: `deliver::chord_mods` leaves it out of the chord (Num Lock was never in `MacMods`).

## c17: idle-inhibit and a client that hangs up (2026-10-11)
- AUDIT (e)5 / the H6-H8 QUIRK note ("idle-inhibit never uninhibits on client disconnect") was already worked
  around by P0-wire: `core_loop::sync_inhibitors` runs on every `feed` (so on `Event::ClientGone`) and
  `IdleInhibitPart::inhibited_where` counts only live surfaces. What was missing: a test, and the dead surface
  stayed in `IdleInhibitPart::inhibitors` for the rest of the session. c17 adds
  `idle_inhibit::a_client_that_disconnects_stops_inhibiting` and `IdleInhibitPart::forget_dead`
  (called from `on_inhibit`, `on_uninhibit` and `sync_inhibitors`).
- Unverified (no builds this lane): the new test has not run. If it fails, the dead-surface check is not
  reached at the disconnect (look at the order of `ClientState::disconnected` and the object clean-up in
  wayland-backend), and the fix is to call `sync_inhibitors` from `exec::resync_keyboard`'s caller in `turn`.

## c18: library shape, Step L (2026-10-11)
- F4: `casement::run(RunConfig) -> Result<(), RunError>` holds the startup and the loop; `RunError` is typed
  (`StartError`, `calloop::Error`, wayland `InitError`, `BindError`, `IpcError`, `JailRefusal`) and maps to the exit
  code with `RunError::exit_code` (5 for a jail breach). The Debug1 server (`debug.rs`) moved into the lib behind
  `debug-inject`. `main.rs` reads `CASEMENT_LOG` and `DBUS_SESSION_BUS_ADDRESS` and nothing else.
- F3: `serve_debug` is gone (callers use `serve_debug_at`); `log::init(filter)` takes the filter.
- F2: `casement-ipc`'s `casement-backend` is optional and comes with `debug-inject`.
- F1, F9: `BackendError` has typed sources (`SpecError`, smithay's window, swap, gles and pixman errors,
  `RenderError`, `DeniedOpen { path, source }`); the test kit returns `FuzzFailure` (one variant per oracle) and
  `LogindError`.
- F5: `#[non_exhaustive]` on `Event`, `Effect`, `Cause`, `BackendKind` and every public error enum. In-repo matches
  from other crates gained a fallback arm (`exec::apply`, `cause_travel`, `core_loop` present and announce, the
  fixture's backend accessors). Not marked: structs (the config and spec structs are built by literal across crates;
  they get constructors first) and closed vocabularies.
- F6: the cursor rasters and the lock cover's id live in `TextureBook` (`draw_cache.rs`); the A37 ack policy is
  `State::ack_policy` set from `FixtureSpec::with_ack_policy`, so `relax.rs` has no static.
- F8: `Pull` (grab resize sides), `TreeKind` (presence), `Visibility` (layout) and `fate_of(Option<_>)` replace the
  listed bools. `convert::toggle_of(bool)` stays: it is the one place a smithay `bool` becomes `Toggle`
  (CONVENTIONS 3), and `DndGrabHandler::dropped(_validated: bool)` is smithay's own signature.
- F12, F13: crate docs with a doc test on core, layout, motion, render, backend and wayland; the `casement` lib doc
  says how to embed the compositor.
- Raster `expect`s: they sat in `raster.rs`'s `#[cfg(test)]` module, not on a library path; they are checked
  conversions now anyway.
- Unverified (no builds this lane): none of this has compiled. First places to look: `run.rs` (the `cfg` arms and the
  `#[from]` set), `casement-backend/src/backend.rs` (the `#[from]` error types' paths) and `headless/renderer.rs`
  (`Into<BackendError>` bound on `draw_with`), `casement-testkit/src/fuzz/*`.

## p1z: the P1 delta freeze (2026-10-11)

P1-Z adds the tty, GPU, scheduler, protocol and seam surface that the P1 fill lanes write bodies for. Every item
below has a signature, a doc, a `todo!("<lane>: …")` body and a test written and `#[ignore = "<lane>"]`d. Nothing
here compiled (no builds this lane). `todo!()` in sources by lane: P1-D1 12, P1-D2 28, P1-D3 12, P1-D4 12,
P1-M1 11, P1-M2 20, P1-I1 8, P1-X1 2, P1-C1 7, P1-F2 9; ignored test bodies: D1 2, D2 6, D3 4, D4 4, M1 7, M2 6,
C1 3, F2 3, plus I1 3 and X1 3 whose bodies are `todo!()` after a first assertion. (D1's count includes
`Fixture::session`, which P0-Z2 left; the P0-Z2 `fakes/udev.rs` and `fakes/session.rs` bodies stay implemented.)

### Where each lane works (files; a lane edits only its own)

| Lane | Source files | Fake and test files |
| --- | --- | --- |
| P1-D1 session, udev, VT | `crates/casement-backend/src/tty/{session,session_real,udev,udev_real,vt,real}.rs`, `TtyBackend::{start, dispatch}` in `tty/mod.rs` | `crates/casement-testkit/src/fakes/{session,udev}.rs`, `crates/casement-testkit/tests/it/tty/session.rs` |
| P1-D2 KMS, hotplug, modes | `crates/casement-backend/src/tty/{kms,kms_real,device,connector,hotplug}.rs`, `TtyBackend::announce`; core `output/{mode_pick,arrangement}.rs` are already filled | `crates/casement-testkit/src/fakes/kms/`, `crates/casement-testkit/fixtures/kms/`, `crates/casement-testkit/tests/it/tty/kms.rs`, `dev/hw/` |
| P1-D3 scheduler, scanout | `crates/casement-backend/src/tty/{surface,surface_real,flip}.rs`, `TtyBackend::present`; `crates/casement-core/src/frame/` | `crates/casement-testkit/src/fakes/scanout.rs`, `crates/casement-testkit/tests/it/tty/scanout.rs`, `crates/casement-core/tests/it/frame.rs` |
| P1-D4 GPU plan, multi-GPU | `crates/casement-core/src/gpu/`, `crates/casement-backend/src/gpu/{render_nodes,render_nodes_real,manager,import,feedback}.rs`, `crates/casement-wayland/src/dmabuf.rs` and its handler (tag with `set_node`, A5) | `crates/casement-testkit/src/gpu/{soft_api,sys_tree,topology}.rs`, `crates/casement-testkit/tests/it/topology/mod.rs`, `crates/casement-core/tests/it/gpu.rs` |
| P1-M1 output management, power, gamma | `crates/casement-wayland/src/{output_management,output_power,gamma}.rs`, `crates/casement/src/handlers/{output_management,output_power,gamma}.rs`, `crates/casement/src/exec/gamma.rs`, `crates/casement-core/src/model/output_requests.rs`, `crates/casement-core/src/output/request.rs` | `crates/casement-testkit/tests/it/protocol/{output_management,output_power,gamma}.rs`, `crates/casement-core/tests/it/output_request.rs`, `crates/casement-testclient/src/manage/{output_management,output_power,gamma}.rs` |
| P1-M2 wlr toplevel, workspaces, Space ids | `crates/casement-wayland/src/{wlr_toplevel,workspace}.rs`, `crates/casement/src/handlers/{wlr_toplevel,workspace}.rs`, `crates/casement/src/exec/publish.rs`, `crates/casement/src/space_store.rs`, `crates/casement-core/src/space/{persist,request}.rs`, `crates/casement-core/src/model/{wlr_requests,workspace_requests,space_persist}.rs` | `crates/casement-testkit/tests/it/protocol/{wlr_toplevel,workspace}.rs`, `crates/casement-core/tests/it/space_persist.rs`, `crates/casement/tests/it/space_store.rs`, `crates/casement-testclient/src/manage/{wlr_toplevel,workspace}.rs` |
| P1-I1 input, bindings | `crates/casement-backend/src/input/{devices,config,libinput_real}.rs`, `crates/casement-ipc/src/shell_calls.rs`, `crates/casement/src/exec/{call,spawn}.rs`, `crates/casement/src/spawn.rs` (the real `ProcessSpawner`), `settings/pointer.rs` wiring | `crates/casement-testkit/tests/it/bindings.rs`, `crates/casement-backend/tests/it/input_config.rs` |
| P1-X1 XWayland | `crates/casement/src/xwayland.rs` | `crates/casement-testclient/src/bin/x11probe.rs`, `crates/casement-testkit/tests/it/xwayland.rs` |
| P1-C1 output capture | `crates/casement-wayland/src/image_capture.rs`, `crates/casement/src/handlers/image_capture.rs`, `crates/casement-render/src/pass/capture.rs` | `crates/casement-testkit/tests/it/protocol/image_capture.rs`, `crates/casement-testclient/src/capture/` |
| P1-F2 fake logind | `crates/casement-testkit/src/fakes/logind/` (mod, manager, session, seat), `crates/casement-testkit/src/bin/fake_logind.rs` | `crates/casement-testkit/tests/it/logind.rs` |
| P1-SESS | none in this repo | none |

Already implemented at this freeze, not stubs: `MemorySpaceStore`, `FakeShellCalls`, `FakeSpawner`,
`FakeInputDevices` and `FakeUdev` (small recorders), `Seams`, `StateDirs`, `EphemeralSpaceStore`, `XwaylandConfig`,
the `--xwayland` flags, `VendorFact::from_pci_vendor`, `GammaTable::new`, `OutputChangeSet`, `PointerSettings::defaults`,
`FakeSession`'s `TtySession::{open_device, switch_vt}`.

### Departures from 20-FREEZE-PLAN §4.15 and 22

- **Not advertised.** The six new globals are declared (`GlobalKind` variants with their I7 rows: sandboxed clients
  see none; `WaylandStates` fields; `Part::new` calls in `State::new`; handler files; testkit protocol files) and none is
  advertised: each `Part::new` creates no global and returns the empty part. `globals.txt` and the goldens are
  unchanged. The lane that fills a protocol creates the global in `Part::new`, adds its `globals.txt` row and
  un-ignores its tests.
- **The new `Event`/`Effect` variants are emitted by no model path yet.** Effects: `CallBinding`, `Spawn`, `SetGamma`,
  `PersistSpaces`, `PublishWorkspaces` and one beyond the plan, `PublishToplevel(WindowId)` (wlr handles need a state
  push; `ListWindow` is the ext list's). Events: `OutputRequested`, `GammaSet`, `WlrToplevel`, `Workspace`,
  `SpacesLoaded`. Their `exec` arms and `Model::route` arms exist and reach `todo!()` bodies that nothing calls yet:
  zero `todo!()` on the headless path holds.
- **`binding` actions.** `core_loop/deliver.rs::carry_out` still logs `BindingAction::Call|Spawn` ("from P2"); P1-I1
  replaces the log with `exec::exec(state, vec![Effect::CallBinding(call.clone())])` and `Effect::Spawn(argv.clone())`.
  Left alone so a stray `[bindings]` call entry cannot reach a `todo!()`.
- **`BackendKind::Tty`.** `core_loop::frame` and `announce_outputs` have their arms (`TtyBackend::{present, announce}`); they
  are unreachable because `State::new` still answers `StartError::NotYet` for `--backend tty`. P1-D1 to D3 start the
  backend there and add the loop turn that drains `TtyBackend::dispatch` (connector events into the existing
  `core_loop` connector path, `SessionNote::Paused` stops frames) in `run.rs`.
- **`RenderPlan::scanout` is keyed by `OutputName`, not `OutputKey`** (the sketch says `OutputKey`; detection knows
  connector names, and a key needs the EDID). `GpuFacts::drives` is `Vec<OutputName>` as sketched.
- **`Kms::set_gamma`** is added to the sketch's `Kms` (the `SetGamma` effect needs a destination).
- **`SessionSeam::Libseat(LibseatSlot)` is untouched** (a unit slot). The tty backend takes its session as
  `TtySession`, which `FakeSession` and the real `RealSession` implement; smithay's `Session` stays the fake's own
  interface. The plan's "LibseatSlot filled from `session_real.rs`" is therefore moot: P1-D1 may delete the slot.
- **Real adapters are opaque at the freeze** (`RealKms { fd }`, `RealScanout { connector, crtc }`, …): they name no
  device-crate type, so nothing here depends on an API this lane could not check; each lane adds the fields.
- **Features (A2).** `casement-backend`: `tty` (forwards smithay's `backend_drm`, `backend_gbm`, `backend_udev`,
  `backend_session_libseat`) and `libinput` (`backend_libinput`), both off by default; `renderer_multi` is now always on
  there. `casement`: `tty` and `libinput` forward to those, are the default, and the binary has `required-features =
  ["tty", "libinput"]`; the casement dependency of the workspace and of `wlcs-casement` has `default-features =
  false`. No optional crate was added: smithay's features bring the device crates, already in the lock through the
  union the `casement` crate carried. The device features left `casement`'s own smithay list.
- **Cargo.lock needs regenerating** (nothing here could): `casement-core` gains `serde_json` and `toml` as normal
  dependencies (the Space file codec; they were dev-dependencies), `casement-backend` gains `serde` (KMS fixtures are
  serde values), `casement-testkit` gains `casement-ipc`. `casement-testclient` gains only a bin target.
- **`SpaceFile`'s sill import** reads `by_id` keys of the interim `quire/spaces.json`, because sill has no concrete writer yet
  (`sill-ui-kit/src/look.rs` says so); the shape is a guess P1-M2 pins against whatever sill wrote by then.
- **Settings.** `PointerSettings` (mouse and touchpad profiles) exists in core and is not yet a field of
  `CasementSettings` nor a section of the document: P1-I1 adds both with the "Add a settings key" recipe.
  The Xwayland mode and satellite path are `StartConfig` fields set by `--xwayland` and `--xwayland-satellite`
  (A5: configuration, never the environment); they move to the document if the Settings app should draw them.
- **`StartConfig` gained `state_home` and `xwayland`; `State` gained `seams` and `xwayland`.** `State::new` builds the
  real seams, `State::with_seams` takes injected ones. `main` resolves `$XDG_STATE_HOME` (or `$HOME/.local/state`) for
  every backend but headless. Loading the Spaces file at start (`Event::SpacesLoaded`) is P1-M2's, in `run.rs`.
- **The test kit.** `FixtureSpec` gained `space_store`; `Fixture` exposes `space_store()`, `shell_calls()` and
  `spawner()`; `fakes/udev.rs` now speaks `casement_backend::tty::DeviceEvent`; `gpu::VendorFact` is core's type;
  `SoftGpu` is `#[non_exhaustive]` with `SoftGpu::new` and a `drives` field.
- **Unverified (no builds this lane).** First places to look if it does not compile: `casement-core/src/space/persist.rs`
  (toml 1 error type paths), `casement-backend/src/tty/kms.rs` and `hotplug.rs` (serde derives over `Mode`,
  `PhysicalMm`, `OutputName`), `casement-backend/src/backend.rs` (the three new `#[from]` sources),
  `casement/src/state.rs` and `exec.rs` (module and import names `gamma`, `workspace`, `spawn`), the doc tests (all
  `no_run`), `casement-testkit/src/fixture.rs`, `fakes/session.rs` (`TtySession` over smithay's `Session`), and
  `casement-render/src/pass/capture.rs` (the `ExportMem` bound with `R::Framebuffer<'_>`).

## p1-m2: wlr toplevel, workspaces, Space id persistence (2026-10-11)

Nothing here compiled (no builds this lane). Both globals are advertised: `zwlr_foreign_toplevel_manager_v1` v3 and
`ext_workspace_manager_v1` v1, hidden from sandboxed clients (I7 rows unchanged), with `globals.txt` rows.

- **Publishing is a diff, not an emission at each transition.** `Model::run` takes a `Published` snapshot before a step
  and compares it after (`model/published.rs`): `PublishToplevel(id)` for each window whose title, app id, state kind, key
  flag or output changed (or that appeared), `PublishWorkspaces` when the `SpaceBook` changed, `PersistSpaces` when the
  stored file (`Model::space_file`) changed. No transition had to learn the effects; the cost is one clone of the Space
  book and a small record per window per step. Closing is `UnlistWindow`, whose exec arm now also calls
  `wlr_toplevel::apply_closed`. Core tests that assert an exact effect list on a step that changes a window's key flag
  (`model_lock.rs` 218-242 if a key window exists there) would see a `PublishToplevel` too.
- **`import_sill_json`: sill writes no spaces file.** `sill-ui-kit/src/look.rs` still says quire has no `spaces.json`
  store, and `sill-model` only reads the protocol's `id` as `stable_id`. The shape imported is the minimal one the codec
  pins: `{"by_id": {"<space id>": {"output": "<output key>", "index": 0}}}`, both record fields optional, unknown keys
  ignored. A record without an output is `StoredSpace::homeless` (so `StoredSpace::output` is now `Option<OutputKey>`;
  `StoredSpace::new` still takes one) and the first output plugged adopts it. Nothing reads the file at start: the path
  is sill's to name; wire `import_sill_json` where `load_stored_spaces` (`core_loop.rs`) loads `spaces.toml` once sill
  writes one.
- **Loading.** `core_loop::announce_outputs` reads the file through the seam first and feeds `Event::SpacesLoaded` (the
  headless fixture and `run.rs` both call it once, so one edit covers both; FINDINGS p1z said `run.rs`). The model keeps
  the file as `remembered`. A plugged output adopts its stored Spaces (`Model::adopt_ids`): the desktops it got by
  default are renamed in row order to the stored ids (windows follow), extra stored ones are made, and the stored
  "shown" choice is applied. A Space whose output is not plugged waits and stays in what `space_file` writes. A file that
  does not parse is logged and the run goes on with fresh ids.
- **Restart test.** The fixture's entropy is fixed, so an unchanged restart would mint the same ids with no file at all.
  `the_workspace_id_survives_a_restart` edits the stored text to an id the entropy never mints and checks the protocol
  shows that one.
- **Departures.** (1) `apply_toplevel` takes the window's smithay `Output`s (`&[Output]`) and `apply_spaces` takes the
  `wl_output` globals by key, because the protocol must name a client's `wl_output`; the freeze had `&[OutputName]` and
  no outputs. (2) `WlrToplevelPart` lost its `listed` field (the part keeps what each handle was told instead) and both
  parts gained private fields. (3) A Space has no name in the model: workspaces are named "Desktop N" (N = row position
  + 1), "Full Screen", "Split View" or "Agent"; `create_workspace`'s name is ignored. (4) Full-screen, Split View and
  agent Spaces are published as workspaces too, with only `activate`; only desktops can be removed or assigned, and an
  output keeps at least one desktop. (5) The wlr `activate` request counts as a valid activation (a dock click is the
  person's hand; the protocol carries no token). (6) `unset_minimized` restores through the activate path, so it also
  keys the window. (7) `set_rectangle` converts only layer surfaces (a dock's tile) by their arranged origin; any other
  surface is taken to sit at the origin. The rectangle's output is the one containing its origin, else the window's.
  (8) The model stores one `MinimiseTarget` per window (`Model::minimise_target`), pruned when the window goes; the
  minimise `Cause` uses it instead of the bottom-centre point.
- **Not done.** The wlr `parent` event is not sent. A `wl_output` a client binds after the manager is told about at the
  next publish, not at once. A workspace client's requests are processed at `commit` in order, not atomically in the
  model (each is its own step). When the last output is unplugged the model drops its Spaces, and the file loses
  them once written (the remembered file only holds what was loaded).
- **Hardware to check.** A real dock (sill-dock over `desktop_wl`'s Wlr arm) minimising to its tile: the genie goes to
  the layer surface's rectangle; a restart on the tty backend with two outputs keeps each Space on its connector.
- **Goldens written by hand.** `transcripts/wlr_toplevel-list.txt` and `workspace-publish.txt` are predicted from the
  code, with ids numbered by first appearance (`normalised`) and the `state` / `id` arguments masked; re-bless once with
  `CASEMENT_BLESS=1` and read the diff.
- **Unverified (first places to look if it does not compile).** `casement-wayland/src/workspace.rs` (bitflags types of
  the enum arguments: `State`, `GroupCapabilities`, `WorkspaceCapabilities`; `Resource::data`, `Client::create_resource`
  generics), `casement-wayland/src/wlr_toplevel.rs` (the `make` fn-pointer closures; `Output::client_outputs` in an
  edition-2024 closure), `casement-core/src/model/space_persist.rs` (`is_none_or`, borrows in `adopt_ids`),
  `casement-core/src/model/published.rs`, `casement/src/exec/publish.rs`, and the toml 1 paths in `space/persist.rs`.
