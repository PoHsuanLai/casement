# Architecture

casement is the desktop's Wayland compositor: smithay underneath, the Mac's window model on
top (03), agent-native from P4, testable completely headless. `CONVENTIONS.md` holds the rules,
`FINDINGS.md` the open items and every frozen `todo!()`, `QUIRKS.md` the hardware facts. The
plan this code freezes is `~/rs-wt/compositor-scope/20-FREEZE-PLAN.md` with
`21-FREEZE-AMENDMENTS.md` over it.

Reading order: section 1 (find the crate), section 2 (find the home), section 3 (find the
trait), section 4 (copy the recipe).

## 1. Crates and allowed edges

The workspace holds the pure crates (P0-Z1), the smithay-facing crates, the composition root,
the binaries and the test kit (P0-Z2). casement-intents joins at P4-Z, casement-chrome at P3-Z.
`wlcs-casement` (the wlcs shim, MIT, its own one unsafe module `src/ffi.rs`) and
`casement-scene-css` (the CSS layout oracle, P3) are never members: each has its own
`[workspace]` table and the root lists them under `exclude` (B16).

| Crate | Purpose | I/O |
| --- | --- | --- |
| `casement-core` | the pure model: ids, geometry, time data (`Mono`, `VblankClock`, `TimerBook`, `VirtualClock`), apps, windows, Spaces, layers, the lock, focus and press routing, input origins, reach and bindings, output facts and policy, connection policy (I7), settings, `Event`, `Effect`, `Model::step`, invariants; from P1-Z the GPU plan (`gpu`), the frame scheduler's maths (`frame`) and the Space ids file's codec (`space::SpaceFile`) | none |
| `casement-layout` | the `SceneLayout` seam: `SceneSpec`, `Arrangement`, `RustScene`, `spec_of(&Model)`; taffy only in `crates/casement-layout/src/taffy_bridge/` | none |
| `casement-motion` | `MotionBook` of ds-motion spring legs, `retarget`, `sample` into an `Eq` `FrameScene` in 1/120 px | none |
| `casement-render` | the `Compositor` renderer trait, `CasementElement`, `output_elements` (FrameScene to elements), `render_output` (damage-tracked), blur v0, the lock cover, output read-back (`pass::capture_output`, P1-Z), `golden` (feature `testing`); the workspace's one unsafe module `src/gl_raw.rs` (A17) | the GPU through smithay |
| `casement-wayland` | one module per protocol: a `<Protocol>Part` (smithay's state plus our BTreeMap tables), `Part::new` behind the I7 filter, `on_*` (request to `Event`s), `apply_*` (effect to wire); `WaylandStates`, `GlobalFilter`, `ClientState`, `convert` | the wire |
| `casement-backend` | the headless backend (virtual outputs, vblank source, render-cost model, `VirtualInput: InputBackend`), winit, the session seam, the soft-GPU pieces (`SoftPixman`, `FixedUdmabuf`, A4); from P1-Z the tty seams (`tty::{Kms, KmsOpener, ScanoutSurface, TtySession, DeviceSource}`, `input::InputDevices`, `gpu::{RenderNodes, FormatSource}`), `TtyBackend` over them, and their thin real adapters (`*_real.rs`) compiled only under the features `tty` and `libinput` (off by default; A2) | devices under the features, none headless |
| `casement-agent` | the privileged-connection hand-out (P2), agent seat, EIS, masked capture, trusted surface, kill (P4); a skeleton at P0 | sockets |
| `casement-ipc` | the inject and wait grammars (its `casement-backend` dependency is optional, on with `debug-inject`; `serve_debug_at(address)` takes the bus address, the library reads no environment); `org.quire.Casement.Debug1` (feature `debug-inject`), `org.quire.Compositor1` (P2-Z); `shell_calls::{ShellCalls, ZbusShellCalls}`, the one place a binding's D-Bus call is made (A4); `IpcHandles` | D-Bus |
| `casement` | lib + bin: `State` (frozen), `delegates.rs` (frozen), `handlers/<protocol>.rs`, `core_loop`, `exec`, `clients`, `display`, `time/` (the only clock), `config/` (ds-settings), `cli`, `run` (`casement::run(RunConfig) -> Result<(), RunError>`: the startup and the event loop; `main.rs` is only arguments, `$CASEMENT_LOG`, `$DBUS_SESSION_BUS_ADDRESS` and the exit code), `debug` (Debug1 in the loop, feature `debug-inject`), `jail_check` (features `testing`, `debug-inject`), `log` (takes its filter as an argument); from P1-Z `seams` (the injected `SpaceStore`, `ShellCalls` and `Spawner`), `space_store`, `spawn`, `xwayland`; its features `tty` and `libinput` (default on) turn the backend's real adapters on | everything, through the crates above |
| `casementctl` | lib + bin: `debug` (Debug1), `shot` (P1), `report` (P5) | D-Bus, the wire |
| `casement-testclient` | typed wayland-client helpers per protocol, `Transcript`, `Pattern`, shm buffers; bin `casement-probe`; no smithay (B9) | the wire |
| `casement-testkit` | `Fixture` over the real `State`, `FixtureSpec`, `TestOrigin`, `FixtureInput`, fakes (session, udev, logind, palmrest, KMS, scanout, libinput settings, the Spaces file, sill's D-Bus methods, the process table), the soft GPU topology and the jail's fabricated nodes and sys tree, `Env` (goldens), typed fuzz ops; bin `fake-logind` | the jail's scratch only |

Layers, bottom up: `casement-core` ← `casement-layout` ← `casement-motion` (C24) ←
`casement-render`; `casement-core` ← `casement-wayland`; `casement-core` + `casement-render` ←
`casement-backend`; all of them ← `casement` (the composition root, C28: handlers live there
because smithay's traits and `State` meet only there) ← `casement-testkit`. `casement-testclient`
sits beside the stack on the client side.

Allowed edges (checked by `scripts/check-boundary.sh`, rows B1 to B20 of the plan's §2.4, plus
A1 and A4):

| Crate | May reach |
| --- | --- |
| `casement-core` | ours: casement-protocols (default features: `vocab` only), ds-core, ds-style and ds-motion (default features off: no dioxus), ds-behaviour, keycap-core, palmrest-protocol, porter-core, prov; nothing of smithay, wayland, calloop, zbus, tokio, drm, gbm, input, udev, libseat, taffy, dioxus, Blitz, Stylo, ds, ds-shell, ds-blitz, ds-settings, parley, vello (B1) |
| `casement-layout` | core's list plus taffy, named only in `taffy_bridge/` (B2) |
| `casement-motion` | core's list plus casement-layout; its source never names taffy (B3) |
| `casement-render` | smithay's renderers; names no `smithay::backend::{drm, udev, session, libinput}`; reaches no wgpu, vello, anyrender, Blitz, Stylo, parley, fontique (B4) |
| `casement-wayland` | smithay's wayland frontend and desktop; names no device backend, reaches neither drm nor gbm (B6) |
| `casement-backend` | smithay's renderers, EGL, session and winit; names no `casement_wayland` and no `smithay::wayland` module but dmabuf (B5); only its thin adapters name the device crates (B7), and they compile only under the features `tty` and `libinput` (A2) |
| `casement-testclient` | wayland-client and the protocol crates; no smithay (B9) |
| `casement-ipc`, `casement` | ds-settings (B13; the test kit reaches it only through `casement` and names none of it) |

Every crate: no agent-tier or shell crate (B12), no text stack outside casement-chrome (B15), no
`HashMap`/`HashSet` in wayland, render, layout and motion (B17), no `unsafe` outside
`crates/casement-render/src/gl_raw.rs` (B18), no clock outside `crates/casement/src/time/` (B8,
A1: `Instant::now`, `SystemTime::now`, smithay's `Clock::now` and calloop's `Timer`), no
`UdmabufAllocator::create_buffer` (A4), and the release binary enables neither `testing` nor
`debug-inject` (B10). Comment lines are not code to these rows; B20 checks comments too.

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
| key bindings and their resolution | `casement-core::input::{Bindings, resolve}`; the shell's own chords come from the desktop keymap, `casement::keymap` (chordkit layers through keycap-client's `KeycapSource`), behind casement's `[bindings]` entries |
| first-sight scale, mode choice, output placement | `casement-core::output` |
| which connection sees which global (I7) | `casement-core::policy::advertised` |
| typed settings | `casement-core::settings::CasementSettings` (the stored document: `casement::config`) |
| the closed sets of inputs and outputs of the model | `casement-core::{Event, Effect}` |
| where windows go | `casement-layout` (`SceneLayout`, `RustScene`) |
| how they travel there; the drawn frame | `casement-motion` (`retarget`, `sample`, `FrameScene`) |
| the one float-to-unit step of motion | `casement_motion::quantise` |
| wire enums and codes | casement-protocols `vocab` (never redefined here) |
| springs, velocity, motion level | ds-motion and ds-style |
| a renderer and what it can do | `casement-render::Compositor`, `EffectSupport` |
| colours handed to a renderer (whole steps of 1/255, A18) | `casement-render::Rgba8` |
| the one `unsafe` (EGL display, GLES renderer: A17) | `casement-render/src/gl_raw.rs` |
| golden comparison and its tiers | `casement-render::golden` (feature `testing`); `CASEMENT_BLESS` read only by `casement-testkit::Env` |
| a protocol's state and its request and effect functions | `casement-wayland::<protocol>` |
| which globals a client sees (the filter that wraps I7) | `casement-wayland::GlobalFilter` |
| a client's key, role and identity | `casement-wayland::ClientState` (inserted by `casement::clients::insert_client`) |
| smithay's types becoming ours | `casement-wayland::convert` and the handlers, never deeper |
| which serials answer a press | `casement-wayland::serial::SerialBook` |
| a virtual output's spec and its text grammar (C27) | `casement-backend::VirtualOutputSpec` |
| a connector appearing, changing or going | `casement-backend::ConnectorEvent` (headless and KMS alike; `tty::hotplug::reconcile` makes them from a KMS device) |
| a KMS device's resources and how it is driven | `casement-backend::tty::{Kms, KmsResources, KmsCommit}` (P1-Z); its fake is `casement-testkit::fakes::FakeKms` |
| which GPU renders and which scans out; the dma-buf feedback tranches | `casement-core::gpu::{plan_for, feedback_for}` (pure); detection is `casement-backend::gpu::RenderNodes` over an injected `SysRoot` (A6) |
| when to render a frame; whether a frame made its vblank; the cursor plane choice | `casement-core::frame` (pure; P1-Z) |
| the Space ids file's bytes (parse, render, import of sill's `spaces.json`) | `casement-core::space::SpaceFile` (pure codec, A1) |
| reading and writing that file | `casement::space_store::SpaceStore` (injected; `StateDirs` from `main`; fake `MemorySpaceStore`) |
| a binding's D-Bus call | `casement-ipc::shell_calls::ShellCalls` (A4; fake `FakeShellCalls`); `casement::exec` names the call and never zbus |
| starting a program (a binding's, xwayland-satellite) | `casement::spawn::Spawner` (A5; fake `FakeSpawner`) |
| X11 clients | `casement::xwayland` (`XwaylandConfig`: mode and satellite path from configuration, never the environment) |
| libinput device settings and their fake | `casement-backend::input::{InputDevices, DeviceSettings}`; `casement-testkit::fakes::FakeInputDevices` |
| reading an output's frame back | `casement-render::pass::capture_output` (P1-Z) |
| virtual input devices and their events | `casement-backend::VirtualInput` |
| the soft GPU pieces (A4) | `casement-backend::gpu::{SoftPixman, FixedUdmabuf}` |
| the session clock and the virtual clock's event source (A1) | `casement::time::{ClockSource, SharedClock, VirtualTimerSource}` |
| carrying out an effect | `casement::exec::exec` (the one place) |
| the stored settings document and its schema | `casement::config` |
| the command line | `casement::cli` (`StartConfig`) |
| the jail check in Rust | `casement::jail_check` (features `testing`, `debug-inject`) |
| the inject and wait grammars | `casement-ipc::{parse_inject, parse_wait}` |
| a test's compositor, clients and clock | `casement-testkit::Fixture` |
| the origins a test may mint | `casement-testkit::TestOrigin` |
| a test client's received events on virtual time | `casement-testclient::Transcript` |
| the jail's fabricated GPU nodes (A3) | `casement-testkit::gpu::fabricated_nodes` |

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

```rust
// casement-render: GlesRenderer (llvmpipe in the jail, hardware from P1), PixmanRenderer, and
// smithay's DummyRenderer under `testing`. ExportMem is asked for where pixels are read back.
pub trait Compositor: Renderer + ImportAll + ImportMem + Sized {
    fn effects(&mut self) -> EffectSupport;   // Shaders | Plain (effects draw solid stand-ins)
}

// casement-backend: smithay's input backend over queued virtual devices, the path libinput
// takes. Devices are plugged and events pushed only under `testing` or `debug-inject`.
impl smithay::backend::input::InputBackend for VirtualInput { /* one event type per family */ }

// casement-wayland (smithay has no module for our protocols): the composition root's side of
// quire_window_v1, reached through our own GlobalDispatch2/Dispatch2 impls.
pub trait QuireWindowHandler: 'static {
    fn quire_window_part(&mut self) -> &mut QuireWindowPart;
    fn window_of_toplevel(&self, toplevel: &XdgToplevel) -> Option<WindowId>;
    fn quire_window_events(&mut self, events: Vec<Event>);
}
```

P1-Z adds these seams. Every one has a thin real implementation and a fake exported by
`casement-testkit`; the traits take `std::fmt::Debug` as a supertrait so a `Box<dyn …>` can sit in
a `#[derive(Debug)]` struct.

```rust
// casement-backend::tty (real: kms_real.rs, surface_real.rs, session_real.rs, udev_real.rs under
// feature `tty`; fakes: FakeKms, FakeScanout, FakeSession, FakeUdev)
pub trait Kms: Debug {
    fn resources(&self) -> KmsResources;
    fn test(&mut self, commit: &KmsCommit) -> CommitVerdict;         // TEST_ONLY
    fn commit(&mut self, commit: KmsCommit) -> Result<(), KmsError>;
    fn set_power(&mut self, connector: ConnectorId, power: OutputPower) -> Result<(), KmsError>;
    fn set_gamma(&mut self, connector: ConnectorId, ramp: &GammaRamp) -> Result<(), KmsError>;
    fn events(&mut self) -> Vec<ConnectorEvent>;
}
pub trait KmsOpener: Debug { fn open(&mut self, path: &DevicePath) -> Result<Box<dyn Kms>, KmsError>; }
pub trait ScanoutSurface: Debug {
    fn queue(&mut self, frame: QueuedFrame) -> QueueOutcome;
    fn presented(&mut self) -> Option<Presented>;
}
pub trait TtySession: Debug {
    fn open_device(&mut self, path: &DevicePath) -> Result<OwnedFd, SessionFault>;
    fn switch_vt(&mut self, vt: VtNumber) -> Result<(), SessionFault>;
    fn notes(&mut self) -> Vec<SessionNote>;
}
pub trait DeviceSource: Debug { fn present(&self) -> Vec<DevicePath>; fn next_event(&mut self) -> Option<DeviceEvent>; }

// casement-backend::input (real: libinput_real.rs under feature `libinput`; fake: FakeInputDevices)
pub trait InputDevices: Debug { fn apply(&mut self, device: DeviceId, settings: &DeviceSettings) -> Applied; }

// casement-backend::gpu (real: render_nodes_real.rs under feature `tty`, reading an injected SysRoot;
// fake: the same adapter over FabricatedSysRoot's tree, SoftGraphicsApi for the renderers)
pub trait RenderNodes: Debug {
    fn nodes(&self) -> Vec<GpuFacts>;
    fn open(&mut self, node: &DrmNodeId) -> Result<GpuHandle, GpuError>;
}

// casement-ipc (real: ZbusShellCalls; fake: FakeShellCalls)
pub trait ShellCalls: Debug {
    fn call(&mut self, call: &DbusCall) -> Result<(), ShellCallError>;
    fn failures(&mut self) -> Vec<FailedCall>;
}

// casement (real: FileSpaceStore, ProcessSpawner; fakes: MemorySpaceStore, FakeSpawner)
pub trait SpaceStore: Debug {
    fn read(&self) -> Result<Option<String>, SpaceStoreError>;
    fn write(&mut self, text: &str) -> Result<(), SpaceStoreError>;     // atomic
}
pub trait Spawner: Debug { fn spawn(&mut self, spec: SpawnSpec) -> Result<Spawned, SpawnError>; }
```

`casement::State::new` builds the real seams (`Seams::real`); `State::with_seams` takes injected
ones, which is how the test kit's `Fixture` gets its fakes.

## 4. Recipes

**Add a core event.**
1. Add the variant to `Event` in `crates/casement-core/src/event.rs`, its data types beside the
   concept they belong to.
2. Route it in `Model::step` (`crates/casement-core/src/model.rs`) to the machine that owns it.
3. Add a row to `crates/casement-core/tests/it/model.rs`; `check_invariants` runs after every row.
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
3. Table rows in `crates/casement-layout/tests/it/rust_scene.rs`; anything it cannot honour goes to
   `Arrangement::reports`.

**Add a motion quantity.**
1. A key in `MotionKey`, its legs beside `window_motion.rs` / `space_slide.rs`.
2. `retarget` starts each new leg from the old leg's state; `sample` quantises through
   `quantise`.
3. A golden in `FrameScene::text` form, blessed once with a reason.

**Add a protocol.**
1. A module `crates/casement-wayland/src/<p>.rs`: `<P>Part { pub state, ... BTreeMaps }`,
   `Part::new::<D>(display, role_filter)` with smithay's constructor bounds copied (filtered
   constructors take `role_filter.for_global(GlobalKind::<P>)`), `on_*` and `apply_*`.
2. Its field in `WaylandStates` (`states.rs`) and its `Part::new` in `casement/src/state.rs`
   (both frozen: a freeze or an interface ask).
3. `crates/casement/src/handlers/<p>.rs`: `impl <Smithay>Handler for State`, accessors returning
   `&mut self.wl.<p>.state`, callbacks converting and calling `on_*`, then `core_loop::feed`.
   `delegates.rs` needs nothing more: `delegate_dispatch2!` routes every global.
4. A `GlobalKind` variant and its I7 row (casement-core), a `tests/golden/globals.txt` row
   (`<interface> <version> <module>`), `crates/casement-testkit/tests/it/protocol/<p>.rs` with its
   `mod` line in `tests/it/protocol/mod.rs` (`scripts/check-protocol-coverage.sh`), a testclient
   helper module, and for a private request a fuzz op.

**Declare a protocol before its lane fills it (a delta freeze).** Everything in "Add a protocol",
except that `Part::new::<D>(display, role_filter)` creates no global and returns the empty part,
so no client sees the protocol and `globals.txt` stays as it is. The `GlobalKind` variant (and its
I7 row), the `WaylandStates` field, the `Part::new` call in `State::new`, the handler file, the
testkit protocol test (ignored, lane-named) and its `mod` line all exist; the lane creates the
global in `Part::new`, adds the `globals.txt` row and un-ignores the tests.

**Add a seam with its fake.**
1. The trait beside its types in the crate that drives it, with `Debug` as a supertrait, typed
   errors (thiserror, typed sources) and no `bool`s. Real I/O goes in a `*_real.rs` adapter, behind
   a feature when it needs system libraries (A2); a new adapter joins `ADAPTERS` in
   `scripts/check-boundary.sh`.
2. The fake in `crates/casement-testkit/src/fakes/` as a cloneable shared handle (`Arc<Mutex<…>>`):
   the compositor holds one clone, the test the other, and the test asserts on what the fake
   recorded. Export it from `fakes/mod.rs`.
3. Inject it: a field of `casement::seams::Seams` (or of `TtySeams` for the tty backend), a real
   default in `Seams::real`, the fake in `Fixture::start`.
4. A doc test on the trait's use, and the behaviour test through the fake.

**Add a real adapter under a feature.** The file is named `<thing>_real.rs`, is listed in the
boundary script's adapter list, is declared in its `mod.rs` under `#[cfg(feature = "tty")]` (or
`libinput`), and is the only place that names the device crates. Add no crate to a manifest when
smithay's own features bring it (`tty = ["smithay/backend_drm", …]`).

