# Findings

Open items and standing facts. An entry names the condition that closes it.

## Stubs behind frozen interfaces

Every `todo!()` in the workspace's sources (27 from P0-Z1 in the pure crates, 270 from P0-Z2
below: 297), its fill lane and the ignored test that proves it. Un-ignoring the test is part
of the fill.

### P0-Z1: the pure crates

| File | Item | Lane | Proven by |
| --- | --- | --- | --- |
| `crates/casement-core/src/app/key.rs` | `AppKey::of` | P0-F1-window | `tests/window.rs` `an_app_key_is_proven_only_by_its_peer` |
| `crates/casement-core/src/space/mint.rs` | `mint_space_id` | P0-F1-window | `tests/window.rs` `minted_space_ids_are_porter_ids` |
| `crates/casement-core/src/space/mint.rs` | `SpaceSeed::next` | P0-F1-window | `tests/window.rs` `a_seed_mints_a_fixed_sequence_of_distinct_ids` |
| `crates/casement-core/src/space/book.rs` | `SpaceBook::shown_on`, `row`, `get` | P0-F1-window | `space::book::tests::queries_answer_from_the_book` |
| `crates/casement-core/src/layer/book.rs` | `LayerBook::usable_area` | P0-F1-output | `layer::book::tests::exclusive_zones_shrink_the_usable_area` |
| `crates/casement-core/src/output/scale_policy.rs` | `first_sight_scale` | P0-F1-output | `tests/output.rs` `first_sight_scales` |
| `crates/casement-core/src/output/mode_pick.rs` | `pick_mode` | P0-F1-output | `tests/output.rs` `picking_modes` |
| `crates/casement-core/src/output/arrangement.rs` | `place_outputs` | P0-F1-output | `tests/output.rs` `placing_outputs` |
| `crates/casement-core/src/input/reach.rs` | `reach` | P0-F1-input | `tests/input.rs` `the_reach_table` |
| `crates/casement-core/src/input/chord.rs` | `Chord::parse`, `Chord::text` | P0-F1-input | `tests/input.rs` `chords_parse_in_any_order_and_print_in_the_macs`; `tests/round_trip.rs` `chords_and_bindings_store_as_text` |
| `crates/casement-core/src/input/binding.rs` | `resolve` | P0-F1-input | `tests/input.rs` `agent_remote_and_feed_never_fire_a_binding` |
| `crates/casement-core/src/policy/globals.rs` | `advertised` | P0-F1-input | `tests/input.rs` `the_i7_table` |
| `crates/casement-core/src/focus/step.rs` | `focus::step` | P0-F1-focus | `tests/focus.rs` `the_focus_table` |
| `crates/casement-core/src/focus/press.rs` | `route_press` | P0-F1-focus | `tests/focus.rs` `press_routing` |
| `crates/casement-core/src/model.rs` | `Model::step`, `Model::check_invariants` | P0-F1b-model | `tests/model.rs` (six rows and the T0 proptest `random_sequences_keep_every_invariant`) |
| `crates/casement-layout/src/knobs.rs` | `LayoutKnobs::of` | P0-F1-layout | `tests/knobs.rs` `tile_margins_follow_their_setting` |
| `crates/casement-layout/src/of_model.rs` | `spec_of` | P0-F1-layout (proven after P0-F1b-model) | `tests/of_model.rs` `one_output_one_space_one_window` (ignored as P0-F1b-model: it needs `Model::step`) |
| `crates/casement-layout/src/rust_scene/mod.rs` | `RustScene::arrange` | P0-F1-layout | `tests/rust_scene.rs` (cascade, floating and Space-origin tables; the proptest `every_window_fits_and_layout_is_deterministic`) |
| `crates/casement-layout/src/rust_scene/floating.rs` | `floating_target` | P0-F1-layout | as `RustScene::arrange` |
| `crates/casement-layout/src/rust_scene/cascade.rs` | `cascade_rect` | P0-F1-layout | as `RustScene::arrange` |
| `crates/casement-motion/src/book.rs` | `MotionBook::is_moving` | P0-F1-motion | `tests/motion.rs` |
| `crates/casement-motion/src/retarget.rs` | `retarget` | P0-F1-motion | `tests/motion.rs` (`a_retarget_keeps_position_and_velocity` and five more) |
| `crates/casement-motion/src/sample.rs` | `sample` | P0-F1-motion | `tests/motion.rs` |

