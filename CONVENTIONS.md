# Conventions

casement follows the program's shared conventions, whose text lives in quire's `CONVENTIONS.md`
(identical in quire, sill, shell-host, palmrest and detent): types, traits, effects, errors,
tests, comments, change discipline, borrowing. Read it first. `ARCHITECTURE.md` here adds the
crate map, the one-home table, the traits, the recipes and the repo rules; where the two
disagree, `ARCHITECTURE.md` wins for casement.

What casement adds, each with its reason (the first twelve are the freeze plan's addenda):

1. **Clean room.** smithay and its `anvil`, `smallvil` and `wlcs_anvil` examples (MIT) are the
   only compositor code anyone working here may read or port; a ported piece keeps an
   attribution line in its file header and a row in a `THIRD-PARTY.md`. niri, cosmic-comp, jay,
   pinnacle, catacomb, strata, labwc, KWin, mutter, wlroots, sway, Hyprland, Wayfire, scenefx,
   xdg-desktop-portal-cosmic or -luminous and Otto's lay-rs are never opened as source: their
   issues, wikis and release notes are facts, nothing more. Behaviour facts go into `QUIRKS.md`
   in our own words, keyed by facts (driver, PCI class, connector type, toolkit) with the issue
   link. `scripts/check-boundary.sh` (row B20) trips on any of their names in code.
2. **No panics in the event loop.** Client misbehaviour is a protocol error; backend failure is
   logged and degrades; `unwrap`/`expect` only in tests or with a same-line proof.
3. **The edge converts.** smithay's `bool`s, loose integers and serial types become our enums
   and newtypes in `casement/src/handlers/*` and `casement-wayland`, never deeper.
4. **The handler recipe** (one shape for every protocol): smithay callback in
   `casement/src/handlers/<p>.rs` → `casement_wayland::<p>::on_<request>(&mut wl.<p>, …) ->
   Vec<core::Event>` → `Model::step` → `Vec<Effect>` → `casement::exec` →
   `casement_wayland::<p>::apply_<effect>` / backend / render.
5. **No text in the compositor.** Exception from P3: `casement-chrome` (SSD titles).
6. **Time is an argument.** Only `casement/src/time/` reads a clock (row B8; calloop's `Timer`
   and smithay's `Clock::now` read the wall clock, so they live there too); core takes `Mono`;
   motion samples `now`; no `sleep` in a test.
7. **No hash order reaches the wire or a render order** (row B17): `BTreeMap`, or `IndexMap`
   where insertion order is the meaning.
8. **Floats only** inside motion sampling, layout's Taffy bridge and render passes; every stored
   and every wire value is an integer unit (`SubPx`, `Fraction`, `Scale`, `Millihertz`).
9. **Hardware values come from detection plus an override**, never this machine.
10. **One harness:** tests drive the real `casement::State` through `casement_testkit::Fixture`;
    no test assembles its own `State`. (The pure crates test their functions directly.)
11. **New global, new test file** (`tests/protocol/<global>.rs`), enforced by
    the protocol-coverage check (P0-Z2); a new private request also gets a fuzz op.
12. **Goldens change only with a reason** in the commit; `CASEMENT_BLESS=1` is deliberate.

And the repo's own:

13. **`todo!()` lives on master only behind a frozen interface** (freeze plan C42): every one has
    a row in `FINDINGS.md` naming its fill lane and the test that proves it, and that test is
    written and `#[ignore = "<lane id>"]`d. A `todo!()` body binds its arguments
    (`let _ = (a, b);`) so the signature keeps its names without an unused-variable warning,
    and private state a frozen body will read carries `#[expect(dead_code, reason = "<lane>")]`,
    which turns into an error the moment the fill reads it, so the marker cannot outlive its
    reason.
14. **`Copy` for heap-free values of at most two words** (geometry, `Mono`, ids), as quire's
    `Rect` is; anything holding a `String` or a `Vec` is `Clone` only.
15. **`cargo fmt -p <crate>`, never `cargo fmt --all`, to write.** `--all` also formats every
    path dependency's workspace (quire, porter, casement-protocols). `cargo fmt --all --check`
    is read-only and is the gate's form; it needs every sibling checkout of those workspaces.
16. **Every crate's manifest says `license = "MIT OR Apache-2.0"`** through the workspace;
    `cargo deny` reports an unlicensed root otherwise.
