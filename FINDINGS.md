# Findings

Open items and standing facts. An entry names the condition that closes it.

## Stubs behind frozen interfaces

Every `todo!()` in the workspace (27), its fill lane and the ignored test that proves it.
Un-ignoring the test is part of the fill.

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

## Open items

| Item | Closes when |
| --- | --- |
| `taffy` (git rev `4863877b`, A14/A24) is declared here outside the block copied from quire's `docs/workspace-deps.toml` | quire's pinned block carries the line and this workspace copies it verbatim |
| ds-core's `Scale` has no serde, so stored types write it through `geometry::scale_serde` | ds-core derives `Serialize, Deserialize` (transparent) on `Scale` (interface ask to quire); the helper module is deleted |
| ds-style without dioxus still pulls `image` (png, jpeg, moxcms) into casement-core's graph (A22, SP8) | quire makes `image` an optional ds-style feature |
| ds-behaviour is a dependency of casement-core with no caller yet | the switcher, hot-corner and Space-swipe machines run in casement (C41, P3-Z), fed through `Mono::stamp`, with their `*Params` in `CasementSettings` |
| `CasementSettings::defaults` carries no bindings (C30 says the settings document holds the defaults) | P0-F1-input or P1-Z writes the default table (Space steps, placement chords, hide and quit) into `settings/bindings.rs` |
| `GlobalKind` lists the P0 globals from the casement-wayland module list | P0-Z2 checks it against its delegates; each delta freeze adds its globals and the I7 rows |
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