Ignored tests by lane: P0-F1-window 4, P0-F1-output 4, P0-F1-input 5, P0-F1-focus 2,
P0-F1b-model 8, P0-F1-layout 5, P0-F1-motion 6, P3-Z 1 (`crates/casement-layout/tests/spread.rs`,
the Mission Control spread's properties from SP8, for the P3 freeze to inherit).

P0-F1-ids has no stub: ids, geometry and time are implemented at the freeze, with their
tables (the clock's ds-core conformance cases, stop order, chunk invariance, ten minutes
without drift, `buffer_size` at 1, 1.25, 1.5 and 2).

### P0-Z2: the smithay-facing crates, the root, the test kit

`todo!()` by lane (270): P0-R-scene 23, P0-H1 18, P0-H2 51, P0-H3 15, P0-H4 11, P0-H5 6, P0-H6 3, P0-H7 12, P0-H8 12, P0-B-headless 79, P0-B-loop 14, P0-F1-output 3, P0-T-client 6, P0-T-dev 11, P1-D1 1, P1-F2 2, P2-Q2 2, P3-Z 1. Seventy-six more sit in ignored test bodies (the protocol
tests' client steps, the scene builder, the fuzz box), each naming the same lane: the fixture
setup and the assertion are written, the client steps wait for the helper the lane writes in
`casement-testclient`. The excluded crates add 20 (`wlcs-casement`: 16 for P0-T-wlcs in
`src/lib.rs`, `src/adapter.rs` and `src/ffi.rs`, checked to compile; `casement-scene-css`: 4 for
P3-CSS, three in `src/` and its ignored parity test, never built at P0-Z2).

| File | Items | Lane | Proven by |
| --- | --- | --- | --- |
| `crates/casement-render/src/damage.rs` | render_output (1) | P0-R-scene | `crates/casement-testkit/tests/scene.rs` (ten base scenes, blur v0); `crates/casement-render/tests/render.rs` (lock cover, damage oracle) |
| `crates/casement-render/src/element/blur_v0.rs` | Element::id, Element::current_commit, Element::src, Element::geometry, Element::damage_since, Element::opaque_regions, Element::is_framebuffer_effect, RenderElement::draw, RenderElement::capture_framebuffer (9) | P0-R-scene | `crates/casement-testkit/tests/scene.rs` (ten base scenes, blur v0); `crates/casement-render/tests/render.rs` (lock cover, damage oracle) |
| `crates/casement-render/src/element/cursor.rs` | cursor_elements (1) | P0-R-scene | `crates/casement-testkit/tests/scene.rs` (ten base scenes, blur v0); `crates/casement-render/tests/render.rs` (lock cover, damage oracle) |
| `crates/casement-render/src/element/layer.rs` | layer_elements (1) | P0-R-scene | `crates/casement-testkit/tests/scene.rs` (ten base scenes, blur v0); `crates/casement-render/tests/render.rs` (lock cover, damage oracle) |
| `crates/casement-render/src/element/lock_cover.rs` | Element::id, Element::current_commit, Element::src, Element::geometry, Element::damage_since, Element::opaque_regions, RenderElement::draw (7) | P0-R-scene | `crates/casement-testkit/tests/scene.rs` (ten base scenes, blur v0); `crates/casement-render/tests/render.rs` (lock cover, damage oracle) |
| `crates/casement-render/src/element/popup.rs` | popup_elements (1) | P0-R-scene | `crates/casement-testkit/tests/scene.rs` (ten base scenes, blur v0); `crates/casement-render/tests/render.rs` (lock cover, damage oracle) |
| `crates/casement-render/src/element/solid.rs` | solid_rect (1) | P0-R-scene | `crates/casement-testkit/tests/scene.rs` (ten base scenes, blur v0); `crates/casement-render/tests/render.rs` (lock cover, damage oracle) |
| `crates/casement-render/src/element/window.rs` | window_elements (1) | P0-R-scene | `crates/casement-testkit/tests/scene.rs` (ten base scenes, blur v0); `crates/casement-render/tests/render.rs` (lock cover, damage oracle) |
| `crates/casement-render/src/scene.rs` | output_elements (1) | P0-R-scene | `crates/casement-testkit/tests/scene.rs` (ten base scenes, blur v0); `crates/casement-render/tests/render.rs` (lock cover, damage oracle) |
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
| `crates/casement/src/config/convert.rs` | to_core (1) | P0-F1-output | `crates/casement/tests/config.rs` |
| `crates/casement/src/config/doc.rs` | load (1) | P0-F1-output | `crates/casement/tests/config.rs` |
| `crates/casement/src/config/schema.rs` | write_schema (1) | P0-F1-output | `crates/casement/tests/config.rs` |
| `crates/casement-testclient/src/bin/probe.rs` | main (1) | P0-T-client | `crates/casement-testclient/tests/infra.rs`; `crates/casement-testkit/tests/globals.rs`; `crates/casement-testkit/tests/fuzz/main.rs` |
| `crates/casement-testclient/src/conn.rs` | TestClient::over, TestClient::connect_env, TestClient::pump (3) | P0-T-client | `crates/casement-testclient/tests/infra.rs`; `crates/casement-testkit/tests/globals.rs`; `crates/casement-testkit/tests/fuzz/main.rs` |
| `crates/casement-testclient/src/shm.rs` | shm_buffer (1) | P0-T-client | `crates/casement-testclient/tests/infra.rs`; `crates/casement-testkit/tests/globals.rs`; `crates/casement-testkit/tests/fuzz/main.rs` |
| `crates/casement-testkit/src/fuzz/ops.rs` | run_ops (1) | P0-T-client | `crates/casement-testclient/tests/infra.rs`; `crates/casement-testkit/tests/globals.rs`; `crates/casement-testkit/tests/fuzz/main.rs` |
| `crates/casement-ipc/src/dbus/debug1.rs` | Debug1::inject, Debug1::wait, Debug1::clock, Debug1::output, Debug1::windows, Debug1::scene, Debug1::fault (7) | P0-T-dev | `crates/casement-ipc/src/inject.rs`, `wait.rs` tests; `crates/casementctl/src/command.rs` test; `dev/accept.sh --jobs 8` |
| `crates/casement-ipc/src/inject.rs` | parse_inject (1) | P0-T-dev | `crates/casement-ipc/src/inject.rs`, `wait.rs` tests; `crates/casementctl/src/command.rs` test; `dev/accept.sh --jobs 8` |
| `crates/casement-ipc/src/wait.rs` | parse_wait (1) | P0-T-dev | `crates/casement-ipc/src/inject.rs`, `wait.rs` tests; `crates/casementctl/src/command.rs` test; `dev/accept.sh --jobs 8` |
| `crates/casementctl/src/command.rs` | parse, run (2) | P0-T-dev | `crates/casement-ipc/src/inject.rs`, `wait.rs` tests; `crates/casementctl/src/command.rs` test; `dev/accept.sh --jobs 8` |
| `crates/casement-testkit/src/fixture.rs` | Fixture::session (1) | P1-D1 | P1-Z's session suite |
| `crates/casement-testkit/src/bin/fake_logind.rs` | main (1) | P1-F2 | P1-Z's logind tests |
| `crates/casement-testkit/src/fakes/logind.rs` | FakeLogind::serve (1) | P1-F2 | P1-Z's logind tests |
| `crates/casement-agent/src/socket.rs` | hand_out, role_for (2) | P2-Q2 | `crates/casement-agent/src/socket.rs` `only_the_shell_and_the_agent_tier_get_privileged_roles` |
| `crates/casement-testkit/src/fixture.rs` | Fixture::palmrest (1) | P3-Z | P3-Z's palmrest tests |

Ignored tests by lane (P0-Z2): P0-H1 17, P0-H2 16, P0-H3 7, P0-H4 10, P0-H5 7, P0-H6 3,
P0-H7 5, P0-H8 7, P0-B-headless 7, P0-B-loop 4, P0-R-scene 4, P0-T-client 5, P0-T-dev 6,
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
| `CasementSettings::defaults` carries no bindings (C30 says the settings document holds the defaults) | P0-F1-input or P1-Z writes the default table (Space steps, placement chords, hide and quit) into `settings/bindings.rs` |
| The smithay fork `PoHsuanLai/smithay` (branch `casement`, C2) does not exist; the pin names upstream Smithay at the same rev `118e34ff` | the first casement patch to smithay creates the fork and moves the pin to it |
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
| `MotionBook`'s `Motion` enum is never built | P0-F1-motion's `retarget` builds it; its `expect(dead_code)` is removed then |
| `rust_scene/floating.rs` and `cascade.rs` helpers are not called | P0-F1-layout's `RustScene::arrange` calls them; their `expect(dead_code)` is removed |
| Motion goldens (S-L8) are not written: their values come from the fill | P0-F1-motion writes and blesses them in `FrameScene::text` form (SP8's `mc_entry` scene is the model) |
| Mission Control spread quality: scale about 0.30 at 20 windows, 0.19 to 0.22 at 30 to 50 (A21) | the user decides at P3 whether to enable refinement trials or a better packer |
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
