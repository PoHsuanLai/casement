# Architecture

casement is the desktop's Wayland compositor: smithay underneath, the Mac's window model on
top (03), agent-native from P4, testable completely headless. `CONVENTIONS.md` holds the rules,
`FINDINGS.md` the open items and every frozen `todo!()`, `QUIRKS.md` the hardware facts. The
plan this code freezes is `~/rs-wt/compositor-scope/20-FREEZE-PLAN.md` with
`21-FREEZE-AMENDMENTS.md` over it.

Reading order: section 1 (find the crate), section 2 (find the home), section 3 (find the
trait), section 4 (copy the recipe).

## 1. Crates and allowed edges

The workspace holds the pure crates today (P0-Z1). The smithay-facing crates, the composition
root, the test kit and the binaries join at P0-Z2 (casement-render, casement-wayland,
casement-backend, casement-agent, casement-ipc, casement, casementctl, casement-testclient,
casement-testkit; casement-intents at P4-Z, casement-chrome at P3-Z). wlcs-casement and
casement-scene-css are never members.

| Crate | Purpose | I/O |
| --- | --- | --- |
| `casement-core` | the pure model: ids, geometry, time data (`Mono`, `VblankClock`, `TimerBook`, `VirtualClock`), apps, windows, Spaces, layers, the lock, focus and press routing, input origins, reach and bindings, output facts and policy, connection policy (I7), settings, `Event`, `Effect`, `Model::step`, invariants | none |
| `casement-layout` | the `SceneLayout` seam: `SceneSpec`, `Arrangement`, `RustScene`, `spec_of(&Model)`; taffy only in `crates/casement-layout/src/taffy_bridge/` | none |
| `casement-motion` | `MotionBook` of ds-motion spring legs, `retarget`, `sample` into an `Eq` `FrameScene` in 1/120 px | none |

Layers, bottom up: `casement-core` ← `casement-layout` ← `casement-motion` (C24: layout needs
core's ids and motion consumes layout's `Arrangement`, so motion cannot live in core).

