# Q-C5: xwayland-satellite or smithay's X11 window manager (P2-XW memo, 2026-10-11)

Question (04-plan-phases-effort, Q-C5 and the P2 decision gate): keep serving X11 programs through
xwayland-satellite, or take smithay's own X11 window manager ("XWM", MIT) into casement.

## Recommendation

Keep xwayland-satellite for P2 and beyond. Re-open the question only if one of the triggers at the end
happens on a real run; none can be judged without the jail runs listed in FINDINGS `## p2-xw`.

Honesty note: this memo was written without building or running anything. Statements about what
satellite or smithay do come from their READMEs and from SP6 (`spike-SP6.md`, S-T7), not from a run in
this lane. Claims marked "unverified" are what the acceptance scripts must settle.

## What each route is

- **Satellite** (Supreeeme/xwayland-satellite, MPL-2.0). A separate process. It starts a rootless
  Xwayland, acts as the X11 window manager itself, and shows every X11 window to casement as an ordinary
  Wayland window (`xdg_toplevel` or popup). casement only owns the listening sockets and starts it on
  demand (`casement::xwayland`, P1-X1). niri uses the same route.
- **smithay XWM** (`smithay::xwayland`, MIT). A library inside casement's process. casement itself
  would run Xwayland, speak X11 (x11rb) to it, and manage X11 windows as a window manager: map and
  configure requests, `_NET_*` properties, selections, focus. Each X11 window is a special kind of
  surface that casement-core has to model.

## Comparison

| Concern | Satellite | smithay XWM |
|---|---|---|
| Mixed-DPI scale | Satellite gives Xwayland one scale for the whole X screen and publishes `Xft.dpi`; a window dragged to an output with another scale is scaled by us, so it is sharp on one output and soft on the other (unverified: `xwayland-scale.sh` records the facts). Upstream is aware and still changing here. | We would be the one deciding what scale X11 clients see, and could pick per-window or the primary output's scale. X11 itself has no per-monitor scale, so the same limit applies: one `Xft.dpi` for the screen. The gain is control, not a fix. We would own the bug list. |
| Clipboard | Satellite bridges `CLIPBOARD`, and `PRIMARY` as far as Wayland offers it. Large pastes (INCR) and text-versus-image choice are satellite's code (unverified: `xwayland-clipboard.sh` covers plain text only). | smithay has selection helpers (`SelectionHandler`); we write the bridge to `wl_data_device` ourselves, including INCR, target conversion and ownership races. |
| Drag and drop | Between X11 and Wayland windows it is limited upstream; we cannot patch it. | smithay has DnD pieces for XWM; still ours to wire, and X11 Xdnd is a long protocol. |
| Focus and activation | Satellite turns `_NET_ACTIVE_WINDOW` and X11 focus requests into `xdg_activation_v1`; casement's existing activation rules (no token, no focus steal) apply as for any app. Unverified: what `xwayland-activation.sh` records. | We would handle X11 focus (`WM_TAKE_FOCUS`, input hint, override-redirect) ourselves, in the compositor loop, next to Wayland focus. More control and more cases (focus fights, never-focus windows). |
| Window model | X11 windows are plain Wayland windows: Spaces, window capture, `quire_window` placement, the Mac window model all work with no X11 special case. | casement-core needs an X11 window kind, X11 geometry in the arrangement, stacking sync with the X server, and every protocol handler (toplevel list, capture, placement) must know it. |
| Crash isolation | Satellite or Xwayland dying loses only X11 windows; casement restarts it (P1-X1, now with a crash-loop guard, P2-XW). | An X11 protocol bug or Xwayland death is inside the compositor's loop; no isolation. |
| Maintenance | One maintainer, active; pinned by git revision (not on crates.io, SP6). A stall is a risk; we can fork a separate process without a licence problem beyond MPL file-level rules. | smithay is widely used, the XWM part moderately; every smithay bump may move it. The code is ours to maintain once adopted. |
| Licence | MPL-2.0. Fine as a separate process; we never link or copy it (no-GPL-linking rule keeps MPL out of our crates, too). | MIT, fits the permissive stack. |
| Effort | Done (P1-X1); hardening is small. | Estimated 2 to 3 weeks of Sonnet lanes plus a model change in casement-core, and a long tail of X11 quirks. A guess from the size of smithay's `xwm` module and anvil's use of it (about 1.5 to 2.5 k lines of our glue), not a measurement. |

## Why satellite wins now

1. The Mac window model is built on Wayland surfaces. Satellite lets X11 windows enter it with no special
   case. The XWM route costs a second window kind through the whole compositor.
2. The strongest reason to switch is mixed-DPI sharpness. X11 has one screen scale, so an XWM does not
   remove that limit; it only moves the decision into our code.
3. Isolation and licence both favour a separate process.
4. What remains is clipboard and activation edge cases, which are small to fix in satellite's
   configuration and casement's activation rules, and cheap to find with the jail scripts.

## What P2-XW did

- `casement::xwayland`: a crash-loop guard (at most 4 starts in 30 seconds; then the client waits and
  `DISPLAY` stays), `SatelliteRendering::Software` for machines without a render node (`-glamor none`).
- Tests over the fake spawner for both (no X server). Real-app checks stay in `dev/accept/xwayland-*.sh`.

## What would change the answer

Switch to the XWM if, on a real run, any of these holds and satellite upstream will not fix it:

- `xwayland-scale.sh` shows X11 windows blurred or wrongly sized on the output the user mainly uses, on
  a machine with outputs at two scales, and a per-window scale from an XWM would cure it.
- The clipboard loses large or non-text content that users hit in daily work (browsers, office tools).
- Activation or focus of X11 windows fails in a way casement's rules cannot adjust (for example a
  needed X11 hint that satellite drops).
- Satellite is unmaintained for a release cycle and a fork is more work than the XWM.
- A feature needs the X11 window itself (global hotkeys of X11 tools, window placement control that a
  Wayland window cannot express).

Cost of switching later is the same estimate above; staying on satellite now does not make it larger,
because the Wayland side of casement is unchanged by either route.