**Add a debug verb.**
1. A `DebugRequest` variant and its `#[zbus::interface]` method in
   `crates/casement-ipc/src/dbus/debug1.rs`; the event loop answers it with a `DebugReply`.
2. Its `casementctl debug` subcommand (`crates/casementctl/src/command.rs`) and its `dev/lib`
   shell function if scripts use it.

**Add a fake fixture.**
1. Recorded bytes under `crates/casement-testkit/fixtures/<kind>/<name>` (from the user's
   optional `dev/hw/smoke.sh` run only, never this machine by an agent).
2. Served by the fake that owns the kind (`fakes/session.rs` for device files; FakeKms at P1).
3. A QUIRKS row when the fixture records a behaviour fact.

**Add a golden.**
1. A scene built through the `Fixture` (or a renderer-only element test in casement-render),
   compared with `casement_render::golden::compare` under the right tier: `Exact` for pixman
   and aligned elements, `Soft { channel: 2, parts_per_million: 1000 }` for llvmpipe effects.
2. Colours as `Rgba8`, chrome rasterised at the output's native scale, never
   `TextureFilter::Nearest` (A18).
3. Bless once with `CASEMENT_BLESS=1` and a reason in the commit; `tests/golden/MANIFEST.toml`
   names the pixman, Mesa and LLVM versions it was blessed on.