Allowed edges (checked by `scripts/check-boundary.sh`, rows B1 to B20 of the plan's §2.4):

| Crate | May reach |
| --- | --- |
| `casement-core` | ours: casement-protocols (default features: `vocab` only), ds-core, ds-style and ds-motion (default features off: no dioxus), ds-behaviour, keycap-core, palmrest-protocol, porter-core, prov; nothing of smithay, wayland, calloop, zbus, tokio, drm, gbm, input, udev, libseat, taffy, dioxus, Blitz, Stylo, ds, ds-shell, ds-blitz, ds-settings, parley, vello (B1) |
| `casement-layout` | core's list plus taffy, named only in `taffy_bridge/` (B2) |
| `casement-motion` | core's list plus casement-layout; its source never names taffy (B3) |

Every crate: no agent-tier or shell crate (B12), no ds-settings outside casement-ipc and
casement (B13), no text stack outside casement-chrome (B15), no `HashMap`/`HashSet` in layout
and motion (B17), no `unsafe` (B18), no clock outside the composition root's time module (B8).
The script carries the P0-Z2 rows already; each passes while its crate is absent and bites when
it lands.

## 2. One home per concept

| Concept | Home |
| --- | --- |
| a window, layer, lease or device id; an output's connector name and stable key | `casement-core::ids` (crate root re-exports) |
| logical, physical and sub-pixel geometry; the one logical-to-physical conversion | `casement-core::geometry` |
| the scale type | ds-core's `Scale` (120ths, C26); its stored form is `geometry::scale_serde` |
| the session instant, refresh rates, vblank instants, timers, the virtual clock | `casement-core::time` |
| conversion to ds-core's millisecond `Stamp` (for ds-behaviour machines) | `Mono::stamp` only (C56) |
| an app's identity (proven or claimed) | `casement-core::app::AppKey` over porter-core's `identity_of` (C39) |
| a window's state, limits (C51), hints | `casement-core::window` |
| Spaces and their ids (casement mints, C16) | `casement-core::space` (`SpaceSeed`, `mint_space_id`) |
| the usable area after layer exclusive zones | `LayerBook::usable_area` |
| focus, key and main windows, press routing | `casement-core::focus` (03 §1.2) |
| where input came from, and what it may reach | `casement-core::input::{Origin, reach}` (C19, I3, I8) |
| key bindings and their resolution | `casement-core::input::{Bindings, resolve}`; defaults in `settings` (C30) |
| first-sight scale, mode choice, output placement | `casement-core::output` |
| which connection sees which global (I7) | `casement-core::policy::advertised` |
| typed settings | `casement-core::settings::CasementSettings` (the stored document: casement's config module, P0-Z2) |
| the closed sets of inputs and outputs of the model | `casement-core::{Event, Effect}` |
| where windows go | `casement-layout` (`SceneLayout`, `RustScene`) |
| how they travel there; the drawn frame | `casement-motion` (`retarget`, `sample`, `FrameScene`) |
| the one float-to-unit step of motion | `casement_motion::quantise` |
| wire enums and codes | casement-protocols `vocab` (never redefined here) |
| springs, velocity, motion level | ds-motion and ds-style |

## 3. Canonical traits

```rust
// casement-layout: two or more implementations (RustScene; FixedScene in tests;
// CssScene, the excluded oracle).
pub trait SceneLayout {
    fn arrange(&self, spec: &SceneSpec, knobs: &LayoutKnobs) -> Arrangement;
}

// casement-protocols: every wire enum.
pub trait WireCode: Sized + Copy + 'static {
    const ALL: &'static [Self];
    fn code(self) -> u32;
    fn from_code(code: u32) -> Result<Self, CodeError>;
}
```

P0-Z2 adds `Compositor` (the renderer seam) and `VirtualInput: InputBackend`; P1-Z adds `Kms`,
`ScanoutSurface`, `InputDevices`, `RenderNodes`.

## 4. Recipes

**Add a core event.**
1. Add the variant to `Event` in `crates/casement-core/src/event.rs`, its data types beside the
   concept they belong to.
2. Route it in `Model::step` (`crates/casement-core/src/model.rs`) to the machine that owns it.
3. Add a row to `crates/casement-core/tests/model.rs`; `check_invariants` runs after every row.
4. The edge that produces it (a handler, the backend, the clock) converts into it there.

**Add an effect.**
1. Add the variant to `Effect` in `crates/casement-core/src/effect.rs`.
2. Carry it out in the composition root's exec module (the one place effects happen).
3. A model row asserts the step that emits it.

**Add a settings key.**
1. Add the typed field to the section in `crates/casement-core/src/settings/` with its default
   in that section's `defaults()` (03 §1.17's value where one exists).
2. Add the stored key to the ds-settings document and its conversion (casement's config module).
3. Extend `crates/casement-core/src/settings/tests.rs`.

**Add a layout strategy.**
1. One file under `crates/casement-layout/src/rust_scene/` per placement (P3-Z: tile_zone, fill,
   full_screen, split, spread, expose, space_row, show_desktop).
2. If it is flex or grid, build it through `taffy_bridge` (new tree without Taffy's rounding,
   `solve`, `rect_of`); never name taffy elsewhere.
3. Table rows in `crates/casement-layout/tests/rust_scene.rs`; anything it cannot honour goes to
   `Arrangement::reports`.

**Add a motion quantity.**
1. A key in `MotionKey`, its legs beside `window_motion.rs` / `space_slide.rs`.
2. `retarget` starts each new leg from the old leg's state; `sample` quantises through
   `quantise`.
3. A golden in `FrameScene::text` form, blessed once with a reason.

**Freeze a new item.** Signature and doc, a `todo!()` body naming the lane, a FINDINGS row, and
its behaviour test written and ignored with that lane's id.

## 5. The test harness

The pure crates test their functions directly: table tests per function, proptest for
properties (T0), integration tests under each crate's `tests/` with shared builders in
`tests/support/`. `FixedScene` (`crates/casement-layout/tests/support/fixed_scene.rs`) stands
in for any layout. Everything runs inside the jail (`~/desktop/harness/jail.sh`): built
outside, archived with nextest, run inside. From P0-Z2, protocol and scene tests drive the real
`State` through the test kit's `Fixture`, with the virtual clock as a calloop event source
(21-FREEZE-AMENDMENTS A1).

## 6. Repo rules

- The gate (until P0-Z2 brings its dev gate script): `~/rs-wt/gate-lane-jailed.sh <checkout>`, which
  runs `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features -- -D
  warnings`, the nextest archive inside the jail, doc tests, `./scripts/check-boundary.sh` and
  `cargo deny check licenses`. Every exit code is checked.
- `unsafe_code = "deny"` at the workspace. The one unsafe module is reserved for the renderer
  (a gl_raw module in casement-render, row B18) and does not exist until a reviewed interface
  ask shows a GL pass smithay's safe API cannot express; `EGLDisplay::new` and
  `GlesRenderer::new` will live there (A17).
- `missing_docs` warns at the workspace, so every public item says what it means.
- Sibling repos are path dependencies (`../quire`, `../porter`, `../casement-protocols`); a lane
  worktree links them beside itself.
- Nothing goes upstream to smithay (DECISIONS); patches live in our fork, pinned by rev (P0-Z2).
