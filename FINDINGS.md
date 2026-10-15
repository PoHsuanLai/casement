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

### P2-Z: the private protocols, window capture, the hand-out

`todo!()` by lane: none left (P2-Q1, P2-Q2 and P2-Q3 filled all 33 and un-ignored all 21 tests). Lane ids and file sets:
`~/rs-wt/compositor-scope/23-P2-FILL-LANES.md`.

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
| `casement-scene-css` is filled (lane P3-CSS, branch lane/cas16-scenecss) but never built or run: that lane had no compiler | the first `dev/scene-css.sh` run, after the two dependency lines below are added to `casement-scene-css/Cargo.toml` |
| Protocol tests carry `todo!()` client steps: the fixture setup and the scenario are written, the testclient helpers are not | each H lane writes its helper module and the steps in the same change |
| `taffy_bridge` (`new_tree`, `solve`, `rect_of`, `fixed_size`) has no caller outside its test | DONE for Split View (wave-2026-10-15/split: `taffy_bridge/split.rs`, called by `rust_scene/split.rs`); the `expect` on the module is removed. Exposé is the next caller but needs none of it to compile |
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
  and are not bound. `ScreenshotTools` runs `sill screenshot area`.
- **Mac semantics (follow-up)**: Cmd+W, Cmd+M, Cmd+H, Cmd+Opt+H, Cmd+Q and Ctrl+Cmd+F are app menu commands on macOS,
  not window-server ones, so the focused app must receive them. Close, Minimise, Hide, HideOthers, Quit and FullScreen
  are not in `ROWS` (a test asserts none of those chords is bound), and `DesktopCommand::ToggleFullScreenKey`, added
  for full screen and used by nothing else, is dropped. ForceQuit (Cmd+Opt+Esc, system-level), Space switching and
  the sill hand-offs stay.
- **Dependencies**: `chordkit` (rev ba094c62) and `keycap-client` (rev 31e21db1) are git dependencies of the
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

## p1-c1: output capture (ext-image-copy-capture-v1 with output sources)

Nothing here compiled (no builds this lane). `ext_output_image_capture_source_manager_v1` and
`ext_image_copy_capture_manager_v1` are advertised at version 1 to every non-sandboxed connection
(`GlobalKind::ImageCapture` was already `Advertise::No` for sandboxes); `globals.txt` has both rows and the
globals test's `KINDS` lists `ImageCapture`; `sandbox.rs` checks that both are hidden. `casementctl shot
--output NAME OUT.png` captures over the wire; `--window` and `--masked` stay exit 2 (P2, P4); the default
output and `--region` still go through Debug1.

### Departures

- **A capture is the last presented frame, not a new render.** `handlers/image_capture/frames.rs` copies the frame
  the headless backend kept when the output presented (`HeadlessBackend::last_frame`, already read back through
  `ExportMem`) into the client's shm buffer, so capture equals scanout by construction. `pass::capture_output` (the
  same read-back for a framebuffer, plus `CapturedFrame::from_rgba`) is written and tested on pixman but the
  headless path does not call it: the frame it would read is gone by the time a client asks. **The tty backend has
  no read-back yet**: a frame request there fails with `unknown` until P1-D3's scanout keeps a frame or renders one
  for the request. A real-hardware run must check a capture on KMS.
- **No dma-buf.** Constraints are shm only (`argb8888`, `xrgb8888`, the output's mode size, `dma: None`). dma-buf
  capture follows with P1-D4's GPU manager (the render node to announce). `BufferConstraints { dma }` exists because
  `casement-backend` always builds smithay with `renderer_multi`, which brings `backend_drm`; a build of
  `casement-wayland` alone does not construct it.
- **The cursor option is carried and ignored.** The frame is scanout's, with whatever cursor scanout draws. A
  `paint_cursors`-off capture of a frame with a software cursor needs the cursor-free render P4's mask pass brings.
- **One unsafe function.** smithay lends a shm pool as `*mut u8`, so `casement-render/src/pass/capture/shm.rs` is
  `#![allow(unsafe_code)]` for one `slice::from_raw_parts_mut` (its SAFETY comment says why); the row copy beside it is
  safe and unit-tested. This breaks the "no unsafe" style rule; the alternative is not to write the client's buffer.
- **A mode change fails the next frame.** If the output's mode size differs from the session's constraints, the
  handler sends the new constraints (`update_constraints`) and fails the frame with `buffer_constraints`; the client
  allocates again. smithay already fails buffers smaller than the announced size.
- **`ImageCapturePart` holds smithay's states.** It is no longer `Default`: the pure bookkeeping is `CaptureBook`
  (`Default`, unit-tested without a display), the free functions take it, and only `apply_output_gone` takes the part
  (it drops the held `Session`s, whose `Drop` is the `stopped` event). `CaptureBook::sessions` maps to the output, not
  the source, because a client may destroy a source right after creating its session. `WaylandFault` gained
  `UnknownCapture(u64)`.