**Freeze a new item.** Signature and doc, a `todo!()` body naming the lane, a FINDINGS row, and
its behaviour test written and ignored with that lane's id.

## 5. The test harness

The pure crates test their functions directly: table tests per function, proptest for
properties (T0), integration tests under each crate's `tests/it/` (one binary per crate: a `mod` per topic, a
guard test that fails when a file is not included) with shared builders in `tests/it/support/`.
`fuzz/` and `relax.rs` in the test kit stay their own binaries (cost). The ack policy is no longer process-global: it is `State::ack_policy` (`testing` only), set from `FixtureSpec::with_ack_policy`.

Everything else drives the real `State` through `casement_testkit::Fixture` (CONVENTIONS
addendum 10): the headless backend, a `Display` whose poll fd sits in calloop as a duplicate
(no `unsafe`, SP3), clients on non-blocking socket pairs (`casement_testclient::TestClient`),
the virtual clock as a calloop event source (A1). `pump` runs until a round moves nothing;
`advance` stops at every timer and vblank on the way, so how an advance is chopped never shows;
transcripts carry virtual time only; no test sleeps. Protocol tests live in
`crates/casement-testkit/tests/it/protocol/<p>.rs`, one per module; scene goldens in
`crates/casement-testkit/tests/it/scene/` (`mod.rs`, `desk.rs`, `build.rs`) with PNGs under `tests/golden/scenes/`; the globals
golden is `tests/golden/globals.txt`.

