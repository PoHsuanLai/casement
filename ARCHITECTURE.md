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
| `casement-core` | the pure model: ids, geometry, time data (`Mono`, `VblankClock`, `TimerBook`, `VirtualClock`), apps, windows, Spaces, layers, the lock, focus and press routing, input origins, reach and bindings, output facts and policy, connection policy (I7), settings, `Event`, `Effect`, `Model::step`, invariants; from P1-Z the GPU plan (`gpu`), the frame scheduler's maths (`frame`) and the Space ids file's codec (`space::SpaceFile`) and, from P2-Z, the shell's and the desktop manager's requests and notes (`shell`, `desktop`) and, from wave 2, per-app consent for the privileged globals (`privilege`, `policy::{power_of, trusted_powers}`) | none |
| `casement-layout` | the `SceneLayout` seam: `SceneSpec`, `Arrangement`, `RustScene`, `spec_of(&Model)`; taffy only in `crates/casement-layout/src/taffy_bridge/` | none |
| `casement-motion` | `MotionBook` of ds-motion spring legs, `retarget`, `sample` into an `Eq` `FrameScene` in 1/120 px | none |
| `casement-render` | the `Compositor` renderer trait, `CasementElement`, `output_elements` (FrameScene to elements), `render_output` (damage-tracked), blur v0, the lock cover, output read-back (`pass::capture_output`, P1-Z), `golden` (feature `testing`); the workspace's one unsafe module `src/gl_raw.rs` (A17) | the GPU through smithay |
| `casement-wayland` | one module per protocol: a `<Protocol>Part` (smithay's state plus our BTreeMap tables), `Part::new` behind the I7 filter, `on_*` (request to `Event`s), `apply_*` (effect to wire); `WaylandStates`, `GlobalFilter`, `ClientState`, `convert`; from P2-Z `capture_source` (window capture) and, behind the feature `quire` (default on, A8), `quire::{desktop, shell}` (the private `quire_desktop_manager_v1` and `quire_shell_v1` servers) | the wire |
| `casement-backend` | the headless backend (virtual outputs, vblank source, render-cost model, `VirtualInput: InputBackend`), winit (a window is one output at its own size and scale, drawn flipped, and one seat whose keyboard and pointer go through the generic input conversion; no touch), the session seam, the soft-GPU pieces (`SoftPixman`, `FixedUdmabuf`, A4); from P1-Z the tty seams (`tty::{Kms, KmsOpener, ScanoutSurface, TtySession, DeviceSource}`, `input::InputFeed`, `gpu::{RenderNodes, FormatSource}`), `TtyBackend` over them, and their thin real adapters (`*_real.rs`) compiled only under the features `tty` and `libinput` (off by default; A2) | devices under the features, none headless |
| `casement-agent` | the privileged-connection hand-out (`connection_role`, `hand_out`: the role check on what the bus thread proved, and the socket pair; P2-Z types, P2-Q2 bodies), agent seat, EIS, masked capture, trusted surface, kill (P4); from P2-T3 `eis` (the remote EIS server for the RemoteDesktop portal: `open`, `RemoteEisSource`, `RemoteInput`, `portal_may_connect`, by the role the caller proved; every event tagged `Origin::Remote`) | sockets |
| `casement-ipc` | the inject and wait grammars (its `casement-backend` dependency is optional, on with `debug-inject`; `serve_debug_at(address)` takes the bus address, the library reads no environment); `org.quire.Casement.Debug1` (feature `debug-inject`), `org.quire.Compositor1` (P2-Z: `Connect(role) -> (h)`, `serve_compositor_at(address, CallerSetup)`; every caller is rate counted and proven through the caller-identity library on the bus thread before a bounded queue of `CONNECT_QUEUE` into the loop: `CallerCheck`, `Intake`, `Proven`); `privacy::{PrivacyLink, PrivacyBoard, PrivacyEdit, PrivacyModule}`, the Privacy list for Settings as a live settings module (`org.quire.SettingsModule1` at `/org/quire/Compositor1/privacy`); `a11y::{A11yWorker, verify}`, the accessibility-root owner check (PORTAL-15); `shell_calls::{ShellCalls, ZbusShellCalls}`, the one place a binding's D-Bus call is made (A4); `space_registry::{SpaceRegistry, ZbusSpaceRegistry, NoSpaceRegistry, Pace}`, `Spaces1.Register` through `porter_dbus::SpacesProxy` on a background thread (csvc; fake `FakeSpaceRegistry`); `IpcHandles` | D-Bus |
| `casement-privacy` | the privacy files: the person's book and the administrator's floor, read and written safely (`PrivacyStore`, `FilePrivacyStore`, `FloorFiles`: `ds_helpers::{read_capped, write_atomic, move_aside}`, a floor file believed only when it and its folder are the administrator's alone), the policy over them (`load_book` sets a damaged book aside and starts empty; `load_floor` never leaves a failed floor more permissive than none) and `list`, the list `casementctl privacy list` prints; no bus, no UI, no clock | files |
| `casement` | lib + bin: `State` (frozen), `delegates.rs` (frozen), `handlers/<protocol>.rs`, `core_loop`, `exec`, `clients`, `display`, `time/` (the only clock), `config/` (ds-settings), `cli`, `notify` (`Notify`, `SocketNotify`, `Notice`, `Watchdog`: systemd's READY=1, STOPPING=1 and WATCHDOG=1; csvc; fake `FakeNotify`), `space_sync` (`SpaceSync`: registers the desktops it has not sent yet through `SpaceRegistry`; csvc), `run` (`casement::run(RunConfig) -> Result<(), RunError>`: the startup and the event loop; `main.rs` is only arguments, `$CASEMENT_LOG`, `$DBUS_SESSION_BUS_ADDRESS` and the exit code), `debug` (Debug1 in the loop, feature `debug-inject`), `jail_check` (features `testing`, `debug-inject`), `log` (takes its filter as an argument); from P1-Z `seams` (the injected `SpaceStore`, `ShellCalls`, `Spawner` and `LockMarker`), `space_store`, `lock_marker` (the session lock's marker file in the runtime directory: a restart while the shell's lock is held comes back covered), `spawn`, `xwayland`; from P2-Z `connect` (`Compositor1.Connect` answered in the loop: the unit check through `casement-agent`, the new client inserted with its role); from wave 2 (`pa2-croot`) `privacy` (the model's book and the files: `start`, `reload_floor`, `keep`, `edited`, `publish`), `a11y` (a claimed accessibility root is queued and adopted only when the bus says it is the client's own), `hangup` (`SIGHUP` reads the floor again), `clients::{Judge, Standing}` (who a public client's program is, judged on a thread of its own with a deadline: `clients::probes`), `clients::accept` (a listener that rests on a failed accept instead of ending the loop), `clients::roster` (the connection caps: 384 in all, 48 per app or process), `open_files` (the soft file limit raised at start), `latest_writer` (a file written off the loop, latest text wins: the Spaces file and the privacy book) and `time::WallClock` (the date); its features `tty` and `libinput` (default on) turn the backend's real adapters on | everything, through the crates above |
| `casementctl` | lib + bin: `debug` (Debug1), `shot` (P1), `report` (P5), `privacy list \| reload` (reads the two privacy files through `casement-privacy`; `reload` sends `SIGHUP` to the process that holds `org.quire.Compositor1`, after checking it is a casement) | D-Bus, the wire, files |
| `casement-testclient` | typed wayland-client helpers per protocol, `Transcript`, `Pattern`, shm buffers; bins `casement-probe`, `casement-shell-exec`; no smithay (B9) | the wire |
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
| `casement-privacy` | casement-core, casement-protocols, ds-helpers (no features: no bus), thiserror, tracing; reads no clock (the date is an argument) and no environment (the folders are arguments) |

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
| where input came from, and what it may reach | `casement-core::input::{Origin, reach}` (C19, I3, I8); a virtual device is the person's hand only when turnstile labelled it (`phys`) for keycap (`Maker`, `Origin::of_kernel_device`; never by bus alone or vendor id) |
| key bindings and their resolution | `casement-core::input::{Bindings, resolve}`; the shell's own chords come from the desktop keymap, `casement::keymap` (chordkit layers through keycap-client's `KeycapSource`), behind casement's `[bindings]` entries |
| first-sight scale, mode choice, output placement | `casement-core::output` |
| the ramp an output is driven with by night light, and which outputs it leaves alone (`night_light.skip_outputs`: a connector name or an output key) | `casement-core::settings::NightLightSettings::{warming_of, ramp_for}` (pure); driven by `Model` (`model/night_light.rs`) from `Event::NightLightTick(MinuteOfDay)` as `Effect::SetGamma`; a gamma-control client that holds an output with a table of its own wins until it lets go |
| how strongly the blur behind a layer surface frosts (its shell role picks `BlurStrength`, a surface with none is `Regular`), and Reduce transparency (quire's `appearance.transparency` from `appearance.toml`, live: every blur becomes the scheme's paper at 0.96) | `casement-core::layer::BlurStrength::of` (pure); `casement-layout::reduced_stand_in`; `casement::appearance::{stored_in, blur_finish}`; `casement-render` `BlurLook::of`, `BlurFinish`, `BlurV0Element::with_finish`, `under_finish`; the frame carries both in `LayerDraw::{blur_strengths, finish}` |
| which connection sees which global (I7) | `casement-core::policy::{advertised, advertised_to}`; a public client is not shown the globals of a power it was refused at connect time (`advertised_to` with `Privilege::hidden_for`) |
| which global needs the person's consent (the seven `Power`s of the shell protocol's vocabulary), and which desktop service roles are trusted without asking | `casement-core::policy::{power_of, trusted_powers}`; `power_of` is exhaustive over `GlobalKind` |
| whether a program may use a power: the administrator's floor and the person's book, first match wins (floor refusal, book refusal, floor pre-approval, book allow, the floor's default, a question) | `casement-core::privilege::{decide, decide_trusted}` over `Floor` and `Book` (pure; the text codecs `Book::{from_toml, to_toml}` and `Floor::from_toml` cap rows, bytes and strings; the files themselves are the composition root's seam) |
| the questions to the person: one per program and power, one on screen, at most `OPEN_ASKS` open, a deadline, a pause after a dismissal, a rate limit per client; what a use comes to; what a change in Settings ends | `casement-core::privilege::Privilege`, a field of `Model` (`model/privilege/`), driven by `Event::{ClientStanding, PrivilegeUse, PermissionAnswered, PrivilegeGranted, PrivilegeRevoked, PrivilegeForgotten, ShellReady, ShellGone, BookLoaded, FloorLoaded}` and answering with `Effect::{Resolve, AskPermission, WithdrawAsk, SaveBook}`; its deadline timer is `TimerKey::AskDeadline` |
| who a public client's program is, as the consent machine needs it (the seam to the caller-identity library) | `casement-core::privilege::ClientStanding` (core's own vocabulary: `Service`, `Program`, `Sandboxed`, `Unverified`); the composition root converts the identity library's standing into it when the connection is accepted. Core depends on no identity library (B1) |
| the person's privacy book and the administrator's floor on disk, and what a damaged or untrusted file comes to | `casement-privacy::{PrivacyStore, FilePrivacyStore, FloorFiles, load_book, load_floor, save_book}` (the model's side is `casement-core::privilege::{Book, Floor}`); the compositor feeds and writes them in `casement::privacy` |
| the date a kept answer is stamped with | `casement::time::WallClock` (`seams.wall`; the only wall-clock reader besides the night light) |
| typed settings | `casement-core::settings::CasementSettings` (the stored document: `casement::config`) |
| the closed sets of inputs and outputs of the model | `casement-core::{Event, Effect}` |
| what the shell asks of the desktop (apps, windows, Spaces) and what the model says back | `casement-core::desktop::{DesktopRequest, AppAsk, WindowAsk, SpaceAsk, DesktopNote, DesktopRef}` (P2-Z) |
| what the shell says of its surfaces and what it is told | `casement-core::shell::{ShellRequest, ShellNote}` (P2-Z) |
| who may have a `Shell`, `Agent` or `Portal` connection, and the socket pair | `casement-ipc::dbus` proves the caller (role `shell`, `agent_tier` or `portal`, all the way: image and unit as the manager states it) before the loop hears of the call; `casement-agent::{connection_role, hand_out}` (the only hand-out, B14) checks the proven role again; inserted by `casement::connect`. The `Portal` connection (NEW-FA-2, `Connect("portal")`, role `portal`) is how transom captures: it sees the globals the public socket sees (never `quire_shell_v1` or `quire_desktop_manager_v1`) and its model class is the standing of a `Full` portal on the public socket (`ClientStanding::portal()`), trusted for the screen and the windows (`trusted_powers`) and asked about the rest. Remote EIS socket for the portal (role `portal`): `casement-agent::open_remote_eis`, `Compositor1.ConnectEis`, fed by `casement::connect::answer_remote`; each connection has its own device slot (`casement-agent::RemoteSlots`, minted in `connect.rs`, CASW-19) and `Compositor1.EndEis(session)` (`connect::answer_end`, same caller check) closes every socket of a portal session through `casement::remote_sessions::RemoteSessions` |
| `org.quire.Compositor1` | `casement-ipc::dbus::Compositor1` (the object), answered by `casement::connect::answer` |
| Settings > Privacy (the list of programs and their powers) | `casement-ipc::privacy` (keys from `Privilege::rows()`; writes are `PrivacyEdit`s the root turns into `PrivilegeGranted`/`PrivilegeRevoked`/`PrivilegeForgotten`); read by `settings` and `shell`, written by `settings` only |
| whether a window's accessibility root is its own program's | `casement-ipc::a11y::{A11yWorker, A11yAsk, A11yVerdict}` |
| the window a foreign-toplevel identifier names | `casement_wayland::foreign_toplevel_list::{identifier_of, window_of_identifier}` |
| what changed on an output since a capture client's last frame | `casement_wayland::image_capture::{DamageLogs, Since}` (fed by `casement::handlers::serve_captures` from `PresentedFrame::damage`) |
| where the pointer is on an output's frame, and the cursor's image for capture | `casement_wayland::image_capture::place`; `casement_render::{cursor_face, cursor_pixels}` |
| turning a point or a size with an output's transform (a cursor's position, hotspot and size in a capture) | `casement_core::geometry::{turn_point, turn_size}` |
| a capture into or out of a client's dma-buf; a client-drawn cursor's pixels | `casement_render::pass::{write_dmabuf, read_dmabuf, ArgbImage}`, called through `{HeadlessBackend, TtyBackend}::{write_dmabuf, read_dmabuf}`; the offer is `casement_wayland::image_capture::dma_offer` |
| which way a capture frame reaches a client's buffer (GPU draw into a dma-buf, or read back) | `casement_backend::capture_path::{capture_path, placement}` (pure, table-tested); the GPU draws are `casement_render::pass::{blit_texture, render_window_dmabuf}`, the output frames kept on the GPU are `casement_backend::tty::{CaptureGpu::draw_scene, TtyBackend::blit_output_dmabuf}` on a seat and the headless GL canvas in the jail; the composition root's call is `handlers/image_capture/{direct,frames}.rs` |
| a window's capture size and when it is captured | `casement_wayland::capture_source::{capture_size, ToplevelCaptureBook}` (damage gate) |
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
| how many objects of each kind one client may hold, and the cut when it asks for more (`wl_display.no_memory`) | `casement-wayland::limits::{Limit, admit, cut}`; each call site counts from the collection that owns the objects (toplevels in `XdgShellPart::records`, popups, layers, waiting screencopies, window-list managers, icon buffers, exports, a keyboard's keymaps) |
| which client files may be read on the loop (a keymap, a gamma ramp) | `casement-wayland::client_file` (memory files only: `fcntl(F_GET_SEALS)`, which asks the filesystem nothing) |
| which keys may switch the VT | `casement-core::input::Origin::switches_vt` (kernel keyboards only), asked in `casement::core_loop::deliver` before `vt_chord` |
| the shortest idle timeout a peer is served | `casement-wayland::idle_notify::timeout_floor` (5 s for unproven or sandboxed peers) |
| who may take the keyboard, take over a dead locker's lock, or reserve room (shell versus another client) | `casement-core`: `LayerSpec::owner` (`LayerOwner`) and `Model` (`shell_clients`, the `LockHolder` inside `LockState`), learned from `ConnectionRole::Shell`; a security context gives its clients an app only when its creator is the desktop's sandbox launcher (the table's `sandbox_launcher` row, `Maker::SandboxLauncher`, judged by `clients::maker_of`) and is itself proven; any other creator leaves them an unknown app (`casement-wayland::security_context::identity_for_clients`) |
| whether the session was locked when casement last ran | `casement::lock_marker::LockMarker` (injected; real `FileLockMarker` in `$XDG_RUNTIME_DIR/casement`; fake `MemoryLockMarker`), written and removed by `LockReply::{Remember, Forget}` |
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
| telling systemd ready, stopping, alive | `casement::notify::Notify` (csvc; real `SocketNotify` over `$NOTIFY_SOCKET` as `main` reads it; fake `FakeNotify`); the ping timer is `casement::time::ping_every` (B8) |
| registering a Space with the account service | `casement-ipc::space_registry::SpaceRegistry` (csvc; real `ZbusSpaceRegistry` on the session bus address `run` is given; fake `FakeSpaceRegistry`), driven by `casement::space_sync::SpaceSync` from `Effect::PublishWorkspaces` and once after startup |
| starting a program (a binding's, xwayland-satellite) | `casement::spawn::Spawner` (A5; fake `FakeSpawner`) |
| X11 clients | `casement::xwayland` (`XwaylandConfig`: mode and satellite path from configuration, never read inside the crate: `main` resolves a flag, else `xwayland.*` in settings.toml, else `PATH`, through `xwayland::from_file`) |
| libinput device settings and their fake | `casement-backend::input::{InputFeed::apply, DeviceSettings}`; `casement-testkit::fakes::FakeInputDevices` |
| reading an output's frame back | `casement-render::pass::capture_output` (P1-Z) |
| virtual input devices and their events | `casement-backend::VirtualInput` |
| the soft GPU pieces (A4) | `casement-backend::gpu::{SoftPixman, FixedUdmabuf}` |
| the timing lines (`window-ready`, `key-window`, `xwayland-ready`, `session-ready`: target `timing`, INFO, `CLOCK_MONOTONIC` ns; workflow spec) | `casement::timing` (pure records and formatters; `Timing` in `State`; `hear` functions called at each moment); the clock reading is `casement::time::monotonic_ns` and `ClockSource::monotonic_epoch_ns` |
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

### Consent for the privileged globals (casement-core)

A program on the public socket may use seven powers only with the person's consent (the owner's
"ask per app", like the Mac's privacy prompts): see the screen, type and click for them, help them
type, read and replace the clipboard, lock the screen, change the displays, see and manage the
windows. The shell, the portal and the desktop's own services are trusted by identity and never
asked. The portal gets that trust through its own connection: transom asks `org.quire.Compositor1.Connect("portal")`
on the session bus, where the bus proves it at `Full`, and captures on the connection casement hands back
(`ConnectionRole::Portal`: the public socket's globals, the screen and the windows trusted, nothing more). On
the public socket a peer reaches at most `Proof::Exe`, so a portal that connects there is an ordinary program
and is asked (NEW-FA-2). The consent machine is pure; this is the contract between it and the composition root and
the protocol layer (wave 2: `pa2-ccore` is this half, the others follow it).

- **The seam for who is asking.** `Event::ClientStanding { client, standing: ClientStanding }` is
  sent once per public connection, right after `Event::ClientConnected`. `ClientStanding` is core's
  own small vocabulary: `Service { role, exe }` (a row of the desktop's callers table), `Program
  { exe }` (a root-owned image), `Sandboxed { app, engine }` (vouched for by the sandbox, never by a
  name), `Unverified { exe_name }` (a writable or hidden image, a peer that is gone, a program that
  changed its image after connecting). The composition root converts the caller-identity library's
  standing into it; core never reads a process and links no identity library. Until the event
  arrives a public client is `Unverified` (fail closed). A client that is later reported as someone
  else loses every power it was given and every question it waited on.
- **A grant binds to a `Subject`** (`Program(path)` or `Sandboxed { app, engine }`), never to a
  display name or a claimed app id. `Unverified` clients and interpreters (`python*`, `perl`,
  `ruby`, `node`, `deno`, `bun`, `java`, `lua*`, `sh`, `bash`, `zsh`, `fish`, `wine*`, plus the
  floor's `[never_remember]`) are asked with `Memory::ThisTimeOnly` (Allow Once) and nothing about
  them is written to the book or read from it.
- **Use.** The protocol layer sends `Event::PrivilegeUse { client, power }` at the first use of a
  gated power and holds the request (at most `HELD_REQUESTS`) until `Effect::Resolve { client,
  power, outcome }` says `Allowed` (replay), `Denied` (the protocol's denial) or `Revoked` (end the
  use at once, mid-capture included). It keeps the latest outcome per client and power. The model
  resolves at once when the records decide, and later when the person answers.
- **Asking.** `Effect::AskPermission { ask, power, subject, memory }` goes to the shell as
  `permission_ask`; `Effect::WithdrawAsk` as `permission_withdraw`; the person's answer comes back
  as `Event::PermissionAnswered { ask, answer, at }` (`at` is the wall-clock second the root read;
  core reads no clock). The shell is "ready" after `Event::ShellReady` (it bound the protocol at
  the version that carries the question) and gone after `Event::ShellGone` or the end of its
  connection. Questions wait `SHELL_WAIT` for a shell after start, are not shown while the lock is
  up, time out after `ASK_TTL` (the deadline timer is `TimerKey::AskDeadline`, armed and cancelled
  through the usual timer effects), and are not asked again for `REASK_AFTER` after a dismissal.
- **The records.** The root reads `/etc/quire/privacy.toml` (the floor, `Floor::from_toml`, strict:
  a wrong file is an error) and the person's `privacy.toml` in the state directory
  (`Book::from_toml`, lenient: an unreadable row is dropped) and sends `Event::FloorLoaded` and
  `Event::BookLoaded`. `Effect::SaveBook(book)` is the whole book as it now is, to be written
  atomically. Settings changes arrive as `Event::{PrivilegeGranted, PrivilegeRevoked,
  PrivilegeForgotten}`. `Privilege::rows()` is what Settings lists (the answers; `casement_ipc::privacy::listed` adds, for each program they name, a
  switch standing at `ask` for every power it has no row for, on Settings' Privacy page, at most `MAX_GRANTS` programs; a `Set`
  on one is the first answer, `PrivacyEdit::Allow`/`Deny`).
- **Hiding.** A refused power's globals are not advertised to a public client that connected
  under the refusal: `Privilege::hidden_for(role, standing)` gives the `PowerSet`, and
  `policy::advertised_to` consults it. `policy::advertised(global, role)` is the same with
  nothing hidden. Nothing is re-announced when an answer changes later.

### Consent at the wire (casement-wayland, `casement/src/handlers/consent.rs`)

How the protocol layer carries the contract above (wave 2: `pa2-cwl`).

- **One mechanism.** `casement_wayland::consent::admit(state, client, power)` is called at the first request that uses a power
  and answers `Go`, `Hold` or `Refuse` from the `Ledger` (the latest word per client and power, shared through `ClientMint` and
  forgotten when the client disconnects). A client with no word, or a refusal, is asked of the model: `admit` sends
  `Event::PrivilegeUse` (after `Event::ClientStanding` when `ClientState::image` says the program executed another image since
  accept) and reads the ledger again, because the model usually answers inside the same `feed`. `Effect::Resolve` writes the
  ledger and then calls each protocol's `on_resolved`: replay what was held on `Allowed`, give the protocol's own denial on
  `Denied`, end the use on `Revoked`. A client casement did not insert is refused.
- **Held requests.** `consent::Held<T>` keeps at most `HELD_REQUESTS` per client (`Pushed::Full` hands the item back to be
  refused) and drops what belongs to a gone client at the next push. What is held is the request that creates something: the
  `copy`, the frame request, the `lock`, the `apply`/`test`, the manager bind; for objects that cannot wait (a control, a manager)
  the object is made inert and registered when the person allows. A virtual keyboard keeps its own requests (`virtual_keyboard/gate.rs`).
- **Per protocol.** `Screen`: `zwlr_screencopy` (`copy`; `failed`), `ext_image_copy_capture` for outputs and windows (`frame`;
  the frame `failed`, the session `stopped`; every session of the client is stopped on revoke). `Keys`: a virtual keyboard
  (dead when refused, its held keys released on revoke). `Lock`: `ext_session_lock.lock` (`finished`; a lock already held is not
  ended by a revocation: only its locker can unlock). `Displays`: `zwlr_output_manager` `apply`/`test` (`failed`, checked
  against the heads as they are when allowed), `zwlr_output_power` and `zwlr_gamma_control` (`failed`; revoke fails the control
  and lights the output or restores the identity curve). `Windows`: `zwlr_foreign_toplevel_manager` and `ext_workspace_manager`
  (`finished`; a handle or a `commit` of a client without the power changes nothing).
- **Gated through smithay's hooks (it owns the dispatch).** Four globals are served entirely by smithay, which holds their
  state; our fork (`PoHsuanLai/smithay`, rev `005eebf1`) asks the compositor before it acts, through a defaulted handler method
  that returns `smithay::wayland::Gate { Open, Hold, Refuse }`, and settles what it held with `release(&mut self, display,
  &ClientId, Gate)` on the protocol's state. `consent::gate(state, client, power)` is the one place casement answers: it calls
  `admit` and maps `Go` to `Open`, `Refuse` to `Refuse`, and `Hold` to `Hold` after parking the client's connection in the
  `Ledger` (`park`; forgotten when the client goes). `Effect::Resolve` then calls `handlers::consent::settle_served`, which
  releases the protocol's state with `consent::release_gate(outcome)` (`Allowed` opens, `Denied` and `Revoked` refuse).

  | Global | Power | Hook (handler in `casement/src/handlers/`) | Held | Refused | Released on allow |
  |---|---|---|---|---|---|
  | `ext_data_control_manager_v1` | `Clipboard` | `data_control_gate` (`ext_data_control.rs`) | a device or a source, at `get_data_device` / `create_data_source` | device `finished`, source `cancelled`, registered nowhere | the device is registered and told the clipboard |
  | `zwlr_data_control_manager_v1` | `Clipboard` | `data_control_gate` (`wlr_data_control.rs`) | the same | the same | the same |
  | `zwp_input_method_manager_v2` | `TypingHelp` | `input_method_gate` (`input_method.rs`) | the input method made by `get_input_method` | `unavailable` | it becomes the seat's input method (activated for a field already enabled) |
  | `ext_foreign_toplevel_list_v1` | `Windows` | `foreign_toplevel_list_gate` (`foreign_toplevel_list.rs`) | the list bound | `finished` | every window is listed, then each new one |

  A held object is inert: it receives no events and its requests are ignored except `destroy`. smithay keeps at most
  `MAX_HELD_PER_CLIENT` (16) held objects per client and protocol and refuses the rest itself; casement keeps no count of its own
  for these (the 256 of `HELD_REQUESTS` is for the queues casement holds). A client that disconnects while held is dropped by
  smithay at its next hold, and its parked connection is forgotten with its ledger rows. None of the refusal events is a
  destructor, so none needs `consent::finish_later` (the pa2-cwl lesson is about `finished` on the workspace and window-list
  managers, which are destructors). A power taken back (`Revoked`) ends what smithay already registered for the client: `settle_served`
  looks the connection up in the `Ledger` (`Ledger::connection`, written by `clients::insert_client` right after the insert (the system
  libwayland backend never calls `ClientData::initialized`), and
  forgotten with the client's rows; a client allowed at once was never parked, so `parked` is not enough), then, after the `release`
  with `Gate::Refuse` that settles what was held, `end_served` calls the fork's `revoke`: `DataControlState::revoke::<State>(&id)` on
  both clipboard managers (every device of the client is taken off its seat and sent `finished`), `InputMethodHandle::revoke::<State>(&mut
  State, &id)` on the seat's handle (cloned first: it needs the state; the input method is sent `unavailable`, leaves the seat, and
  its popup and keyboard grab go) and `ForeignToplevelListState::revoke(&id)` (its lists are sent `finished`, its window entries
  `closed`). A selection the client already set stays until another client replaces it, and its sources are not ended. The next use asks
  again. Tests: `a_manager_the_person_switches_off_is_finished` (ext and wlr), `an_input_method_the_person_switches_off_is_unavailable`,
  `a_list_the_person_switches_off_is_finished`.
- **The program behind an app (`quire_desktop_app_v1.subject`, version 2; QUIRE-12).** The desktop manager is served at version 2 (the app
  objects take the manager's version; windows and Spaces stay at 1). In an app's burst, right after `identity`, the manager sends
  `subject` to a client that bound it at 2: the text `ClientStanding::subject_text` gives for the first window of the app whose client
  has one (`Privilege::subject_of(window.client)`). `Program` is its image path, `Sandboxed` the sandbox's app id, `Service` its
  image path when the table gave one and nothing otherwise, `Unverified` nothing (no event means no subject is known). It is sent once
  per value, so a later change of the program behind the app (a new window of another program) sends it again; a standing that
  changes with no change to the app is not published until the next time the app is.
- **The global filter.** `GlobalFilter` takes the decision function as a field (`advertised_to`); `ClientState` carries the
  `hidden: PowerSet` decided at accept (`ClientState::with_standing_under(&Privilege, standing)`), so a power refused before
  the client connected is not advertised to it, and a refusal that comes later changes no registry.
- **The shell's side.** `quire_shell_v1` is version 6 (5 carries the question, 6 the display trial below): `permission_ask`/`permission_withdraw` go to every bound object of
  version 5 or later (`quire/shell_permission.rs`), the part remembers the questions it sent so an object that binds later is
  shown them, `permission_answer` becomes `Event::PermissionAnswered` (an answer for a question not sent is ignored),
  `Event::ShellReady` follows the bind of a version 5 object and `Event::ShellGone` the unbind of the last one. The date an
  answer is stamped with comes from `QuireShellHandler::quire_shell_wall` (the root's `WallClock`), passed to `on_answer`.
  An answer that arrives less than `ANSWER_NOT_BEFORE` (500 ms) after its question was first sent is a protocol error for that
  shell connection only (`quire_shell_v1::Error::AnsweredTooSoon`): sill's sheet is disarmed for 600 ms, so a faster
  answer did not come from a person looking at it. The part stamps each `SentAsk` with the loop's clock
  (`QuireShellHandler::quire_shell_now`, the root's `ClockSource`, so a test with the virtual clock decides the edge); a question
  shown again to a shell that binds later keeps its first stamp, and an answer that is too soon settles nothing: the question
  stays open. The same rule, with the trial's first-sent stamp, applies to `displays_trial_answer`.
- **The composition root's half (`pa2-croot`).**
  - *Who is asking.* At accept `run.rs` holds the public client's process (`Pinned::of_stream`), reads its facts (`Prober`) and judges them against the desktop's caller table (`clients::Judge`; the table is `/etc/quire/desktop-callers.toml`, one load at start through `clients::desktop_table(&SystemDir)`, the one place it is made; a failed load is an empty table and one error log: nobody privileged; in a `testing` or `debug-inject` build whose run has `SystemDir::At(dir)`, `clients/dev_callers.rs` adds the rows of `dir/desktop-callers.toml` after it as developer rows, see *Test caller rows* below). `clients::insert_client(state, stream, role, peer, standing)` takes `clients::Standing::{Unjudged, Judged { standing, watch, maker }}`: `Judged` builds `ClientState` with `with_standing_under(model.privilege(), standing)`, `with_image_watch(watch)` and `with_maker(maker)` (`Maker::SandboxLauncher` only for the table's `sandbox_launcher` row shown by its image; the security-context handler reads it from the creator, CASW-04: any other creator's context makes clients with no app id, a Flatpak scope name or app id in the context counting for nothing) and sends `Event::ClientStanding` right after `Event::ClientConnected`; the hand-out, a sandbox and a bare test socket are `Unjudged`. The watch (`PinnedImage`) holds the process by its descriptor and says `Unverified` when `/proc/<pid>/exe` names another image than at accept. The library's standing becomes core's `ClientStanding` in `clients::client_standing`: a service only with `Proof::Exe` or better (a control-group-only service is unverified), a root-owned image a `Program`, a user-writable or hidden one `Unverified`. The porter `PeerIdentity` (the Dock's `AppKey`) is `Proven` from a service's app, a sandbox's app or the launcher's scope name of a root-owned program (`clients::peer_of`); the scope name is a claim that only groups windows, a grant binds to `Subject`.
  - *The files.* `seams.privacy: Box<dyn PrivacyStore>` (real: `FilePrivacyStore` over `$XDG_STATE_HOME/casement/privacy.toml`, from `StartConfig::state_home`; none for an ephemeral run) is read once before the first client (`privacy::start`: `Event::FloorLoaded`, `Event::BookLoaded`), written by `Effect::SaveBook` (`privacy::keep`), and the floor read again on `SIGHUP` (`privacy::reload_floor`). The floor is `/etc/quire/privacy.toml` and `privacy.d/*.toml`, or `--system-dir` (`StartConfig::system_dir`, `RunConfig::with_system_dir`; in a build with `testing` or `debug-inject` the files of an injected folder belong to the running user, anywhere else to the administrator). Never an environment variable. The same two builds have a second exception beside the owner: `privacy::writable_image` (`privacy/injected.rs`, called from `clients::Judge::arrival`) takes a program whose image the person can write as a `Program` when the floor was read from an injected folder (`SystemDir::At`) and a row of it names that image, so a scenario's floor can pre-approve what it runs from `build/`; the book never names one, a floor in `/etc/quire` never does, and a release build has the no-op body (the module is compiled out), so such a program stays `Unverified` and is asked every time.
  - *Test caller rows (cas13).* The third test-only exception, same two features: `clients/dev_callers.rs` reads `desktop-callers.toml` from the injected folder (`SystemDir::At`) through peephole's `TableFiles::load_dev` (a plain file, not a link, owned by the person or root, not group- or world-writable, within the table size cap; anything else adds no row and logs one line, a missing file nothing) and merges it after the system table (`Table::merge`; the system table wins where both name a program). Its rows are `Proof::Dev`, the lowest proof. Two places accept exactly that proof and only in these builds: `dbus::require` (the roles of Compositor1 and the Privacy list: `Proof::Full`, or `Proof::Dev` of the role asked) and `clients::client_standing`/`peer_of` (a Dev service is a `ClientStanding::Service`/`Proven` app). The feature chain is `casement/testing` and `casement/debug-inject` -> `casement-ipc/dev-rows` -> `peephole-bus/dev-rows`; nothing else enables it and a release build has no `Proof::Dev` at all. Row format: `[[peer]]` with `app`, `role`, `exe = ["/abs/path"]` (no `unit`).
  - *Settings.* `run.rs` creates `PrivacyLink::new()`, gives the bus thread `CallerSetup::desktop(table).with_privacy(link)` (`.with_test_units(...)` under `debug-inject`), reads the edit channel (`privacy::edited`: `edit.into_event(wall.now())`) and publishes `model.privilege().rows()` to the board at start, after every `SaveBook` and every edit.
  - *The date.* `seams.wall: time::WallClock` (`System` in a real run, `Fixed` in a test; the seam default is the epoch) stamps answers; `QuireShellPart::wall` is a copy of it.
  - *The accessibility root.* `quire_window.set_a11y_root` reaches the model as `A11yRoot::Unknown`; the claim is queued on `seams.a11y` (`A11yChecks`, real: `casement_ipc::a11y::A11yWorker`, fake: `FakeA11y`) with the client's pid from `Processes`, and the verdict becomes `WindowHintChange::A11y` (`a11y::adopt`). A client casement keeps no process for (the hand-outs, a sandbox) never verifies.

- **Displays a program changed (CASW-10; the display trial).** A change to the displays made by a client other than the shell (mode,
  scale, position, on or off, through `zwlr_output_manager` `apply` or `zwlr_output_power`) is applied and then kept only if the
  person keeps it. `Event::OutputRequested` carries `by: output::Requester` (`Shell`, `Program` or `Restore`; the wire layer reads it
  from the connection's `ConnectionRole`, `consent::requester_of`). A `Program` change that alters a configuration takes the model through
  `model/display_trial.rs`: the output's previous configuration is stored (the first one per output in the trial), `TimerKey::DisplayTrial`
  is armed for `output::DISPLAY_CONFIRM_WITHIN` (15 s: a bad mode can blank the screen, and someone staring at a black screen is back in
  a quarter of a minute) and `Effect::AskKeepDisplays { trial, within }` goes to the shell as `displays_trial(trial, seconds)`
  (`quire/shell_trial.rs`, `quire_shell_v1` version 6). `Event::DisplaysAnswered { trial, answer }` (`displays_trial_answer`) settles it: Keep
  keeps, Revert puts the stored configurations back at once (`Effect::ApplyOutput` per output still plugged); the timer firing is a
  Revert. Every end sends `Effect::EndDisplaysTrial` (`displays_trial_end`): answered, timed out, replaced, or the shell changed the
  displays itself (`Requester::Shell` applies and settles a trial in flight as kept; it is never asked). A further `Program` change while
  a trial is open keeps the stored configurations, restarts the time and asks again with the same id. `Requester::Restore` is the
  compositor putting back what a program changed (an output a program darkened is lit again when its control goes): applied, never asked,
  a trial in flight untouched. casement's own hotplug and tty reconfiguration do not go through `OutputRequested` and are never asked.
  The part remembers the one open trial, so a shell that binds (version 6) while it is open is sent `displays_trial` at once with the
  seconds left; a shell older than 6 is never asked and the change is reverted when the time runs out, which is also the fail-safe when
  the shell goes away mid-trial. An answer for a trial that is not open is ignored; one less than `ANSWER_NOT_BEFORE` after the first
  send is the protocol error `answered_too_soon`. Tests: core `tests/it/display_trial.rs`; wire `protocol/quire_shell_trial.rs`
  (virtual clock, scripted shell).

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

P2-Z adds the two private protocols' handler traits and the hand-out's seam, each with the
wire glue written in the protocol's module (`Part::new`, `bind`, the dispatch of every request,
the refusal of management on an agent connection) and its `on_*` and `apply_note` bodies frozen:

```rust
// casement-wayland::quire::desktop (feature `quire`): the root provides the part and what it reads, split
// into disjoint borrows. `Part::new::<D>` creates the global.
pub trait QuireDesktopHandler: EventSink + 'static {  // EventSink::feed(Vec<Event>) is casement-wayland::handler's, shared by every handler trait
    fn quire_desktop_parts(&mut self) -> DesktopParts<'_>;          // { part, view: DesktopView { model, outputs } }
}
pub fn on_bind(part: &mut QuireDesktopPart, view: &DesktopView<'_>, manager: QuireDesktopManagerV1, standing: Standing)
    -> Result<(), WaylandFault>;
pub fn on_get_window_for(part, view, manager: &QuireDesktopManagerV1, window: WindowId, object: QuireDesktopWindowV1)
    -> Result<(), WaylandFault>;
pub fn on_create_space(part, output: OutputKey) -> Vec<Event>;     pub fn on_stop(part, manager: &QuireDesktopManagerV1);
pub fn on_app(part, app: &AppKey, ask: AppAsk) -> Vec<Event>;      pub fn on_window(part, window: WindowId, ask: WindowAsk) -> Vec<Event>;
pub fn on_space(part, space: &SpaceId, ask: SpaceAsk) -> Vec<Event>;
pub fn apply_note(part, view: &DesktopView<'_>, note: &DesktopNote) -> Result<(), WaylandFault>;
pub enum Standing { Manager, ReadOnly }                           // Shell manages, Agent reads; ReadOnly is refused with not_permitted

// casement-wayland::quire::shell
pub trait QuireShellHandler: EventSink + OutputLookup + 'static {
    fn quire_shell_part(&mut self) -> &mut QuireShellPart;
    fn layer_of_surface(&self, client: &Client, surface: &WlSurface) -> Option<LayerKey>;
}
pub fn on_set_role(part, layer: Option<LayerKey>, surface: &WlSurface, role: ShellRole) -> Result<Vec<Event>, RoleRefusal>;
pub fn on_set_minimise_target(part, dock: Option<LayerKey>, toplevel: &str, local: LogicalRect) -> Result<Vec<Event>, TargetRefusal>;
pub fn on_clear_minimise_target(part, toplevel: &str) -> Result<Vec<Event>, TargetRefusal>;
pub fn apply_note(part: &mut QuireShellPart, note: &ShellNote) -> Result<(), WaylandFault>;

// casement-wayland::capture_source (window capture; the global is declared, not created)
pub fn on_toplevel_source(book, window: WindowId) -> ToplevelSourceId;   pub fn on_source_destroyed(book, source);
pub fn on_session(book, source) -> Result<ToplevelSessionId, WaylandFault>;
pub fn on_frame_request(book, session, buffer: PhysicalSize, content: ContentStamp) -> Result<FrameVerdict, WaylandFault>;
pub fn apply_content(book, window, content: ContentStamp) -> Vec<ToplevelCaptureRequest>;
pub fn apply_ready(book, session, content, at: Mono) -> Result<(), WaylandFault>;
pub fn apply_failed(book, session, why: CaptureFailure) -> Result<(), WaylandFault>;
pub fn apply_window_gone(book, window) -> Vec<ToplevelSessionId>;
pub fn capture_size(logical: LogicalSize, scale: Scale) -> PhysicalSize;  // implemented: round(l x s)

// casement-agent: the role check on what the bus thread proved, and the socket pair (the root inserts the server end)
pub fn connection_role(held: &RoleName) -> Option<ConnectionRole>;         // shell -> Shell, agent_tier -> Agent, portal -> Portal
pub fn hand_out(wanted: &ConnectionRole, held: &RoleName) -> HandOut;      // Granted(Handed { role, server, client }) | Refused(RefuseWhy)
pub fn portal_may_connect(held: &RoleName) -> bool;                        // role portal

// casement-ipc::dbus (CASW-05): who calls, proven on the bus thread
pub trait CallerCheck: Debug + Send + Sync + 'static { fn who<'a>(&'a self, sender: &'a str) -> WhoFuture<'a>; }  // BusPeers implements it
pub struct CallerSetup;   // ::desktop(Table), ::with(|connection| Arc<dyn CallerCheck>), .window(d), .with_privacy(PrivacyLink)
pub async fn require(check: &dyn CallerCheck, sender: Option<&str>, roles: &[RoleName]) -> fdo::Result<Proven>;  // Proof::Full, any pin (+ Proof::Dev of the role asked, feature dev-rows only)
pub struct Proven { pub app: AppKey, pub role: RoleName }                  // what ConnectAsk carries instead of a cgroup text
pub const CONNECT_QUEUE: usize = 8;  pub const CALLS_PER_WINDOW: u32 = 4;  pub const RATE_WINDOW: Duration = 1 s;

// casement::connect: the loop's side of Compositor1.Connect
pub fn attach(handle: &LoopHandle<'static, State>, calls: Channel<ConnectCall>) -> Result<(), LoopError>;
pub fn answer(state: &mut State, ask: ConnectAsk) -> ConnectReply;   // hand_out(&ask.role, &ask.caller.role); AgentState::handed: ClientKey -> AppKey

// casement::clients (pa2-croot): every client enters here
pub fn insert_client(state: &mut State, stream: UnixStream, role: ConnectionRole, peer: PeerIdentity, standing: Standing) -> io::Result<Client>;
pub enum Standing { Unjudged, Judged { standing: ClientStanding, watch: Arc<dyn ImageWatch> } }
pub struct Judge;   // ::desktop(), ::new(Table), .arrival(&UnixStream) -> Arrival { peer, standing }

// casement-privacy / casement::privacy / casement::a11y (pa2-croot): the seams the root injects
pub trait PrivacyStore: Debug {
    fn read_book(&mut self) -> Result<Option<String>, StoreError>;   fn write_book(&mut self, text: &str) -> Result<(), StoreError>;
    fn set_book_aside(&mut self, at: UnixSeconds) -> Result<(), StoreError>;   fn read_floor(&mut self) -> Vec<FloorText>;
}   // real FilePrivacyStore; NoPrivacyStore keeps nothing
pub trait A11yChecks: Debug { fn check(&self, ask: A11yAsk) -> Result<(), A11yFault>; }   // real A11yWorker; fake FakeA11y
```

P1-Z adds these seams. Every one has a thin real implementation and a fake exported by
`casement-testkit`; the traits take `std::fmt::Debug` as a supertrait so a `Box<dyn …>` can sit in
a `#[derive(Debug)]` struct.

```rust
// casement-backend::tty (real: kms_real.rs, surface_real.rs, soft_real.rs, session_real.rs, udev_real.rs under
// feature `tty`; fakes: FakeKms, FakeScanout, FakeSession, FakeUdev)
pub trait Kms: Debug {
    fn resources(&self) -> KmsResources;
    fn test(&mut self, commit: &KmsCommit) -> CommitVerdict;         // TEST_ONLY
    fn commit(&mut self, commit: KmsCommit) -> Result<(), KmsError>;
    fn set_power(&mut self, connector: ConnectorId, power: OutputPower) -> Result<(), KmsError>;
    fn set_gamma(&mut self, connector: ConnectorId, ramp: &GammaRamp) -> Result<(), KmsError>;
    fn events(&mut self) -> Vec<ConnectorEvent>;
    fn scanout(&mut self, connector: ConnectorId, config: &OutputConfig)   // P1-T: the D2 to D3 hand-off
        -> Result<Box<dyn ScanoutSurface>, KmsError>;
    fn pause(&mut self);                                              // the seat was lent away
    fn resume(&mut self) -> Result<(), KmsError>;
}
pub trait KmsOpener: Debug { fn open(&mut self, path: &DevicePath) -> Result<Box<dyn Kms>, KmsError>; }
// CASB-07: what draws an output. `RenderMode::{Gpu, Software}` is how the session draws (`--safe` is `Software`:
// pixman into dumb buffers, no GBM, EGL or GLES is ever opened, no render plan, no dma-buf offer, no hardware cursor).
// `Routes::open` is the one selection over `ScanoutMaker { gpu, software }`: Software and an output whose GPU surface
// failed before get `software` (soft_real.rs, `DrmCompositor` over `DumbAllocator` + `PixmanRenderer`); otherwise `gpu`
// (surface_real.rs), and a failure falls back to `software` for that output alone, said once in the log. The flip
// count both real surfaces share is the pure `Pace`. Fakes: `FakeKms::drawing(mode)` runs `Routes` over a fake maker.
pub trait ScanoutSurface: Debug {
    fn queue(&mut self, frame: QueuedFrame) -> QueueOutcome;
    fn presented(&mut self, now: Mono) -> Option<Presented>;
    fn blank(&mut self) -> Result<(), KmsError> { Ok(()) }            // DPMS off
    fn reset(&mut self) {}                                            // the seat came back
}
pub trait TtySession: Debug {
    fn open_device(&mut self, path: &DevicePath) -> Result<OwnedFd, SessionFault>;
    fn switch_vt(&mut self, vt: VtNumber) -> Result<(), SessionFault>;
    fn notes(&mut self) -> Vec<SessionNote>;
    fn settled(&mut self, note: SessionNote) {}                       // P1-T: the backend acted on it
    fn sleep_link(&mut self) -> Option<SleepLink> { None }            // P1-T: PrepareForSleep reaches us
}
pub trait DeviceSource: Debug { fn present(&self) -> Vec<DevicePath>; fn next_event(&mut self) -> Option<DeviceEvent>; }

// P1-V2: `TtySeams::writeback: Option<Box<dyn Writeback>>` is the vkms writeback connector, set only by
// `--drm-node` mode (real: writeback_real.rs; fake: FakeWriteback). `--drm-node` also builds `DirectSession`
// (a TtySession moved only by its SleepLink), `FixedNode` (the one DeviceSource) and `NoGpus` (no detection,
// no input): direct.rs, direct_real.rs.
pub trait Writeback: Debug {
    fn begin(&mut self, crtc: CrtcId, size: PhysicalSize) -> Result<(), WritebackError>;
    fn finish(&mut self) -> Result<RawCapture, WritebackError>;        // after FLIPS_TO_WAIT flips
}

// casement-backend::input (real: libinput_real.rs under feature `libinput`; fakes: FakeInputDevices,
// FakeInputFeed). P1-T: the seat's input is one seam, `InputFeed`, held in `TtySeams::input`; its
// events are converted once (`input_kind_of`) and the devices are opened through the session.
pub trait InputFeed: Debug {
    fn apply(&mut self, device: DeviceId, settings: &DeviceSettings) -> Applied;
    fn poll(&mut self, context: &PollContext) -> Vec<FedInput>;
    fn suspend(&mut self);
    fn resume(&mut self);
}

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

// casement (real: BackgroundSpaceStore (writes on a thread), ProcessSpawner (binding programs in their own systemd user scope); fakes: MemorySpaceStore, FakeSpawner)
pub trait SpaceStore: Debug {
    fn read(&self) -> Result<Option<String>, SpaceStoreError>;
    fn write(&mut self, text: &str) -> Result<(), SpaceStoreError>;     // atomic
}
pub trait Spawner: Debug {
    fn spawn(&mut self, spec: SpawnSpec) -> Result<Spawned, SpawnError>;
    fn exited(&mut self, now: Mono) -> Vec<Pid>; // the one that starts a child reaps it and says which ended
}
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

1. A map in `MotionBook` (keyed by what moves), its legs beside `window_motion.rs` / `space_slide.rs`.
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
4. A `GlobalKind` variant and its I7 row (casement-core), its entry in `GlobalKind::ALL`, its
   `power_of` row (a power, or none, decided on purpose) and its row in casement-core's own
   `tests/golden/globals.txt`, a `tests/golden/globals.txt` row in the test kit
   (`<interface> <version> <module>`), `crates/casement-testkit/tests/it/protocol/<p>.rs` with its
   `mod` line in `tests/it/protocol/mod.rs` (`scripts/check-protocol-coverage.sh`), a testclient
   helper module, and for a private request a fuzz op.

**Declare a protocol before its lane fills it (a delta freeze).** Everything in "Add a protocol",
except that `Part::new::<D>(display, role_filter)` creates no global and returns the empty part,
so no client sees the protocol and `globals.txt` stays as it is. (The `quire` parts now create their
globals in `Part::new`; `ToplevelCapturePart` cannot hold smithay's state without creating the global, so its lane adds the
field and the call together.) The `GlobalKind` variant (and its
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
- Features (A8): `casement-wayland` has `quire`, on by default: the private `quire_*` servers
  (`quire::desktop`, `quire::shell`) and their fields in `WaylandStates`. `casement` turns it on
  explicitly; a consumer that wants the standard protocol set alone takes the crate with
  `default-features = false`. `quire_window` stays outside the feature for now (FINDINGS, p2-z).
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