- **The test client is not `DesktopClient`.** `casement_testclient::capture::OutputCapture` has its own wire on a
  side queue (the `misc::idle` pattern) and folds events with `desktop_client::reduce`, reading `desktop_client::Frame`
  (`to_rgba8`). `DesktopClient::new` and `roundtrip` block on the compositor, which the fixture serves on the same
  thread, so they cannot be driven there. `casementctl shot` uses the full `DesktopClient` (feature `client`). One
  `OutputCapture` is one session and one frame (the reducer's `take_ready` ends the capture).
- **`desktop-client` is not in the sibling checkout this lane saw.** `../casement-protocols/desktop-client` exists on
  that repo's `lane/dc-fill` branch only (a0ca174 master lacks it), so the workspace does not load here until it
  lands; this lane read the API from that branch.

### Spots a real build checks first

`handlers/image_capture.rs` and `frames.rs` (smithay's `Session`/`SessionRef`/`Frame` paths, `insert_if_missing_threadsafe`,
`BufferConstraints { dma }`), `casement-wayland/src/image_capture.rs` (`new_with_filter` bounds with the concrete
`Clone` closure), `casement-testclient/src/capture/copy_session.rs` (wayland-client signatures: `create_session`
options type, `Format::try_from`), `casementctl/src/command/capture.rs` (desktop-client's paths), `shm.rs` (the closure
returning `Result` through `with_buffer_contents_mut(..)?`).
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
## p1-m1: output management, output power, gamma control (2026-10-11)

P1-M1 advertises `zwlr_output_manager_v1` (4), `zwlr_output_power_manager_v1` (1) and `zwlr_gamma_control_manager_v1` (1);
`globals.txt` and the sandbox hidden-list have their rows. Nothing here compiled (no builds this lane).

- **One sync hook.** `core_loop::feed` ends with `handlers::sync_outputs`: the heads (`apply_heads`, which compares the
  model's outputs with what was announced and sends nothing when they match), the power controls (`output_power::follow`)
  and the gamma controls (`gamma::follow`) follow the model after every batch. Heads for a changed `OutputFacts` are
  finished and announced again; a changed `OutputConfig` only resends the head's state. The manager serial moves once per
  change; a configuration made at an older one is `cancelled`.
- **`apply_heads` takes a `DisplayHandle` and the dispatch type** (`apply_heads::<State>(part, display, outputs)`): the
  frozen signature had neither, and a new head is a server-created object.
- **Disabled is dark.** `disable_head` is `OutputPower::Off`, `enable_head` is `On` (the output keeps its place in the
  arrangement; no output is removed from the layout). A disabled head sends only `enabled 0`.
- **Every head must be configured.** `apply` or `test` with a head left out is the protocol error `unconfigured_head`
  (what wlroots does); the frozen `Configuration` doc said "left out are disabled" and now says the protocol makes the
  client name them all. `already_configured_head` and `already_used` are posted too.
- **Custom modes** match an offered mode by size and rate (the fastest when the rate is 0); one the display does not
  offer reaches `resolve_change` as asked and is refused: `failed`, not a protocol error. Adaptive sync is not offered:
  `set_adaptive_sync` is ignored and no `adaptive_sync` event is sent. Scales take 0.25x to 8x (`resolve_change`).
- **A new `CoreNote::RefusedOutput(OutputRefusal)`** (an `exec::note_of` arm logs it) for a refused `OutputRequested`.
- **Gamma on headless** is recorded on the `VirtualOutput` (`HeadlessBackend::{set_gamma, gamma_of}`, table size 256) and
  read back through `Fixture::gamma_of`; no pixel changes. On tty `Effect::SetGamma` goes to `Kms::set_gamma` of the
  device that drives the connector. **Real-hardware check:** the KMS device reports no gamma table size
  (`CrtcFacts` has no field), so a tty output announces 256 steps; P1-D2 should add the real `GAMMA_LUT_SIZE` and
  `handlers/gamma.rs::gamma_table_size` should read it. A client that sends a ramp the CRTC cannot take gets the
  `failed` event only once `RealKms::set_gamma` reports it (not wired).
- **One control per output** for power and gamma: a second `get_*` on a controlled output, or on one that is gone,
  gets an inert object that says `failed` at once; destroying the holder frees the output (gamma: identity restored).
- **Test kit.** `Fixture::offer_modes` (an output with several modes: the virtual output offers one) and
  `Fixture::gamma_of`; the catalogue lists the three managers and `errors.rs` their error enums.
## p1-i1: input settings, device configuration, bindings that call out (2026-10-11)

Nothing here compiled (no builds this lane).

- **Settings.** `CasementSettings` gained `pointer: PointerSettings`; the document gained `[pointer]` (mice and
  trackballs) and `[touchpad]` sections (`accel`, `natural_scroll`, `tap`, `scroll_factor` per-mille, `disable_while_typing`),
  on the Mouse & Gestures page. `AccelProfile` now derives `Serialize`, `Deserialize` and `Word`.
- **xkb and repeat.** `SeatPart::new` already compiled the keymap from `KeyboardSettings`; `xkb_of` is the names the seat
  compiles from. A live change of `keyboard.*` after start is not applied to the running seat (unchanged; not this lane's file).
- **Device settings.** `device_settings` maps a class to the touchpad or mouse profile (every non-touchpad class gets the
  mouse profile). Nothing in the loop calls `InputDevices::apply` yet: `Seams` has no input-devices slot (a shared file) and
  the tty loop that sees libinput devices appear is P1-D1/D2's. `RealInputDevices::track` and `forget` are what that loop calls.
- **Scroll factor** is not a libinput setting; the adapter does not report it as unsupported and the compositor must scale the
  axis event with `DeviceSettings::scroll_factor` where it delivers it (`core_loop/deliver.rs::scroll`): not done here.
- **Real adapter departure.** `RealInputDevices::open` opens device nodes directly (`DirectOpen`), not through logind's
  `TakeDevice`; P1-D1's session replaces it. Real-hardware check: with the seat's real session, `apply` on a touchpad
  changes tap, natural scroll and disable-while-typing, and a mouse answers `Partly` for tap and dwt.
- **ZbusShellCalls.** One thread with a current-thread tokio runtime, the session bus connected on the first call, one task per
  call, failures returned by `failures()` and logged by `exec::call`. A call carries at most four string arguments
  (`ShellCallError::TooManyArgs`); names are validated before anything is queued. Real check: the default launcher binding
  reaches sill once sill exports `org.quire.Sill1` (the fake accepts anything).
- **ProcessSpawner.** `Command` with null stdin, its own process group, `spec.env` added, the `pass` descriptors' close-on-exec
  cleared with rustix, a thread reaping the child.
- **Bindings.** `carry_out` sends `Effect::CallBinding` and `Effect::Spawn` through `exec::exec`; the stale "not reached"
  comment above those arms in `exec.rs` is left for the orchestrator. `exec::spawn` adds `Xwayland::env()` when satellite runs
  (that body is P1-X1's).
## p1-x1: XWayland through xwayland-satellite, started on demand (2026-10-11)

Nothing here compiled (no builds this lane). `todo!()` left for the lane: 0.

- **Shape.** `casement::xwayland`: `Listening` (binds `<dir>/X<n>` and the abstract `@<dir>/X<n>`, first free `n` up to 63,
  removes the path socket on drop), `XwaylandState::{Off, Listening, Running}`, `bind` (config to state), `serve` (a client
  waiting on a socket starts satellite once, through the `Spawner` seam), `reap` (a satellite child that exited returns the
  state to `Listening`, so the next client restarts it), `env` (`DISPLAY`, exported from the moment the sockets are bound, so
  satellite is started on demand, not at login). `Xwayland::start` changed from the freeze signature: it takes the bound
  `Listening` and the `WaylandSocket` name and hands the sockets back on failure; a failed start is retried next turn.
- **Satellite's command line** is `<binary> :N -listenfd <fd> -listenfd <fd>` with `WAYLAND_DISPLAY` in its environment; the
  fds are `try_clone`d so casement keeps the sockets across restarts. `-listenfd` is from satellite's README (v0.8.3): a real
  run must confirm it takes both the path and the abstract socket this way. `-glamor none` (SP6: `off` panics satellite) is
  not in the argv; `dev/lib/x11probe.sh` adds it through a wrapper script for the jail.
- **Wiring (shared `run.rs`, three lines):** `xwayland::attach` after the Wayland socket exists, `xwayland::turn` each loop
  turn. `attach` prints `casement: X11 display :N` on stderr; `x11_compositor` reads it from casement.log.
- **For P1-I1:** `exec/spawn.rs::spawn_program` should add `state.xwayland.env()` to the program's environment (its doc
  says so; nothing here edits it). `reap` uses `waitpid(WNOHANG)`, which sees only a child of this process, so P1-I1's
  `ProcessSpawner` must keep the satellite as a plain child (no double fork, SIGCHLD not ignored).
- **`casement-x11probe`** (x11rb 0.13, new workspace dependency, already in the lock via smithay/winit): `session` reads
  commands on stdin (`map`, `unmap`, `close`, `focus`, `activate`, `info`, `own-clipboard`, `paste`, `sync`, `quit`), prints
  `x11: ...` lines. The loop polls the connection every 10 ms (a stdin thread feeds a channel) because `RustConnection`'s
  `AsFd` could not be checked.
- **Real-run checks** (`dev/accept/xwayland-*.sh`, jail only): which WM_CLASS string satellite uses as app id (the scripts
  assume the class, the second string; the instance is set to a different value so a wrong pick fails); whether the pointer
  position in `button-press` is in X pixels (physical) or logical at scale 2; whether satellite publishes `Xft.dpi` per
  output scale; whether `xwayland-scale.sh` can get a window on each output (it skips with a reason when both land on one);
  whether the compositor honours or refuses `_NET_ACTIVE_WINDOW` (the activation script records the outcome and asserts only
  consistency); `override-redirect` windows keeping the position the client asked for.

## p2-xw: XWayland hardening and the Q-C5 memo (2026-10-11)

Nothing here compiled (no builds this lane). `todo!()` left: 0. Shared files touched: none (`run.rs`, `main.rs`,
`state.rs` untouched; `xwayland::turn` reads `state.clock`, which already exists).

- **Departures.** `XwaylandState::serve` takes a fourth argument, `now: Mono` (the session clock, so the crash-loop guard
  reads no clock); `ServeStep` gained `Held { until }`; `XwaylandConfig` gained `rendering: SatelliteRendering`
  (`Automatic`, `Software` = `-glamor none` appended after the `-listenfd` pairs) with `with_rendering`. New public types:
  `Restarts`, `Permit`, `MAX_STARTS` (4), `WINDOW` (30 s) in `xwayland/guard.rs`. `Listening` carries the `Restarts`.
- **Memo.** `docs/xwayland-decision.md`: recommends keeping satellite; its "unverified" cells are the list below.
- **Only a real X11 run proves:** (1) whether satellite takes `-glamor none` after the `-listenfd` pairs, in that order;
  (2) mixed scale: whether `Xft.dpi` is one value per screen and how a window moved between outputs looks and is sized
  (`xwayland-scale.sh` records it; judge sharpness by eye); (3) clipboard with large (INCR) and image content, and
  `PRIMARY`; (4) drag and drop between an X11 and a Wayland window; (5) whether `_NET_ACTIVE_WINDOW` from an X11 app
  reaches casement as an activation request with a valid token, and whether a focus-stealing X11 window is refused;
  (6) that a satellite which exits at start is held after four starts and a waiting client is served after the window
  (kill it in a loop); (7) that the held state logs nothing at `warn` (a person sees only that the program waits; check the
  wording is enough or promote it).
- **Ozone hint.** The risk table wants Wayland-native Electron defaults in the session; that is the session's environment
  (dist), not `XwaylandState::env`, whose test pins `DISPLAY` as the only variable. Left to the dist lane.

## p1-f2: the fake logind (2026-10-11)

- **Departures.** The frozen unit structs `Manager`, `Session`, `Seat` and `FakeLogind` became real: the three objects are
  `#[zbus::interface]` types holding a shared `Recorder`, and the test-facing drivers (`lock`, `unlock`, `prepare_for_sleep`,
  `pause_device`, `resume_device`, `calls`, `held_inhibitors`, `wait_released`) are on `FakeLogind`, which owns a one-worker
  tokio runtime, so they block and must not be called from inside an async runtime. `LogindCall` is `#[non_exhaustive]` and
  gained `TakeControl`, `ReleaseControl`, `Activate`, `ReleaseDevice`, `PauseDeviceComplete`; `IdleHint(String)` is
  `IdleHint(IdleHint)`. `LogindError::Bus` carries a `zbus::Error`; new variants `Runtime`, `Emit`, `Pipe`, `StillHeld`.
- **Inhibit.** The client gets the write end of a pipe, the fake watches the read end (tokio `AsyncFd`); end of file on it is
  the release (`InhibitReleased`). Every mode counts as held; `wait_released` is how a test sees the sleep may go on.
- **`PauseDevice` and `ResumeDevice` are signals** in real logind (the brief calls them recorders): the fake sends them
  (`pause_device`, `resume_device`) and records the session's answer, `PauseDeviceComplete`.
- **Control interface** `org.quire.Casement.FakeLogind1` at `/org/quire/Casement/FakeLogind1` (Lock, Unlock,
  PrepareForSleep(b), Calls, HeldInhibitors) lets a jailed scenario drive the `fake-logind` binary with `busctl`; the binary
  takes the address from argv[1] or `DBUS_SYSTEM_BUS_ADDRESS` and refuses to start without one.
- **`PrivateBus`** (testkit, `fakes::PrivateBus`) is the casement-ipc test helper made public: dbus-daemon on
  `dev/private-bus.conf`, cleared environment, killed on drop. D1's session tests use it with `FakeLogind::serve`.
- **Tests** use `zbus::export::futures_core::Stream` (doc-hidden re-export) to poll signal streams, to avoid a new dependency.
- **Unverified (no builds this lane).** `fakes/logind/signals.rs` (`SignalEmitter::emit` bounds, `Fd::from(&OwnedFd)`),
  `manager.rs`/`session.rs` (interface-macro signatures: sync methods returning `()` and tuples with `bool`, properties
  with `name = "VTNr"`), `inhibit.rs` (`Errno::AGAIN` comparison, `AsyncFd::new` on an `OwnedFd`), the test helper
  `next_signal` (`Pin<Box<MessageStream>>` as a `Stream`). Real hardware: nothing; no real logind is touched.
## p1-d2: KMS, hotplug, modes (2026-10-11)

Nothing here compiled (no builds this lane). `todo!()` left for D2: none.

- **Departure: no `DrmCompositor` in `kms_real.rs`.** A `DrmCompositor` needs a renderer, an allocator and a framebuffer
  exporter, which the device-level `Kms` seam has not got and the scanout lane (P1-D3, `surface_real.rs`) owns.
  `RealKms` holds smithay's `DrmDevice` (opened with `disable_connectors = false`, so the console handover does not
  flicker) and one `DrmSurface` per lit connector; D3 takes it with `RealKms::take_surface` and builds the compositor,
  and takes the vblank source with `take_notifier`. Both are reachable only on a `RealKms`, not through `Box<dyn Kms>`:
  D3 either adds an `as_any`-style accessor to `Kms` or builds the surface where it opens the device.
- **`Kms::test` cannot be a kernel `TEST_ONLY`** before a framebuffer exists. Both the fake and the real device judge a
  commit by the pure `tty::judge` (attached connector, free CRTC the connector can use, a listed mode, variable refresh
  only where monitor and CRTC both support it, a mode or CRTC change is a modeset). The kernel's own test happens when D3
  queues the first frame. `KmsRefusal::ModesetForbidden` was added for a forbidden modeset.
- **DPMS:** `Off` clears the surface; `On` only records the intent and the next queued frame lights the output, so D3
  must queue a frame after `On` (and `output_power` in M1 should ask for one).
- **`RealKmsOpener::new(Box<dyn FdSource>)`** replaces `Default`: the opener needs descriptors from the session, so D1's
  `real_seams` must implement `FdSource` over its `RealSession` (a shared handle) and build the opener with it.
- **Announcement.** `TtyDevices` holds an `Announced` state: until `announce()` runs, `added`, `changed` and `removed`
  return no events (the announcement reports every connector known at that moment, once). D1's `start` may therefore call
  `added` and drop the events; a device added later reports its plugs from `added`. If D1's `dispatch` tests expect
  events before the announcement they must call `announce()` first.
- **EDID.** `parse_edid` reads the base block only: make is the PNP id, model the 0xFC name (else the product code in hex),
  serial the 0xFF text (else the nonzero numeric serial). CTA extension blocks are not read. An unreadable EDID is an
  anonymous display (`Unknown`/`Unknown`, keyed `connector:<name>`).
- **Variable refresh policy** is `VrrPolicy {Off, WhenCapable}` and `vrr_state`: on only when asked and when both the
  connector (`vrr_capable`) and the CRTC (`VRR_ENABLED` exists) are capable. Where `gpu`/settings hold the user's choice
  is open: nothing reads a setting yet, `plan_outputs` takes the policy as an argument.
- **Fixtures.** `fixtures/kms/*.json` are hand-written in the recorded shape (no recording was possible in the jail);
  replace them with `dev/hw/smoke.sh` output (raw sysfs and `drm_info`, converted by a lane) when the owner has run it.
- **Real-hardware checks** for the owner: `RealKms::open` takes master on a free VT; `resources()` reads names such as
  `eDP-1`, modes with plausible millihertz (`clock * 1e6 / (htotal * vtotal)`), the EDID blob, `vrr_capable`;
  a commit creates a surface and lights nothing until D3 queues a frame; `set_gamma` resamples a table to the CRTC's
  gamma size; unplugging a monitor yields one `Unplugged`.
- **Unverified (no builds).** Most likely not to compile: `kms_real.rs` (the drm-rs and smithay paths:
  `smithay::reexports::drm::control::{ResourceHandle, ModeTypeFlags, PlaneType}`, `DeviceFd::from(OwnedFd)`,
  `handles.filter_crtcs`, `plane.formats` field types, `u32::from(handle)`), `hotplug.rs` (`Option::is_none_or`, needs
  Rust 1.82), `fakes/kms/fake.rs` (guard borrows in `retrain`), `tests/it/tty/kms.rs` (`Option::is_none_or`).
## p1-d1: session, udev, VT switch, pause/resume (2026-10-11)

Nothing here compiled (no builds this lane). `todo!()` left for the lane: none.

- **`SessionSeam::Libseat(LibseatSlot)` deleted** (as P1-Z allowed). `SessionFault` gained `Unavailable(String)` for a seat or
  udev monitor that could not open; the libseat and udev errors reach it as text, not as typed sources (the adapter is the only
  file that names those crates, and `SessionFault` is not an adapter file). Open failures through libseat become `DeniedOpen`.
- **Seat state.** `tty::SeatState {Held, Lent}` (with `after(SessionNote)`) is a new field `TtyBackend::seat`; `present` (P1-D3)
  must return `Ok(None)` while it is `Lent`. `FakeSession` notes only real changes of hands (a second pause is no note).
- **`TtyBackend::start` touches no device and makes no plan.** Opening the devices present is `announce` (P1-D2); `plan` stays
  `None` until P1-D4's `plan_for` exists. `dispatch` order: session notes first; if the seat is held, a return re-reads every
  open device (`TtyDevices::changed`), then the udev queue is applied. While lent the udev events stay queued.
- **VT switch is ordinary code.** `vt_of_keysym` maps `XF86Switch_VT_1..12`; `TtyBackend::request_vt(Keysym)` asks the session.
  Nothing calls `request_vt` yet: the libinput feed (P1-I1) must call it for a key event whose keysym names a VT, before
  bindings see it. `SessionNote::VtSwitch(n)` arriving from a session is turned into the same request.
- **Loop turn.** `core_loop/tty.rs::drain`, called from `core_loop::turn` (so `run.rs` is unchanged): connector notes go to the
  existing `connector` path; `Activated` re-applies every output's configuration as `Effect::ApplyOutput`, which re-modesets once
  `exec::apply_output` has its tty arm (P1-D2; today it only handles headless).
- **`State::new` starts the tty backend.** `Seams` gained `tty: Option<TtySeams>` and `Seams::with_tty`; `with_seams` takes the
  injected seams, else builds `real_seams(SysRoot::host(), GpuChoice::Auto, loop handle)`. `real_seams` gained the loop handle
  (libseat and the udev monitor are event sources). `gpu.render` is not a setting yet, so the choice is `Auto`.
- **PrepareForSleep.** Through the fakes: `FakeSession::prepare_for_sleep(SleepPhase)` (the type is P1-F2's, in
  `fakes/logind/manager.rs`) pauses and resumes; the fake logind server will call it. The real path is not built: libseat does not
  report `PrepareForSleep`; a zbus subscription (with the inhibitor from P1-F2/SESS) must push `Paused`/`Activated` into
  `RealSession`'s queue. Real-hardware check: suspend and wake must leave the displays lit and the seat held.
- **Fakes.** `SharedSession` (a clonable `FakeSession`) lets a backend and a test hold the same session. `Fixture::session()` returns
  a `FakeSession` the fixture holds, not wired to the headless backend.
- **`RealKmsOpener` is a unit struct**, so `real_seams` cannot give it the session: P1-D2 must add a field (a
  `LibSeatSession` clone, or take the descriptors from `RealSession`) and change the one line in `real.rs`.
- **Ignored with a reason, not P1-D1's:** `udev_events_reach_the_backend_in_order` and `hotplug_waits_out_a_pause` have real bodies
  but call `TtyDevices::{added,changed,removed}` and `FakeKmsOpener::open` (P1-D2); un-ignore when those land.
- **Not covered by a test:** the loop turn (`Activated` re-modeset) needs a tty `State`, which needs P1-D2/D3 fakes; the
  `Seams::with_tty` seam is there for that suite.
## p1-d3: frame scheduler, presentation feedback, scanout surface, cursor plane (2026-10-11)

Nothing here compiled (no builds this lane).

- **Scheduler** (`casement-core/src/frame/`): `predict` is the worst of the last 8 render costs (8 ms when empty): a
  missed vblank costs more than rendering a little early. `next_deadline` takes the first vblank after `now` whose
  `vblank - (predicted + slack) >= now`, looking at most 600 vblanks ahead. `judge` counts `presented - wanted`
  vblanks (early is on time).
- **Cursor plane policy: hardware when the output has a cursor plane and the logical cursor size scaled to physical
  pixels (rounded up) fits the plane's largest size; composited otherwise.** No downscaling to fit. The real
  compositor (smithay `DrmCompositor`) promotes or composites the cursor element itself; `cursor_plane_for` is what
  casement uses to pick the cursor image size and to report the choice. A hardware run must check: plane size caps
  (`DRM_CAP_CURSOR_WIDTH/HEIGHT`) are read into `CursorPlaneCaps` by P1-D2's `kms_real`, the pointer stays one frame
  fresh when it moves with no other damage, and VRR panels do not stutter with the cursor on a plane.
- **Seam changes** (`tty/surface.rs`): `QueuedFrame<'a>` gained `Draw(&dyn FrameSource)` (the surface draws with the
  renderer its device uses; manual `Debug`), and `QueueOutcome::Queued` now carries the `VblankSeq` the frame is for
  (only the surface knows the next vblank, and `present` has no clock). New `tty/scanouts.rs`: `Scanout` (surface,
  `FlipTracker`, `VblankClock`, `RenderHistory`, `FrameTally`) and `ScanoutSet` (per output, `Flow::{Running,Held}`).
  `TtyBackend` gained a `scanouts: ScanoutSet` field: P1-D1's `start` must build it with `ScanoutSet::empty()`, P1-D2
  attaches a `Scanout` per lit output and calls `flow(Flow::Held)` on `SessionNote::Paused`, `Running` on activate.
- **`RealScanout`** now owns a `DrmCompositor<GbmAllocator, GbmFramebufferExporter, (), DrmDeviceFd>` and a
  `GlesRenderer`; the DRM event source (loop side, D1/D2) calls `flip_done(seq, at)` on each vblank event. Its
  constructor signature is a guess against the pinned smithay; the render node should come from the GPU manager (D4) so
  multi-GPU can swap the renderer. Hardware check: `Draw` falls back to compositing when direct scanout is refused.
- **Test drive**: `tests/it/tty/scanout.rs` runs 36,000 virtual frames at 60 Hz (cost 1 ms: zero misses) and a
  scripted 25 ms frame every 40 (exactly one miss per slow frame; two in a row miss once).
## p1-d4: GPU detection, plan, multi-GPU import (2026-10-11)

Nothing here compiled (no builds this lane).

- **Plan rule** (`core/gpu/plan.rs`). Auto: of the GPUs that drive a connected monitor, the firmware's (`boot_vga`)
  else the lowest node; with none driving a monitor the same rule over all GPUs. `gpu.render = <by-path id>` picks that
  GPU (an id detection does not list is `PlanRefusal::UnknownGpu`, also with an empty list). Scanout stays on the GPU
  that drives each connector. The owner's machine (AMD iGPU on the panel, NVIDIA dGPU) is a table case in
  `casement-core/tests/it/gpu.rs`; no code names it.
- **Feedback** (`core/gpu/feedback.rs`): a Render tranche (render GPU), then one Import tranche per other GPU that scans
  an output out, each format-and-modifier only at its first tranche, empty tranches left out. There is **no Scanout
  tranche yet**: it needs the KMS planes' formats, which `FormatTable` does not carry (P1-D2/D3 add them).
- **Tagging (A5).** The linux-dmabuf protocol does not tell the server which tranche a client allocated from, so
  `DmabufPart::source_device(format)` answers with the target of the tranche that offers the buffer's format and
  modifier (unambiguous because `feedback_for` offers each pair once; the main device for a pair nobody offers). The
  handler in `casement/src/handlers/dmabuf.rs` still does the `DrmNode` conversion (B6 keeps drm out of the protocol
  crate); it was not moved into the render-node adapter. `backend::gpu::tag_node` is the same rule on the plan.
- **Departures.** (1) `SysRenderNodes` is no longer behind the `tty` feature: it only reads files, and the test kit
  must run it over the fabricated tree. (2) `GpuStack` gained `handles` (open render and scanout nodes; `#[non_exhaustive]`)
  and holds no `GpuManager`: the manager is built over real devices by the frame loop (P1-D3). (3) `route_for` assumes a
  direct-importing render GPU; `route_with(source, plan, CrossSupport)` takes the capability (`Copy`, `Refused`).
  (4) `GpuError::NoFormats` (a recorded table nobody can load). (5) `DmabufOffer` is `#[non_exhaustive]` with
  `tranches` and `from_feedback`; `State::new` still offers `DmabufOffer::soft()` and the tty start (P1-D1..D3) must build the
  offer with `DmabufOffer::from_feedback(&feedback_for(plan, &format_table(..)?))`. (6) apply_offer adds every tranche
  as a preference tranche: smithay sends the builder's "main" tranche after all preference tranches, which would put the
  render GPU last. (7) Testkit: `FormatSet::LinearCodes`, `GpuTopology::pair/facts`, `SoftGpu::node/by_path`,
  `SoftDevice`, `SoftApiError`; `FormatSource` is implemented for `GpuTopology`. `SoftPixman` has no `Offscreen` impl.
- **Sysfs layout read** (`render_nodes_real.rs`): `sys/dev/char/226:<128..191>/device/{vendor,boot_vga}`,
  `device/drm/card<N>/card<N>-<NAME>/status`, `dev/dri/by-path/pci-<addr>-render -> renderD<minor>`; a GPU without a
  by-path link falls back to the file name of its `device` link.
- **Counters asserted** (`tests/it/topology`): Direct is B `bind_dmabuf` 1, no `copy_texture`, no A `import_memory`,
  A `import_dmabuf` >= 1 (SP4 saw 2; the exact figure is unexplained, so it is a lower bound); CopyOnly is B
  `copy_texture` 1, A `import_memory` 1, B `bind_dmabuf` 0.
- **Most likely not to compile:** `casement-backend/src/gpu/soft.rs` (the `Frame`/`Renderer` delegation against smithay
  dc7ca68; SP4 ran e3d461a), `casement-testkit/src/gpu/soft_api.rs` (`GraphicsApi` bounds), `tests/it/topology/support.rs`
  (generic `draw` over `MultiRenderer`), `casement-wayland/src/dmabuf.rs` (`add_preference_tranche` fold).
- **Real hardware must check:** that `/sys/dev/char/226:N/device/drm/card*/card*-*/status` exists on the target kernel
  (connector status is read from there, not from `/sys/class/drm`); that the by-path link set matches the `gpu.render`
  ids Settings shows; a hybrid laptop with the NVIDIA proprietary driver (its render node may list no connectors, and
  `boot_vga` may be 0 on both GPUs, which falls to the lowest node).

## p1-t: tty integration (2026-10-11)

Nothing here compiled (no builds this lane). P1-T joins what D1 to D4, I1, M1 and C1 each left loose, so that
`casement --backend tty` is one path. The definition of done is `tests/it/tty/integration.rs`: the real `State` over
`TtyWorld`'s fakes (session, udev, KMS, input, GPUs) starts with two outputs, presents frames, switches VT away and
back, hotplugs, goes dark and lit (DPMS), loads a gamma ramp, captures an output, takes input and suspends and resumes
through the fake logind on a private bus.

### What landed, by item

- **Input.** `input::InputFeed` (a seam beside `InputDevices`) is the seat's input: `poll(&PollContext)` returns
  `FedInput::{Appeared, Gone, Event}`, `suspend`/`resume` follow the seat. It lives in **`TtySeams::input`, not in
  `casement::Seams`**: the real feed needs the libseat handle `real_seams` builds, so the slot sits with the session it
  must share. `RealInputFeed` opens libinput through smithay's `LibinputSessionInterface` over a libseat handle (never
  direct opens), reads it as an event source and queues raw events; `poll` converts them with `input_kind_of`, one
  generic conversion over smithay's input traits (round-tripped against the virtual backend in
  `tests/it/input_convert.rs`). `core_loop::tty::input` configures a pointer or touchpad that appeared with the stored
  profile (`device_settings`, then `InputDevices::apply`) and feeds events to the model. A `Ctrl+Alt+Fn` chord is
  `XF86Switch_VT_n` after xkb: `deliver_key`'s filter asks `tty::vt_chord` first (before the lock, a shortcut inhibitor
  and the bindings), which calls `TtyBackend::request_vt` and takes the press (its release is swallowed too).
  `modified_syms()` is what is read, because the chord's raw symbol is `F3`. The **scroll factor** (I1's open item) is
  applied where the tty loop takes the event in (`scaled_scroll`, per profile by the event's origin class).
- **Suspend.** `casement_ipc::sleep::SleepWatch` is a thread (current-thread tokio runtime, like `ZbusShellCalls`) that
  subscribes to `PrepareForSleep` on the system bus (or the private bus a test gives) and holds a `sleep` delay
  inhibitor while awake. `casement::suspend::watch` joins it to the session's `SleepLink` (`tty::sleep_channel`): on
  `true` it sends `Paused` and **waits (2 s at most) until the backend acted on it** (`TtySession::settled`, called by
  `dispatch` after the devices were paused), then releases the inhibitor; on `false` it sends `Activated` and takes the
  inhibitor again. `RunConfig::sleep_bus` (`SleepBus::System` by default) names the bus; `run` attaches it for a tty
  backend. Tested against `FakeLogind` (the inhibitor is released only after the pause was acted on).
- **GPU.** `TtyBackend::start` detects the GPUs and makes the plan (`plan_for`; a machine with none logs and has no
  plan), and opens the display devices udev reports present (D2 had left that to nobody: `announce` only reported
  devices already open). `TtyBackend::dmabuf_feedback` builds the format table from `TtySeams::formats` (a
  `FormatSource`; real: `RealFormats`, a GLES renderer on each render node) and `feedback_for`; `State::with_seams`
  offers `DmabufOffer::from_feedback` for a tty backend and keeps the soft offer only when there is no feedback.
  `FormatTable` gained `scanout` (the primary planes' formats per GPU, from `scanout_formats`) and `feedback_for`
  puts a **Scanout tranche first** for the render GPU's own planes: planes' formats that the GPU also samples, each
  pair once. A plane of another GPU makes no Scanout tranche (it is the `Import` tranche's copy). `OfferTranche` gained
  `purpose` and `apply_offer` sets `TrancheFlags::Scanout` for it.
- **KMS and scanout.** `Kms` gained `scanout(connector, &OutputConfig)`, `pause` and `resume`; `ScanoutSurface` gained
  `blank` and `reset`, and `presented(now)` (a surface that learns of a flip without a time on the session clock stamps
  it so). `TtyBackend::apply` is `exec::apply_output`'s tty arm: DPMS off blanks the scanout; otherwise one commit
  (modeset allowed) on the CRTC the output is lit on or the first free one, and a surface whenever the mode, scale or
  transform changed (the old scanout is dropped first: a device keeps one surface per connector). `Lighting::Relit`
  (DPMS On) makes `exec` queue a frame at once. `dispatch` follows the seat: `Flow::Held` and `Kms::pause` on `Paused`;
  `Kms::resume`, `ScanoutSet::reset` and `Flow::Running` on `Activated`, then the loop re-applies every output. An
  unplugged connector's scanout, lighting and capture shadow are pruned. `CrtcFacts::gamma_size` (serde default 256, so
  old fixtures read) is `GAMMA_LUT_SIZE`; `handlers/gamma.rs` answers the CRTC's size.
- **Capture on KMS.** `tty::Readback`: while a capture session is open on an output, each frame queued is drawn again
  through pixman into a headless-backend shadow output of the same mode, scale and transform
  (`TtyBackend::last_frame`); with no session nothing is drawn. `core_loop::frame` turns it on from the capture book
  (`CaptureBook::sessions`). The first frame request of a session fails (`unknown`) until one vblank has drawn a frame.
- **`gpu.render`.** `GpuSettings` in core, `GpuSection` (`gpu.render`, `auto` or a PCI address, Appearance page,
  advanced) in the document, conversion in `config/convert.rs`, rows in `tests/it/config.rs` and core's settings tests.
  A value that is neither is logged and renders as `auto`. `State::new` passes it to `real_seams`. It takes effect at
  the next login.

### Departures

- **Frames are timer-driven, not vblank-driven.** The loop still draws at the virtual or wall clock's vblank timers
  (`ClockSource::plug_output`) and `present` queues when no flip is pending; a flip completing (`DrmEvent::VBlank`) is
  noted for the next tick. `Scanout::deadline` (the scheduler) is not used by the loop. The real surface counts vblanks
  itself (`RealScanout::next`), not the kernel's sequence, so a missed vblank is **not measured on hardware** (the fake
  presents on time by construction; the scheduler is covered by the D3 virtual-clock suite).
- **One GLES renderer per KMS device**, shared by its outputs, made with `casement_render::gles_on(gbm)`. The cross-GPU
  copy (render GPU draws, another scans out) is not wired to real devices: a second GPU's outputs are drawn by that
  GPU. `gles_on` generalizes `surfaceless_gles`'s two `unsafe` calls over any native display; **no new `unsafe` block**,
  but the signature of the one unsafe module changed.
- **`RealKmsOpener<D>` is generic** over the loop's data type and inserts each device's `DrmDeviceNotifier` into the
  loop itself (`RealKms::open(fd, &handle)`); `take_notifier` and `take_surface` are gone (the hand-off is
  `Kms::scanout`). `RealInputDevices` became `RealInputFeed` (it opens through the session).
- **Event time.** libinput's timestamps are on the kernel's clock; the feed stamps each event with the poll's `now`.
- **Absolute devices** (touch, tablet in mouse mode) are placed over the union of the outputs (`PollContext::extent`),
  not over a chosen output. A mapping per device is open.
- **Not re-applied after a sleep or VT return:** gamma ramps (the CRTC's table may be lost), VRR beyond the commit.
- **`SettingsChanged` is not re-applied to devices already present**; a profile change reaches a device at the next plug.
- **Hotplug of a second GPU** does not make a new dma-buf offer (the offer is made once at start).
- **Direct scanout of client buffers** is left on (`NodeFilter::All` in the framebuffer exporter), as D3 expected.
- **Real adapters list.** `scripts/check-boundary.sh` ADAPTERS gained `tty/real.rs` (it names libseat for the input
  handle) and `gpu/formats_real.rs`.

### Only a real login proves

- `Ctrl+Alt+F3` switches and `Ctrl+Alt+F1` returns with the displays lit and input back (libseat pause/enable order
  against `Kms::pause`/`resume`, `DrmDevice::activate(false)`, `DrmCompositor::reset_state`, libinput `suspend`/`resume`
  reopening through the session). The default xkb layout must carry `srvr_ctrl(fkey2vt)`; the integration test asserts
  the chord switches on the jail's xkb data.
- Suspend and wake on the owner's laptop: the delay inhibitor is held, released after the pause, retaken; the panel is
  lit and the seat held after the wake.
- A real `DrmCompositor::new` with `OutputModeSource::Static`, `NodeFilter::All`, the `[Xrgb8888, Argb8888]` colour
  formats and the renderer's dma-buf render formats; the first frame queued lights the CRTC; `blank` (`clear`) darkens it
  and the next frame lights it; two outputs on one device share one renderer.
- `RealFormats` (a GLES renderer on `renderD*` through GBM) and the Scanout tranche's formats as clients see them
  (`weston-simple-dmabuf-feedback` or the testclient's feedback helper).
- `gamma_length` from `get_crtc` on the owner's panel and monitor; a ramp loads.
- libinput: classes (`class_of`), the touchpad profile taking effect, `Partly` for a mouse, absolute placement.
- A capture on KMS: a pixman shadow of a 4K scene each vblank is slow; check the cost, and that a client dma-buf the
  pixman renderer cannot import is simply missing from the capture.
- Hybrid GPUs: the plan, the Scanout tranche (render GPU only), the second GPU's outputs drawn by that GPU.

### Shared files touched

`state.rs` (the tty arm: settings choice, dmabuf offer), `exec.rs` (the tty arm of `apply_output`), `run.rs`
(`RunConfig::sleep_bus`, `suspend::attach`), `core_loop.rs` (the tty arm of `frame`), `core_loop/deliver.rs` (the VT
chord in the key filter), `lib.rs` (`suspend`), `ARCHITECTURE.md` (seam signatures), `scripts/check-boundary.sh`,
`FINDINGS.md`. No `Cargo.toml` or `Cargo.lock` changed; `tokio`'s `sync` feature (workspace) is used by
`casement-ipc::sleep`.

### Spots most likely not to compile

`casement-backend/src/tty/{kms_real,surface_real}.rs` (`DrmCompositor::new`'s argument types: `Size<u32, Buffer>` for
the cursor size, `Some(gbm)`; `OutputModeSource::Static`; `DrmEvent` arms; `gbm::Device::new(DrmDeviceFd)`),
`input/libinput_real.rs` (`Event::<LibinputInputBackend>::device`, `LibinputInputBackend::new(context.clone())`,
`insert_source` error type), `gpu/formats_real.rs` (`ImportDma::dmabuf_formats().iter()` items, `Format::code as u32`),
`input/convert.rs` (the generic helpers' bounds: `placed::<B, _>`), `casement-ipc/src/sleep.rs` (`MatchRule` builder
errors, `Message::body().deserialize`, `tokio::select!` with `&mut oneshot::Receiver`), `core_loop/deliver.rs` (the
filter closure now takes `state`), `casement-testkit/tests/it/tty/integration.rs` (client helper signatures).

## smithay fork 9c8f762a (2026-10-11)

The typed fuzz box (seed 8, once P1-C1's globals shifted object ids) panicked smithay: `wl_subsurface.place_above`
or `place_below` with the sub-surface itself as the reference removed it from its parent's list and inserted past
the end. The protocol makes that reference a `bad_surface` error; the fork's `PrivateSurfaceData::reorder` now
refuses it (branch `casement`, 9c8f762a on dc7ca684). An upstream PR waits for the owner's word (upstream PRs on
hold).
## p1-v: vkms acceptance and writeback goldens (2026-10-11)

Nothing here compiled or run (no builds, no vkms). User-run only (C3).

- **Delivered:** `dev/vkms/accept.sh` and `README.md` (the person's setup commands, what each step asserts);
  `casement-testkit/src/writeback.rs` (pure: `decode`, `tolerance`, `compare_raw`, `PixelFormat`, `RawFrame`), the
  `writeback-compare` binary (copied by `dev/bootstrap.sh` next to the other tools), tests in `tests/it/writeback.rs`
  (run in the normal gate), and a scene `empty-desktop-1024x768` in `tests/it/scene`.
- **The golden PNG does not exist yet.** `tests/golden/scenes/empty-desktop-1024x768.png` is created by one
  `CASEMENT_BLESS=1` run of the scene test (the lane could not render). Until it is blessed the scene test fails with
  "differs (golden missing)"; bless, review the image, commit. The scene assumes output spec
  `HEADLESS-1:1024x768@60000:120:0,0` (120 = 1x) and the pointer at the middle.
- **Departure: the script cannot run end to end today.** casement has no way to run the tty backend on one named
  node: the real seams use libseat and `GpuChoice::Auto`, which would take the person's real VT and GPU. So the script
  needs four hooks on casement, listed in its header ("casement contract") and probed with `casement --help`
  (exit 4 when missing): `--drm-node PATH` (direct fd on that node, checks driver `vkms`), `--kms-report FILE` (JSON
  lines: flips, missed, seat, outputs with name/mode/power), `--writeback-raw PATH` (on SIGHUP, one writeback frame as a
  raw buffer plus `PATH.meta` "W H PITCH FOURCC"), SIGUSR1/SIGUSR2 as session Paused/Activated. KMS writeback is not in
  the backend at all (`Kms` has no writeback connector); a follow-up lane (P1-D2/D3 owners) adds it behind
  `--drm-node` mode only. `casementctl` is jail-only, so the script cannot use it; DPMS goes through `wlopm`
  (wlr-output-power-management, P1-M1).
- **Hotplug is a status flip, not mkdir/rmdir.** Connector subitems created by the kernel after the setup are
  root-owned in configfs, so a user cannot link `possible_encoders` or write `status` on a connector it created. The
  setup pre-creates `hotplug1` disconnected and the script writes `status` 1 / 2 on it.
- **Most likely wrong (kernel side, unchecked on 7.2):** the configfs attribute names (`planes/*/type` values, `crtcs/*/writeback`,
  connector `status` values 1 and 2, `possible_crtcs` / `possible_encoders` link dirs); `create_default_dev=0`; whether
  `chown -R` on configfs persists for later-created attribute files; the udev `TAG-=` rules actually keeping the
  desktop off the card; the default vkms mode being 1024x768@60 (the report's mode is what the script reads, only the
  golden assumes 1024x768); whether writeback includes the cursor plane.
- **Most likely not to compile:** `writeback_compare.rs` (the `map_or(Ok(3), |_| number(...))` type inference may
  need a turbofish), `tests/it/writeback.rs` (`assert_eq!` on `Result<GoldenOutcome, WritebackError>` needs both to be
  `PartialEq`; `GoldenOutcome` derives it).

## p1-v2: the vkms hooks (2026-10-11)

Nothing here compiled or run (no builds, no vkms). `dev/vkms/accept.sh` can now run end to end in principle; it is
still user-run only (C3).

- **Built.** `casement --backend tty --drm-node PATH [--kms-report FILE] [--writeback-raw PATH]`, a test mode that
  `--help` says is one. `--drm-node` needs `--backend tty` and the other two need `--drm-node` (`CliError::Needs`).
  `StartConfig::tty: TtyMode::{Seat, Direct(DirectTty)}`; `state.rs` picks `direct_seams` for `Direct` (the file's
  FROZEN header notwithstanding: the same interface ask P1-T made).
  - *Direct seams* (`casement-backend/src/tty/direct.rs`, pure; `direct_real.rs`, adapter): the node is opened once by
    path, as the user, and `DrmDeviceFd::get_driver()` (the DRM version ioctl) must say `vkms` (`require_vkms`) or
    startup fails before anything else is built. `DirectSession` (a `TtySession`) opens nothing and refuses VT
    switches; its notes come only from its `SleepLink`. `FixedNode` is the one `DeviceSource`. `NoGpus` is the
    `RenderNodes` (nothing enumerated, so no render plan and no dma-buf feedback: clients get shm). No `InputFeed`: no
    libinput, so no other device is opened. libseat and logind are not touched; `run` skips `suspend::attach`.
  - *Signals.* `casement-ipc/src/signals.rs`: a thread with a current-thread tokio runtime waits for SIGUSR1, SIGUSR2
    and SIGHUP. **New feature on an existing dependency**: tokio's `signal` (workspace `Cargo.toml`); no new crate and
    no lock change (`signal-hook-registry` is already there through tokio's `process`). calloop's `signals` needs `nix`,
    which is not in the lock; signal-hook-registry's `register` is `unsafe`. `casement/src/test_mode.rs` maps SIGUSR1 to
    `SleepLink::paused` and SIGUSR2 to `SleepLink::activated`, the path a suspend takes, so a simulated VT switch runs
    the same `TtyBackend::dispatch` code as a real pause.
  - *Hotplug.* There is no udev monitor in this mode, so `FixedNode::next_event` reports the node `Changed` once every
    `POLL_EVERY + 1` turns of the backend and the backend reads its connectors again; the step's 15 s limit allows it.
  - *Report.* `KmsReport` (pure value, `TtyBackend::report()`; `json_line()`); `ReportPacer` decides on the session
    clock when a second has passed; `Hooks::turn` appends the line. `flips`/`missed` are the scanouts' tallies, and
    `ScanoutSet::detach` now keeps a departed scanout's tally so the totals only grow. While the seat is lent the loop
    still turns every 250 ms (`TURN`) so the `lent` line is written.
  - *Writeback.* `Writeback` seam (`tty/writeback.rs`), `TtySeams::writeback`, `RealWriteback` (`writeback_real.rs`),
    `FakeWriteback` (testkit). SIGHUP: `request_writeback` queues a job on the first lit output's CRTC at its mode size;
    `take_writeback` reads it once the output has flipped `FLIPS_TO_WAIT` (3) more times; `test_mode::write_capture`
    writes the raw bytes, then `PATH.meta` (`W H PITCH XR24`) last. The real job is one atomic commit on the writeback
    connector (`CRTC_ID`, `WRITEBACK_FB_ID`) with an `XRGB8888` dumb buffer, read through the drm crate's safe
    `map_dumb_buffer`, then detached; the commit is made on a duplicate of the display device's descriptor (same open
    file, so the same DRM master). `ClientCapability::WritebackConnectors` is set only when `--writeback-raw` is given,
    and `kms_real::connector_facts` skips writeback connectors so they never become an output. No `unsafe` added.
- **Script fix.** The modeset `wait_for` test was `.outputs | length >= 1 and ... and .flips > 0`; the pipe binds
  loosest, so `.flips` was read from the outputs array. Parenthesised. The header contract and README were brought in
  line (`XR24`, meta last, hotplug by polling, the flags' dependencies).
- **Tests (all over fakes).** `cli.rs` parse tests (flags, dependencies, usage text the script greps);
  `report.rs` value, JSON and pacer tests; `test_mode.rs` (signals to link, files); `tests/it/tty/direct.rs` (pause and
  activate through the link in a fake tty world, report over a lit fake output, report cadence, writeback frame bytes and
  meta line over `FakeWriteback`, the refusals, `FixedNode` polling).
- **Only a real vkms run proves:** the version ioctl naming `vkms`; opening the card node as the user and becoming
  master on it; `DrmCompositor` on vkms through a GLES renderer on the card node (vkms has no render node: this needs
  Mesa's software GBM/EGL path, and is the likeliest first failure); the writeback commit being accepted without
  `WRITEBACK_OUT_FENCE_PTR`, and the frame being composed within three flips (no fence tells us), including whether it has
  the cursor plane; the buffer's `XRGB8888` being on the connector's `WRITEBACK_PIXEL_FORMATS`; the commit needing
  `ALLOW_MODESET` and working beside `DrmCompositor`'s own state; a vkms configfs status change being visible to a
  non-forced `get_connector`; `DrmDevice::pause`/`activate` on a dup'd descriptor shared with the writeback adapter;
  the idle step's flip rate and zero missed vblanks (flips are still timer-driven: see p1-t).

## p1-sh

Added `casement --state-home DIR` (an explicit directory always wins, headless included; without it headless stays
ephemeral and the others use `$XDG_STATE_HOME/casement`, else `$HOME/.local/state/casement`) and
`casement-probe workspaces` (one JSON line per workspace: `id`, `name`, `group`, `active`). The placement decision is
a pure function in `casement/src/main.rs` over (flag, backend, env). `State::new` builds its store from
`Seams::real(&config.state_home)` for every backend, so headless needed no wiring to save and load `spaces.toml`.
Surprise: `cargo fmt -p` needs the sibling checkouts, absent in this worktree, so the files were formatted with
`rustfmt --edition 2024`; nothing was compiled by the lane.

## p1-sh follow-up: the probe's tap (2026-10-11)

- `casement-probe workspaces` timed out in the desktop scenario: a `TestClient` talks through its tap, which relays
  bytes only when the client is pumped. The verb pumped only the side-queue watch, so the binds never left the tap.
  Fixtures pump the client every round, which is why the in-process tests never saw it. Any probe verb built on a
  side-queue watch must pump the client too.

## probe-shell: shell-role verbs in the probe session (2026-10-11)

Nothing here compiled or run (the lane does not build).

- **Verbs** (`casement-probe session`, one `probe: {json}` note each; a failure is `{"error": "..."}`):
  `shell-connect` calls `org.quire.Compositor1.Connect("shell")` on `$DBUS_SESSION_BUS_ADDRESS`, runs the first roundtrip on the
  returned fd and makes it the session's client (surfaces made earlier go with the old connection; the session rebinds
  its seat, wm_base and clipboard on the new one); `shell-role ROLE LAYER` (bar, dock, launcher, switcher, overview,
  notifications, osd, control-centre, wallpaper, widget) calls `set_role` on a layer surface made after `shell-connect`;
  `minimise-target TOPLEVEL X Y W H` sets the target local to the surface that last got the dock role;
  `minimise-clear TOPLEVEL` clears it. TOPLEVEL is a window identifier (`w<id>`): the session cannot map its own toplevel
  names to identifiers (they are on another connection), so read the identifier from `casementctl debug windows`.
- **Environment.** The scenario starts casement with `CASEMENT_DEBUG_PEER_UNITS="casement-probe=sill.service"` (see
  debug-role; comm is cut to 15 bytes and `casement-probe` is 14), so the jailed probe's unit reads as `sill.service`.
- **Dependency.** `casement-testclient` gained `zbus` and `tokio` (the bus call, on a throwaway current-thread runtime);
  `Cargo.lock` was edited by hand. B9 (no smithay) is unaffected.
- **Debug windows.** `casementctl debug windows` rows gained `identifier` (`w<id>`) and `minimise_target` (`[x, y, w, h]`
  global, or `null`), so a scenario can assert the stored rectangle.
- Every verb pumps nothing itself: the session loop pumps the client each turn, so requests leave the tap on the next
  turn (wait for the `done N` note, then for the next turn, before asserting on `debug windows`).

## p2-z: the P2 delta freeze (2026-10-11)

Nothing here compiled or run (the lane does not build). Plan: 20 §4.15 and §4.16 over 22. Fill lanes:
`~/rs-wt/compositor-scope/23-P2-FILL-LANES.md`. The stubs and ignored tests are in the P2-Z table above.

- **Built (implemented).** `casement-core`: `desktop` (`DesktopRequest`, `AppAsk`, `WindowAsk`, `SpaceAsk`,
  `DesktopNote`, `DesktopRef`) and `shell` (`ShellRequest`, `ShellNote`); `Event::{Desktop, Shell}`, `Effect::{Desktop,
  Shell}`, `CoreNote::Deferred(Deferred)`; `GlobalKind::{ToplevelCapture, QuireDesktop, QuireShell}` with their I7 rows
  (a `Tier` beside the sandbox table); `LayerBook::role_holder`; the model's routing of both new events
  (`model/desktop_requests.rs`, `model/shell_requests.rs`: roles, dock-local minimise targets, and every desktop ask that
  an existing machine already carries, reached through `wlr_request`, `place`, `workspace_request`, `activation`,
  `hide_app`, `hide_others`, `close_all_of`). `casement-wayland`: feature `quire` (A8), `quire::{desktop, shell}`
  with the part, handler trait, global and object dispatch, the refusal of management on an agent connection
  (`Standing`), the object makers, and `capture_source` (types, the book, `capture_size`, the part);
  `window_of_identifier`. `casement-agent`: `Handed`, `RefuseWhy::NoSocket`. `casement-ipc`: `Compositor1` (object, types,
  `parse_connect_role`, `ConnectRefusal::to_error`), `serve_compositor_at`, the `PeerCgroups` seam, `IpcHandles::compositor`.
  `casement`: `connect::attach`, `CompositorBus`, `RunConfig::with_compositor_bus`, the handler files, the exec arms,
  `AgentState::handed`, the state fields and constructor calls. The test kit: `FakeCgroups`.
- **Departure: the two `quire` globals and the window-capture global are declared, not created.** The task wanted the
  globals behind the filter. `quire::{desktop, shell}::GLOBAL` is a constant (`Advertised::Not`) in front of the
  `create_global` call, which is written and type-checked; the lane flips it. Why: the typed fuzz box connects `Shell`
  and `Agent` roles in a tenth of its clients and `bind_all` binds every advertised global, then sends requests to
  live objects by creation order, so an advertised protocol whose requests reach a `todo!()` would panic the fuzz gate.
  The toplevel capture global cannot be written ahead at all: smithay's `ToplevelCaptureSourceState::new_with_filter` is
  the creation, and the public socket sees it (like output capture), so P2-Q3 adds the state field and the call
  together. The I7 rows are in force already (`the_p2_globals_are_seen_by_the_right_connections`); `globals.txt` is
  unchanged (the public view), and the fuzz oracle maps the three interface names to their kinds.
- **Departure: `quire_desktop_manager_v1` is seen by `Shell` and `Agent`, not by `Shell` only.** The task brief said shell
  only for both; 20 §4.2 (`advertised on Shell and Agent connections only`), 03 and the XML (`not_permitted` on an agent
  connection) say the agent tier reads the desktop. `quire_shell_v1` is `Shell` only. The agent's management requests are
  refused in the dispatch (`Standing::ReadOnly`), before any event is made.
- **Departure: `hand_out` no longer takes a `DisplayHandle`.** `casement-agent` cannot build a client's data (its key and
  its gone queue live in `casement-wayland`'s `ClientState`), so it makes the decision and the socket pair
  (`Handed { role, server, client }`) and `casement::connect::answer` inserts the server end with
  `clients::insert_client`. `CallerUnit::of_cgroup` is new: the unit comes from the peer's `/proc/<pid>/cgroup`, read by
  casement-ipc through the `PeerCgroups` seam (fake `FakeCgroups`), because the question must reach the loop as data.
- **Departure: `ShellNote::MinimiseTarget` carries a `WindowId`, not the identifier string.** The wire layer turns the
  `ext_foreign_toplevel_list` identifier (`w<id>`, `window_of_identifier`) into the id; the model keys everything by
  `WindowId`. `ShellRequest::MinimiseTarget` carries the dock `LayerKey` and the rectangle local to it; the model makes it
  global (the dock's pin, else its anchored place) and stores it in the table P1-M2's `zwlr_foreign_toplevel`
  `set_rectangle` already writes: whichever asked last wins (open item below).
- **Departure: the Compositor1 method signature has no `#[zbus(header)]` yet.** The wire signature is the plan's (`s` in,
  `h` out); the lane adds the header argument to read the sender and asks the bus for its process id.
- **Plan facts P1 changed.** `Model::place` already carries every `PlaceTarget` (halves, quarters, centre, fill,
  restore), so "full `quire_window` placement" is flipping `PLACEMENT_POLICY` to `Act`, sending the mark when a window's state
  changes (`apply_placement` is written and called by nobody) and replacing `request_placement_is_stored`. The minimise table
  (`minimise_rects`) exists. `Model::activate_app`, `hide_app`, `unhide_app`, `hide_others` and `close_all_of` exist, so only
  `ToSpace`, `Assign` and Space `MoveTo` are stubs in the model; `ForceQuit` and `Reopen` answer `CoreNote::Deferred` (P3 and P4).
  `Model::step` had to grow two arms; 04's wording ("`quire_toplevel`, `quire_space`") is superseded by `quire_desktop_v1` (C6).
- **A8 is partial.** The `quire` feature gates the two new servers and their `WaylandStates` fields. `quire_window` stays
  outside it: every app and shell-host's identity probe use it, and moving it needs a conditional field in the P0 aggregate for
  no consumer yet. `casement` names the feature explicitly; the workspace dependency keeps the default.
- **Open items this freeze adds.** (1) A dock's `set_minimise_target` and a window-list client's `set_rectangle` write one
  table; the shell's target should win while a dock is bound (decide in P2-Q2). (2) `ContentStamp` needs a commit counter
  per window surface tree: P2-Q3 counts commits in `handlers/compositor.rs` (every commit, subsurfaces included, enters
  `CompositorHandler::commit` there) and calls `apply_content`. (3) The testclient helpers (`quire/desktop.rs`, `quire/shell.rs`, `capture/toplevel_source.rs`)
  are still one-line stubs: each lane writes its own, on `desktop_client`'s reducers where it can (as `capture/` does).
  (4) `Compositor1` is served on the session bus whenever `$DBUS_SESSION_BUS_ADDRESS` is set, so a casement started before
  P2-Q2 owns the name and `Connect` never answers; harmless (nothing calls it), and a name clash only logs.
- **Shared files touched** (outside the new modules): `casement-core` `event.rs`, `effect.rs`, `lib.rs`, `model.rs`,
  `model/commands.rs` and `model/wlr_requests.rs` (three methods `pub(super)`), `layer/book.rs`, `policy/globals.rs`;
  `casement-wayland` `Cargo.toml`, `lib.rs`, `states.rs`, `foreign_toplevel_list.rs`; `casement` `Cargo.toml`, `lib.rs`,
  `main.rs`, `run.rs`, `state.rs`, `exec.rs`, `handlers/mod.rs`, `handlers/image_capture.rs` (four early returns for window
  sources), `casement-ipc` `error.rs`, `handles.rs`, `dbus/mod.rs`; the test kit's `fakes/mod.rs`, `fuzz/oracles.rs`,
  `tests/it/globals.rs`, `tests/it/protocol/mod.rs`, `tests/it/protocol/quire_window.rs`; `ARCHITECTURE.md`.
- **Spots most likely not to compile.** `casement-wayland/src/quire/desktop.rs` (`Makers`' non-capturing closures coerced to
  `fn` pointers over a generic `D`; `Output::from_resource`; `ForeignToplevelHandle::from_resource`; the borrows of
  `DesktopParts` in the dispatch arms), `quire/shell.rs` (`post_error` on `QuireShellV1`; the `Request::SetRole` field
  types `WEnum<ShellRole>`), `casement-ipc/src/dbus/compositor1.rs` (`#[zbus::interface]` with a `todo!()` body and the
  `zbus::zvariant::OwnedFd` return), `casement-core/src/model/shell_requests.rs` (`Arranged::place`, the partial moves
  out of `self`), `casement/src/handlers/image_capture.rs` (`return capture_source::new_session(self, session, window)` moves
  `session` after `session.source()`), `casement/src/exec.rs` (the new `ShellNote` import beside `mod shell`),
  `casement-wayland/src/capture_source.rs` (`ImageCaptureSourceHandler` as the bound of `new`), and
  `casement/src/state.rs` (the three `::<State>` constructor bounds through the blanket `delegate_dispatch2!`).

## p2-q1: the desktop manager (2026-10-11)

Nothing here compiled or run (the lane does not build). Plan: 23 §1. The eleven `todo!()` of the lane and the nine
ignored tests are gone; their rows left the P2-Z table.

- **Built.** `casement-wayland::quire::desktop`: `on_bind`, `on_get_window_for`, `on_create_space`, `on_stop`, `on_app`,
  `on_window`, `on_space`, `apply_note`, and two child files (`desktop/facts.rs`, `desktop/bound.rs`). An object's
  state is a value (`AppFacts`, `WindowFacts`, `SpaceFacts`, `ActiveFacts`); each bound manager keeps what each of its
  objects was last told (`Told`) and sends only the events whose value differs, so the same note twice sends nothing
  the second time and a new object gets every event. `casement-core`: `WindowAsk::ToSpace`, `WindowAsk::Assign`,
  `SpaceAsk::MoveTo` (`model/desktop_places.rs`) and the emission of `Effect::Desktop` (`model/desktop_notes.rs`, a
  snapshot before and after each step beside `Published`), in the order Spaces, apps, windows, active, then what went.
  The test client: `quire::desktop::DesktopWatch` (a side queue, like `WorkspaceWatch`; it also binds the window list so
  `get_window_for` can name a window). `GLOBAL` is `Advertised::Yes`; the catalogue has `quire_desktop_manager_v1`.
- **Decision (orchestrator, from p2-z-dc): an activation with serial 0 is accepted from the Shell role only.**
  `desktop-client` sends serial 0 on `quire_desktop_app_v1.activate` and `quire_desktop_window_v1.activate` because its
  `DesktopCommand::Activate` has none. The dispatch never read the serial, and refuses every management request on a
  `ReadOnly` (Agent) connection before an event is made, so the rule is: the shell's activation is the person's hand (a
  dock or switcher click, as P1-M2 departure 5 did for wlr activate); an agent's stays refused. Both `activate` arms say
  so in a comment, and the tests send serial 0 from a shell and from an agent.
- **Departure: `DesktopView` owns `rects`.** The geometry of a window is the arrangement's rectangle, which the model
  does not hold (it lives in `State`). `DesktopView` gained `rects: BTreeMap<WindowId, LogicalRect>`, built in both places
  that make a view (`handlers/quire_desktop.rs`, `exec/desktop.rs`) from `state.core.arrangement`; it lost `Copy`.
- **Departure: `exec()` runs `Effect::Desktop` after the batch's relayout** (`exec.rs`, a shared file, one hunk): the
  notes carry the rectangles the relayout just made. A window whose rectangle changes with no change in its model state
  (a tiled window when a layer's exclusive zone moves) is not re-sent; a `geometry` event for it waits for the next note
  of that window. Open item for P2-SQ.
- **Departure: `Bound` has two more fields** (`asked`, `told`); `Model::output_of` is `pub` (the wire layer needs a
  window's output).
- **Departure: `state` events carry `actor_kind` unknown.** The model keeps no provenance for state changes yet; the
  actor is the provenance lane's.
- **Departure: bind order.** Spaces, apps, then a `window` event for every window, then each window's burst, `active`,
  `done`. The XML says "an app, window and space event for each"; a window's burst names its app, Space and parent, so
  those objects exist first. The golden `quire_desktop-bind.txt` was written by hand from the code and may need one
  `CASEMENT_BLESS=1` run; check the diff by eye (the masked events are the ones that depend on the run).
- **Departure: the refusal is raised on the object the request was made on** (the frozen glue's `refused`), with the
  manager's `not_permitted` code (0). The XML says the manager's; a client reads an error code in the object's own
  interface. Fixing it means finding the manager from an app, window or Space object (a lookup in `part.bound`); the tests
  check the code only.
- **Known gaps.** A client that destroys an app, window or Space object it was sent gets it created again at the next note
  for it (the object is forgotten when found dead). `attention` and `activity` are constants until a lane carries them.
  `menu_source` is `none` (the menu lane's). The window `lease` is `free`/`off` until the agent lease lane publishes it.
- **Fuzz seeds.** The catalogue line shifts the typed fuzz box's object ids (as P1-C1's globals did), and the fuzz box now
  binds the desktop manager on `Shell` and `Agent` connections and sends it requests: re-run every seed that failed
  before, and the role fuzz (`PrivilegedOffered`, `GlobalNotForRole`).
- **Spots most likely not to compile.** `quire/desktop/facts.rs` (`wire::<W: TryFrom<u32>>` over the generated enums; the
  cross-protocol enum paths `quire_window_v1::quire_window_v1::{Emphasis, PlacementMark, WindowRole}`,
  `quire_sensitive_v1::quire_sensitive_surface_v1::SensitivityLevel`, `quire_agent_v1::quire_agent_lease_v1::GlowState`;
  `Output::client_outputs(..).next()`), `quire/desktop/bound.rs` (the field-by-field borrows in `window()`; `Told` and
  `Asked` public in a private module), `casement-core/src/model/desktop_places.rs` (`window_to_space`'s borrows),
  `casement-testclient/src/quire/desktop/roster.rs` (client event field names and `event_created_child!`), and the
  `[(&str, fn); 19]` table in the agent test.

## p2-q3: window capture and full placement (2026-10-11)

Nothing here compiled or run (the lane does not build). `ext_foreign_toplevel_image_capture_source_manager_v1`
is advertised at version 1 to every non-sandboxed connection (`globals.txt` row, the sandbox `HIDDEN` table, the
I7 table in casement-core's `input.rs`, the globals test's `KINDS`, the test client's catalogue line).
`quire_window`'s `PLACEMENT_POLICY` is `Act`. Re-run the fuzz seeds that failed before: the catalogue line adds an
interface, which shifts the typed fuzz box's object ids (as P1-C1's globals did).

### The decision: an offscreen pass, not a crop of the output's last frame

A window's capture is its surface tree drawn alone into a fresh image the size of its content
(`casement_render::pass::{window_tree, render_window}`), read back like `capture_output` reads a framebuffer.
Cropping the output's last frame is wrong in three cases the brief names and right in none that matters: a
window under another window shows the other window's pixels; a window off the screen (or partly) has no pixels
in the frame; and an output at 1.5x shows the window at the output's scale, but a crop cannot honour any other
size. The offscreen draw is right for all three by construction (the test
`an_overlapped_window_is_captured_whole` pins the first). The cost is a second draw of one window per delivered
frame, paid only when the damage gate lets a frame through (a static window pays none). The image is transparent
where the window draws nothing, so a translucent window or a rounded client corner keeps its alpha.
The backend owns the renderer, so `HeadlessBackend::render_offscreen` and `TtyBackend::render_offscreen` (the
latter through the capture shadow's pixman renderer, started on the first call) run the pass; the casement
crate only builds the elements (`handlers/capture_source/render.rs`). No golden image was written (a golden needs
a run); `casement-render/tests/it/window_pass.rs` pins the pass on pixman with solid elements instead.

### Departures

- **Capture size is the window's committed size.** The content is `TextureBook::window(..).committed`: the
  declared window geometry, else the bounding box of the tree, at the scale of the output the window is on (its
  Space's output, or its home). It is not the layout rectangle.
- **A scale change is announced lazily.** A window that changes size is told its sessions' new constraints at its
  commit (and a held frame fails with `buffer_constraints`); an output that changes scale has no commit, so the
  session learns at the next frame request, which fails with `buffer_constraints` and carries the new
  `buffer_size`. The brief wanted the announce "again when the scale changes"; a proactive announce needs a hook
  in the output-change path (`core_loop`), which is outside this lane.
- **Source destruction (closed at merge):** `image_capture.rs::source_destroyed` now forwards a `ToplevelSourceId` to `capture_source::on_source_destroyed`. Was: `handlers/image_capture.rs::source_destroyed` only knows output
  sources, so a destroyed window source stays in `ToplevelCaptureBook::sources` until its window closes (a
  session made from it already works). Interface ask: forward a `ToplevelSourceId` in that handler to
  `capture_source::on_source_destroyed` (three lines in a file this lane may not edit).
- **A mark change inside one tile state is not published.** `placement` is sent from `publish_toplevel`, whose
  trigger (`model/published.rs::Listing`) compares `WindowStateKind`; Left to Right (both `Tiled`) changes no
  field, so the object is not told. Interface ask (P2-Q1's file): add `mark: PlacementMark` to `Listing`.
  `apply_placement` keeps the last mark it sent and sends only on a change.
- **`shot --window ID` has no end-to-end test.** `ShotArea::Window` (grammar `window:ID`), the planner and the
  debug object's `window_shot` are covered by the grammar table and the planner test; the debug object needs a
  private bus, so the pixels are proved through the same `window_rgba` by the capture tests.
- **Shared files touched:** `casement/src/exec.rs` (one line in `UnlistWindow`), `handlers/mod.rs`
  (`pub(crate) mod capture_source`), `handlers/compositor.rs` (one hook), `casement-ipc/src/shot.rs`,
  `casement-backend` (`headless/{mod,renderer}.rs`, `tty/{mod,readback}.rs`: the offscreen entry),
  `casementctl/src/command{,/shot}.rs`, `casement-testclient` (`conn/catalogue.rs`,
  `manage/foreign_toplevel_list.rs::handle_of`), `casement-core/tests/it/input.rs`.
- **The test client is its own session.** `WindowCapture` does not use `OutputCapture`: `desktop_client`'s reducer
  ends a capture at its first frame and ignores events after a failure, and these tests take many frames and
  watch the constraints change.

### Spots a real build checks first

`casement-render/src/pass/window.rs` (`render_window`'s bounds with `Bind<T> + ExportMem`, `tracker.render_output`
over `&[CasementElement<R>]` without `Routed`), `casement-backend/src/headless/renderer.rs::offscreen` (the pixman
image type inferred from `create_buffer`, as `canvas` does), `casement/src/handlers/capture_source.rs` and
`.../render.rs` (smithay's `ToplevelCaptureSourceHandler`, `ForeignToplevelHandle::identifier`, the borrows in
`serve` and `committed`), `casement-wayland/src/capture_source.rs` (`new_with_filter` with the concrete closure),
`casement-testclient/src/capture/toplevel_source{,/wire}.rs` (wayland-client signatures, `u32::from(Format)`).

## p2-q2: the shell protocol, Compositor1 and the hand-out (2026-10-11)

Lane P2-Q2 (not compiled; the orchestrator gates). Removed rows: the ten `todo!()` of the P2-Z table and the seven
ignored tests of this lane; two tests were rewritten (below).

- **Units are fixed, not configuration.** The plan names them (`sill.service` the shell, `cuad.service` the agent tier),
  so `role_for` is a `match`. Moving them to a typed list is a one-function change if porter's unit check wants it.
- **Departure: `on_set_minimise_target` takes the window, not the identifier string.** The dispatch turns the identifier
  into a `WindowId` and keeps it only when `QuireShellHandler::window_known` says the model has it; the part gets
  `Option<WindowId>` (`None` is `unknown_toplevel`). `on_clear_minimise_target` still takes the string and refuses one that
  is no identifier. `window_known` is new on the handler trait, implemented in `handlers/quire_shell.rs`.
- **Departure: `QuireShellPart` gained `asked` and `mods`.** `ShellNote::Role` carries the layer, not the surface, so
  `on_set_role` remembers the surface each layer asked from and `apply_note` moves it into `roles` when the model confirms.
  `mods` is the last set sent; `apply_note` sends `modifiers` only when it changed, so the edge may report after every key.
- **Modifiers are reported after every delivered key** (`core_loop/deliver.rs::tell_shell_modifiers`), including a key a
  binding took; the part drops a repeat. Modifier changes with no key (a focus change re-syncing the keymap) are not reported.
- **Role release.** `on_layer_gone` is called from `core_loop/surfaces.rs` on `Event::LayerUnmapped` (a null-buffer commit
  or a destroy that had mapped) and from `handlers/layer_shell.rs::layer_destroyed` (a destroy of one that never mapped);
  not from `handlers/compositor.rs`, which P2-Q3 edits.
- **Policy (p2-z open item 1 closed):** while any layer holds `ShellRole::Dock`, a window-list client's `set_rectangle`
  is ignored (`Model::with_window_list_rect`, one call site in `wlr_requests.rs`); the dock's target is the only writer.
  With no dock the window-list client's rectangle works as before.
- **`hand_out` sets the server end non-blocking** (the display reads it from the loop). A failed socket pair is logged by
  `casement::connect::answer` (`casement-agent` has no `tracing` dependency and Cargo files are frozen).
- **`ProcCgroups` read errors are `IpcError::Bus(String)`**, not a typed I/O source: `error.rs` is not this lane's. A
  variant `IpcError::Proc { pid, source: std::io::Error }` would be the typed form (interface ask).
- **`ConnectAsk::new`** was added: the struct is `#[non_exhaustive]`, so the test kit could not build one.
- **Test kit:** `Fixture::connect_through_handout(&dyn PeerCgroups, pid, role)` (`src/fixture/handout.rs`) runs
  `connect::answer` with the cgroup text from the fake table and pumps the granted client in. It does not keep the
  compositor's `Client` handle, so `toplevel_of` and friends are for `Fixture::connect` clients.
- **Private-bus test:** `casement-ipc/src/dbus/compositor_serve/tests.rs` carries its own `PrivateBus` copy (the Debug1
  one lives under the `debug-inject` feature) and a local cgroup table (`FakeCgroups` is in the test kit, which depends on
  casement-ipc). It needs `dbus-daemon` on PATH and `GetConnectionUnixProcessID` allowed by `dev/private-bus.conf`.
- **Not proven by a test:** that a minimised window travels to the dock's rectangle (the model test for the stored target
  and `space_persist.rs` for the genie target together cover it); the `hot_corner` and `reveal` emitters are P3's.
- **Fuzz:** `quire_shell_v1` is advertised and its interface is in the test client catalogue, so the typed fuzz box's
  object ids shifted: re-run every seed that failed before the P2-Q2 merge, and the role fuzz (`PrivilegedOffered`,
  `GlobalNotForRole`) for all four roles.
- **Spots most likely not to compile.** `casement-wayland/src/quire/shell.rs` (`quire_shell_v1::MacMods::from_bits_truncate`,
  `Corner`/`ScreenEdge`/`Shown` `TryFrom<u32>` from the scanner's enums, `shell.client() == holder.surface.client()`),
  `casement-ipc/src/dbus/compositor1.rs` (`#[zbus(header)]` and `#[zbus(connection)]` together, `BusName::from(sender.clone())`,
  `DBusProxy::get_connection_unix_process_id`, `OwnedFd::from`), `compositor_serve/tests.rs`,
  `casement/src/connect.rs` (partial moves out of `handed`), `casement-testkit/tests/it/protocol/quire_shell.rs`
  (the key code 125 reaching `logo` -> Command; `Anchor::Bottom` dock placement; `fixture.client(id).create_surface()`).


## csvc

casement as a systemd service, and its Spaces registered with accountd. Nothing here was compiled or run.

- **New dependency.** `porter-dbus = { path = "../porter/crates/porter-dbus" }` in the workspace table (beside
  `porter-core`, the same layout), named by `casement-ipc` only; `casement-ipc` also names `porter-core` (`SpaceId`).
  `Cargo.lock` needs `cargo update -w`/a build to gain porter-dbus and whatever it adds (its `zbus`, `serde_json`,
  `tokio` are already in the graph). porter must be at 71f779a2 or later (`SpacesProxy::register`). B1's allowed list
  covers core only, so no boundary row changes; check-boundary was not run.
- **Readiness.** `Notify` seam (`casement::notify`), real `SocketNotify` over std's `UnixDatagram` (a path or `@abstract`
  `$NOTIFY_SOCKET`), no-op when unset, no new crate. `run` sends `Ready { status: "listening on wayland-N" }` after
  `keymap::attach` (socket listening, outputs announced, Spaces loaded and the first registration queued) and `Stopping`
  after the loop returns, error or not. `main` reads `NOTIFY_SOCKET`, `WATCHDOG_USEC` and `WATCHDOG_PID` once
  (`Watchdog::from_env`: half the period; off when the pid names another process).
  The ping is a calloop timer in `time/watchdog.rs` (B8 keeps timers there); it fires only while the loop turns.
  `RunConfig` gained `notify` and `watchdog` (builders `with_notify`, `with_watchdog`).
- **Registration.** `SpaceRegistry` seam in `casement-ipc` (a background thread with its own runtime, one call at a time,
  queue of 64, `Pace`: eight calls then a 60 s wait, no clock read; the service allows ten new a minute). `SpaceSync`
  (`casement::space_sync`) registers each Desktop-content Space once, named `Space::plain_name()` ("Desktop N", a new core
  method now also used by `workspace.rs`), look `""` (porter: empty is the default look). Triggered by
  `Effect::PublishWorkspaces` (creation, renumbering) and once at startup; `poll()` runs every turn.
  Full Screen, Split View and Agent Spaces are not registered (transient, nothing to link to).
- **Outcomes.** AccessDenied -> logged once, no more calls this run. Service not on the bus, or `LimitsExceeded` -> the
  Space leaves the sent set and is asked again at the next Spaces change (so an accountd that starts later is only met at
  the next change; a retry timer is a possible follow-up). Anything else is logged and dropped.
- **Seams.** `Seams` gained `space_sync` and `notify` with no-op defaults in `Seams::new`, plus `with_space_sync` and
  `with_notify`, so the test kit's `Seams::new` call compiles unchanged. `run` installs the real ones: the registry only
  when `RunConfig::compositor_bus` is `At(address)` (the session bus), so a run without a bus registers nothing.
  A jail run with `DBUS_SESSION_BUS_ADDRESS` set to its private bus registers there and logs a "later" at debug.
- **Tests.** Private-bus: `casement-ipc/src/space_registry/tests.rs` (stand-in `org.quire.Accounts1` serving
  `Spaces1.Register`: arrival with the default look, refusal once and then silence, no service -> `Later`; needs
  `dbus-daemon`). Pure: `Pace`, the datagram text, a real datagram socket in a tempdir (path and abstract), `Watchdog`,
  `Space::plain_name`, `FakeNotify`; `casement-testkit/tests/it/space_sync.rs` over `FakeSpaceRegistry`.
  Not tested: `run` ordering (ready after the outputs), the timer.
- **Unit.** `dist/casement.service` (Type=notify, NotifyAccess=main, PartOf and After the graphical-session targets,
  WatchdogSec=30, Restart=on-failure with a start limit). The expected ExecStart and environment are in its header.
- **Only a real login proves:** that accountd's cgroup check gives the `compositor` role to the unit's main process (the
  Register then succeeds and `Changed(id, "created")` goes out once per new Space); that systemd accepts READY=1 within
  TimeoutStartSec on the tty backend; the watchdog under `WatchdogSec=30` over a long session; that the session bus is
  reachable from the unit (`DBUS_SESSION_BUS_ADDRESS` imported into the user manager), else registration and Compositor1
  are skipped silently.
- **Spots most likely not to compile.** `casement-ipc/src/space_registry/worker.rs` (`zbus::Error::FDO(inner)` with
  `&**inner`, `fdo::Error::{AccessDenied, ServiceUnknown, NameHasNoOwner}` tuple variants, `MethodError(name, _, _)` arity,
  `tokio::sync::mpsc::error::TrySendError`, `Builder::address(..)?.build()`); `space_registry/tests.rs` (`#[zbus::interface]`
  on a struct with `fdo::Result`, `.name(..)?.serve_at(..)?` chain order); `casement/src/notify.rs`
  (`SocketAddrExt::from_abstract_name`, `UnixDatagram::{bind_addr, send_to_addr}`, the `wrap` closure used three times,
  `Option::is_none_or`); `run.rs` (`ran` result after the borrow by the loop closure, `Ok(ran?)`); the doctests in
  `notify.rs`; `casement-core/src/space/space.rs` test imports (`crate::ids::OutputName`).


## debug-role

Test unit overrides for jailed desktop scenarios (no systemd, so no unit in `/proc/<pid>/cgroup`). Not compiled or run.

- **Contract.** Environment variable `CASEMENT_DEBUG_PEER_UNITS="<program-or-pid>=<unit>[,<program-or-pid>=<unit>...]"`.
  Left side all digits: a process id; anything else: a program name compared with `/proc/<pid>/comm` (the kernel keeps
  15 bytes, so a longer name is cut to 15 before comparing; sill and cuad fit). Right side: a service unit name
  (`name.service`, no slash). Entries are comma-separated, spaces around them are trimmed, the first matching entry
  wins. Example for "sill is sill.service": `CASEMENT_DEBUG_PEER_UNITS="sill=sill.service"`; with the agent tier too:
  `"sill=sill.service,cuad=cuad.service"`. A malformed value makes casement exit 64 with the reason on stderr.
- **Only with `debug-inject`.** `crates/casement/src/debug_peer_units.rs` (the only place the variable is named) is a
  module of the binary compiled under `cfg(feature = "debug-inject")`; `main.rs` reads it there and passes the typed
  `PeerUnitTable` through `RunConfig::with_debug_peer_units` (the field exists only under the feature). A build without the
  feature does not contain the name, so the variable has no effect; `main.rs` has a test that the module is gated and that
  the name appears nowhere else in `main.rs`.
- **Mechanism.** `casement-ipc/src/dbus/debug_units.rs` (same feature): `PeerUnitTable::parse` (pure), `PeerNames` seam
  with `ProcComm` (`/proc/<pid>/comm`), and `DebugPeerUnits`, a `PeerCgroups` that answers a matching peer with a
  synthesised cgroup text ending in the unit, else asks the real `ProcCgroups`. `run.rs::with_debug_units` wraps it
  around `ProcCgroups` for Compositor1 and logs once at warn level "TEST ONLY: test unit overrides are active" (only when
  the table is non-empty). `role_for` and `CallerUnit::of_cgroup` are untouched, so a peer still needs a unit the plan names.
- **Pid namespace.** `GetConnectionUnixProcessID` returns the pid in casement's namespace; the jail runs casement and
  sill in one, so `comm` is readable. The wrapper never trusts a peer's own claim, only the table the compositor's own
  environment gave it.
- **Desktop-facing.** `dev/lib/compositor.sh` header documents how a scenario sets it (the variable is inherited by the
  casement child).
- **Tests.** Table parsing, precedence over a fake inner seam, pid rules, first match and the comm cut
  (`debug_units.rs`); env wrapper (`debug_peer_units.rs`); the gate test in `main.rs`. Nothing proves a real Connect from a
  bubblewrapped sill: that needs the scenario run.
- **Spots most likely not to compile.** `run.rs` (`#[cfg]` on a function parameter and on a call argument; the `dyn
  PeerCgroups` coercions in `attach_compositor`); `main.rs` (the `return` inside the cfg'd `let ... = match` and the shadowed
  `config`; the `include_str!("main.rs")` gate string with escapes); `debug_units.rs` (`PeerMatch` closure borrow in
  `unit_of`, `assert_eq!(ok.rules(), [..])` comparing a slice with an array).


## p2-t3: remote input over EIS (2026-10-11)

Lane P2-T3, casement half (not compiled; the orchestrator gates). No `todo!()` was in `eis.rs` (it did not exist).

- **Crate.** `reis` 0.7.1 (MIT), already pinned in the workspace table and in `Cargo.lock` through smithay. The only manifest
  edit: `reis = { workspace = true, features = ["calloop"] }` in `casement-agent` (the event-loop source), plus the matching
  `"reis"` line under `casement-agent` in `Cargo.lock`. No other crate was added; calloop is smithay's re-export.
- **Shape.** `casement-agent/src/eis.rs`: `open(session, kinds, extent) -> RemoteGrant { source, client }`. The source is a
  calloop `EventSource` (`RemoteEisSource`) yielding `RemoteEvent::{Input(RemoteInput), Ended}`; `RemoteInput` carries
  `Origin::Remote(PortalSessionId)`, a virtual `DeviceId` (high bit set) and the `InputKind`; the root stamps the time with
  `RemoteInput::at`. Devices: keyboard, relative pointer, absolute pointer and touchscreen, each only if the portal's `types`
  bits allow it and the app binds it; absolute ones have one region at (0, 0) of the desktop's size at connect time, mapped
  back onto the desktop (the extent is not updated if outputs change during a session). An app that asks to receive input gets
  no devices. Keys and buttons still held when the app goes are released before `Ended`.
- **Hand-out.** `org.quire.Compositor1.ConnectEis(session: t, devices: u) -> (h)`. `ConnectAsk` gained `remote:
  Option<RemoteAsk>` (and `ConnectAsk::remote_eis`); `casement::connect::answer` branches to `answer_remote`, which checks
  `portal_may_connect` (`transom.service`; `role_for` is unchanged, the portal has no Wayland role: sill gets
  `RoleNotAllowed`, an unknown unit `UnknownUnit`) and inserts the source into `state.event_loop`. `State` is not edited.
- **Visibility changes.** `core_loop::tty` is `pub(crate)` and `desktop_extent` is `pub(crate)` (the extent for absolute devices).
- **Tests.** Pure: the translation functions, the device bits, the origin tag, release on end, `open` in `eis.rs`. Private
  bus: `connect_eis_reaches_the_loop_as_a_remote_ask_with_the_callers_unit`. Fixture: `tests/it/remote_eis.rs` (the unit check
  through `Fixture::connect_remote_eis`; remote keys of a bound chord fire nothing, through `Fixture::deliver_remote`).
  Not tested: a real EIS client talking to the source (no crate outside `casement-agent` may name `reis`, B14); the `transom`
  acceptance script `dev/accept/remote-desktop.sh` does that against a real compositor.
- **Spots most likely not to compile.** `eis.rs`: `inner.process_events(...)` closure types against reis' `EisRequestSource`,
  `smithay::reexports::calloop::Result`, `caps.into_iter().collect()` into `BitFlags`, the `before_done_cb` closure typed as
  `for<'a> FnOnce(&'a Device)`, `eis::handshake::ContextType` comparison, the `KeyState`/`ButtonState` paths and a
  `_` arm over them. `connect.rs`: `insert_source` with a `RemoteEisSource` whose `Ret` is `()`.


## wave-2026-10-15/session

Lane `session` (not compiled; the orchestrator gates). Three findings from the first real logins.

- **VT switch.** The existing path (`core_loop::tty::vt_chord`, before bindings and clients) only matched
  `XF86Switch_VT_n`, which xkb gives `Ctrl+Alt+Fn` only when the layout carries the server-control group
  and the F-row sends real F-keys. A keyboard in Mac mode sends media keys on the F-row (Fn held gives F1..F12),
  and a layout without the group gives no such symbol. New pure `casement_backend::tty::vt_of_chord(VtHeld,
  modified, raw) -> Option<VtNumber>`: the keymap's `XF86Switch_VT_n` wins; with Ctrl and Alt both held, an F1..F12
  symbol (modified or raw) also asks for that VT. `TtyBackend::switch_to(VtNumber)` carries it out. Table test
  `ctrl_alt_and_a_function_key_ask_for_that_vt_when_the_keymap_names_none`. Only a real run proves the session
  accepts the switch (libseat `change_vt`) and that the chord is swallowed. A Mac-mode keyboard needs Fn held to
  produce F1..F12 at all; that is the keyboard's, not ours.
- **Multi-GPU.** Not wired. Landed: pure `RenderPlan::route_of` / `routes` and `OutputRoute::{Direct, Copy}`
  (`casement-core/src/gpu/route.rs`, table test `each_output_is_drawn_on_the_render_gpu_and_copied_...`), and
  `TtyBackend::route_of`. Still open (large, needs a compiler and a second GPU): smithay's `GpuManager`/
  `MultiRenderer` through `DrmCompositor` (a `MultiRenderer` is not a `GlesRenderer`, so `FrameSource::gles`,
  `RealScanout` and `RealKms::renderer` need a generic or boxed renderer type); one `GlesRenderer` per
  render node shared across devices; `RealScanout::open` taking the route; the client dma-buf feedback already
  names the render GPU, so clients keep allocating there. Today each output is drawn by its own device's renderer, which
  works whenever the render GPU also scans out, as on the owner's NVIDIA-driven monitors.
- **Natural scroll.** `device_settings(pointer, class)` maps Touchpad to the touchpad profile and every other
  class (mouse, trackball) to the mouse profile; `configure` applies it when a Pointer or Touchpad appears;
  `class_of` makes a pointer with finger taps a touchpad. Both defaults are On. Table tests added
  (`every_pointing_class_gets_natural_scroll_from_the_defaults`, a fixture test for both devices). Gaps noted, not
  changed: settings are applied only when a device appears (a changed setting reaches a device after a replug or
  restart); a `settings.toml` written before 8b10803 holds `pointer.natural_scroll = "off"` and wins over the new default.
## wave-2026-10-15/layers

- **On-demand layers take the keyboard on mapping.** Departure from the 2026-09 rule (only when no key
  window held it): a `Top` or `Overlay` layer that asks `on_demand` takes the keyboard when it maps, over a
  key window, unless the session is locked or an exclusive layer holds it. Unmapping gives it back to the
  key window (`Model::demand` is cleared with the layer). A `Bottom`/`Background` on-demand layer still
  takes it only when pressed. A click still takes it (`press_layer`); a window pressed or keyed takes it
  back. Tests: `an_on_demand_panel_takes_the_keyboard_on_mapping_and_gives_it_back` (model_layers.rs),
  `layer_keyboard.rs` (the old "a mapped layer does not take it from a window" assertion now says the
  opposite).
- **Layer placement was already per-bar; the hit box was not clipped to it.** `Arranged` stacks two bars on
  one edge (tests exist, plus `bars_stacked_on_one_edge_each_own_their_box`). The hit test, though, used only
  the surface's own input region, so a surface whose buffer outran its placed box (a drawn shadow margin)
  took input there. `Candidate` now carries `clip: Option<LogicalRect>`; a layer's root is clipped to its
  placed box, popups and windows are not. Needs a real run: a sill panel with a shadow margin must not
  swallow a click on the window beside it, and its shadow must still draw.
- **Shadows are not clipped by casement.** `layer_elements` renders the whole surface tree
  (`render_elements_from_surface_tree`) from the placed origin with no crop to the configured size, and
  `committed` is the bbox of the tree. A transparent margin therefore renders. If sill's shadows look cut
  off, check the buffer sill commits (size vs the layer size it was configured to) before suspecting us.
  Note placement follows the configured size, so a client that draws a margin must ask for a layer size
  that includes it.
- **A bound chord fires over an exclusive layer (intended).** `deliver_key` resolves bindings for every
  keyboard target except under a lock or a shortcuts inhibitor, as macOS's Cmd+Tab works over everything.
  The binding takes only the chord's final key (and its release); the modifier's press and release are
  forwarded, so the switcher hears the Cmd release. Test:
  `a_chord_fires_while_an_exclusive_layer_holds_the_keyboard_and_it_hears_the_modifier_release`
  (casement-testkit tests/it/bindings.rs).
## wave-2026-10-15/wlcs

Written without a build or a wlcs run (wave rules); every claim below is from reading the wlcs tests,
smithay's source at fork rev 9c8f762 and our code. The orchestrator's run settles them.

| Finding | Root cause | What changed |
|---|---|---|
| Text-input `enter` must precede the toplevel's first configure (2 tests) | The lenient policy sent a bare initial configure at the first commit and mapped (keyed) at a second pass, so every configure was ahead of the keyboard enter. | `casement_wayland::xdg_shell::keyed` (`map_keyed_on_first_buffer`, `send_initial_configure`), used by `handlers/compositor.rs` under `testing` + `Lenient`: a first commit that carries a buffer maps the window with a valid token, the model keys it and queues its state, then the one configure goes out. Test: `casement-testkit/tests/relax.rs`. |
| Decoration `configure` expected after `set_mode` with no commit (3 tests) | The reply rides on an `xdg_surface.configure`, and none is due before the first commit. The decoration XML allows a configure at any time. | `handlers/xdg_decoration.rs`: under `Lenient` (`testing` only) the initial configure goes out with the decoration's answer. Production is unchanged. |
| W9: window at y = -5 | Layout pushes a floating window inside the usable area (the person-facing rule). A test-requested position is not the person's drag. | `Event::WindowPlaced`, `WindowPlacement::Placed`, `Model::is_placed_by_hand`: a hand placement is kept as given (size still fitted) until a drag or the unmap; the shim uses `Fixture::place_window_by_hand`. The drag rule is unchanged. Tests: `rust_scene.rs`, `model/tests.rs`. |

Kept, with reasons (also in `dev/wlcs/expected-failures.txt`):

- W3 (4 xdg_wm_base tests): smithay's `get_xdg_surface` has no hook; the error must be raised in the fork.
- W4 `wm_capabilities_are_sent`, decoration `orphaned`: smithay's send point and error object; a configure at creation would break the text-input order. Fork patches.
- W7 (6 tests): four are smithay's subsurface position model (applied at the child's commit and by `commit_sync_surface_tree`, not at the parent's state); two (`place_above/below_simple`) assert a focus no compositor can give. Fork patch described in the file.
- W12 primary selection (5): kept policy, a program that is not used cannot replace the selection.
- `#[ignore]`: `casement-layout/tests/it/spread.rs` (P3-Z) and `casement-scene-css/tests/parity.rs` (P3-CSS) wait for code that is not written (`rust_scene/spread.rs` does not exist; CssScene bodies are `todo!`); they stay ignored. (Both are now filled: spread.rs by P3-Z, parity.rs by P3-CSS, `#[ignore]` removed.)

To check in a real run: wlcs counts (expect 6 more passes, no new failures; the first configure of every
lenient toplevel now carries its final state, so a wlcs test that counts configures could move); that
`Event::WindowPlaced` windows stay put through other windows mapping.
## wave-2026-10-15/capture

Nothing here compiled or ran (the lane does not build). Real session finding: `grim` failed with "failed to copy
output".

- **Cause 1: no `zwlr_screencopy_manager_v1`.** casement served only ext-image-copy-capture. Added
  `casement_wayland::screencopy` (version 3; smithay has no plumbing for it, so the wire is ours on
  `wayland-protocols-wlr`'s server side, through smithay's re-export): `capture_output`, `capture_output_region`,
  `copy`, `copy_with_damage`, shm `argb8888` (and `xrgb8888` from version 3, with `buffer_done`). The frame is the one
  `ext_image_copy_capture` serves (`handlers/image_capture/frames.rs::last_frame`), so the two paths agree byte for byte.
  Region: output-local logical pixels, `round(l x scale)` per edge, cut to the output (`physical_region`, table-tested),
  cropped from the frame (`CapturedFrame::cropped`). A region of a turned (non-Normal transform) output, or one with
  nothing of the output in it, is `failed`; a whole-output copy of a turned output is the mode-sized frame, as in the
  ext path.
- **Cause 2 (likely the actual one on a tty): the first frame of a session failed.** On a real seat the shadow draws
  only while a session is open, so a one-shot client (grim) always asked before any shadow frame existed and got
  `failed` (FINDINGS p1-t listed this as accepted). Now a request that finds no frame is held (ext:
  `ImageCapturePart::waiting`; wlr: the pending list) and served after the next frame the loop presents
  (`handlers::serve_captures`, called at the end of `core_loop::frame`). Because a still desktop flips nothing
  (`PresentOutcome::Unchanged`), `TtyBackend::present` now also draws the shadow's first frame on `Unchanged`
  (`Readback::draw_first`). Real run must prove: `grim` on the tty session returns within a frame or two on a static
  desktop, and the shadow frame is not stale-looking (it is drawn from the same scene source as the flip).
- **Limits (departures).** No `linux_dmabuf` offer (shm only). `flags` is always 0 (rows top first). The cursor
  and per-region damage limits this lane listed are closed in `wave-2026-10-15/capture2`.
- **Privacy decision.** The existing policy already allows grim: output capture (`GlobalKind::ImageCapture`) is
  visible to the public socket, the agent tier and the shell, and hidden from sandboxes (I7); window capture is the same
  (P2-Q3). There is no consent step in either path today. I kept that, and gave `zwlr_screencopy_manager_v1` the same
  `ImageCapture` kind (no new `GlobalKind`), so exactly the connections that could use ext output capture can use it:
  the person's own tools launched in the session work as grim does elsewhere, a Flatpak/sandbox client never sees the
  global. If a consent prompt for the public socket is wanted later it belongs on `ImageCapture` for both protocols at
  once.
- **Expected grim invocations after this change** (to confirm in a real run): `grim out.png`, `grim -o HEADLESS-1
  out.png`, `grim -g "x,y wxh" out.png` (grim crops itself on the ext path and asks the compositor for the region on
  the wlr path; both are served), `grim -` to stdout. `slurp` needs `zwlr_layer_shell_v1` (already served).
- **Tests.** `tests/it/protocol/screencopy.rs` (whole output equals the scanout frame; region crops at 1x and at 2x
  logical pixels; region off the output fails; `copy_with_damage` reports the whole area first; other output unaffected),
  `sandbox.rs` row, the globals golden row `zwlr_screencopy_manager_v1 3 screencopy`, the fuzz oracle's interface map;
  unit tests for `physical_region` and `CapturedFrame::cropped`; client `casement_testclient::capture::ScreenCopy`.
  Not covered here (needs a real seat): the tty first-frame hold.
## wave-2026-10-15/capture2

Nothing here compiled or ran (the lane does not build). Follow-ups to `wave-2026-10-15/capture`.

- **The frame without the cursor.** Until now every capture was the scanout frame, cursor included (the capture
  section's "the cursor is never painted" was wrong on headless and on the tty shadow: both draw the scene with the
  cursor). `HeadlessBackend::present` now draws each frame a second time without the elements whose `Kind` is
  `Cursor` (`headless/bare.rs`: a `BareLayer` of its own canvas and damage tracker per output, made with the first
  frame, so the second render is incremental too) and keeps it as `PresentedFrame::bare` beside `pixels`. The tty
  shadow is a `HeadlessBackend`, so it gets the same. Cost: a second render of every frame the headless backend
  presents (jail, tests) and of every frame the shadow draws while a capture is open; nothing on a real seat with no
  capture. If that is too dear, make `bare` lazy behind a `Watch`-like switch; the handlers only need `bare`
  to be present when asked for.
- **wlr `overlay_cursor`.** 0 reads `bare`, anything else reads `pixels` (`CursorPaint::from_overlay`, carried in
  `PendingCopy::cursor`). `grim -c` is `overlay_cursor` 1. Region copies crop whichever frame was chosen. If a renderer
  made no `bare` (the dummy renderer has no pixels at all), the frame with the cursor is served.
- **ext `paint_cursors`.** `SessionRef::draw_cursor()` picks the same two frames. The existing behaviour changed: a
  session without `paint_cursors` no longer shows the cursor (the spec says it must not). Tests that compare a
  capture with the scanout now ask for `Painted` (`a_capture_equals_the_scanout_frame`, the wlr whole/region tests).
  Window capture (`ext_foreign_toplevel_image_capture_source`) never had a cursor and ignores the option.
- **ext cursor sessions** (`ext_image_copy_capture_cursor_session_v1`; smithay serves the wire,
  `handlers/image_capture/cursor.rs` the content). Only output sources have one: a window source is stopped. After
  every frame the output presents (`serve_captures`) and when the session is made, `sync` tells each session on the
  output `enter`, `hotspot` and `position`, or `leave`. Position is the hotspot in the frame's pixels and the
  hotspot event the offset inside the image, both at the output's scale (`casement_wayland::image_capture::place`,
  pure and table-tested at 1x, 1.5x, 2x, under every transform and across two outputs). The image is the compositor's
  own cursor raster at the output's scale (`casement_render::{cursor_face, cursor_pixels}`, the same raster the
  element draws), announced as the session's buffer size and re-announced (the frame fails with
  `buffer_constraints`) when the scale changes. A request is answered once and held until the pixels differ.
  Departures: (1) a cursor a client drew itself (`CursorDraw::ClientSurface`) is placed and sized from its surface
  view, but its frame fails (`unknown`): reading the client's buffer back is not done; (2) under a rotated output the
  position is turned with the frame but the image is sent upright, the hotspot unturned; (3) the cursor's corner is
  `round(position) - round(hotspot)` in physical pixels, which can differ by one pixel from where the element lands
  at a fractional scale; (4) the session reports `enter` when any pixel of the image meets the frame, as the spec
  says, not when the hotspot does.
- **Real damage.** The renderer already tracks it: `render_output` returns the tracker's damage and the headless
  backend now keeps it on each `PresentedFrame` (empty for a frame that re-showed the last). The composition root
  notes every presented frame once into `ImageCapturePart::damage` (`DamageLogs`, per output: the last 32 frames with
  damage, merged to one bounding box past 16 rectangles). Both protocols ask `since(output, the instant of the
  client's last frame)`: `Whole` for a client's first frame or one older than the log, else the rectangles of the
  newer frames, cut to the copied area and moved to its corner (`Since::within`).
  - wlr `copy_with_damage` and every ext frame after a session's first now wait until the damage since the last
    frame is not empty (the spec lets ext wait; wlr always waited). Headless presents a frame each vblank, so the old
    rule ("a newer frame") answered every vblank; the new one answers on a change.
  - Limits: the damage is the frame with the cursor's; a session that hides the cursor is still woken by pointer
    movement (the frame it gets is unchanged). Damage is in the frame's own pixels, so a turned output reports it
    turned like the frame (the transform event stays `normal`, as before). A tty shadow is dropped when no capture is
    open, so a client that returns after a gap gets a whole frame (the new shadow's first frame is whole damage).
- **wlr copies now watch the tty shadow.** `core_loop::tty::capture_watch` only looked at ext sessions, so a one-shot
  `grim` on a real seat never got a shadow frame: its copy waited for a frame that was never drawn (the capture lane's
  "likely actual cause" was only half fixed). It now also counts a waiting `zwlr_screencopy` copy
  (`ScreencopyPart::wants`). Real run must prove: `grim` on the tty returns. A client that copies repeatedly
  (`wf-recorder`) keeps the shadow only while a copy is queued; between two copies the shadow may be dropped and
  redrawn (a whole frame each time) until a long-lived watch for screencopy is added.
- **Tests.** Table tests: `DamageLog`/`Since` (accumulation, repeats, an old client, too many rectangles, the cut to an
  area), `place` (scales 1, 1.5, 2, the eight transforms, partial overlap, two outputs, off the output),
  `image_size`. Protocol tests (`image_capture.rs`, `screencopy.rs`): painted equals the scanout and hidden differs
  only in the cursor's box, for ext and for wlr; a second ext/wlr frame waits and then reports the cursor's old and new
  place; region damage is in region pixels; a cursor session's `enter`/`hotspot`/`position`/`leave` across two outputs;
  position and hotspot at 1.5x; the cursor image frame (24 x 24 at 1x, an opaque body, a clear corner, not resent).
  The test client got `OutputCapture::open_cursor`/`take_cursor_events`, `ScreenCopy::capture` takes a `CursorMode`
  and `Copied::damage` lists every rectangle. Catalogue: `ext_output_image_capture_source_manager_v1`,
  `ext_image_copy_capture_manager_v1` (and through it the cursor session) are roots now; the globals golden is
  unchanged (no new global).
- **Needs a real run.** `grim -c out.png` shows the arrow; plain `grim out.png` does not. A screencast (OBS through
  the portal, ext path) shows no cursor unless it asks for one. A cursor session in a client that composites it
  itself (a remote-desktop viewer) follows the pointer across two outputs and at a fractional scale. `grim` returns on
  the tty.

## wave-2026-10-15/ssd

Server-side decorations (the owner's first-login review: apps that draw no frame showed the client's grey bar). Written
without a compiler (wave rule); the orchestrator builds.

- **Negotiation.** `xdg_decoration::mode_for`: `server_side` and `unset` (and a decoration object that never wished
  anything) are answered server-side, `client_side` stays client-side. The part remembers the answer per window
  (`XdgDecorationPart::mode_of`, dropped in `toplevel_destroyed`); the model is not told (departure from the handler recipe:
  the answer is a protocol-edge fact only drawing and hit-testing read, so no `Event`). A window whose client never bound
  the manager draws its own frame. Golden `xdg_decoration-mode.txt` now says `configure(2)`; the protocol test was rewritten.
- **Where the code is.** Pure and tested: `casement-core/src/window/frame.rs` (titlebar and lights geometry, hit map, edges),
  `frame_press.rs` (double-click tracker, `chrome_action`), `frame_draw.rs` (resolved colours as data);
  `casement-layout/src/frame.rs` (chrome lengths from `ChromeToken`, colours from `ColourToken`, `light_state`'s reveal rule);
  `casement-render/src/element/frame{.rs,/raster.rs}` (titlebar and shadow rasters, analytic coverage, no text);
  `casement/src/core_loop/frames.rs` (which windows, hover, press glue). No new crate and no Cargo edit: casement-core has no
  ds-style, so the lengths travel as `FrameMetrics` data, filled in layout; casement reaches ds-style types through
  `casement_layout::{Scheme, GroupHover}` re-exports.
- **Geometry choice.** The titlebar is drawn just above the window's content rectangle (the model rect stays the client's
  size), so a window at the top of the usable area has its bar under the menu bar. The resize zones are 4 px strips outside
  the outline plus 12 px corner squares from the outer corners (so 8 px reach inside the content at each corner).
- **Hit and input.** `frames::under` walks the scene's surfaces front to back and tests a decorated window's frame at its
  root's place, so a window in front hides the frame behind it; popups come first. A press on a frame is never delivered, its
  release is swallowed (`seat.taken`), the model routes it as a titlebar or edge press (activate, raise), then the part acts:
  close = `Effect::SendClose`, minimise and zoom = `ToplevelRequest`, drag = `ToplevelRequest::Move` (the existing grab),
  edge = `Resize`, double click = `windows.double_click_title`. Lights act on release inside, as on the Mac (chrome2).
- **Unreachable from quire's pure crates, so written here (asks below):** the window radius 10 (`--r-window` is a CSS string,
  and still 26 until quire's batch), `--shadow-window` (CSS string, hand-copied as two `ShadowLayer`s), the three light hues
  and their mark inks (stylesheet candy colours), and (until chrome2) the colour scheme.
- **Not done.** (1) Title text: casement had no
  text stack (`casement-chrome`, B15), so the titlebar title was empty. (Done by chrome3: `casement-chrome` draws it.) (2) The green light has no Move and Resize menu, and
  the minimise animation target is untouched. (3) The frame is not drawn in the Mission Control and Space-slide passes (they
  go through `output_elements` with the same `LayerDraw`, so it follows there only if `frames` is filled, which
  `core_loop::frame` does for every output frame). (4) The shadow is a square-cornered blur at one raster pixel per logical
- **Not done.** (1) Title text: casement has no
  text stack (`casement-chrome`, B15), so the titlebar title is empty. (2) and (3) are done by winmenu (see `wave-2026-10-15/winmenu`: the green light's window menu, built up to the wire; the minimise target already
  existed and is tested; the frame in the moving passes). (4) The shadow is a square-cornered blur at one raster pixel per logical
  pixel, scaled up; cost on a window being resized is a new raster per frame (cache limit 64 entries). (Done by
  chrome2: layout reserves the titlebar, lights act on release, Option-click zoom, the colour scheme from settings; done by
  clip: the rounded bottom corners.)
- **Interface asks for quire** (exact):
  1. `ds-style/src/tokens/shape.rs`: `impl Radius { pub fn px(self) -> u16 }` returning the number `value()` writes (Window
     10 in the next batch), so casement stops hard-coding `WINDOW_RADIUS`.
  2. `ds-style/src/tokens/elevation.rs`: numeric `Shadow::layers(self, scheme: Scheme) -> &'static [ShadowLayer { dx, dy, blur,
     spread, rgba }]` for `Window`.
  3. `ds-style/src/tokens/chrome/light.rs`: `pub fn light_hue(light: Light, scheme: Scheme) -> (Hex, Hex)` (disc, mark ink)
     for close, minimise and zoom; the candy colours are stylesheet-only today.
  4. A title font family and size token reachable as numbers (`--fs-control` 13/600) for the future `casement-chrome`.
- **Gate and real run.** Gate: everything here is uncompiled; likeliest compile faults are `frames.rs` (let-chain over
  `(styles.get, contents.get)`, `Over` visibility through `FrameOver`), `frame.rs` raster (`MemoryRenderBuffer::from_slice`
  and `from_buffer` argument types copied from the cursor), `light_state`/`LightFill`/`Shown` matches (a `non_exhaustive`
  would need a wildcard). Real run: a foot window shows the bar, lights (grey when inactive, marks on hover), shadow; drag
  moves; double click zooms; edges resize; click on the red light closes; a Qt app that insists on client-side keeps its bar.

## wave-2026-10-15/chrome2

Follow-ups to the server-side frame (the ssd lane's "Not done" list). Written without a compiler (wave rule); the
orchestrator builds.

- **Layout reserves the titlebar.** `WindowSpec::reserve_top` (new field, 0 for a window that draws its own frame) is the
  height of the frame above the content. `RustScene` arranges a window in `below_chrome(usable, reserve)`, the usable area
  minus its top `reserve` rows, so the outer frame (content plus bar) is what fill, first placement (centre and cascade),
  the size fit and the keep-inside clamp hold inside the usable area; a hand-placed (`Placed`) window keeps its position and
  only its size is fitted. The model's rect stays the client's content size. The model does not know who draws a frame (the
  decoration answer is an edge fact), so `casement_layout::spec_of_framed(model, &BTreeMap<WindowId, u32>)` takes the
  reserves and `spec_of` is `spec_of_framed` with none; `core_loop::relayout` passes `frames::reserves(state)`. The xdg
  decoration handler now runs a relayout when a window already in the model is answered server-side (the answer can arrive
  after the first placement). Tiled windows have no desktop target in layout today (`WindowPlacement::Tiled` is skipped in
  `rust_scene/desktop.rs`), so the tile reserve has nothing to hold; when tiling is arranged it must use `below_chrome` too.
  A drag in flight is not held by the reserve (the grab follows the pointer); layout clamps it when the drag ends.
- **Lights act on release.** `casement_core::window::LightArm` (`frame_arm.rs`) is the pure machine: `Press` arms and shows
  pressed, `Release` over the same light fires (`ArmOutcome::Fire`), `Move` off it cancels, back on re-arms, a release of
  another button leaves it held, `Lost` forgets. `frames::press` arms on a light and does nothing else; `frames::released`
  (called from `deliver::press_edge` where the swallowed release is recognised) steps the machine at the pointer's place and
  carries out what fires. `casement_layout::pressed(style, light)` draws the held light darker with its mark.
- **The green light.** A plain click is `ChromeAction::FullScreen(Toggle)` -> `ToplevelRequest::FullScreen { toggle, output:
  None }` (it exists). Option held at release is zoom (`Maximise`), today's behaviour. `chrome_action` takes the `MacMods`.
  Double click on the title still follows `windows.double_click_title`. The hover menu of the Mac's green light (Move and Resize,
  tile) is not built. A full-screen window draws no frame, so leaving full screen is by the app, its key or the shell.
- **The colour scheme.** `casement::appearance` reads quire's `appearance.toml` (`AppName::QUIRE`, the file ds-settings owns)
  through a `Store`, puts `casement_layout::scheme_of(appearance.theme)` in `RenderState::scheme`, and follows the file with
  `Store::watch` (ds-settings' task runs on a small thread-per-task `Spawner`, `block_on` by parking; casement runs no async
  runtime). The watch forwards the scheme over a calloop channel; the next vblank draws the frame in it (`styles` reads it each
  frame). `theme = "system"` paints Light: following the desktop needs the settings portal, which is ds-settings' `quire-desktop`
  feature and is off in the workspace's `ds-settings` dependency. Ask for the orchestrator: if `system` should follow the
  desktop, enable that feature (a Cargo edit) and read `SystemPrefsSource::Portal` once at startup and on its watch.
  Only the frame follows the scheme; the cursor and everything else casement draws is scheme-free.
- **Departures.** `LightArm` lives in `casement-core::window` with the other frame machines, and the scheme reader is
  `casement::appearance` (not `config`, which is the casement document). New public items: `spec_of_framed`, `scheme_of`,
  `pressed`, `Theme` (re-export), `LightArm` and friends, `appearance::scheme_in`; each has a doc test.
- **Goldens and expectations.** None edited by hand. A real-frame golden or a testkit scenario that maps a server-side window at
  a `Floating` rect whose top is within 28 px of the usable area now moves that window down by the difference; the xdg
  decoration protocol test places nothing by hand, so none is expected to change, but the gate will say.
- **Gate and real run.** Likeliest compile faults: `frames.rs` `decorated()` (returned `impl Iterator` over the model's
  `BTreeMap` iter, edition 2024 capture), `appearance.rs` (the `Watch<AppearanceFile>` moved into a `Send` future; `Loaded`
  field names; `ds_core::spawner` reachable without a feature), `LightArm` being `non_exhaustive` (matched only through
  `==`/`step`), `casement_layout::Theme` re-export. Real run: press a light and drag off it (it un-darkens), back on (darkens),
  release off it (nothing happens), release on it (acts); green click goes full screen, Option-click zooms and a second one
  restores; a window opened at the top of the screen has its bar under the menu bar's exclusive zone, not beneath it; fill
  leaves the bar visible; set `appearance.theme = "dark"` in `~/.config/quire/appearance.toml` and the frames turn dark
  without a restart; windows with client-side frames are unchanged.
## wave-2026-10-15/clip

Rounded bottom corners for the windows casement decorates (chrome2's design, built). Written without a compiler (wave rule);
the orchestrator builds.

- **What is clipped.** A window in `LayerDraw::frames` (its client was answered server-side and it is not full screen or in the
  dock; `core_loop::frames::framed` is that rule, table-tested), minus any window in `LayerDraw::full_screen`. A filled (zoomed)
  window keeps the radius, as on the Mac; full screen has square corners. A window with its own frame is not in `frames`, so it
  draws its own corners. The radius is the frame's (`FrameLook::radius`, `WINDOW_RADIUS` 10), times the output scale, rounded.
  Only the two bottom corners round: the titlebar above rounds its own top corners and meets the content square.
- **The element.** `RoundedClipElement<R>` (variant `Clip` of `CasementElement`, `element/rounded_clip.rs`) owns the
  window's `WaylandSurfaceRenderElement` and a `ClipShape` (the content rectangle in output pixels plus the physical radius,
  `rounded_clip/shape.rs`, pure). `window_elements` takes `Option<ClipShape>` and wraps every surface of the window's tree, so
  subsurfaces are clipped to the same rounded rectangle. Popups come from `popup_elements` and are never wrapped. `id` is the
  surface's id namespaced (2) so a window gaining or losing the clip is a new element to the damage tracker; `current_commit`,
  `src`, `geometry` and `damage_since` forward; `opaque_regions` forwards minus the two radius-sized corner squares (smithay
  draws opaque regions with blending off, which would paint the corners over); `underlying_storage` is none, so a clipped window
  is never scanned out unclipped.
- **The hook and the shader.** `Compositor::draw_rounded_clip` (default: the surface drawn unclipped) beside the blur hooks;
  `Routed::draw` in `damage.rs` sends `Clip` to it; the GLES implementation is `rounded_clip/gles.rs`. One texture shader
  (`compile_custom_texture_shader`, compiled once per EGL context and kept in `egl_context().user_data()`) multiplies the
  texel (after smithay's own `NO_ALPHA` and alpha handling) by an analytic coverage, a signed distance to the rounded rectangle
  with radius 0 above the middle and the window radius below it, `clamp(0.5 - distance, 0, 1)`, about one pixel of anti-alias.
  The shader finds its pixel by inverting smithay's texture matrix from `v_coords` (uniforms `to`, `from`, `shape`, `radius`;
  derivation in the file header), the same formula as `ClipShape::coverage`, which carries the table test. No `gl_raw.rs` change
  and no unsafe.
- **Known gap.** Pixman, the dummy renderer and the multi-GPU copy path draw the window unclipped (the default of the hook), so
  the bottom corners are square there. On GLES a surface whose texture is rotated, flipped or upside down (`is_y_inverted`; a
  surface with a buffer transform) or a single-pixel-buffer surface is also drawn unclipped. A window capture
  (`pass::window_tree`) and the window's shadow are unchanged: the capture shows the square corners of the client's buffer.
- **Tests.** Table tests: `ClipShape` coverage inside, outside, on the curve and at the corner pixel at scale 1, 1.5 and 2, the
  ramp width, the corner squares, the radius per scale (`shape.rs`); which window states are framed (`frames.rs`). A doc test
  on `ClipShape::new`. Testkit: `tests/it/scene/clip.rs` maps a decorated 400x300 window of one colour on llvmpipe and reads the
  shot: the two bottom corner pixels are not the client's colour, 12 px in they are, the top corners and the edges beside the
  curve are. No golden changed (no existing scene binds the decoration manager).
- **Gate and real run.** Likeliest compile faults: the `render_elements!` variant `Clip = RoundedClipElement<R>` (the macro adds
  `TextureId: 'static` and `Renderer`, the impls ask `Texture + 'static`), `surface.id().namespaced(...)` on a borrowed element
  in `RoundedClipElement::new`, `Compositor::draw_rounded_clip`'s default (`clip.surface().draw` needs `RenderElement` in scope),
  `egl_context().user_data().insert_if_missing` (a non-threadsafe entry, as the blur's cache), `Uniform::new` with tuple values,
  `OpaqueRegions` collected from `Vec`s in `opaque_regions`. Real run on foot: the bottom corners are round in light and dark
  (set `appearance.theme` in `~/.config/quire/appearance.toml`); a full-screen window has square corners; a zoomed one keeps
  them; there is no seam or light line between the titlebar and the content at the sides (both use the same rounded edges of
  `window_content`); a window moved or resized across a fractional scale keeps a one-pixel anti-aliased edge; a Qt app that
  insists on client-side decoration is unchanged.

## wave-2026-10-15/winmenu

The green light's window menu, the minimise target, and the frame in the moving passes (ssd's "Not done" 2 and 3). Written
without a compiler (wave rule); the orchestrator builds.

- **The menu, pure.** `casement_core::window::menu`: `MenuEntry` (15, `ALL` in menu order, `label()` in Mac words, `group()`
  Top / Move & Resize / Full Screen Tile, `action()` -> `MenuAction::{FullScreen(Toggle), Place(PlaceTarget), SplitTile(SplitSide)}`,
  `code()` / `from_code()` the wire numbers), `menu_for(&WindowState, SplitSupport) -> Vec<MenuItem>` (which entries are enabled:
  full screen offers only Exit Full Screen, a window in the dock nothing, a tile zone it already fills and Fill when zoomed are
  dimmed, Return to Previous Size only when tiled or zoomed), `enabled_mask` (bit per entry code), `menu_point` (left edge of the
  light, 4 px under it). Table tests in `window/menu/tests.rs`.
- **Hover intent, pure.** `window::menu_dwell`: `Dwell` machine (`Idle`, `Waiting`, `Shown`, `Spent`) stepped by `DwellInput::{Over,
  Tick, Pressed, Lost}` with `DwellTiming::MAC` (500 ms) as data; answers `Arm(instant)`, `Cancel`, `Show(light)`. Only the zoom
  light counts. A click spends the light until the pointer leaves, so a click never opens a menu a half second later. Table test
  of 18 rows in `window/menu_dwell/tests.rs`.
- **In the loop.** `core_loop/menu.rs`: `frames::hover` feeds the pointer's light to the machine, `frames::press` feeds `Pressed`;
  the timer is `TimerKey::LightDwell` on the model's book (`Effect::ArmTimer` / `CancelTimer`, the way `ImeHold` is armed);
  `feed` hears `Event::TimerFired(LightDwell)` (`menu::hear`) and, when the machine says `Show`, feeds
  `ToplevelRequest::WindowMenu { at }` for the window. That is the same request `xdg_toplevel.show_window_menu` makes
  (`handlers/xdg_shell.rs` already feeds it), so a client-drawn frame's menu and the green light's go one way.
- **In the model.** `model/window_menu.rs`: `WindowMenu { at }` now answers `Effect::Shell(ShellNote::WindowMenu(WindowMenuShow
  { window, at, items }))` (it was ignored). The shell's answer is `ShellRequest::WindowMenuPick { window, entry }`; the model
  carries it out through `place` (tile zones, Fill, Centre, Return to Previous Size = `PlaceTarget::Restore`) and
  `request(FullScreen)`, and only when the entry is enabled for the window's state now (a stale pick changes nothing). Tests:
  `tests/it/model_window_menu.rs`.
- **Departure: the wire step is not built.** The XML has no event for "show the window menu" or request for a pick, and
  `casement-protocols` is not this lane's. `casement_wayland::quire::shell::apply_note` has an explicit empty arm for
  `ShellNote::WindowMenu` naming this section; until the protocol lands the menu is asked for and nothing shows. No new
  interface, so nothing was added to the test client catalogue (the ask below adds the event and the request; the catalogue's
  `quire_shell_v1` root already binds the object).
- **Departure: Full Screen Tile is dimmed.** "Left of Screen" and "Right of Screen" put two windows in a Split View; the model
  has the Space kind (`SpaceContent::SplitView`) but no command that makes one. `window_menu.rs` passes
  `SplitSupport::Unavailable`, so both entries are sent dimmed and a pick of one does nothing. Flip the const when a command exists.
- **Minimise target (item 2).** It was already built: sill sends `set_minimise_target` (dock-local rectangle per toplevel,
  `sill-services` `DesktopCommand::SetMinimiseTarget`), the model stores it in global pixels (`with_dock_target`; a window-list
  client's `set_rectangle` only counts with no dock), `minimise_destination` picks the tile else a point at the bottom centre of
  the window's output, and `casement-motion` flies a minimising window to it (`Cause::Minimise`, the `Move` spring, 380 ms
  response; the window shrinks to the tile's size and fades). New: a table test of the choice
  (`model_shell.rs::a_window_minimises_toward_its_tile_else_the_bottom_centre_of_its_output`). Not done: the setting
  `windows.minimise_effect` (Genie / Scale) is stored but motion has one effect; and the renderer draws a surface at its
  committed size (no rescaling wrapper, `window_elements`' note), so the flight is a move plus fade today and the shrink shows
  only once that wrapper exists.
- **Frame in Mission Control and Space-slide (item 3).** Both passes are `output_elements` over the sampled `FrameScene`
  with the `LayerDraw` that `core_loop::frame` builds, so a framed window's frame, shadow and clip are drawn wherever motion
  places it (a Space slide moves `at`; the frame follows). There is no separate overview pass in the loop yet (no overview
  request is handled), and surfaces are not scaled, so there is no "scaled size" to draw the frame at: when the rescaling wrapper
  lands, `scene.rs` computes `content` (`window_content`) from `placed.size` over `placed.committed` and the frame follows from
  that one call. Two real gaps found and fixed: (a) `frame_elements` ignored the window's opacity, so a fading window kept an
  opaque titlebar and shadow (now `Some(alpha)` on both memory elements); (b) `framed` is false for a minimised window, so the
  frame vanished the instant a window was minimised instead of flying with its content (`frames::drawn` adds the dock state for
  drawing only; hit, reserve and clip rules use `framed` as before). A window that closes still loses its frame at once (no
  model entry to style it).
- **Interface asks.** In the lane report: the `quire_shell_v1` XML (event `window_menu`, request `window_menu_pick`, enum
  `window_entry`) and sill's side.
- **Gate and real run.** Likeliest compile faults: `window/menu.rs` (`Toggle`/`PlaceTarget`/`TileZone` imports and the match on
  `entry.action()` in `zone`), `menu_dwell.rs` or-pattern with bindings in `over`, `model/window_menu.rs` calling `self.request`
  / `self.place` (both `pub(super)` in `requests.rs`; `request` consumes `self`), the new `TimerKey::LightDwell` variant (only
  `model/timers.rs` matched it exhaustively), `core_loop/menu.rs` using `frames::{contents, styles}` now `pub(super)`,
  `styles_where` taking a `fn` pointer built from a path, `frame_elements`' new argument. The 1920x1080 assumption in the
  bottom-centre test (`rect(960, 1080, 0, 0)`). Real run (after the wire lands): hold the pointer on the green light for half
  a second, the shell shows the menu under it; click at once and no menu appears; Option-click and plain click still act on
  release; Left / Top Right / Fill / Center / Return to Previous Size move the window; Enter Full Screen works and a full-screen
  window's menu (from a client-drawn frame) offers only Exit Full Screen; minimise a framed window and the bar and shadow fade
  with it; open one and its bar fades in with the content.

## wave-2026-10-15/fork

Written without a build or a run (wave rules); the orchestrator pushes the fork, moves the rev and runs wlcs. Fork branch
`lane/wlcs-w3-w4-w7` from 9c8f762a, one commit per group.

| Group | Fork change | wlcs tests that should now pass |
|---|---|---|
| W3 | `handlers/wm_base.rs`, `GetXdgSurface`: `xdg_wm_base.role` when `compositor::get_role` is some; `invalid_surface_state` when the surface has a buffer in its pending or current `SurfaceAttributes` or in the renderer's `RendererSurfaceStateUserData` (the renderer takes the committed buffer out of the attributes). | `creating_xdg_surface_from_wl_surface_with_{existing_role,attached_buffer,committed_buffer}_is_an_error` |
| W4 | `handlers/surface/toplevel.rs` `send_creation_capabilities`, called from `get_toplevel`: the capabilities event only, version >= 5. `XdgToplevelSurfaceRoleAttributes::capabilities_sent` makes the first configure repeat it only if `new_toplevel` changed the set. | `XdgToplevelStableTest.wm_capabilities_are_sent` |
| W7 | `compositor/placement.rs` (`Placements`, `SubsurfacePlacements`): `set_position` goes to the parent's pending state; `commit` takes it; `merge_into` writes the child's current `SubsurfaceCachedState::location`. `SubsurfaceCachedState` no longer carries a position through the child's own commit. `PrivateSurfaceData::commit` takes only the transaction a sync child holds; `commit_sync_surface_tree` is removed. Unit tests in `placement.rs`. | `subsurface_does_not_move_when_parent_not_committed`, `desync_subsurface_moves_when_only_parent_committed`, `subsurface_with_sync_parent_does_not_move_when_only_grandparent_committed`, `subsurface_does_not_move_when_grandparent_commit_is_before_sync_parent_commit` |

Kept in `dev/wlcs/expected-failures.txt`, with reasons there (9 rows left):

- `attaching_buffer_to_unconfigured_xdg_surface_is_an_error`: wlcs attaches and roundtrips with no commit and expects `unconfigured_buffer` at the attach. The XML puts it at the first configure/commit and smithay checks it in the commit; `wl_surface.attach` is smithay's compositor module and has no xdg knowledge. No patch.
- `destroying_toplevel_before_decoration_throws_orphaned`: the decoration XML defines `orphaned` in `zxdg_toplevel_decoration_v1`'s enum; wlcs expects it on the manager. Smithay already raises it on the decoration. No patch.
- `place_above_simple`, `place_below_simple`: wlcs-side (a focus no compositor can give).

Departures and edges:

- The text-input order: no configure is sent at creation. The capabilities event precedes the first configure, as the XML requires ("once before the first xdg_surface.configure").
- A `set_position` of a subsurface whose role object was destroyed (parent unset) is dropped; one that was already queued in the old parent may still write that surface's location when the old parent commits (harmless, the next parent resets the position at its own commit).
- Locking: `Placed::place` takes the child's surface lock while the parent's is held (inside `Transaction::apply`); that is the parent-then-child order the rest of the tree code uses.
- A fresh get_xdg_surface on a wl_surface whose earlier xdg_surface was destroyed (role already xdg_toplevel/popup) is now `xdg_wm_base.role`, as the XML says; a client that reuses a wl_surface this way would be disconnected.

To check in a real run: wlcs counts (expect 8 more passes, 9 listed); the casement-testkit protocol tests (`xdg_shell` goldens filter `wm_capabilities`; `subcompositor-tree.txt` has no positions); the fuzz box (`get_xdg_surface` on a surface with a role or buffer now disconnects the client; the fuzz oracle should treat it as a clean protocol error); a real client with subsurfaces (a video player, a browser) still moves its subsurfaces after the parent's commit.

### capture2 gate fixes (orchestrator)

- **The test client's second frame.** `desktop-client`'s capture reducer folds one frame per capture and forgets
  the session when its frame is taken, so a second `request` on the same session found "no session is open".
  `OutputCapture` now keeps the session's last complete constraints and starts the next frame from them.
- **`not_permitted` on the manager.** An agent connection's refused request on a desktop app, window or Space object
  posted the manager's error code on the child object, where it is undocumented (fuzz seed 3, reached once the
  catalogue gained the capture roots). The children's data now carries their manager and the error goes there, as the
  XML says.
- **Negative `wl_surface.damage`.** smithay built a negative `Size` and panicked inside dispatch (fuzz seed 4). Fork
  commits `65518b3a` and `90efde43` ignore it, as `damage_buffer` already did, and a negative `xdg_positioner.set_parent_size` (seed 4 again).

## wave-2026-10-15/winwire

The window menu's wire, end to end (casement-protocols `lane/winwire`, casement, sill). Written without a compiler.

- **Protocol.** `quire_shell_v1` (version 1, changed in place as the draft allows; no pinned bytes touched): enum
  `window_entry` (15, codes = `MenuEntry::code`), event `window_menu(toplevel, x, y, enabled)`, request
  `window_menu_pick(toplevel, entry)`. casement-protocols also has vocab `WindowEntry` and the desktop-client
  `ShellEvent::WindowMenu` / `DesktopCommand::PickWindowMenu`. Land casement-protocols first.
- **apply_note.** `ShellNote::WindowMenu` sends `window_menu(identifier_of(window), at.x, at.y, show.enabled())` to
  every bound shell object. **Dispatch.** `window_menu_pick` resolves `window_of_identifier` (and the model knowing the
  window) and `MenuEntry::from_code`; `on_window_menu_pick` feeds `ShellRequest::WindowMenuPick`. An unknown window or
  entry is ignored, no protocol error (the menu may be on screen while the window goes away).
- **Test client.** `ShellHandle::window_menu_pick`, `ShellHandle::window_menus()` (`WindowMenuSeen`).
- **Tests.** `protocol/quire_shell.rs`: `resting_on_the_green_light_shows_the_menu_and_a_pick_tiles_the_window`,
  `a_pick_for_nothing_is_ignored`; a doc test on `on_window_menu_pick`. No golden or `globals.txt` changed (no version bump).
- **Check at the gate.** The light test computes the zoom light from `rect_of(window).origin` as the content origin and
  `frame_style(..).around(content)`; if the layout rectangle includes the titlebar, the expected point is off by the bar
  height and the test fails at the first `window_menus()` assert. `Fixture::advance` must fire `LightDwell` (the
  timer book). Multiple bound shell objects each get the event.
- **Real run.** Rest the pointer half a second on the green light: sill's menu appears under it; a pick moves the window;
  clicking away sends nothing.
## wave-2026-10-15/chrome3

Title text, quire's numbers, `theme = "system"`. Written without a compiler (wave rule); the orchestrator builds.

- **casement-chrome (new crate, a workspace member).** `TitleRaster::system(Margins)` reads quire's `title_font(Typeface::System)`
  (Inter 13 px, weight 600), shapes with parley (`FontContext` made once, kept; fontique scans the system fonts on the first
  title, never per frame), rasterises with vello_cpu to premultiplied RGBA8 as big as the titlebar at the output's native scale
  (`device`: half-up, the rounding of `buffer_size`). The family is `Inter, system-ui, sans-serif`, so a missing Inter falls to any
  sans (no bundled face; the spike bundled ds-style's `FACES`, casement does not). Ink: `ColourToken::Ink` for the key window,
  `InkSoft` otherwise, in the frame's scheme (`ink(scheme, activity)`). Layout (`place`): the title is centred in the bar, pushed
  off the lights (left margin 73 = inset 13 + three 12 px lights + two 8 px gaps + one gap) and off a 13 px right margin, and
  when even that room is too little it is cut with an ellipsis (binary search over the characters, `longest_fit`). Not the CSS's
  84 px each side (`TitleInset`): that cut a title on a narrow window that has room beside the lights. Titles are tidied
  (`tidy`: control characters become spaces, 256 characters) because a client's title is untrusted. Cache: `TitleKey`
  (title, logical size, scale, scheme, activity), 32 entries, least recently used out; each raster has a unique `id`.
- **Wiring.** `casement-render` takes the pixels as data: `TitleDraw { id, width, height, rgba }` in `LayerDraw::titles`,
  `frame_elements` gets `Option<&TitleDraw>` and puts a memory element over the titlebar's (same size and place, so one pixel to
  one pixel); a title not the titlebar's size is not drawn. The draw cache key gained a trailing `u64` stamp (the title's id; 0
  for the titlebar and the shadow). `casement::core_loop::titles::of` makes them per frame from the model's title, the emphasis
  and `RenderState::scheme`; `RenderState::titles` is `Option<TitleRaster>`, made on the first server-side frame so a session
  without one never scans fonts. A raster is made only when the key changes, so a window being resized makes one per frame as
  its titlebar already does. The titlebar's own raster still has no text, so a golden of a framed window changes (title only).
- **Departures.** (1) `check-boundary.sh` B15 exempted nothing, yet `casement` must depend on the crate: the graph rule now skips
  `casement-chrome`, `casement` (the consumer) and `casement-testkit` (reaches it through `casement`), and a new source rule
  says only `crates/casement-chrome/` names `parley::`, `fontique::` or `vello_cpu::`. B11 is unchanged (its pattern is `vello`,
  not `vello_cpu`). (2) `ShadowLayer` (casement-core) gained `dx` because quire's layers have one; the shadow raster honours it.
  (3) `casement_layout::pressed` takes the scheme (the pressed hue is `light_hue`'s for it).
- **quire's numbers.** `WINDOW_RADIUS`, the two hand-written `ShadowLayer`s and `HUES` are deleted; `frame_style` reads
  `Radius::Window.px()`, `Shadow::Window.layers(scheme)` and `light_hue(light, scheme)`. Tests in `casement-layout/src/frame.rs`
  pin the agreement for both schemes (radius, every shadow field and colour, every disc and mark ink, pressed). The light hues
  change from the macOS candy values casement copied (`#FF5F57`...) to quire's (`#e8483c`...; dark `#ff6b60`...): that is
  quire's token, the point of the lane. FrameLook holds two shadow layers; a test pins quire's count at two.
- **theme = system.** `casement_ipc::system_scheme::SchemeWatch` is the follower: a thread with a current-thread tokio runtime
  (the pattern of `sleep` and `signals`; the portal's zbus needs a runtime and casement has none), `SystemPrefsWatch::start` on
  it, the first answer and each change passed to a callback. `casement::appearance` starts it the first time the stored theme is
  `system` (at start, or when the file changes to it), and its answers join the file's over the same calloop channel
  (`Input::Theme`/`Input::System`); `resolve(theme, system)` picks the scheme. Until it answers, and with no portal, `system`
  paints Light. The seam is `ConfigHome`: `Xdg` reads the portal, `At(dir)` a fixed default answer, so no test reaches a bus
  (`SystemPrefsSource::Fixed`). `casement-ipc` has no ds-style dependency, so its callback takes its own `DesktopScheme`
  (it compares the portal's scheme with `scheme_from_portal(1)` rather than naming `Scheme`); if a ds-style line is added to its
  manifest, `DesktopScheme` can become `Scheme`. A custom `--config-home` therefore does not follow the desktop (a jail).
- **Tests.** Table: `place` (device rounding, room, left edge, longest fit, margins), `key` (tidy, device size, key fields),
  `ink` (token to ink, per scheme and activity), `cache` (LRU, bound), `TitleDraw::argb`, `appearance::{resolve, Heard,
  system_source}`, `titles::margins`, `DesktopScheme::of_portal`, the quire agreement tests. Needing fonts, but asserting only
  invariants: `raster.rs` (size per scale, premultiplied, errors, kept raster is the same, bounded, ink stays between the margins).
  Doc tests: `ink`, `TitleKey::new`, `tidy`, `Margins::in_device`, `device`, `room`, `left_edge`, `longest_fit`,
  `TitleRaster::system`, `TitleDraw::argb`, `pressed`, `appearance::{system_source, resolve}`, `SchemeWatch`, `DesktopScheme`.
  No golden was blessed.
- **Gate and real run.** Likeliest compile faults: `raster.rs` against parley 0.11.1 and the vello_cpu fork (copied from
  `spikes/casement-sp8/chrome`: `ranged_builder(.., scale, true)`, `FontFamily::Source(String.into())`, `Pixmap` into `render`,
  `Level::try_detect`), `ColourToken::Ink.value(scheme)` shape, the non-exhaustive `TitleFont` held by value, `Cargo.lock` (a new
  package, no `--locked`), `core_loop/titles.rs` field-wise borrow of `state.render` beside `state.core.model`, `FrameKey` now
  seven wide, `prefs.scheme = ...` assignment on a `non_exhaustive` struct in casement-ipc's test, `Light::ALL.map`. Real run on
  foot: the title sits centred in the bar in Inter 13/600 (or the system sans), dim on a window that is not key, ellipsis on
  a narrow window with the start of the title left of the right margin and clear of the lights; sharp at 1x, 1.5x and 2x; changes
  with the window's title with no flash; with `theme = "system"` in `~/.config/quire/appearance.toml`, the frame follows the
  desktop's light/dark toggle without a restart (KDE/GNOME portal; check `busctl --user` shows no second connection per frame);
  with `theme = "dark"` it does not. Open: right-to-left titles (parley shapes them; the margins are not mirrored), and
  the title is not in a window capture of a window without its frame (as the frame is not).

## wave-2026-10-15/mini

Minimising looks like macOS: the Scale effect, the Genie effect, chosen by `windows.minimise_effect` (Genie is the default, as
on a Mac). Written without a compiler (wave rule); the orchestrator builds.

- **Motion knows the flight.** `casement_motion::Flight { window, tile, progress }` is a new field on `PlacedWindow`
  (`flight: Option<Flight>`). `retarget` stores the two ends on the window's legs for a `Cause::Minimise` that travels (the
  window's rectangle when it left, or the goal rectangle when it comes back, and the dock tile); `sample` reports a flight
  only while the window still moves and `progress` (thousandths, 1000 = in the tile) is read off the pose against the two
  ends, on the channel (x, y, w, h) where they differ most. That one number serves both directions (minimise 0 to 1000,
  restore 1000 to 0) and a reversal mid-flight, with no second state. Same Move spring and 380 ms response as before: the
  duration and curve are unchanged and the genie follows the same progress. Reduced motion has no flight (it snaps).
  `FrameScene::text` adds ` flight <permille>` to a window's line only while it flies, so no existing golden changes (the
  two motion goldens are Mission Control entries).
- **Genie geometry, pure.** `casement_motion::genie::strips(window, tile, progress, count) -> Vec<Strip>` (`STRIPS = 48`).
  Row `v` (0 top, 1 bottom) has gone `ease(clamp(1.6 p - 0.6 (1 - v)))` of the way to the tile, so the bottom leads by `LAG =
  0.6` of the flight; centre, width and top edge are linear in that. Strips share their edges by construction, a running
  maximum keeps rows from folding when the tile is above the window. Table tests: ends at 0 and 1, shared edges at 21
  progress values, the bottleneck narrowing, a hand-computed halfway row, a tile above the window.
- **Scale effect: no wrapper needed.** Smithay's `render_elements_from_surface_tree` takes its scale per axis, so
  `window_surfaces` draws the whole tree at `view.scale * (placed.size / real content size)` with the origin stretched
  too (the geometry offset scales). Damage, opaque regions, the source rectangle and the rounded clip all follow from that
  with no new element, which is less code and less risk than wrapping each surface in `RescaleRenderElement`. Departure from
  the brief's "a render element wrapper": the strips below do use smithay's `RescaleRenderElement` + `CropRenderElement`
  (+ `RelocateRenderElement`). `drawn_content` (`element/flight.rs`) gives the frame's content rectangle at the sampled size
  and `scene.rs` and `core_loop/titles.rs` use it, so the titlebar, its title raster, the shadow and the clip are made for the
  shrunk window.
- **Scaling only during a flight.** The brief says draw at `placed.size` instead of `placed.committed`. I scale only while
  `placed.flight` is set. A resize in flight (`size != committed`) still shows the old buffer until the client commits, as
  before: stretching there would stretch a client that cannot reach the layout size (a minimum size, a stuck client) for
  good. The ratio is against the real content size (window geometry, else committed), not `committed`, so a client with a
  shadow margin is not mis-scaled. If the owner wants the stretch everywhere, `flight_ratio` is the one place.
- **Genie: strips of smithay's wrappers, no shader.** `genie_elements` draws the window as it stands at rest (`at_rest`:
  the flight's window end), shares each surface element and the frame's memory elements (`SharedElement`, an `Rc`, because
  smithay's elements are not `Clone`), and for each strip makes `StripElement` = `Relocate(Crop(Rescale(element)))` with
  its own id (`Id::namespaced(64 + strip)`). `strip_plans` turns `genie::strips` into the physical numbers (origin, per-axis
  scale, crop band, shift) and is pure with table tests. Two new `CasementElement` variants: `SurfaceStrip`, `MemoryStrip`.
  Departures: during a genie the rounded clip is not applied (the strips draw plain surfaces), the shadow is not drawn
  (the strips are cut to the frame's outline), popups are not drawn, blur v0 is not drawn. Each of these is also dropped
  for a Scale flight except the clip and the shadow, which scale with the window (popups and blur are left out there too).
- **Seams.** Every strip but the last runs one physical pixel into the next so rounding leaves no gap; where the window is
  translucent that row blends twice. The strip's width is the mean of its two rows (a trapezoid drawn as a rectangle), so
  the silhouette is stepped by up to `(window width - tile width) / 48 * slope` pixels per strip.
- **Pointer.** A window with `flight` set is left out of `core_loop::scene::stack` and `frames::contents`: it takes no
  pointer input while it flies, and the hit test elsewhere keeps the real geometry.
- **Settings.** `MinimiseEffect` derives `Default` (Genie) so `LayerDraw` stays `Default`; `LayerDraw.minimise_effect` is set
  in `core_loop::frame` from `model.settings().windows.minimise_effect`.
- **Tests added.** `casement-motion`: `genie.rs` (6 table/property tests + a doc test), `flight.rs` (2),
  `tests/it/behaviour.rs::a_minimise_flies_as_a_flight_out_and_back_and_other_causes_do_not`, `Flight::new` doc test.
  `casement-render`: `element/flight.rs` (6: the effect choice, the size ratio, scaled content, strip plans at rest, no
  gap at 21 progress values at three scales, an empty window, rest place; plus a `scale_ratio` doc test) and `element/strip.rs`
  (3, with a solid element: where a strip lands, an unreached band, distinct ids). No golden changed.
- **Gate and real run.** Likeliest compile faults: `element/strip.rs` and `shared.rs` (the generic `Element` and
  `RenderElement` impls over smithay's wrappers, and the two new `CasementElement` variants through `render_elements!`),
  `flight.rs`'s `Point`/`Rectangle` conversions with `.into()`, `window.rs` (`Scale::from((f64, f64))`,
  `scaled_origin`), `retarget.rs` (`toward` borrows `known` then consumes it), `scene.rs` (the genie branch with
  `continue` in the closure). Only a real run proves: frame time on a large window with 48 strips x (titlebar, title, surface
  tree) elements through the damage tracker (a 4K window is the case to measure; lower `STRIPS` if it drops frames), seams
  between strips at fractional scales, the genie's look (lag 0.6 and the smooth step are first guesses), a Scale flight of
  a window with subsurfaces and a client-drawn frame, and that restore plays the reverse.
## wave-2026-10-15/split

Split View end to end in the model and layout (the window menu's Full Screen Tile entries are on). Written without a
compiler (wave rule); the orchestrator builds.

- **The command.** `Model::split_view(window, side)` (`model/split.rs`), reached by `ShellRequest::WindowMenuPick` with
  `TileLeftOfScreen` / `TileRightOfScreen` (`window_menu.rs` now passes `SplitSupport::Available`). The window goes full
  screen on a Space of its own (`enter_full_screen`, now `pub(super)`), exactly as Enter Full Screen does. No key binding
  yet: `DesktopCommand` is serialised in the bindings document, and `SplitSide` has no serde form; add `SplitKey(SplitSide)`
  with the binding names when the owner picks chords.
- **Choosing the other half.** When other windows can fill it, the model holds a `SplitChoice { first, side, space }`
  (`Model::split_choice`) and emits `ShellNote::SplitChoose(SplitChoices { first, side, candidates })`. Candidates: ordinary
  (not `WindowRole::Shell`, no parent) windows of apps that are not hidden, not in the dock, not full screen, on a desktop
  Space of the first window's output, most recently stacked first. `ShellRequest::SplitPick { window }` completes the pair
  (the window must still be a candidate; the first must still be alone in its Space); `ShellRequest::SplitCancel` (Escape or a
  click elsewhere, which only the shell sees) leaves the first window alone, full screen. The choosing ends, with
  `ShellNote::SplitChoiceEnd(first)`, on a pick, a cancel, the first window leaving full screen or closing. With no other
  window the first just goes full screen and nothing is asked.
- **The pair.** The Space becomes `SpaceContent::SplitView(SplitPair { leading, trailing, divider: 500 })`; both windows are
  `WindowState::FullScreen { space, restore }` with the same Space, so the whole full-screen machinery (the green light's
  Exit Full Screen, the toplevel states, drawing above the layers) works for both unchanged. The `full_screen_spaces_agree`
  invariant now also requires both windows of a Split View to hold the Space (`FullScreenOutOfStep`).
- **Leaving.** `leave_full_screen` restores every window full screen in that Space and retires it (the desktop to its left
  takes them), so Exit Full Screen on either window ends the Split View with both back at their previous states. Closing one
  window (`unmap`) turns the Space back into `FullScreen(other)` (`split_survivor`); before this lane an unmapped Split View
  window turned the Space into an empty Desktop (the old `_ =>` arm, never reached).
- **Layout.** `rust_scene/split.rs`: the halves share the output's whole rectangle (`OutputSpec::full`) with an 8 px divider
  strip; the leading width is the divider's share of the rest, held so each half keeps its window's minimum width; when the
  minimums cannot both fit, the share stands and `LayoutReport::MinUnmet` is reported for the window(s) that fall short. The
  row is solved by `taffy_bridge::split_rects` (`new_tree`, `solve`, `rect_of`, `fixed_size`: the bridge now has its caller, and
  the `expect(dead_code, unused_imports)` on the module is gone); `by_arithmetic` is the fallback if Taffy refuses the tree.
  Both halves are always sent a `Resize` (as a full-screen window is).
- **Divider drag: model and layout only.** `ShellRequest::SplitDivide { window, share }` sets the share (clamped to 200..800
  thousandths) and relays out. Not built: the hit test on the divider strip and the pointer grab in `core_loop`, the resize
  cursor, and a divider rectangle in `Arrangement` (it is the 8 px gap between the two targets). The shell can draw a handle
  in the gap and send `split_divide` as it drags; until the protocol lands the divider is fixed at 50/50.
- **Departure: whole output, not the usable area.** The task said "usable area"; both windows are `FullScreen` and are drawn
  above the top layers (`full_screen_windows`), so a halves-in-usable-area layout would leave the bar's strip uncovered. Layout
  uses `OutputSpec::full`. Changing it is one line (`let area = output.full;` in `rust_scene/split.rs`).
- **Departure: the picker has no wire.** The XML has no event or request for it and `casement-protocols` is not this lane's.
  `casement_wayland::quire::shell::apply_note` has an explicit empty arm for `SplitChoose` and `SplitChoiceEnd` naming this
  section. Until the protocol lands the menu entry makes the window full screen and the model waits for a pick that cannot
  come (it ends when the window leaves full screen or closes). No test client catalogue entry (no new interface).
- **Departures.** A window's maximum size is not honoured by a half (it is as wide as the half, like a full-screen window);
  windows on other desktops of the output are candidates, not only the desktop the first came from (the model does not record
  which desktop that was); switching Space while choosing does not cancel it; the divider share is not persisted.
- **Tests.** `casement-core/tests/it/model_split.rs` (enter, candidates, pick both sides, cancel, stale picks, first leaving,
  exit from either window, close either window, divider clamp); `casement-layout/tests/it/split.rs` (widths table, unmet
  minimums, the output's place, a missing window); in-file test of `taffy_bridge/split.rs`. Changed: `model_window_menu.rs`
  ("a Full Screen Tile entry ..." now ends full screen), `casement-testkit/tests/it/protocol/quire_shell.rs`
  (`resting_on_the_green_light...` expects both Full Screen Tile entries enabled).
- **Gate.** Likeliest compile faults: `model/split.rs` (the `restore.filter(..)` let-else in `split_pick`, `Moved::then(Model::choice_over)`,
  `with_all(Option<Effect>)`), `invariants.rs` (the tuple match), `requests.rs` `leave_full_screen` closures, `desktop.rs`
  (`if let SpaceArrangement::Split(pair) = space.kind` copies the `Copy` pair out of a borrowed place), `taffy_bridge/split.rs`
  (`Style { flex_shrink, flex_grow }`, `tree.layout(node)?`). The proptest model run (`model_props.rs`) never issues a Split
  command, so it is unchanged; add the menu picks to its strategies if wanted. Real run: Left of Screen with two windows open
  fills the left half and (once the wire lands) the shell offers the second; today Left of Screen only goes full screen.
## wave-2026-10-15/splitwire

The Split View picker and divider on the wire (casement-protocols `lane/splitwire`, casement, sill). Written without a compiler.

- **Protocol.** `quire_shell_v1` (still version 1): enum `split_side`, events `split_choose(first, side)` +
  one `split_candidate(toplevel)` per candidate + `split_choice_end(first)`, requests `split_pick`, `split_cancel`,
  `split_divide(toplevel, share)`. Land casement-protocols first. This closes the "picker has no wire" departure of
  `## wave-2026-10-15/split`.
- **Dispatch.** `casement_wayland::quire::shell_split` (new file; `shell.rs` was at 400 lines): `send_choose`,
  `send_choice_end`, `on_split_pick`, `on_split_cancel`, `on_split_divide`. A pick/divide naming an unknown window is
  ignored, no protocol error. `share` above 1000 is cut to 1000; the model clamps to 200..800.
- **Test client.** `ShellHandle::{split_pick, split_cancel, split_divide, split_choices, split_ends}`
  (`SplitChoiceSeen`). No new interface, so no catalogue change.
- **Tests.** `protocol/quire_shell.rs`: `left_of_screen_offers_the_other_window_and_a_pick_makes_both_halves`,
  `a_cancelled_choice_leaves_the_first_window_alone`; doc tests on the three `on_split_*`. No golden or `globals.txt` changed.
- **Check at the gate.** The tests assume both toplevels (no app id) are candidates of each other and that
  `windows()` is in id order; the second window must not be hidden or in the dock.
- **Divider drag: still open (model side).** The wire carries `split_divide`, but casement has no hit test on the 8 px
  strip and no pointer grab, and no divider rectangle in `Arrangement`. sill draws the handle only if a shell surface
  can receive the drag; see sill's findings. A real drag needs: (1) the gap between the two halves routed to a shell
  layer surface or a compositor grab, (2) a resize cursor, (3) persistence of the share if wanted.

## wave-2026-10-15/split2

Split View finished the Mac way: the divider drag, maximum widths, candidates from one desktop, cancel on a Space switch and a
key command. Written without a compiler (wave rule); the orchestrator builds.

- **Divider drag, compositor-side (sill draws no handle).** `Arrangement` gains `dividers: Vec<DividerTarget>` (the 8 px strip,
  the row it divides, both windows and their minimum widths; pure `hit(point)` and `share_at(x)`) and `backdrops`. In the loop,
  `core_loop/divider.rs` holds `RenderState::divider: DividerPointer { Away, Over, Dragging }`. Motion (`deliver::motion`, right
  after the window-grab follow) asks `divider::moved`: on a strip of the Space the output shows, the cursor is `ResizeEw` and the
  pointer moves with no focus (clients are left); a press (`press_edge`, before the frame pick) swallows the press and starts the
  drag with the reach table's say (`InputUse::Press` on the leading window); each motion feeds
  `ShellRequest::SplitDivide { window: leading, share: share_at(x) }`; the release of the starting button ends it
  (`ends_with`, so `pressed` refocuses afterwards). The share is `SplitPair::divider`, so it lasts as long as the Space (no
  persistence across sessions). `share_at` keeps 200..800 (`SplitPair::NEAREST`) and, when both minimum widths fit inside that,
  each window's minimum; the model clamps again to 200..800 and layout still holds minimums.
- **Departures.** (1) Any pointer button starts a drag (there is no left-button constant in the loop; frames do the same).
  (2) No extra throttle: like an interactive resize, each motion is one `feed`, which makes one relayout and one configure per
  window; the model ignores an unchanged share, so a still pointer sends nothing. (3) A divider yields to the lock, an overlay
  layer and any window surface in front of it (a popup over the strip); layers under the full-screen windows do not. (4) The
  divider changes the Space with `Cause::Configure` (snap), not a spring, so the halves follow the pointer exactly.
- **Backdrop.** Every Split View Space gets a `BackdropTarget`; the loop sets `LayerDraw::backdrop` (dark grey, 28/28/30) for the
  output whose shown Space it is, and `output_elements` draws one opaque `LockCoverElement` (same element id as the lock cover,
  which never shares a frame with it) behind the full-screen windows and above the top layers. So the strip is a solid bar, not
  the wallpaper showing through, and a window centred in its half has something behind it. During a Space slide the backdrop
  appears and goes at once (it follows the shown Space, not the slide).
- **Maximum width.** `rust_scene/split.rs::within_max`: a half whose window has a bounded maximum smaller than the half (a zero
  side means none) centres that size in the half; the window is asked for it. Not done: the share does not shrink a half to a
  window's maximum (the Mac sizes the halves the same way: the pointer decides).
- **Candidates from the first window's desktop.** `SplitChoice::desktop` records the first window's `SpacePlace` before it goes
  full screen; `can_share` (now a free function) requires it. A first window on every desktop (`AllSpaces`) falls back to any
  desktop of its output, as before.
- **Switching Space cancels.** `Model::show` ends with `choice_unless_on(shown)`: another Space than the choice's own ends the
  choosing and sends `SplitChoiceEnd(first)`; the first window stays full screen.
- **Key command.** `DesktopCommand::SplitTile(SplitSide)`, `SplitSide` now `Hash + Serialize + Deserialize` (`left` / `right`),
  words `["desktop", "split_tile", "left"]` in the bindings document (`config/action.rs`, header comment updated), no default
  chord. It runs `split_view` on the key window. casementctl exposes no binding command list (its `DesktopCommand` is the
  desktop client's, a different type), so nothing changed there; the bindings document is the list.
- **Tests.** Layout table tests: `tests/it/divider.rs` (strip hit, share from pointer x with limits and minimums),
  `tests/it/split.rs` (divider strip per share, centred maximum widths and the backdrop). Model (`model_split.rs`): the key
  command, candidates from one desktop, cancel on a Space switch. `round_trip.rs` pins the serde form; `config.rs` round trips
  the new words; `model_props.rs` and `model_input.rs` include `SplitTile`. Testkit: `protocol/quire_shell_divider.rs` drives a
  real pointer through the fixture (press on the strip, drag, release, a press beside the strip). No golden changed.
- **Gate.** Likeliest compile faults: `core_loop/divider.rs` (borrow of `state.render.divider` before `feed(state, ..)` in
  `moved`; `bool::then` returning a clone), `deliver.rs` (`press_on_divider`, the early return in `motion`), `model/split.rs`
  (`can_share` as a free fn over private `Model` fields, `Option::is_none_or`), `spaces.rs` (`move` closure after `then`),
  `casement-render/src/scene.rs` (`cover(.., colour)`). The test that needs the most from reality is
  `dragging_the_strip_sets_the_share_...`: it assumes the fixture's two test clients' buffers do not reach the strip and that
  `pointer_to` then `button` hit the compositor path above (a test client that committed a larger buffer would block the strip).
  `desktop_filter` test assumes a window mapped while desktop 2 is shown lands on desktop 2.
- **Real run.** Tile Left of Screen with two windows, pick the second: the strip is a dark bar, the cursor over it is left-right,
  a drag moves the split live and both clients re-lay out; released, nothing else moves; the share stays until Exit Full Screen.
  A window with a maximum width sits centred in its half on dark grey. Switching Space while the picker is up closes it.
  Windows of another desktop are not offered. Look at the frame time during a drag on a large window.

## wave-2026-10-15/shadow

The window shadow stops re-rastering per frame. Written without a compiler (wave rule); the orchestrator builds.

- **What it is now.** `element/frame/shadow.rs` is a pure function `tiles(layers, radius, scale) -> Tiles`: four corner tiles
  and four edge strips, no window size in the inputs. `slices.rs` is the pure geometry: where the eight pieces go round an
  outline of any size and which part of each tile they show. `frame.rs::shadow_elements` draws the pieces (corners as they
  are, strips stretched along their axis). The cache key is `(scale numerator, radius, layers)` (`shadow_set::key`); the
  window's size and the hover/activity colours of the titlebar are not in it, so a resize (and a light hover) hits the cache
  every frame. `DrawCache` keeps the tiles per key (limit 8) and, per window, the uploaded buffers (limit 64): two windows
  share the raster but not a buffer, because two elements of one frame must not share an id.
- **Rounded, and sharp.** A corner tile is the exact coverage of the layer's rounded rectangle (dx/dy moved, spread grown, radius
  plus spread) blurred by a separable Gaussian (sigma half the blur radius, as before), so the blur follows the curve. Tiles are
  rastered at the output's scale (pixel count `round(logical * scale)`), not one pixel per logical pixel. An edge strip is
  the one-dimensional blurred edge (erf), one pixel across. The corner reaches `inner_reach` logical pixels into the window
  (radius + 3 sigma + offset + spread + 1, at most 160) so the strip takes over with no seam; a test compares the strip with
  the corner's inner column.
- **Departures.** (1) Eight pieces, not nine: the middle is not drawn (item 3, nothing under the opaque body); strips reach
  2 px under the edge for the anti-aliased edge and the clipped corners. The corner tiles do include the square of the window
  inside the curve (up to `inner`), which the opaque body covers; a translucent window would show that part (the old shadow
  was the same). (2) A window smaller than twice `inner` (about 120 to 150 logical pixels) crops its corners to half its size,
  so a tiny window's shadow is a little stronger than a true blur of a tiny shape; no strip is drawn where the corners meet.
  (3) The look is the old one for a window of any size where it is square-cornered: a test builds the old full raster (square
  shape, erf product) for four sizes and checks the pieces within 4/255 per channel (40 for the two windows that crop). The
  look changes where it should: near the corners the shadow now follows the radius, so it is lighter at the corner than the old
  square blur. No pixel golden pinned the shadow; none was blessed. (4) At a fractional scale a tile's pixel size is rounded, so
  a piece is stretched by under a pixel over its length; the src rectangle is taken by ratio so the seams stay within that.
- **Tests.** Table: tile sizes at scale 1 and 2; no window-sized buffer in the tiles; mirror symmetry without an offset; a falling
  shadow is stronger below; a strip meets the corner's inner column (no seam); rounded versus square corner; sharpness at
  scale 2; pieces versus the old raster; `slices` (no overlap, every src inside its tile, the area of the ring, corners mirror
  images, tiny windows, the dest changes with size but the src sizes do not). `DrawCache::shadow`: 100 sizes make the tiles
  once, two windows share the raster but not the buffers, a new scale is a new raster. The old `shadow` raster and its test
  are deleted (`raster.rs`).
- **Gate and real run.** Likeliest compile faults: `Corner::ALL.map` with a closure that mutates `made` (FnMut is allowed for
  `array::map`), `each_ref().map(to_buffer)` in `ShadowSet::new` (fn item over `&Raster`), the `pub(crate)` re-exports through
  `element/mod.rs`, a glob `use super::*` in `shadow/tests.rs` reaching `Px`, `over`, `bytes`. The first shadow at a scale costs a
  few tens of milliseconds of CPU (two layers, four corners, or two when dx is 0); only a real run shows if that hitches the first
  window on a 2x output. Real run: resize a large window (say 1600x1000) by dragging an edge on a 2x output and look at the
  frame time (no per-frame raster, only the elements' rectangles change); the shadow is sharp at 2x, follows the rounded
  bottom corners, and has no seam where a strip meets a corner (look at light and dark); the dim shadow under a fade (`opacity`)
  fades with the window; a very small window (a 100x60 dialog) has a plausible shadow.

## wave-2026-10-15/overview

Mission Control, Sonoma style: the shown Space's windows fly to scaled slots below a strip for the Spaces bar, a click picks
one, Escape or the same key leaves. Written without a compiler (the lane rules); the gate is the first build.

- **Spread (`casement-layout/src/rust_scene/spread.rs`, `spread_fit.rs`, `overview.rs`).** A21's recipe in whole numbers (no
  float): one common scale in parts per million; the start is the largest scale whose cells (box plus gap) fill at most 55 % of the
  area and fit it, replaced by 1 when within a fifth of it; per trial each axis's centres are mapped monotonically onto the area,
  then a greedy pairwise push in canonical order (position, then id), at most 24 passes, each pair separated along its smaller
  overlap; a failed trial scales by 85 %, never below `spread_floor`; at the floor a shelf pack is tried, then the cells are
  clamped inside and `LayoutReport::SpreadOverflow` is reported. A cell is a window's box grown by half the gap on every side, so
  non-overlap means the whole gap. The box is the scaled content plus the frame's `reserve_top` at its own (unscaled) height,
  because the frame is drawn at full metrics round the scaled content. The strip is 112 px (a quarter of the area at most).
  Targets stay at their desktop rectangles, so the windows fly back when the mode returns to a desktop. `OverviewSlot` still has
  no `output` (A29): `Goals` finds the output in the target. Only Mission Control makes slots; App Expose and Show Desktop still
  arrange as a desktop. `LayoutReport::RowsCrowded` is unused. No `SpaceThumb`s: sill draws the Spaces bar.
- **`tests/it/spread.rs` un-ignored.** It was P3-Z's property (`overview.len() == windows`, inside the usable area, a gap of 20
  unless overflow reported, deterministic). `.proptest-regressions` kept. Table tests beside the code: `spread.rs` (empty, one
  window keeps its size below the strip, order kept, the frame's reserve, a crowd shrinks every window the same, 60 windows at the
  floor report overflow) and `spread_fit.rs` (overlap, push, hold inside, shelf, start order).
- **Model (`casement-core`).** `Overview { Closed, Entering, Open, Leaving }` (`overview.rs`), `Model::overview()`,
  `toggle_overview`, `open_overview`, `close_overview(Option<OverviewPick>)` (`model/overview.rs`). Entering and leaving each
  `Relayout(Cause::Overview)`, arm `TimerKey::OverviewSettle` (350 ms: the windows' spring settles, then open or closed) and tell
  the shell (`ShellNote::Overview`). A leave with a pick feeds `FocusIn::Press` on the window first, so it is raised and keyed as
  a press would. Leaving lays the windows out as a desktop again (the model reads `Desktop` for `Leaving` and `Closed`), so
  motion flies them back. A locked session ignores every way in. Not done: a Space switch, a lock or a new window does not close
  the overview (a new window gets a slot at once); the model does not close it when its last window goes.
- **The ways in.** `DesktopCommand::MissionControl` (a toggle; the config word `mission_control`; the chordkit standard action
  `MissionControl` is bound to it: Ctrl+Up on our desktop) and the shell's own `quire_shell_v1.overview(mode, toggle)` request
  (`ShellRequest::Overview`, `quire/shell_overview.rs`): Mission Control on or off, the other modes change nothing. The wire
  event `overview_state(mode, state, progress)` is sent (progress 0 entering and closed, 1000 open and leaving). The wire
  already had all of this; no protocol change. sill's `mission_control_chrome()` is still an empty function.
- **Input (`core_loop/overview.rs`, `deliver.rs`).** While the overview is not closed, the windows take no input: motion goes to
  the seat with no focus and sets the hovered window (the front-most slot with its frame above it), a press picks that window
  (or leaves from the picture) and its release is swallowed, scrolls, gestures and touches are dropped, Escape (no modifiers,
  entering or open) is turned into the toggle command. Keys are otherwise delivered to the key window. A press while leaving
  does nothing.
- **Drawing (`casement-render`).** `LayerDraw.overview: OverviewDraw` (`Off`, `On { hover }`). While on, a window not in a
  minimise flight is drawn as a `Scale` flight of zero progress, so the existing rescaling path does everything: surfaces at
  `placed.size` over the window geometry, the frame and its clip round `drawn_content`, no popups, no blur. `titles::of` makes the
  title for the scaled bar by the same rule. New: a dimming layer (premultiplied black, alpha 64, behind the windows and in front of
  the bottom and background layers, so the bar and dock stay bright), a 3 px ring round the hovered window's frame, and its title
  under it (`label_element`, a second `TitleRaster` with no margins, Dark scheme so the ink is light; 160 to 480 px wide, 22 px
  tall, built every frame while hovering).
- **Departures.** (1) The frame is not scaled with the window: lights and titlebar keep their full size round a small slot (the
  existing Scale-flight behaviour). macOS scales it. The box reserves that size. A fix is a scaled `FrameStyle`. (2) The ring colour
  is the Mac blue written as a constant in `element/overview.rs`, and the dim alpha too: no ds-style token is read. (3) The
  title under the window has no app icon. (4) The hover label's buffer is made per frame while hovering (no cache). (5) The
  motion goals, settle time and the spring are the `Overview` cause's `Move` response; 350 ms is a guess to confirm in a real
  run. (6) Windows of other Spaces, full-screen and Split View Spaces: the overview of such a Space has no slots (the
  windows just fly to nothing: they are not in `overview`, so they stay put), where macOS shows the Spaces bar only.
  (7) `casement-scene-css/tests/parity.rs` (P3-CSS) stays ignored: its bodies are `todo!` for the whole CSS scene (every layout
  of 500 specs), none of which this lane's code touches.
- **Tests.** casement-core `model_overview.rs` (state walks for the command and the shell request, relayout and timer effects, no
  output, a pick raises and keys, a leave without a pick, picks while closed or leaving); `model_props.rs` gets the command in its
  generator. casement-testkit `protocol/quire_shell_overview.rs` (shell opens it and hears Entering then Open with progress, a
  click on a slot picks, a click on the picture leaves with nothing picked, Escape leaves, the shell closes it and App Expose is
  ignored). `casement/src/core_loop/overview.rs` (the reach of a slot with its frame), `keymap/shell.rs` (Ctrl+Up),
  `tests/it/config.rs` (the word round-trips), `element/overview.rs` (physical rectangles at 1x and 2x).
  Ignored test un-ignored: `casement-layout/tests/it/spread.rs`. No golden changed.
- **Gate and real run.** Likeliest compile faults: `core_loop/deliver.rs` (the Escape check in the `apply_key` filter closure reads
  `state` as `&State` while the closure may hold `&mut`), `scene.rs` (`in_overview` and the let-chain in `draw_windows`, the
  closure passing `renderer` into `and_then`), `spread_fit.rs::start` (a closure taking a `fn` pointer), `model/overview.rs`
  (`Model { .. }.then_pick`), `shell_overview.rs` (`code_of` over the shell's enums), the testclient's `Toggle` re-export from
  the desktop protocol. Real run: Ctrl+Up opens it, the windows fly out in about a third of a second and keep their left/right
  and top/bottom order; the Spaces strip at the top stays empty (sill draws it); hover rings a window and shows its title; a
  click raises and keys it and the windows fly back; Escape and Ctrl+Up leave; the pointer is an arrow; typing in the key window
  still works; the bar and dock are not dimmed; 1.5x and 2x scales have no seam in the ring; with 20 windows the scale is about 0.3
  and at 50 the overflow report is logged.
## wave-2026-10-15/capture3

Nothing here compiled or ran (the lane does not build). Closes the four "Capture limits" of `capture` and `capture2`.

- **dma-buf capture (offer).** An ext output session, an ext window session, a cursor session and a `zwlr_screencopy`
  frame now offer dma-buf beside shm. The set is `casement_wayland::image_capture::dma_offer(renderable, accepted)` (pure,
  table-tested): the formats the backend's renderer can draw a capture into (`HeadlessRenderer::capture_formats`: `Argb8888`
  and `Xrgb8888` of the set `Bind<Dmabuf>::supported_formats` reports; pixman is linear only) among the formats the dma-buf
  global accepts (`DmabufPart::accepted`), so a client can always create the buffer it allocates. ext sends
  `dmabuf_device` (the dma-buf global's main device) and `dmabuf_format` per format with its modifiers; wlr sends
  `linux_dmabuf` per format (no modifiers on that wire) before `buffer_done`. smithay's `DmabufConstraints.node` is a
  `DrmNode`, made with `DrmNode::from_dev_id` (reads no file): in a jail without `--fabricate-gpus` the node cannot be typed
  and the ext offer is shm only (the tests allow both); wlr needs no node. No dma-buf global (`--dmabuf none`, the default
  in the jail) means no dma-buf offer at all.
- **dma-buf capture (drawing).** `casement_render::pass::write_dmabuf<R: Renderer + ImportMem + Bind<Dmabuf>>` uploads the
  frame as a texture, binds the client's dma-buf as the framebuffer, clears it, draws the texture 1:1 at the corner and waits
  for the fence. The composition root's `handlers/image_capture/dma.rs::deliver` writes shm directly and sends a dma-buf to
  `{HeadlessBackend, TtyBackend}::write_dmabuf`; all four ways to answer a frame (ext output, ext window, ext cursor, wlr)
  call it. Shm stays the fallback and the answer to an unknown buffer type is a failed frame.
  Departures: (1) **the renderer that draws is the backend's software one**, not a hardware GLES renderer. Headless: its own
  renderer (pixman, or GLES on llvmpipe, whose render-format set may be empty). Tty: the capture shadow's pixman, so the
  offer on a real seat is **linear only** (what OBS asks for when a GPU modifier is refused); a client buffer on another GPU is
  mapped by pixman through the dma-buf, which is correct for linear buffers only. Drawing into the hardware renderer needs
  its `GlesRenderer` handed out by `RealScanout` (not done: it lives behind the `tty` adapters and a frame in flight owns it).
  (2) The dummy renderer binds no dma-buf: it offers none and fails a dma-buf frame. (3) The winit backend offers none.
  (4) Needs a real run (below): the rows' orientation (the framebuffer is bound as smithay binds a scanout buffer) and the
  fence wait.
- **A client-drawn cursor (`CursorDraw::ClientSurface`).** The cursor session's frame no longer fails. `raw_image` takes the
  surface's buffer from its renderer state: shm is read straight from the pool (`ArgbImage::from_shm`; `Argb8888` kept,
  `Xrgb8888` made opaque; any other format fails the frame), a dma-buf through the renderer (`read_dmabuf`, import then
  `copy_texture`). The image is cut to the announced size (nearest neighbour, pixel centres: `ArgbImage::fitted`) and turned
  with the output. Departures: the surface's viewport crop and `buffer_transform` are not applied (a cursor with a cropped
  viewport or a rotated buffer is read as its whole buffer); a single-pixel or other non-shm, non-dma buffer fails the frame
  (`Renderer`). The one new `unsafe` is `gl_raw::with_shm_pool`, the read-only twin of the capture write's slice.
- **Rotated outputs.** The image is sent turned with the frame (`ArgbImage::turned`, `casement_core::geometry::turn_point`,
  the rule of smithay's `Transform::transform_point_in`, now one home with `turn_size`), the announced buffer size is
  `sent_image_size` (width and height swapped under a quarter turn), and the hotspot is turned inside the image so that
  `position - hotspot` is the turned image's corner. The frame's transform event stays `normal` (the image is already turned),
  as the capture frames' do. Pixel (x, y) moves with the index rule: `turn_point(t, (x, y), (w - 1, h - 1))`.
- **Fractional scales.** `place` now finds the image's corner as the element does: `round((at - hotspot - origin) x scale)` once
  (`on_output` in `casement-render/src/scene/convert.rs`, the rule of `to_physical`), the hotspot offset is `round(hotspot x
  scale)` and the position is their sum. Before, position and hotspot were rounded separately, which put the corner a pixel
  off whenever both fractions round the same way (the arrow at (100, 50) and 1.5x was at (142, 70); the screen draws it at
  (143, 71)). Changed values: the 1.5x protocol test now expects `Position(151, 76)`; the hotspot is still `(8, 5)`.
- **Tests.** Table: `turn_point`/`turn_size` (casement-core); `place` (1x, 1.5x, 2x, the corner equals the element's for scales
  130 to 270 and three origins, every transform's position and turned hotspot, the turned corner equals the turned box, scaled
  plus turned, the sent size); `dma_offer`; `ArgbImage` (strided unpack, xrgb opaque, short pool, nearest resampling up and
  down, an empty source, all eight turns, rgba order). Protocol (`capture_dma.rs`, `capture_cursor.rs`): an ext session's
  dma-buf formats and device (with `--dmabuf soft`; none without, none when the jail typed no node), a wlr frame's
  `linux_dmabuf` (ARGB8888 and XRGB8888 at the mode size, none without a global), a client-drawn cursor is a 16 x 16 frame of
  its own pixels with its hotspot and position, a turned output's events and image for all eight transforms (the image equals
  the straight one moved by the index rule), and at 1.5x every opaque pixel of the cursor image is in the painted frame at
  `position - hotspot`. The test clients got `OutputCapture::{dmabuf_formats, dmabuf_device}` and `ScreenCopy::dmabuf_offers`.
  **Not covered**: a frame actually drawn into a client dma-buf (the side-queue clients have no dma-buf import; a test needs a
  udmabuf-backed `wl_buffer` on the capture's queue), and reading a client dma-buf cursor.
- **Gate and real run.** Likeliest compile faults: `Bind::<Dmabuf>::supported_formats(&**renderer)` on the boxed renderers;
  `write_dmabuf`'s `Frame`/`Texture` trait imports and the `render` borrow of the framebuffer; the `DmabufConstraints` and
  `DrmNode` paths in `handlers/image_capture/dma.rs`; closures-to-`fn` in the `TURNS` const of `capture_cursor.rs`; the doc tests
  of `write_dmabuf`/`read_dmabuf` (`no_run`, GLES renderer types). Real run: OBS through the portal on the tty session
  negotiates dma-buf (linear) and shows the desktop right way up and not green or black (if it is upside down, `flipped` on
  the bind in `write_dmabuf` is the lever); the cursor of a toolkit that draws its own (GTK/Qt apps set one) follows in a
  cursor capture; on a rotated output a remote-desktop viewer's cursor points the right way; at 1.5x the cursor in a
  screencast sits exactly where it does on screen.
## wave-2026-10-15/flight2

Minimise, restore and close without anything dropped (the follow-ups "Genie and Scale gaps" and "A closing window loses its frame
at once"). Written without a compiler (wave rule); the orchestrator builds.

- **Genie: clip.** The rounded clip is applied to the surface strips that reach the bottom corners (`ClipShape::touches`, the
  strip's `source` rows against the radius band, plus a pixel), as the new variant `ClipStrip` (`clip_strip.rs`) with a
  `Compositor::draw_clip_strip` hook (default: the plain strip, so pixman and the dummy renderer stay unclipped as before). The
  GLES draw reuses the clip shader (`rounded_clip/gles.rs::draw_masked`) but takes the mask's src/dst from the shared surface
  element as it stands at rest, not from the strip's own draw arguments. The coverage is therefore a function of the window's own
  pixels and the strip carries the cut along under any scale and move; no per-strip ellipse maths. Strips above the corner band
  stay plain `SurfaceStrip`s. A clipped strip offers no opaque regions.
- **Genie: shadow.** Cut into the same strips, not a nine-slice. Reason: the shadow already exists as eight memory pieces, the strip
  wrappers take them unchanged, and a strip follows the bent silhouette at every row; a nine-slice scaled over the bounding
  trapezoid would leave the halo straight while the window bends. Each shadow strip is the window strip's plan `widened` by the
  shadow margin sideways (the first strip also above, the last also below). Cost: up to (strips x 8) crop attempts per frame, most
  return `None` at once.
- **Genie: blur.** One `BlurV0Element` whose region is the blur rectangles of the window mapped through every strip
  (`StripPlan::mapped`, rows clipped to the strip's drawn band so no two rectangles share a pixel). A blur per strip would be one
  framebuffer capture each. `BlurV0Element` now has an alpha (`with_alpha`; the stand-in, the GLES draw, the commit counter and the
  opaque regions follow), set to the window's opacity.
- **Scale: blur.** Stretched by the flight's ratio about the window's corner and faded with the window's opacity
  (`blur_v0::behind_scaled`). Only windows in a flight take the opacity; a window that is merely fading in or out keeps a full-strength
  blur as before (no golden moves). Open question for the owner: fade the blur for every fading window.
- **Popups.** Scale: they ride with the window (`popup_elements_riding`: the corner moves toward the window's corner by the ratio,
  the tree is drawn at the ratio, alpha is the window's). Genie: not drawn, from the first frame of the flight. Reason: a strip
  can only carry what lies inside the window's rows, and a menu hangs outside; macOS closes menus before a window minimises.
  The popup surface itself is not dismissed (the client is not told); it comes back when the window has landed. If the owner wants
  the client told, that is a `popup_done` on minimise (core change, not this lane).
- **Genie silhouette.** `casement_motion::genie::strips_fine`: the 48 base strips, each cut at equal heights into up to 6 pieces
  when the curve's edge moves more than `2 x TOLERANCE` (1 px) across it, with a total budget of `MAX_STRIPS` (120); when the wish
  passes the budget the extra cuts are shared in proportion. At rest it is exactly 48. A strip is still a rectangle (its edges are
  the mean of its rows' edges), so the stepped silhouette stays within about 1 px only where the budget allows; a 1600 px window
  squeezing to a 48 px tile is limited by the budget (a few px at the steepest rows). A true trapezoid would need a custom vertex
  or a per-row fragment remap: a new shader and element for every kind of strip (surface and memory), judged not worth it blind.
  The strip count never passes the window's physical height (no strip thinner than a row).
- **Genie: no double blend.** The 1 px overlap is gone. `strip_plans` rounds the edge between two strips once and both use it, so
  strips meet exactly. The price is up to one source row of repeat or skip at a seam (each strip is cropped at integer pixels of
  its own scaled copy), not seen unless a strip is stretched hard. Test: solid strips never overlap (`genie.rs`), and meet exactly
  at 21 progress values and three scales.
- **Closing window keeps its frame.** The loop keeps, for every framed window while it lives, its `FrameStyle`, title and activity
  (`RenderState::closing`, `core_loop/closing.rs`, refreshed every frame, forgotten once the window is neither in the model nor in
  the motion scene). When the model no longer has a window that motion still places, `LayerDraw::closing` carries the snapshot and
  `output_elements` draws its frame (`closing_elements`: titlebar, lights, title, shadow) round the sampled rectangle at the
  sampled opacity; the title is rastered again for the shrinking bar (`titles::of_closing`). Tests: `leaving` (which windows),
  `closing_content` (the sampled rectangle).
- **Not solved: the content of a closed window.** `core_loop/windows.rs::sync` drops the window from the surface book on
  `ToplevelUnmapped` (the client has detached or destroyed its surface; there is nothing left to draw). So today a closing window's
  CONTENT already vanishes at once, and with this change its frame fades alone: a titlebar and shadow with an empty body for
  about 380 ms. Whether that reads better than vanishing is a real-run call. To fade the body too the loop must keep a
  texture of the window (render it into an offscreen texture on the frame before the unmap), a separate piece of work. To turn the
  ghost frame off: build `LayerDraw::closing` empty in `core_loop.rs::frame`.
- **Tests added.** `casement-motion` `genie.rs`: at rest exactly the base strips, limit == base is the plain strips, steep part
  finer within the limit with shared edges, edges within tolerance with room (+ doc test). `casement-render`: `genie.rs` (6: at
  rest/shared edges, exact meeting at progress x scale, finer halfway, short window, source rows cover once, no area; halo; solid
  strips never overlap), `strip.rs` (widened band, mapped rectangle), `shape.rs` (`touches`), `popup.rs` (corner by ratio),
  `closing.rs` (+ doc test), doc tests on `strips_fine`, `with_alpha`, `faded`. `casement`: `closing.rs` (2). The old strip tests moved
  from `flight.rs` to `genie.rs`; the overlap assertion became "end equals next start". No golden, no `expected-failures.txt` change.
  No doc test on `StripPlan` methods (the struct is `non_exhaustive`, a doc test cannot build one).
- **Gate and real run.** Likeliest compile faults: `clip_strip.rs` (`Element`/`RenderElement` bounds on `ClipStripElement<R>`, `Mask`
  re-exported `pub(crate)` from a private module), `Compositor::draw_clip_strip`'s default (same shape as `draw_rounded_clip`'s),
  `genie.rs::shared` generic helper and the partial moves out of `GenieParts`, `blur.remapped` closure returning a `Vec`,
  `scene.rs` (`&&PlacedWindow` coerced to `&PlacedWindow` in `GenieWindow`/`closing_elements`), `core_loop/closing.rs::leaving`'s
  lifetimes (an `impl Fn + 'a` captured with the model), the `RenderState` field using a `pub(crate)` type. Real run: Genie with a
  framed window: rounded bottom corners during the flight, the shadow bends with it and fades, a blur-behind window keeps a blur
  that fades, a translucent window shows no light line at strip seams, no stepped edge on a 700-1000 px window at 1x and 2x; frame time
  with up to 120 strips x (titlebar, title, tree, shadow pieces) elements (lower `MAX_STRIPS` if it drops); Scale with a menu open:
  the menu shrinks and fades with the window; Genie with a menu open: the menu is gone from the first frame; close a framed
  window: the titlebar and shadow fade and shrink in step and the title stays centred.
## wave-2026-10-15/corners

Rounded corners and shadows right everywhere (wave 5). Written without a compiler (wave rule); the orchestrator builds.

- **Clip on every renderer.** New `element/rounded_clip/bands.rs`. A clipped draw that has no corner shader draws the window in
  pieces: the body (damage minus the two radius-sized corner squares) and the fully covered runs of the squares at full strength,
  then each partly covered pixel of the curve alone with the surface's alpha times its coverage. The pieces are `damage`
  rectangles in the destination's space and go through `Frame::render_texture_from_to` (or `draw_solid` for a single-pixel
  buffer), so the texture's own transform never enters. Coverage is `ClipShape::coverage` at the pixel centre, in 32 steps
  (`LEVELS`; error under 1/64), so a corner costs at most 31 extra draws whatever the radius. It is the default of
  `Compositor::draw_rounded_clip`, so Pixman, the dummy renderer and a multi-GPU copy renderer (none is wired in the tty path
  yet; a future `Compositor` impl gets it for free) clip. On GLES `gles::draw` falls back to it for a turned or flipped texture,
  a y-inverted texture and a single-pixel buffer.
- **Departures from the brief.** (1) No Pixman mask image per corner, and no clip transformed into texture space: alpha steps
  on 1-pixel damage rectangles need neither the pixman crate (no Cargo edit) nor the texture matrix, and work for any transform.
  Cost: up to 31 small draws per clipped surface per frame on those paths (the GLES shader path for an upright texture is
  unchanged: one draw). (2) The edge is quantised to 32 steps, not exact. (3) The dummy renderer draws nothing, so its only
  proof is that it compiles and calls the same pieces; the pieces are table-tested as a pure `plan`.
- **Window capture** (`pass::window_tree`) and the opaque regions are unchanged.
- **Shadow cut-out.** The tiles are transparent under the window's rounded shape (`shadow::punch`): factor
  `1 - cover(distance + 0.5 / per, per)`, with the distance from the pixel centre to the window's rounded outline (the same
  radius as the frame). The cut starts half a device pixel inside the edge, so the window's own anti-aliased edge pixel keeps
  part of its shadow beneath it, and is complete one pixel in. Edge strips are cut the same way (their 2 px under the edge go
  to zero from 1 px in). The corner squares inside the curve now show shadow where the window is not (outside the curve) and
  none where it is. A window drawn at opacity below 1 (fade) has no shadow beneath its body either, as the Mac.
- **Small windows.** A window with one side under twice the corner reach (`inner_reach`, about 120 to 150 logical px for
  quire's shadow) and neither side over 240 gets one raster of its own size plus the margin, the true separable Gaussian blur of
  its rounded shape (`shadow/whole.rs`, sharing `blurred` with the corner tiles), cut out under the window. It is cached by
  (scale, radius, layers, width, height) in `DrawCache::wholes` (16 sizes), and uploaded once per window and size
  (`whole_sets`). A window past the limit on a side (a long thin one) keeps the tile pieces and their cropped, slightly strong
  shadow. Departure: resizing a small window through many sizes makes a raster per size (tens of ms at 1x, up to a couple of
  hundred at 2x for a 240 px window), so a drag across the threshold may hitch once per size; the cache is bounded.
- **Tests.** Table: `bands.rs` (every pixel of a 120x80 window is drawn at its coverage, once at most, at scales 1, 1.5 and 2
  and at an offset; corner pixels; only the damaged part; at most 31 draws; a subsurface inside the curve); `shadow/tests.rs`
  (cut-out table at scales 1 and 2 for corner and strips, the half-pixel start, rounded versus square corner, sharpness at
  scale 2 rewritten for the cut, the reference raster now cut out, a large window's whole raster against its pieces within 4/255,
  a tiny window's whole raster lighter than its cropped pieces, whole raster sizes and cut-out at 1, 1.5 and 2, which sizes get a
  raster); `draw_cache.rs` (a raster per size, shared by windows, not the buffers). Testkit `scene/clip.rs` runs the corner
  check on GLES (llvmpipe) and Pixman, for a shm buffer and for a single-pixel buffer under a viewport, and checks the curve
  is blended. No golden changed (no golden scene binds the decoration manager); no `expected-failures` change.
- **Gate and real run.** Likeliest compile faults: `bands::draw` generic over `R: Compositor` with `R::Frame<'_, '_>` and
  `frame.render_texture_from_to` (needs `Frame` in scope), `*color * alpha` on `Color32F`, `Rectangle::subtract_rects` with a
  `copied()` iterator, the glob `use super::*` in `shadow/tests.rs` reaching `punch`, `full`, `wanted`; `SoftRenderer`'s `Debug`
  in the testkit messages; `with_renderer` on the spec. Likeliest test-number faults: the "cropped lighter than whole by 4"
  margin and the "out beyond the corner" shadow being non-zero at 3 px off a radius-20 corner (about 4/255). In a real run:
  a translucent window (a terminal with opacity) shows no shadow through itself, including at the corners, in light and dark; a
  Pixman session (`--renderer pixman`) has round bottom corners; a GL client on a y-inverted dma-buf window and a
  single-pixel-buffer window have round bottom corners without a seam where the curve meets the straight edge; a 100x60
  dialog's shadow is lighter than before and has no seam; resizing it across 120 px does not hitch badly.
## wave-2026-10-15/input2

Lane `input2` (not compiled; the orchestrator gates). Closes the two "Gaps noted, not changed" of `session`.

- **Live apply.** casement watched no `settings.toml` before (only quire's `appearance.toml`). New
  `casement::settings_live` watches the document through ds-settings (the appearance watch's thread
  spawner, now `pub(crate)`), and `settings_live::apply` tells the model `followed(current, doc)`:
  the file's pointer and touchpad profiles and the key repeat; everything else (bindings from the
  keymap, the layout, windows, outputs) stays as running. `core_loop::note_settings` now also calls
  `tty::reconfigure_pointers` on every `SettingsChanged`, whatever its source. The seam side:
  `casement_backend::input::PresentDevices` (pure; remembers each device's class and what it was last
  configured with; `changes(&PointerSettings)` is the diff), held by `TtyBackend`, which records
  appearances, departures and every successful `configure_device`; `TtyBackend::reconfigure_pointers`
  applies the diff through `InputDevices::apply`. The fake (`FakeInputFeed`) needed no change.
  Tests: table `a_settings_change_reaches_exactly_the_devices_it_changes` and three more in
  `casement-backend/tests/it/input_config.rs`; fixture test `a_settings_change_reaches_a_device_already_present`
  (`Fixture::settings_file_changed`); table `the_file_changes_the_pointer_and_the_repeat_and_nothing_else`.
- **Departures.** (1) The keymap layout (`keyboard.xkb_*`) does not follow the file: the seat's keymap is
  compiled at start, and putting the new names in the model without recompiling would make them disagree;
  still a restart. Key repeat does follow (`repeat_info` already ran on `SettingsChanged`). (2) A scroll
  factor change alone also re-applies the device settings (libinput ignores it; the scroll scaling reads the
  stored value per event, so it was already live). (3) Windows, spaces, outputs, GPU settings are still read
  at start only.
- **Old files.** `settings.toml` has a top-level `version` (`config::Version`, none in old files, current 2).
  `config::load` returns `CasementDoc::migrated()`: from no version or an older one, a key that still holds a
  retired default (only `pointer.natural_scroll = "off"`, the old mouse default) takes today's default;
  any other value, and all of a file at version 2 or newer, is kept. `touchpad.natural_scroll` is untouched
  (its default never changed). A migrated read is in memory only; `settings_live::attach` runs
  `config::stamp_version` at start, which prepends `version = 2` to an existing unversioned file as plain
  text (comments, unknown keys and order stay), so a person who then sets mouse natural scrolling off keeps
  it at the next start. Live reloads are not migrated (the stamp is already there, or a tool dropped it and the
  values are deliberate). Cost, unavoidable: a person who set mouse natural scroll off by hand between 8b10803
  and this change, in a file with no version, loses it once. Tests: `an_old_file_drops_the_retired_default_and_keeps_what_the_person_set`
  (table), `migration_touches_only_the_retired_default`, `a_migrated_document_is_written_at_the_current_version`,
  `stamping_writes_only_the_version_and_only_once`, `stamping_a_missing_file_writes_nothing`. Two existing tests
  now compare with `.migrated()` (a missing file loads at the current version). The schema (`casement.settings.toml`)
  does not list `version`, as `[bindings]` is not listed: the schema test skips nothing because a default
  document serialises without it.
- **Only a real run proves.** Editing `~/.config/casement/settings.toml` (or a Settings app save) while a mouse
  and a touchpad are in use: the next scroll/tap follows within the 30 ms debounce and no replug; libinput accepts
  re-applying `set_scroll_natural_scroll_enabled`/tap/accel on a live device (RealInputFeed::apply is the same call
  as at appearance); the watcher thread starts under the compositor's real config home; our own stamp write does
  not loop (it fires one reload that changes nothing).

## wave-2026-10-15/ovpolish

Mission Control polish (the follow-ups "Overview polish" and "Hard-coded colours to quire tokens"). Written without a compiler
(wave rule); the orchestrator builds.

- **The frame shrinks with the window.** The chosen way is "draw the frame at full size into the scaled rectangle", not a scaled
  `FrameStyle`. `FrameDraw` gains `full: LogicalSize` (the content's real size; `content.size` everywhere but Mission Control).
  `frame_elements` rasters the titlebar, the title and the shadow tiles at the full size (so their cache keys do not change while
  the windows fly, and nothing is re-rastered) and draws each into its rectangle shrunk about the content's corner
  (`element/frame/shrink.rs`, whole numbers; both corners of a rectangle are shrunk and rounded, so touching parts still touch).
  The factor is the width ratio `content.w / full.w` for both axes. `shown_outline` and `shown_length` (pub, doc-tested) give the
  drawn outline and a shrunk length: the ring and the label follow the shrunk outline, the bottom-corner clip takes the shrunk
  radius, and the hit test (`core_loop/overview.rs::window_at`) reaches the shrunk titlebar above a slot (the full size is the
  window's desktop target in `arrangement.windows`). `core_loop/titles.rs` makes the title for the full-size titlebar in the
  overview (it used to make it for the scaled bar). Layout's spread reserves the scaled frame: `reserved(reserve, scale)` rounds up,
  the render rounds to nearest, so the frame never leaves its box.
- **Minimise flights.** Unchanged on purpose: a Scale flight still draws the frame at full metrics round the shrunk content
  (`full = content.size`), the Genie and the closing frame too. The same change would fix them (set `full` to the window's real
  size and let the Scale flight's sampled size be `content`), but a minimise flight also fades and moves its popups; left for the
  owner to decide after seeing the overview. Departure: the shadow of a shrunk window is the full-size shadow shrunk, so its blur
  radius shrinks with the window as it would on the Mac, but the corner tiles are stretched (not re-rastered): a slightly softer
  edge at small ratios.
- **Close causes (model).** `Model::leave_overview(None)` (the normal leave, flight home and timer) now also runs on: a Space switch
  (`show`, only when the shown Space really changes; a switch to the Space already shown changes nothing), the lock request
  (`lock_event`, `Unlocked` + `Requested`) and the last seen window of an output closing (`unmap` -> `leave_when_bare`: the closing
  window was seen and not a shell window, and no window other than a shell window is seen on its output any more). A window
  closing on a Space that is not shown changes nothing. Entering and open both close; leaving and closed ignore it.
  A new window while it is open keeps today's behaviour: it joins the Space, gets a slot at the next relayout, the overview stays
  open and the shell hears nothing (a test pins it; macOS does the same).
  Departures: (1) the overview is global, so a Space switch on any output closes it on all; (2) the last-window rule reads the
  output of the closing window, so a close on output B with output A's windows gone does not close it; (3) which relayout cause
  the switch sends first (`SpaceSwitch`, then `Overview`) is moot: exec keeps the first and both travel on the Move spring.
- **Themed colours.** Boundary: `scripts/check-boundary.sh` has no row against `ds-style` in `casement-layout` (B1 limits only
  `casement-core`'s own dependencies; B13 limits only ds-settings), and `frame.rs` already reads ColourToken there. So the values
  are read in a new `casement-layout/src/look.rs` and reach the renderer through the loop: `overview_ring(scheme)` is
  `ColourToken::Accent` (quire's accent fill, Blue until the person's accent is carried to casement; casement has no accent setting
  today), `overview_dim(scheme)` is `ColourToken::Scrim` (black at .22: alpha 56 where 64 was written), `split_backdrop()` is
  `ColourToken::Paper` in its Dark value (#1E1E1E) in both schemes (was 28/28/30), because the Mac's backdrop stays dark on a
  light desktop and quire has no separate backdrop token. `OverviewDraw::On` carries an `OverviewLook { dim, ring }` (premultiplied
  `Rgba8`, made by `From<FrameColour>`). The ring is the light scheme's accent fill rather than the dark #0A84FF
  that was written: the colour changes slightly in light. No golden pinned any of these.
- **App icon in the hover label: not done (departure).** casement can resolve an app's icon only when the client sets
  `xdg_toplevel_icon` (`casement_wayland::xdg_toplevel_icon::icon_of(root)` returns the shm buffers or a theme name, nothing reads it
  yet); the usual source (`.desktop` -> icon theme lookup) needs an icon loader, which this lane does not add. To finish: pick the
  buffer nearest 16 px x scale from `icon_of`, read it with the capture's `ArgbImage::from_shm`, and lay it left of the title in
  `label_element` (the label's raster has no left margin to make room: widen `LABEL_MIN`).
- **Tests.** Layout: `spread.rs` (`the_frame_reserve_shrinks_with_the_window` table at four scales, `a_shrunk_window_keeps_only_its_scaled_frame_above_the_slot`);
  `look.rs` (the three colours are quire's tokens). Render: `frame/shrink.rs` tables (lengths, no-shrink cases, touching parts,
  placement about a corner, the titlebar keeps a pixel), `frame.rs` (the shrunk bar and outline land on whole pixels at 1x and 2x:
  (100,186,400x14) and (100,186,400x314) at 1x, doubled at 2x; a frame at its own size is where it was; the shrunk shadow margin),
  doc tests for `shown_outline`, `shown_length`, `OverviewLook::new`, `Rgba8::from(FrameColour)`. Model (`model_overview.rs`): a Space
  switch (entering and open), a switch to the shown Space does nothing, the lock, the last window (one left keeps it open), a close on
  a Space not shown, a window arriving. No golden or `expected-failures.txt` changed.
- **Gate.** Likeliest compile faults: `frame.rs` (`const METRICS: fn() -> FrameMetrics = || ..` in the tests; the `Seen` struct; `Shrink`
  is `pub(crate)` in a private module and used in a `pub fn` body only), `scene.rs` (the `at_rest` local shadows nothing but `shrunk`
  must be computed before `placed` is shadowed; the `if let ... && ...` with a struct pattern), `windows.rs` (`outputs.clone()` in a
  non-move closure, then `outputs` moved into the next), `model/overview.rs` (`WindowRole` is `PartialEq`), `solid.rs` (the `From`
  impl names `casement_core::window::FrameColour`). The test most likely to need a tweak: `a_space_switch_closes_the_overview_with_no_pick`
  assumes `SwitchSpace(Previous)` after a full-screen request lands on the desktop and `Next` goes back (as `model_spaces.rs` does).
- **Real run.** Ctrl+Up with framed windows: titlebar, lights, title and shadow shrink with the window as they fly in, the ring sits
  just outside the shrunk titlebar, no title is missing or blurry at 1x and 2x (the title is stretched from the full-size raster, so
  at small ratios it is downsampled by the GPU: look for shimmer while flying); hovering the shrunk titlebar picks the window;
  a window with 20 siblings sits closer to its neighbours than before (the reserve shrank). Switch Space with the keyboard, lock the
  session, close the last window with a command line tool: the windows fly home each time and a lock cover goes up over it. The
  dim is lighter than before, the ring is the accent's light-scheme blue, the Split View backdrop is #1E1E1E.
## wave-2026-10-15/capgpu

Nothing here compiled or ran (the lane does not build). Closes the two gaps `capture3` left. No new `unsafe`, no Cargo edit.

- **GPU formats and modifiers in the dma-buf offer (tty).** New seam `casement_backend::tty::CaptureGpu` (`capture_gpu.rs`),
  lent by a display device through `Kms::capture_gpu` (default none; `TtyDevices::capture_gpu` takes the first open device that
  lends one). The real lender (`capture_gpu_real.rs`) wraps the device's `Rc<RefCell<GlesRenderer>>`, the renderer that draws the
  scanout, with `try_borrow` so a busy renderer is an error and an empty offer, never a panic. `TtyBackend::capture_formats` is now
  the GPU's `Argb8888`/`Xrgb8888` formats with their modifiers first, then the shadow's linear ones (`merged_formats`, no duplicates);
  `ext` sends them per format with their modifiers, `wlr` sends the fourccs. Pure and table-tested: `capture_route(target, gpu)`
  (GPU when it lists the buffer's format and modifier, else software), `capture_fallback(target, software)` (after a GPU fault,
  software only when it can bind that format and modifier, else the frame fails), `merged_formats`. A bind or draw failure logs one
  `info` line (`the display GPU did not draw a capture; using the software renderer`) and falls back as above. Headless is unchanged
  (its own renderer already offers whatever it binds, GLES on llvmpipe included).
- **Departure: the scene is not rendered straight into the client's buffer.** The brief asked to bind the client's dma-buf and render
  the output or toplevel into it with the GLES renderer. The handlers work from the frame the capture shadow already read back
  (cursor-less or painted, cropped for a region or a window, damage logs keyed to it), and the scene source is not available at
  delivery time. Rendering the scene again per request would bypass the cursor/`bare` choice, region crops and window capture,
  and a change that size cannot be checked blind. So the existing `write_dmabuf` (upload the frame as a texture, bind the dma-buf,
  draw 1:1, wait) now runs on the **display GPU's** renderer. What this buys: the client allocates in a GPU modifier (OBS and
  other portal consumers stop being refused), the buffer is written by the GPU that owns it, no linear detour for the client. What
  it does not buy: the frame is still read back to the CPU once per frame by the pixman shadow; true render-to-dmabuf needs the
  scene source at delivery (a follow-up: `FrameSource::gles` into a bound dma-buf with a fresh `OutputDamageTracker` at age 0, then
  cursor-less variant by dropping `Kind::Cursor` elements).
- **Damage.** Unchanged and shared with shm: the damage the client is told comes from the damage logs, not from the writer, and
  the dma-buf write redraws the whole frame (as the shm write copies it whole).
- **Client cursor viewport and transform.** `raw_image` now reads the surface's `buffer_scale`, `buffer_transform` and
  `SurfaceView::src` and calls `ArgbImage::drawn(&DrawnView)` (`pass/capture/drawn.rs`): the buffer is turned upright by the inverse of
  its transform (smithay's rule: a surface point sits in the buffer at `transform_point_in(p) x scale`), then cut to the viewport
  source (`crop_pixels`: edges rounded to the nearest pixel and kept inside the image, at least one pixel); the existing `fitted`
  then scales to the drawn size (viewport destination x output scale, already what `cursor_face` announces) and turns with the
  output. So the announced size, the image and the hotspot (surface-local, in destination units) agree with the screen.
- **`overlay_cursor` / `paint_cursors`.** Their cursor comes from the scene render (`cursor_elements` uses
  `render_elements_from_surface_tree`), which already applies viewport, transform and scale, so no change was needed there; the
  gap was in the cursor *session's* own image. Not proven by a test of its own (a painted-frame pixel test needs the cursor's
  place in the frame); the new protocol tests cover the session. Check in a real run that `grim -c` with a GTK/Qt cursor matches.
- **Tests.** Pure: `crop_pixels` table (whole, corner, scale 2, fractional rounding, past the edge, sliver, outside), buffer transform
  undone for all eight transforms, viewport crop under a quarter turn (`drawn.rs`); `capture_route`, `capture_fallback`,
  `merged_formats` and three doc tests (`capture_gpu.rs`). Backend with fakes (`tests/it/tty/capture_gpu.rs`, new `FakeCaptureGpu`
  with `GpuBehaviour`, lent by `FakeKms` through `KmsWorld::capture_gpu`): the tty offer is GPU first then linear; no lender, linear
  only. Protocol (`capture_cursor.rs`): a coordinate-encoded cursor with a viewport source (4, 2, 8 x 8) shown 16 x 16, and one with
  a 180 degree buffer transform. **Not covered**: a frame actually drawn into a client dma-buf (the test clients cannot import
  one; a GLES render test needs a GBM/udmabuf-backed `Dmabuf`), a quarter-turn buffer transform at protocol level (the pure test
  covers the maths), `FakeCaptureGpu` being asked to draw (needs a `Dmabuf`).
- **Gate.** Likeliest compile faults: `Kms::capture_gpu` default method with `Box<dyn CaptureGpu>` in `kms.rs`/`fake.rs`; `renderable_formats`
  (`Bind::<Dmabuf>::supported_formats` on `&GlesRenderer`); `use smithay::backend::allocator::Buffer as _` in `TtyBackend::write_dmabuf`
  and in the fake; the `as i64` cast in `crop_pixels`; `with_states` closure returning `Option<(WlBuffer, DrawnView)>` in
  `cursor.rs`; `Viewporter::bind(fixture.client(id))` taking `&mut` for `&`. `tty/mod.rs` is the longest file touched.
- **Real run.** OBS through the pipewire portal on the tty session: it should now negotiate a GPU modifier (not only linear) and show
  the desktop right way up (if upside down, `flipped` on the bind in `write_dmabuf` is the lever, as capture3 said; GLES binds a
  dma-buf with the opposite row order to pixman); `grim` and `wf-recorder` (wlr `linux_dmabuf` offers fourccs only, so the client
  picks the modifier: look for the info line if the GPU refuses and the frame falls back to linear or fails); a GTK/Qt app that sets
  a cursor with a viewport or `buffer_scale 2` (Firefox, KDE apps) in a cursor-session capture (remote-desktop viewer) matches the
  screen; a GPU-modifier buffer after the display GPU is busy mid-frame (`try_borrow` fails: expect the info line, then software).
  With two GPUs only the first device's renderer is lent; a buffer allocated on the other GPU fails over to software.

## wave-2026-10-15/closesnap

A closing window fades and shrinks as one: the body now goes with the frame (the follow-up "A closing window loses its frame at once",
and the "Not solved: the content of a closed window" row of flight2). Written without a compiler (wave rule); the orchestrator builds.

- **What is kept.** The window's last image, drawn alone through the pass window capture uses (`window_rgba` ->
  `render_offscreen`, content size at its output's scale), read back as RGBA and baked into a `ClosingBody` (casement-render
  `element/closing_body.rs`): the bottom corners are cut by `ClipShape::coverage` at the kept radius, the channels are put in
  Argb8888 order, and it is a `MemoryRenderBuffer`. `ClosingFrame` gained `body: Option<ClosingBody>` (and lost `Copy`).
  `closing_elements` draws front (titlebar, lights, title), then the body as one `MemoryRenderBufferRenderElement` stretched over the
  sampled content rectangle at the sampled opacity, then the shadow.
- **Departure: a memory buffer, not an `Offscreen<GlesTexture>` texture.** The task suggested a GPU texture and
  `TextureRenderElement`. The `Compositor` trait is generic over GLES, pixman and the dummy renderer, and the frame is built for
  whichever the backend runs, so a texture type cannot ride in `ClosingFrame`. The existing window-capture pass already renders a
  window offscreen on all three and on the tty seat's pixman shadow, and a memory buffer draws on every renderer (smithay caches the
  upload per renderer). Cost: one GPU read-back at close (a one-off stall, once per closing window) and one upload on the first
  frame. If the stall shows on a 4K window, the follow-up is a GLES-only `Offscreen<GlesTexture>` variant behind `Compositor`.
- **Departure: the rounded clip is baked, not applied per frame.** The corner cut is made once at the window's rest size; the body
  then shrinks with its corners (the frame's own radius does not shrink either, so they stay matched to within the shrink).
- **When it is taken.** Two points, both while the window is still in the model and the old buffer still attached:
  a pre-commit hook added in `new_surface` (`handlers/compositor.rs`) that sees `BufferAssignment::Removed` pending on the root of a
  window the book draws (a client that unmaps by a null commit), and `toplevel_destroyed` (`handlers/xdg_shell.rs`) before the model
  hears of it (a client that destroys the toplevel with its buffer attached). A winit backend has no offscreen pass: no image, the
  window and its frame go at once as before.
- **Which windows.** Only a window the loop kept a frame for (`RenderState::closing`, i.e. framed), with a size, that fits the cap, with
  no image yet (`bodies.rs::verdict`, table test). The task said "motion on, not reduced motion": the loop passes
  `MotionLevel::default()` (Standard) to `retarget` and there is no setting, and `retarget` cross-fades a departing window even under
  Reduced (only the travel snaps), so every framed window animates its close and the level does not gate the snapshot. If a motion
  setting lands, `Facts` gets the level.
- **Freed when.** `closing::forget` (every frame) drops an image whose window is back in the model (a client that hid and re-showed
  it) or no longer in the motion scene (the fade is over). Cap: 6 images and 192 MiB together, oldest dropped first (`Bodies::admit`,
  tested); one image over the byte cap alone is not taken (`TooBig`).
- **The switch is gone.** `ClosingFrames`/`CLOSING_FRAMES` removed. `closing::on` returns a window only when it has an image, so a
  window without one (winit, too big, the read-back failed) has no frame either, as before flight2.
- **Per output.** One image at the scale of the output the window was on, stretched to the sampled rectangle on every output the
  window is drawn on (a window straddling two outputs of different scales is slightly soft on the other).
- **Tests added.** casement-render: `closing_body.rs` unit (bake: channel order, square top, cut bottom corners, bad length/empty;
  equality by id), doc test on `ClosingBody::from_rgba`; `tests/it/closing_body.rs` on pixman (stretched over the content, outside
  stays black; half opacity is half over black). casement: `core_loop/bodies.rs` (verdict table; scale counts in the weight; oldest
  dropped past the count; re-admit replaces; finished windows freed). No golden and no `expected-failures.txt` change were made by
  hand; a testkit scene that closes a framed window mid-fade (pixman, the dummy renderer reads back zeros) may now show the frame
  where it showed nothing: bless with a reason if one exists.
- **Gate.** Likeliest compile faults: `bodies.rs` (`Kept` pattern with `..` on a `pub(crate)` struct from a sibling module; the guard in
  `admit`'s match; `content_and_scale` made `pub(crate)`, `window_rgba` re-exported without the `debug-inject` cfg), the pre-commit
  hook's `with_states` closure type in `before_commit`, `ClosingBody::element`'s `from_buffer` generics, `ClosingFrame` losing `Copy`
  (`scene.rs`, `titles.rs`), `LayerDraw`'s derives with the new field. Real run: close a framed window of a client that unmaps by a null
  commit and one that destroys the toplevel (a terminal, a browser): the body shrinks and fades with the frame, no empty-body ghost, the
  last image is the window as it was (not blank), bottom corners rounded; close two at once; close a 4K window (the read-back stall at
  the moment of closing); an app quitting with many windows (the cap); `grim` after the fade shows nothing left behind; watch memory
  after many closes (the images are freed when the fade ends).
## wave-2026-10-15/kbdlive

Lane `kbdlive` (not compiled; the orchestrator gates). Closes input2's departure 1: the keyboard layout follows
`settings.toml`.

- **What changed.** `settings_live::followed` now also takes `keyboard.xkb_*` from the file. `settings_live::apply`
  first calls `switch_keymap`: if the wanted names differ from the ones the seat's keymap was compiled from, it
  calls smithay `KeyboardHandle::set_xkb_config` (through `casement_wayland::seat_keymap::switch_layout`), and the
  model is told the names the keyboard runs with afterwards (the new ones, or the old ones on failure). Repeat
  rate and delay already followed (`apply_repeat_info`). New pure `casement_wayland::seat_keymap`: `KeymapBook`
  (on `SeatPart.keymap`; the running names and the last refused ones), `plan` -> `KeymapStep::{Unchanged, Refused,
  Compile}`, `refuse` -> `Report::{Say, Quiet}`; `xkb_config(&XkbNames)` is shared with `SeatPart::new`.
- **Bad keymap.** `set_xkb_config` compiles before touching anything, so a failure leaves the keyboard as it was;
  we log one warning per distinct refused set of names (the same bad file read again, or an unrelated edit while it
  stays bad, is quiet) and the model keeps the old layout, so the model and the seat never disagree.
- **Modifier and held-key state (smithay's behaviour, `update_xkb_state`).** Keys still held are replayed onto
  the new xkb state, so a held key stays held and its release is matched; modifiers are recomputed from those
  keys and sent to the focused client after the new keymap. Latched and locked modifiers (Caps Lock, a sticky
  Shift) are NOT carried over; LEDs are updated. A held key's already-sent press is not re-sent or re-interpreted
  and the clients' own repeat timers are untouched. Our shell bindings read keysyms from the live xkb state per
  key (`cmd+space` and the like are modifier plus a key whose symbol is `space` in every layout); a binding on a
  letter follows the new layout's symbol for that key, as on the Mac.
- **Tests.** Pure: `seat_keymap` tests (`a_layout_change_compiles_and_the_same_names_or_a_refused_pair_do_not`,
  `a_refusal_is_reported_once_per_pair_of_names`, `a_switch_forgets_the_refusal`); table
  `the_file_changes_the_pointer_the_repeat_and_the_layout_and_nothing_else` (renamed; the layout row flipped from
  "waits for a restart"); doc test of `KeymapBook::new`. Fixture: `a_layout_change_in_the_settings_file_sends_a_new_keymap`
  (second `wl_keyboard.keymap` after a layout change; none for the same file, an unrelated section, or a layout
  that does not compile; the model keeps the old names). It counts keymap events, it does not read the keymap
  text. No golden or expected-failures changed.
- **Departures.** The test uses `no_such_layout_zz` as the uncompilable layout: it relies on libxkbcommon
  refusing an unknown layout name (it does with the stock xkeyboard-config); if the gate's data lets it through,
  pick another bad name (a bad `xkb_rules`, say). Windows, spaces, outputs stay read at start.
- **Only a real run proves.** Editing `keyboard.xkb_layout` (e.g. `us` to `de`, or `us,de` with
  `grp:alt_shift_toggle` options) in `~/.config/casement/settings.toml` while a terminal has focus: the next key
  types the new layout at once with no restart; a held modifier key still releases cleanly; Caps Lock is cleared by
  the switch; a typo in the layout leaves typing unchanged and one warning in the log; `cmd+space` still works.
  Toolkits that cache the keymap (GTK, Qt, Xwayland through xwayland-satellite) must take the new `keymap` event.

## wave-2026-10-15/mcwire

The three asks of sill's Spaces bar (`## spacesbar`), as `quire_shell_v1` **version 2** (casement-protocols `lane/cas9-mcwire`,
FINDINGS `## wave-2026-10-15/mcwire` there). Written without a compiler (the lane rules); the gate is the first build. Land
casement-protocols first. Built on casement 79456e1: `ovpolish` (6cb6035) is not merged here; the files both touch are
`core_loop/overview.rs` (the `press`, `hover` and `leave` bodies; `window_at`, `draw` and the colours are ovpolish's) and
`model/overview.rs` (untouched here).

- **1. Which output (`overview_output`).** casement's overview is global (one `Overview` in the model, every output flies its
  windows), and that is the Mac's behaviour with separate Spaces: Mission Control opens on every display at once. So the event is
  sent for every output, `shown` while the overview is not closed and `hidden` when it is, just before each `overview_state`
  (`exec/shell.rs`, `shell_space::send_overview_outputs`, only to objects bound at version 2 or later, through
  `Output::client_outputs`: a client that has not bound `wl_output` hears nothing). Not done: an output plugged in while Mission
  Control is open is not told until the next state change; a shell that binds while it is open hears nothing until then.
- **2. Space pictures (`get_space_capture_source(source, output, space_id, width, height)`).** Reuses ext-image-copy-capture: the
  request makes an `ImageCaptureSource` (smithay's custom-source path) whose user data is a `SpaceSourceId`
  (`casement_wayland::quire::space_shot`, a pure book like the toplevel one); `handlers/image_capture.rs` hands such sessions to
  `handlers/space_shot.rs`. The constraints are exactly the size asked for (each side clamped to 1..=1024), shm and the usual
  dma-buf offer. A picture is the windows of the Space the output shows, each drawn alone with `capture_source::window_rgba`
  (no frame, no popups), shrunk to where `arrangement.windows` puts it on the output and composed back to front by
  `casement_render::pass::compose_thumb` (integer box average, premultiplied over, `thumb_dest` rounds both edges), over a
  **transparent** background (the shell draws the wallpaper). Damage: a request is held until the Space's `Look` changed (an FNV
  fingerprint of each window's id, rectangle and commit count; commits are counted only while a session exists), checked after
  every commit (`capture_source::committed` calls `space_shot::committed`) and every relayout (`core_loop::relayout` calls
  `space_shot::poll`). **Departures:** (1) only the Space the output shows now: a source on another Space, a removed Space or a
  Space that is switched away stops its sessions (`stopped`); the bar must ask again after a switch. The other Spaces need their
  windows' places on a Space that is not shown, which `Arrangement` does not carry. (2) The pictures have no wallpaper, no menu
  bar, no dock, no window frames or shadows; a full-screen or Split View Space shows its windows as they sit. (3) Each refresh
  draws every window of the Space again through the software path (one CPU read-back per window): fine for a few small pictures,
  to be measured with many windows. (4) A window moving during a spring (a Space switch) changes `arrangement` only at its end
  state, so the picture jumps to the end. (5) Stacking is `arrangement.windows` order, not the focus stack.
- **3. Dropping a window on a Space.** Pure parts in `casement_core::overview_drag` (table-tested): `SpaceTargets` (per output
  rectangles, last added in front, only plugged outputs hit), `Drag` (`Idle`, `Pressed`, `Dragging`; threshold `DRAG_THRESHOLD`
  = 8 logical px, the distance squared compared) and `Released` (`Pick`, `Drop`, `FlyBack`). The loop (`core_loop/overview_drag.rs`):
  a press on a slot no longer picks at once, it starts a `Drag`; the release picks (as the press did) when it never moved past the
  threshold. Past it the window's overview slot becomes the pointer's travel from the press (the window keeps its place under the
  pointer, its slot moved to the front of the list for hit testing), motion is retargeted with `Cause::Drag` (so the window follows
  the pointer through the existing flight code, no new drawing), the hover ring follows it, the cursor is the closed hand, and the
  shell hears `drop_target(output, space)` whenever the target under the pointer changes (null when it leaves or the drag ends;
  `ShellNote::DropTarget`). A release over a target feeds `DesktopRequest::Window { ToSpace }` (the existing path, `window_to_space`)
  and Mission Control stays open; a release elsewhere calls `relayout(Overview)`, which puts the slot back and the window flies
  home on the `Move` spring. A press on the picture still leaves at once. `leave()` resets the drag. Requests: `space_target_clear
  (output)` and `space_target(output, space_id, x, y, w, h)` (global logical pixels; an empty rectangle is `bad_target`; an unknown
  output or Space id is kept out silently); the table lives on `QuireShellPart::space_targets` and is cleared when the last shell
  object goes. **Departures:** (1) the dragged window is drawn in the usual window stacking, so it can pass under another window's
  slot (the slot list is reordered for hits, the scene is not); raising it in the model would change focus order. (2) The drag
  starts only from entering or open, as a pick did. (3) The target highlight is the shell's: casement draws nothing for it.
  (4) A window dragged to the Space it is already on is moved to it again, which the model ignores (no change). (5) The fly-back
  is the `Overview` spring (350 ms settle timer unchanged), not a dedicated animation.
- **Not asked, not done.** Dragging a window between outputs in Mission Control; dropping on a "new desktop" tile (the Mac adds a
  Space); dropping a window onto an app; a per-output overview (the event shape allows it).
- **Test client.** `ShellHandle::{bind_outputs, space_target_clear, space_target, overview_outputs, drop_targets}`, `BoundOutput`
  (a bound `wl_output` with its connector name), `OutputCapture::open_space`. No catalogue change: `QuireShellV1` is a root and
  its new messages' interfaces (`ext_image_capture_source_v1`, `wl_output`) are reached from it. No `globals.txt` change (the
  private globals are not listed), no golden changed, `expected-failures.txt` untouched.
- **Tests.** casement-core `overview_drag.rs` (hit order and live outputs, clear, threshold table, the window keeps its place, target
  changes reported, release picks/drops/flies back, doc tests); casement-render `pass/thumb.rs` (placement table, box average,
  over blending, clipping, half off the picture, doc tests); casement-wayland `quire/space_shot.rs` (size clamp, damage gate,
  forget, destroyed source, fingerprint) and doc tests on `target_rect`, `clamp_side`, `look_of`; casement-testkit
  `protocol/quire_shell_spaces.rs` (two outputs hear `overview_output` and a version 1 shell does not; a press that hardly moves
  picks; drag, hover event, drop and stays open; release elsewhere flies back; an empty target is a protocol error and a cleared
  one is not hit; a picture on first request, held, sent on commit, background transparent; a picture of a Space not shown stops).
  Existing: `quire_shell_overview.rs`'s click tests now go through press-then-release and still pick.
- **Gate.** Likeliest compile faults: `shell_space::on_request` (the generic `D: QuireShellHandler` also needs
  `Dispatch<ExtImageCaptureSourceV1, ImageCaptureSourceData>`, supplied by the new supertrait `ImageCaptureSourceHandler`; a
  `GlobalDispatch2/Dispatch2` impl that names `D: QuireShellHandler` only may need the supertrait spelled out), `exec/shell.rs`
  (`Shown` is the generated server enum, `Overview` is non-exhaustive), `core_loop/overview_drag.rs` (`Moved` and `Released` are
  non-exhaustive: the `let ... else` and the `_ =>` arm), `handlers/space_shot.rs` (`frame.fail`, `frame.has_failed`,
  `Frame` is consumed by `answer`), the testclient (`Dispatch<WlOutput, Shared<Option<String>>> for Host`;
  `get_space_capture_source` is called with the side queue's handle on an object of the main queue). Tests likeliest to need a
  tweak: `a_space_picture_is_sent_on_damage_only` (assumes the output is `HEADLESS-1` from `small()` and that the second
  `request` is held; the opaque-pixel bounds assume windows do not cover the whole 640x480), the drag tests (the target at
  (10,10) 100x90 must be clear of every slot: slots start below a 112 px strip).
- **Real run (sill (casement)).** Ctrl+Up: the bar appears on every output; hovering a thumbnail shows its picture within a frame
  or two of a change; press a window, move a few pixels (nothing yet), more (the window follows, the closed hand, the Space tile
  under it lights); release on the tile (the window leaves the grid, Mission Control stays); release elsewhere (the window glides
  home); a plain click still picks. With two outputs, each shows its own bar and a window dropped on the other output's tile moves
  to that Space.

## wave-2026-10-15/mc3

The four "Not done" items of `mcwire` and the dragged window's stacking, as `quire_shell_v1` **version 3** (casement-protocols
`lane/cas10-mc3`, FINDINGS `## wave-2026-10-15/mc3` there; land it first). Written without a compiler (wave rule); the gate is the
first build. Built on casement 78b3152.

- **1. Pictures of every Space.** The premise in `mcwire` ("`Arrangement` carries no places for windows on hidden Spaces") was
  wrong: `casement-layout`'s `arrange_output` already arranges **every** Space of an output as a desktop (the targets are "as if
  shown"; motion adds the strip origin), and `spec_of` already puts each Space's windows in its `SpaceSpec`. What was missing was
  the question "which windows does this Space hold", which `space_shot` answered by filtering `arrangement.windows` with
  `Model::shows` (only the shown Space). So no layout change: `Model::pictured(&SpaceId)` (`casement-core/src/model/picture.rs`) is
  the windows on the Space, plus those on every Space homed on its output, minus the dock and hidden apps, back first by stack rank;
  `handlers/space_shot.rs` draws those at their `arrangement.windows` rectangles. A session is stopped only when its Space is gone
  or no longer on the output (it used to be stopped on every Space switch, so the bar need not ask again). Cost is bounded by the
  sessions: `poll` returns at once with none, a look is made only for a Space with a session, and a frame is drawn only when a
  window of that Space committed, moved or came or went (the look carries each window's commit count). Stacking is now the
  model's order (it was `arrangement.windows` order), which also fixes mcwire's departure 5 for the pictures. Layout tests pin the
  premise: `casement-layout/tests/it/hidden_space.rs`.
  Departures: (1) a hidden Space's windows are not asked to redraw (their frame callbacks are not sent), so the picture is as
  live as the client's last commit; (2) a window moved while its Space is hidden shows at its end state; (3) every refresh still
  draws each window of the Space through the software path, one CPU read-back per window; (4) a full-screen or Split View Space
  shows its windows as the layout places them (a Split View: two halves); (5) a window on every Space is in each picture of its
  output.
- **2. Wallpaper.** Confirmed, no code: the pictures are composed over a transparent background (`compose_thumb`, the test
  `a_space_picture_is_sent_on_damage_only` and the new hidden-Space test assert pixels with alpha 0 and some with alpha 255).
  sill draws the wallpaper under each tile. The wire text says so.
- **3. A drag to another output.** `SpaceTargets::hit` already finds the targets of any plugged output and `Over` names the
  output, so a drop on another output's tile reached `window_to_space` (the model's cross-output path) in mcwire. Added:
  (a) `window_to_space` carries the window's rectangles (`Floating`, and the `restore` of `Tiled` and `Filled`) by the difference
  of the two outputs' corners when the output changes, so a window keeps its place about the corner (before, it kept global
  coordinates and layout pushed it to the nearest edge of the new output); (b) a release off every target but over a **different
  output** than the window's own drops it on the Space that output shows (`Released::FlyBack` now carries `at`); (c) after every
  drop the loop lays out again (`relayout(Overview)`), so a drop the model refuses (the Space the window is already on, a
  full-screen Space) flies back instead of leaving the slot where it was dropped (mcwire departure 4 and 5). Tests:
  `model_drop.rs`, `desktop_places.rs` (`shifted`), `quire_shell_drop.rs` (two outputs: drop on a tile, drop on the bare output,
  release on the own output).
- **4. "New desktop" tile.** Wire: `space_target_new(output, x, y, w, h)` and `drop_target_new(output, over: shown)` (version 3).
  Core: `SpaceTargets::set_new_space`, `Landing { Space, NewSpace }` in `Over`, `ShellNote::DropTargetNew`,
  `WindowAsk::ToNewSpace(OutputKey)` (`Model::window_to_new_space`: makes a desktop at the end of the row and moves the window;
  nothing is made for a full-screen window or an unplugged output). The tile is in front of the output's Space targets; one per
  output; an empty rectangle removes it; `space_target_clear` leaves it. Dropping stays in Mission Control and does not switch
  Space. The exec sends `drop_target_new` to version 3 objects only; `drop_target` gets null in the same step the tile lights.
  An empty Space left behind is kept (macOS removes an empty one; here the shell can remove it with the desktop manager).
- **5. The dragged window on top.** `OverviewDraw::On` gains `lift: Option<WindowId>` (the window of a `Drag::Dragging`);
  `output_elements` moves it last in the back-first list of ordinary windows (`lift_last`, doc- and table-tested), so it is
  drawn above every other slot and its frame, ring and label go with it. It stays below layers that are above windows.
- **Tests added.** casement-core: `overview_drag.rs` (tile priority, per-output tiles, clear/clear_all, release remembers the
  pointer), `model_drop.rs` (pictured, new Space, gone output, cross-output shift), `desktop_places.rs::shifted`. casement-layout:
  `hidden_space.rs`. casement-render: `lift_last`. casement-testkit: `quire_shell_drop.rs` (hidden picture follows its own
  windows only, tile lights/unlights/creates, tile beats a Space target, empty rectangle removes and clear keeps it, version 2
  shell hears nothing, three cross-output drags); `quire_shell_spaces.rs` (the "not shown stops" test became "empty Space picture
  is transparent" and "a Space not on the output stops"; the desk now binds version 3). casement-testclient: `space_target_new`,
  `drop_targets_new`. No golden and no `expected-failures.txt` changed; no `globals.txt` change (private global).
- **What sill must do.** Bind `quire_shell_v1` at 3 (`desktop-client` binds `1..=3`). Ask for a picture of each Space tile on
  each output (`CaptureRequest::of_space`), not only the shown one, and keep the sessions across Space switches (they now live
  until the Space is removed); draw the wallpaper under the pictures; send `SetNewSpaceTarget` for the "+" tile after
  `SetSpaceTargets`; light the tile on `ShellEvent::DropTargetNew` (`Overview::over_new_space`).
- **Gate.** Likeliest compile faults: `core_loop/overview_drag.rs` (partial move of `target.landing` then `target.output`; the
  `tell` match on `(&Landing, Option<&Landing>)` with the non-exhaustive wildcard; `shown_space_at`'s `contains(at)` on
  `LogicalRect`), `overview_drag.rs` in core (`hit`'s `BTreeSet<&OutputKey>` and the `live` closure on `&&OutputKey`), `scene.rs`
  (`others` is now `mut`; `lift_last` over `Vec<&PlacedWindow>`), `exec/shell.rs` (the `Shown` mapping), `desktop_places.rs`
  (`carried_between` is called before `self` moves). Tests likeliest to need a tweak: `model_drop.rs`
  (`a_picture_leaves_out_the_dock...` assumes `WindowAsk::Minimise` puts window 3 in the dock at once;
  the cross-output test assumes the second output is to the right and the centred window floats), `quire_shell_drop.rs` (the
  hidden-picture test assumes `desk.tops[i]` is `desk.windows[i]`, and that two outputs at 0,0 and 640,0 map both windows to the
  first; the `s-nowhere` Space source stops its sessions as mcwire's text says).
- **Real run.** Ctrl+Up with two Spaces: the second tile shows its windows (not blank) and updates when one of its apps redraws
  (apps on a hidden Space may not redraw: expect a stale picture); drag a window onto the "+" tile: the tile lights, a desktop
  appears at the end with the window, Mission Control stays; drag across to the other display: the window follows the pointer,
  is on top of the other slots while it is dragged, and lands on the target or the Space shown there, at the same place about
  that display's corner; a drop on a tile of the Space it is on flies back.

## wave-2026-10-15/dmabuf

Nothing here compiled or ran (the lane does not build). Closes the departure of `capgpu`: a capture into a client's dma-buf is drawn
by the GPU with no CPU read-back, for an output, a window and `zwlr_screencopy` (so `grim` and OBS). No Cargo edit, no new `unsafe`.

- **The choice.** `casement_backend::capture_path::capture_path(BufferKind, RendererClass) -> CapturePath` (`Direct` or `ReadBack`), pure
  and table-tested, is the one place that decides; `handlers/image_capture/direct.rs::path_of` feeds it (the buffer from
  `get_dmabuf` and `placement(main device, drawing device)`, the renderer from `HeadlessBackend::capture_class` /
  `TtyBackend::{capture_class, output_class}`). `Direct` only for a dma-buf on the renderer's own device that a GL renderer binds
  (format and modifier listed). Shared memory, a software renderer (pixman, dummy, winit), a refused import and a buffer on another
  GPU are `ReadBack`. A direct draw that fails logs one `info` line and falls back to `ReadBack` for that frame; every caller has it.
- **Output capture (ext output, `zwlr_screencopy`).** The frame stays a GPU image and is blitted: `casement_render::pass::blit_texture`
  (a part of a texture into the bound dma-buf over a cleared buffer, then a wait on the GPU: ext-image-copy-capture has no fence on the
  wire, so "ready" follows a GPU finish). Headless GL: the output's own canvas (or the cursor-less layer's) is the texture
  (`HeadlessBackend::blit_output_dmabuf`); the headless backend still reads each damaged frame back when it presents (goldens, `shot`),
  so only the *capture* is copy-free there. Real seat: the capture shadow is no longer pixman when the display GPU lends scenes
  (`CaptureGpu::scenes`): while an output is watched, `TtyBackend::capture_draw` draws each frame shown once more into two GLES textures
  (with and without the cursor; `tty/capture_scene_real.rs`, owned by the device with the renderer) and keeps a `PresentedFrame` whose
  pixels are the new `FramePixels::Resident(size)` (time and damage from the GPU draw's tracker). Pixman no longer draws the scene
  for that output (the 4K CPU cost of the old shadow), and a shared-memory client is served by `CaptureGpu::read_scene` (a read-back
  of the GPU image, only for the frames it asks for). `Readback::last_frame` prefers the resident frame; a draw the GPU refuses or a
  busy renderer logs an `info` line and the pixman shadow draws that frame instead.
- **Window capture.** `casement_render::pass::render_window_dmabuf` renders the window's surface tree straight into the bound dma-buf
  (fresh tracker at age 0, transparent where nothing draws, wait). `handlers/capture_source/render.rs::draw_window` tries it first
  (`HeadlessBackend` GL renderer, or `CaptureGpu::render_window` on the display GPU, which also draws windows whose buffers are dma-bufs
  the pixman shadow could not import), then falls back to `window_rgba` + `deliver`.
- **Departure: Space pictures stay a read-back.** `space_shot.rs` composes the windows' pixels on the CPU (`compose_thumb`, a few
  hundred pixels wide) and `deliver` uploads the result. A GPU composition would need a scale per window element (rescale and relocate
  elements) and a filter that matches `compose_thumb`'s; the pictures are small and rare. Not done.
- **Departure: cursor sessions stay a read-back.** The cursor image is a client surface (or a theme image) of at most a few tens of
  pixels, already on the CPU after `ArgbImage::drawn`; `deliver` uploads it. A client's dma-buf cursor is still read back through the
  renderer (`read_dmabuf`).
- **Departure: no multi-GPU renderer.** casement has none for captures (`RenderPlan` routes outputs, the lender is the first device's
  renderer). `placement(main device, drawing device)` is `Foreign` when both numbers are known and differ, and a foreign buffer is
  `ReadBack` (uploaded by the lender or the shadow, as in `capgpu`). The drawing device is `RealKms::node` (the render node of the open
  device, `DrmNode::from_file` then `node_with_type(Render)`); the fakes and headless report none, so `Local`.
- **Damage.** The damage the client is told is unchanged (the logs). The whole frame is drawn every time, direct or not: the client's
  buffer is not known to hold the previous frame (ext-image-copy-capture says the compositor fills the buffer), so a partial draw
  would leave stale pixels. The damage-aware saving is the *skipped* frame (nothing changed: no draw at all), which was already there.
- **Tests.** Table: `capture_path` (six buffer/renderer rows) and `placement` (five rows) in `capture_path.rs`. Backend (`tests/it/
  capture_direct.rs`, needs `/dev/udmabuf`, skips without it): a GL renderer that binds linear ARGB blits the frame and the buffer reads
  back equal to the shm read-back (skips on llvmpipe if it binds no dma-buf); pixman refuses the blit and the window render, and the
  read-back + `write_dmabuf` fallback fills the same pixels; an output that presented nothing has no frame. Tty over fakes
  (`tests/it/tty/capture_gpu.rs`, `FakeCaptureGpu` now has `SceneBehaviour`): a lender that keeps scenes holds the watched frame as
  `Resident` and `output_class` is `Gpu(Accepted)` (and `Refused` for a modifier it does not list); one that refuses the draw leaves
  the shadow's `Rgba` frame and `Software`; one without scenes is never asked; unwatching frees the scene. Redundant tests: none (the
  capgpu route tests still cover the upload path, which stays the fallback).
- **Not covered.** A protocol-level test that a client's own dma-buf `wl_buffer` receives the capture (the test clients still cannot
  import one onto the capture queue), a real GPU blit, the display GPU's scenes in `RealCaptureGpu` (needs a device), `render_window`
  on the display GPU.
- **Gate.** Likeliest compile faults: `blit_texture`'s `<R as RendererSuper>::TextureId` bound and `texture.size()`; `Bind<GlesTexture>`
  and `Offscreen` inference in `capture_scene_real.rs` (`create_buffer` with the annotated `GlesTexture`); `DrmNode::from_file(&OwnedFd)`
  and `node_with_type(...).and_then(Result::ok)` in `kms_real.rs`; `Cursorless` made `pub(crate)` (its `cfg(feature = "tty")` re-export);
  the `matches!(presented(..)?.map(|f| &f.pixels), Some(FramePixels::Resident(_)))` in `frames.rs`; exhaustive matches on `FramePixels`
  (only `casement-testkit/src/output.rs` was one); `MainDevice(pub u64)` against `dev_id()`'s type; `RealCaptureGpu::{store, renderer}`
  `RefMut` lifetimes. Test tweaks: the first `present`s of `watched()` in `tty/capture_gpu.rs` assume the fake scanout answers `Flipped`
  (if it waits for a vblank, step a `presented` between frames); the llvmpipe case of the backend test may skip on every host.
- **Real run.** Per the user's rule the manual-checks queue is `sill/docs/manual-checks.md` (casement has no `docs/manual-checks`):
  on the tty session start OBS through the pipewire portal and `grim` / `wf-recorder`, with `RUST_LOG=info`: no
  `reading it back` / `using the software shadow` line in steady state, and on a 4K output the frame time of the capture is a blit
  (compare `casementctl` frame times with capture open before and after: the pixman shadow's per-frame render is gone, GPU time
  rises by two extra scene draws); the picture is right way up, not green or black (if upside down, the lever is `flipped` on the
  bind in `blit_texture` and `write_dmabuf`); `grim -c` and a cursor-less OBS source show/hide the pointer; a window capture
  (OBS window source) of a dma-buf client (a browser, a game) is not blank; with two GPUs a buffer on the second GPU logs the
  fallback and still shows the picture; a shared-memory client (`grim` without dma-buf) still works through `read_scene`.
## wave-2026-10-15/winit

Lane cas12-winit: the winit backend usable for hands-on testing in a host desktop. Built nowhere by the lane (no cargo).

- **Upside down.** The window's damage tracker draws with `HOST_TRANSFORM` (`Transform::Flipped180`, as anvil's winit
  backend does) instead of `Normal`. The flip is the window's own: the output in the model, the config and the
  `wl_output` stay `Normal`. The winit backend has no capture path (`present` keeps no pixels, `capture_watch` is read
  only for the tty), so nothing is flipped twice; a screenshot of a winit output is not offered at all.
- **Size and scale.** `winit_spec::window_spec` turns the window's physical size and the host's scale factor into mode
  size and scale in 120ths (table-tested: 2880x1800 at 2.0 is 240; a non-positive or non-finite factor is 120). `start`
  announces that output, not the requested one. `WinitEvent::Resized { size, scale_factor }` (also what a scale-factor
  change becomes in smithay) rebuilds the tracker and returns `ConnectorEvent::Changed`; a window with no area is ignored.
  In `core_loop::connector` a winit output now takes the window's `OutputConfig` on `Plugged` (through `spec_config`, which
  the headless backend alone used) and on `Changed` (a new `window_config` branch, then `configure_output`), because the
  model keeps a known output's config when only its facts change. The requested refresh and position are kept.
- **Input.** `WinitBackend::dispatch(&PollContext) -> WindowTurn { flow, fed, connectors }` converts `WinitEvent::Input`
  with `input_kind_of` over the desktop extent (absolute pointer positions are the window's 0..1 position times the
  extent) and stamps `Origin::Physical`, device and `context.now`, as `RealInputFeed` does. The first turn also reports a
  keyboard (`DeviceId(1)`) and a pointer (`DeviceId(2)`) as `FedInput::Appeared`; the core ignores device appearance
  today, and nothing is configured (the host owns acceleration, so the stored pointer settings do not apply here).
  `core_loop/nested.rs` drains it (connector changes first, then input events with the scroll factor of the stored profile)
  and `run.rs` calls it before `core_loop::turn`, replacing the old post-turn `dispatch()`. No device removal is
  reported: the devices live as long as the window.
- **Focus loss.** The core has no release-all mechanism, so the backend tracks held keys and buttons (`HeldInput`) and a
  `WinitEvent::Focus(false)` emits their releases as ordinary events.
- **Not wired.** Touch (a host touchscreen) and tablet events: `device_of` drops them, the window announces no touch
  device. Gestures from a host touchpad are pointer-class and pass through `input_kind_of` as it converts them.
- **Keys the host eats.** The host desktop keeps its own shortcuts: Super alone, Super+key, Alt+Tab, Alt+F4, Ctrl+Alt+F-keys,
  media keys never reach the window, so casement's Super-based chords cannot be pressed nested, and the key-up of a key
  whose press the host took never arrives either. Chords on keys the host lets through (Ctrl, Alt, Shift combinations) work as
  on the tty, because modifiers come from the same key events. A host that grabs the keyboard shortcut inhibit protocol
  (KDE: hold the shortcut-inhibit toggle) gives the window everything; winit does not request it. A chord for nested use
  that avoids the host's keys is the owner's call. The tty's VT chords do nothing here (not a tty).
- **Tests.** casement-backend `tests/it/winit.rs`: the spec table, the kept fields, the transform choice, focus release.
  `docs/manual-checks.md` gains the winit entry (new file).
- **Likeliest first-compile faults.** (smithay rev 90efde4) `winit.rs`: `dispatch_new_events(|event| seen.push(event))`
  needs the closure to be `FnMut(WinitEvent)`; `WinitInput` is re-exported from `smithay::backend::winit` through
  `pub use self::input::*`; `WinitGraphicsBackend::scale_factor` and `window_size` sit in an impl block that may carry a
  renderer bound (`spec_of_window` names `GlesRenderer`); `OutputDamageTracker::new(size, scale, HOST_TRANSFORM)` takes the
  size as `(i32, i32)` via `Into<Size<i32, Physical>>`; `Mode { size, ..base.mode }` and `Scale` equality
  (`next.scale != self.output.scale`). `core_loop.rs`: the `if let (Some(key), Some(config)) = ...` arm and
  `key_of` over `Model::outputs().0`. `run.rs`: `BackendKind` import removed (check nothing else uses it). The doc test on
  `window_spec` parses `WINIT-1:1440x900@60000:120:0,0`.
- **By hand.** The manual-checks entry; also whether the first pointer motion lands under the host pointer (absolute
  motion needs a first `PointerMoved` from the host), and whether a 2x window with the host's fractional scale (1.25)
  keeps the picture sharp.

## cas13-mcicons: the app icon under each window in Mission Control (2026-10-15)

`casement-icons` finds the icon, `casement-layout` places it (`OverviewSlot::icon`), `casement-render` draws it
(`OverviewIcon`, `icon_place`, `icon_element`), `casement::icons` (`IconSearch`, `IconBook`) and
`core_loop/overview.rs` join them. Departures from the Mac:

- **The spread does not make room for the icons.** The icon hangs 26 px below its slot (a 64 px icon, its middle 6 px
  below the bottom edge); the spread's gap between rows is 24, so in a crowded row pair the icon overlaps the top of the
  window below it (and its frame). The Mac's cell is taller. Adding the icon's lower half to a cell's height changes
  every spread golden, so it waits for a deliberate bless.
- **The title moves down.** The hovered window's title sits under the icon, not under the window, so the two never
  overlap. The Mac shows no title there.
- **Looked up on the first frame, not at the key press.** The icon is found the first frame Mission Control is drawn
  (`IconBook::draw_of` reads the desktop entry and the icon file once per window and size), then kept until Mission
  Control is closed. A change of icon or app id while it is open shows at the next opening.
- **How the icon arrives.** It is not animated by itself: its size and opacity follow how far the window has flown from its
  desktop size to its slot (`icon_place`), so it grows and fades in with the slot and leaves with it. On the way out the
  layout has no slots, so `IconBook::remembered` keeps the last ones.
- **The theme.** `SystemTheme` looks in `hicolor` (and the themes it falls back to, then `/usr/share/pixmaps`): casement
  has no icon-theme setting yet. `freedesktop-icons` reads the process's own folders (`$XDG_DATA_DIRS`) and cannot be
  pointed elsewhere, so `ThemeLookup` is the seam the tests fake.
- **No icons without a person.** `IconSearch::Off` (the default, and the headless backend in `main`) looks for and draws
  nothing, so no golden and no jail run changes. The test kit passes `Off`.
- **The plate.** An app with no icon gets a plain rounded plate in quire's raised ground (`overview_plate`), drawn by
  `casement_icons::plate`; no letter or glyph.
- **Not looked at.** An `Icon=` with an extension-less absolute path, `.xpm` icons and animated icons are skipped
  (the next place in the order is tried). `Icon=` keys with a locale are ignored.

Spots a real build checks first: `resvg::render(&tree, transform, &mut pixmap.as_mut())` and
`usvg::Tree::from_data(bytes, &usvg::Options::default())` (resvg 0.48.1 with its default features off: no text, so no
fontdb needed); `tree.size().width()`; `Pixmap::take()` (premultiplied RGBA); `image::DynamicImage::resize` with
`FilterType::Lanczos3`; `freedesktop_icons::lookup(name).with_size(u16).with_theme(&str).find()`; `ArgbImage::from_shm`
for the toplevel's own buffers (`casement_render::pass`); the borrow split in `icons_of` (`icons` and `textures` out of
`state.render` while `state.core` is read); `scripts/check-boundary.sh` rows B4/B15 (resvg without `text` must not pull
parley or fontique into the graph).

## casement-scene-css: the exclusions (P3-CSS)

`CssScene` (a blitz-dom document styled by `casement-scene-css/src/scene.css`) answers the layout seam for everything
`RustScene` lays out today except the rows below. `casement-scene-css/tests/parity.rs` compares the two exactly over 500
seeded specs and names each exclusion as a constant; none is hidden by loosening the comparison. `dev/scene-css.sh` runs it.

| Exclusion (test constant) | Why CSS cannot say it | What the test does instead |
| --- | --- | --- |
| Mission Control slots, icon rects and `LayoutReport::SpreadOverflow` (`SPREAD`) | the spread is an iterative push-apart (24 passes per trial, 0.85 scale steps, shelf-pack fallback); CSS has no loop or search | both sides drop `overview` and `SpreadOverflow`; the RustScene slots must be exactly the shown desktop Space's windows that have a target. `CssScene` returns no slots, so S-L6 does not cover the spread or `icon_rect`; its proptests in `casement-layout` do |
| the cascade's cycle length (`CASCADE_CYCLE`), and a step that is not one positive length on both axes (`CASCADE_STEP`) | the CSS form, `mod(nth * step, M + step)` with `M` the tighter axis's whole steps via `round(down, room, step)`, laid out as NaN/zero boxes in blitz at the first run (spec 0, new windows 6 and 7); the cause was not found without a compiler. `document/cascade.rs` now counts the turn in Rust (a copy of `RustScene`'s fit and cycle arithmetic) and `scene.css` does the size, the centre and the walk (`left = centre + turn * step`). Zero and negative steps walk the other way and have no form | the parity check covers a new window's size, centre and plumbing, not the cycle length; specs use equal steps in 1..=80 |
| `WindowPlacement::Tiled` (`TILED`) | not CSS: RustScene gives a tiled window no target (tile zones arrive with P3-Z) | not generated: a `TileZone` is a `casement-protocols` type this crate does not depend on |
| App Exposé and Show Desktop (`APP_EXPOSE`, `SHOW_DESKTOP`) | RustScene arranges them as plain desktops today (P3-Z adds them) | generated and compared as desktops; when P3-Z lands them the CSS and the constants change together |
| `Keep`/`Resize` and `MinUnmet` | they compare a size with the one asked for and with the minimum: not geometry | derived in `read.rs` from the laid-out boxes and the spec, with RustScene's own definitions (a full-screen or Split View window always `Resize`; a full-screen one never `MinUnmet`) |
| `u32`/`i32` saturation and coordinates beyond f32's 2^24 | CSS lengths are f32 | specs stay within about +-6000 logical pixels |

Also stated, not excluded: Space origins carry `--apart` (the row distance from the shown Space) from `document.rs`, and the
cascade's `--nth` is the window's place among the Space's new windows; both are ordinals of the spec, not layout.

Unverified (the lane could not compile or run anything; every blitz-dom name was read in the 063ff2b checkout, every CSS
behaviour below was read in stylo 0.21 or assumed):

- `casement-scene-css/Cargo.toml` needs `porter-core` and `ds-core` (the test builds `SpaceId`, `ClaimedId`, `Scale`; casement-core
  re-exports neither), as path deps the way `casement-layout` has them, and nothing else. Until then `cargo test` fails to compile
  the test; the library builds without them.
- `round(down, a, b)`, `mod()`, `sign()`, `clamp()` and nested `min()`/`max()` inside custom properties: stylo 0.21 parses
  `Round`, `Mod`, `Rem`, `Sign` in `values/specified/calc.rs` with no pref gate, but whether blitz's taffy conversion keeps a
  pure-length `calc` as a plain length for `left`/`width` is assumed. If a function is refused, the box falls back to `auto` and the
  first parity failure shows it.
- `DocumentConfig::default()` has no viewport: the boxes are absolute and sized in pixels, so none depends on it.
- Positions are read with `Node::absolute_position(0.0, 0.0)` (the sum of `final_layout().location` up the layout parents) and
  sizes from `final_layout().size`; `absolute_position` rounds nothing, `final_layout` is blitz's rounded layout.