Everything runs inside the jail (`~/desktop/harness/jail.sh`): built outside, archived with
nextest, run inside. Binaries built with `testing` or `debug-inject` refuse to start outside it
(`casement::jail_check`) and refuse every backend but headless. The jail can fabricate soft GPU
nodes (`--fabricate-gpus N`: `/sys/dev/char/226:(128+n)`, A3) for the multi-GPU topology (P1).

## 6. Repo rules

- The gate: `dev/gate.sh fast -p <crate>...` for a lane, `dev/gate.sh merge` for a merge,
  `dev/gate.sh phase` at a phase exit (20-FREEZE-PLAN §6.2). Steps: preflight, `cargo fmt --all
  --check`, `./scripts/check-boundary.sh`, `./scripts/check-protocol-coverage.sh`, clippy with
  `-D warnings`, the nextest archive built outside and run inside the jail (profiles `fast`,
  `merge`, `phase` in `.config/nextest.toml`); merge and phase add `cargo deny check licenses`,
  doc tests and `dev/accept.sh`; phase adds wlcs and the KMS fixtures. Each step is timed against
  `dev/gate/budgets.toml` (over twice its budget fails with exit 4); exit 2 (a SKIP) fails unless
  `dev/gate/expected-skips.toml` lists it; exit 5 (a jail breach) stops the gate. It prints
  `GATE GREEN (<tier>) <run>`; `dev/out/<run>/summary.json` records every step. Every exit code
  is checked, nothing is piped or tailed. `~/rs-wt/gate-lane-jailed.sh <checkout>` is the
  cross-repo form.
- Features (A2): `casement-backend` has `tty` and `libinput`, both off by default; `casement` has
  `tty` and `libinput`, on by default, and its binary requires both. The workspace dependency
  `casement = { default-features = false }` keeps the test kit, `wlcs-casement` and any other
  headless consumer free of system libraries. The gate builds both feature sets: `cargo build -p
  casement` (everything) and `cargo build -p casement-testkit` (headless only). Building the
  whole workspace at once unifies `casement`'s default features into the test kit; build the test
  kit's tests with `-p casement-testkit` for the jail.
- `unsafe_code = "deny"` at the workspace. The one unsafe module is
  `crates/casement-render/src/gl_raw.rs`: exactly `EGLDisplay::new` and `GlesRenderer::new`
  (A17). Another `unsafe` block there needs a reviewed interface ask showing a GL pass smithay's
  safe API cannot express.
- `missing_docs` warns at the workspace, so every public item says what it means.
- Sibling repos are path dependencies (`../quire`, `../porter`, `../casement-protocols`); a lane
  worktree links them beside itself.
- smithay is pinned by rev to our fork `PoHsuanLai/smithay` branch `casement` (upstream
  `118e34ff` plus the patches FINDINGS lists, A14) with no default features; each crate names the
  features it uses and the `casement` binary carries A14's union. Nothing goes upstream to
  smithay (DECISIONS, A13); patches live only in the fork.
- Frozen files lanes never edit: `crates/casement/src/state.rs`, `crates/casement/src/delegates.rs`,
  every `lib.rs`/`mod.rs`, `crates/casement-wayland/src/states.rs`, every `Cargo.toml`,
  `tests/golden/globals.txt`, `dev/gate/budgets.toml`, `dev/gate/expected-skips.toml`,
  `.config/nextest.toml`, and the docs; a change there is an interface ask.
