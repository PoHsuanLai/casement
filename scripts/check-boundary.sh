#!/usr/bin/env bash
# casement's boundary rows (20-FREEZE-PLAN §2.4, B1..B20; 21-FREEZE-AMENDMENTS A1, A4),
# mechanically. ARCHITECTURE.md section 1 is the table; this is its mechanical form.
#
# "Reaches" rows read each crate's normal-edge dependency graph once (`cargo tree -e normal`)
# and look for forbidden package names in it: a check for OUTPUT, never for exit status
# (`cargo tree -i` exits 101 when a dependency is absent, the state we want; CONVENTIONS §14).
# "Names" rows grep the source. A row about a crate that is not a member yet passes and says so;
# it bites the moment that crate lands.
set -uo pipefail
cd "$(dirname "$0")/.."
fail=0
bad() { echo "LEAK: $*"; fail=1; }
ok() { echo "holds: $*"; }

members=$(cargo metadata --no-deps --format-version 1 2>/dev/null \
  | grep -o '"name":"[^"]*","version"' | sed 's/"name":"\([^"]*\)".*/\1/' | sort -u)
if [ -z "$members" ]; then
  echo "ERROR: cargo metadata found no members; the boundary was not checked"
  exit 1
fi
is_member() { printf '%s\n' "$members" | grep -qx "$1"; }

# Every package a crate reaches through normal edges, one name per line.
reached() {
  cargo tree -p "$1" -e normal --prefix none --format '{p}' 2>/dev/null | awk '{print $1}' | sort -u
}

# reaches_none <crate> <label> <extended regex of package names>
reaches_none() {
  local crate="$1" label="$2" pattern="$3" found
  if ! is_member "$crate"; then ok "$label ($crate is not a member yet)"; return; fi
  found=$(reached "$crate" | grep -E "^($pattern)$" | tr '\n' ' ')
  if [ -n "$found" ]; then bad "$label: $crate reaches ${found% }"; else ok "$label"; fi
}

# names_only <label> <extended regex> <allowed path regex> [search roots...]: the pattern
# appears in Rust source only under paths matching the allowed regex.
names_only() {
  local label="$1" pattern="$2" allowed="$3"; shift 3
  local roots=("$@") hits
  [ ${#roots[@]} -eq 0 ] && roots=(crates)
  hits=$(grep -rnE --include='*.rs' "$pattern" "${roots[@]}" 2>/dev/null | grep -vE "^($allowed)" || true)
  if [ -n "$hits" ]; then bad "$label"; printf '%s\n' "$hits" | head -20; else ok "$label"; fi
}

# The graph rows of B1: what the pure model may never reach.
PURE_FORBIDDEN='smithay|wayland-[a-z-]+|calloop|calloop-[a-z-]+|zbus|tokio|drm|drm-[a-z-]+|gbm|gbm-[a-z-]+|input|input-sys|udev|libudev-sys|libseat|libseat-sys|dioxus|dioxus-[a-z-]+|blitz[a-z-]*|stylo|stylo_[a-z_]+|ds|ds-shell|ds-blitz|ds-settings|parley|fontique|vello[a-z_]*|wgpu|anyrender[a-z_]*'

# B1 casement-core: the forbidden list, taffy included.
reaches_none casement-core "B1 casement-core is pure" "$PURE_FORBIDDEN|taffy"
# B1 (allowed ours): casement-core's direct dependencies of our own are only these.
OURS='casement-[a-z-]+|ds-[a-z-]+|keycap-[a-z-]+|palmrest-[a-z-]+|porter-[a-z-]+|prov'
ALLOWED_OURS='casement-protocols|ds-core|ds-style|ds-motion|ds-behaviour|keycap-core|palmrest-protocol|porter-core|prov'
direct=$(cargo tree -p casement-core -e normal --depth 1 --prefix none --format '{p}' 2>/dev/null \
  | awk 'NR > 1 {print $1}' | grep -E "^($OURS)$" | grep -vE "^($ALLOWED_OURS)$" | tr '\n' ' ')
if [ -n "$direct" ]; then bad "B1 casement-core depends on ${direct% } (not in the allowed list)"; else ok "B1 casement-core's own dependencies are allowed ones"; fi

# B2 casement-layout: B1's list except taffy; only taffy_bridge/ names taffy.
reaches_none casement-layout "B2 casement-layout is pure" "$PURE_FORBIDDEN"
names_only "B2 only casement-layout/src/taffy_bridge/ names taffy" '\btaffy::|use taffy\b|extern crate taffy' \
  'crates/casement-layout/src/taffy_bridge/'

# B3 casement-motion: B1's list (taffy arrives through layout, so the source never names it).
reaches_none casement-motion "B3 casement-motion is pure" "$PURE_FORBIDDEN"
names_only "B3 casement-motion names no taffy" '\btaffy::|use taffy\b' '$^' crates/casement-motion

# B4 casement-render: no session/device backends, no second graphics stack.
names_only "B4 casement-render names no drm/udev/session/libinput backend" \
  'smithay::backend::(drm|udev|session|libinput)' '$^' crates/casement-render
reaches_none casement-render "B4 casement-render reaches no second graphics stack" \
  'wgpu|vello[a-z_]*|anyrender[a-z_]*|blitz[a-z-]*|stylo|parley|fontique'

# B5 casement-backend names no casement_wayland and no smithay::wayland module but dmabuf.
names_only "B5 casement-backend names no casement_wayland" '\bcasement_wayland\b' '$^' crates/casement-backend
hits=$(grep -rnoE --include='*.rs' 'smithay::wayland::[a-z_]+' crates/casement-backend 2>/dev/null | grep -v 'smithay::wayland::dmabuf' || true)
if [ -n "$hits" ]; then bad "B5 casement-backend names a smithay::wayland module"; printf '%s\n' "$hits"; else ok "B5 casement-backend names only smithay::wayland::dmabuf"; fi

# B6 casement-wayland: no device backends.
names_only "B6 casement-wayland names no device backend" 'smithay::backend::(drm|gbm|udev|libinput|session)' '$^' crates/casement-wayland
reaches_none casement-wayland "B6 casement-wayland reaches neither drm nor gbm" 'drm|gbm'

# B7 the thin-adapter allow-list (11 §5.3).
ADAPTERS='crates/casement-backend/src/tty/(kms_real|surface_real|session_real|udev_real)\.rs|crates/casement-backend/src/gpu/render_nodes_real\.rs|crates/casement-backend/src/input/libinput_real\.rs'
names_only "B7 only the thin adapters name the device crates" \
  'smithay::backend::(drm|libinput|session::libseat|udev|egl::EGLDevice)|(^|[^:[:alnum:]_])(drm|gbm|input|libseat)::' "$ADAPTERS"
adapter_lines=$(find crates/casement-backend/src -type f 2>/dev/null | grep -E "^($ADAPTERS)$" | xargs -r cat | wc -l)
echo "B7 thin adapters: $adapter_lines lines (target <= 1500)"

# B8 (+ A1, SP3): only crates/casement/src/time/ reads a clock or arms a real timer.
names_only "B8 only casement/src/time/ reads a clock" \
  '\b(Instant::now|SystemTime::now|Clock::now)\b|calloop::timer|\bClock::<Monotonic>::new' 'crates/casement/src/time/'

# A4: smithay's UdmabufAllocator::create_buffer over-allocates; ours replaces it.
names_only "A4 nothing calls UdmabufAllocator::create_buffer" 'UdmabufAllocator::create_buffer' '$^'

# B9 the test client reaches no smithay.
reaches_none casement-testclient "B9 casement-testclient reaches no smithay" 'smithay'

# B10 test and debug-inject code never reaches the release binary.
if is_member casement; then
  features=$(cargo tree -p casement -e features --prefix none 2>/dev/null | grep -E 'feature "(testing|debug-inject)"' || true)
  if [ -n "$features" ]; then bad "B10 the release casement enables testing or debug-inject"; else ok "B10 the release binary enables neither testing nor debug-inject"; fi
else
  ok "B10 (casement is not a member yet)"
fi

# B11 the binary graph reaches no UI or second graphics stack (vello_cpu only via casement-chrome).
reaches_none casement "B11 the casement binary reaches no UI stack" \
  'blitz[a-z-]*|stylo|selectors|html5ever|dioxus|dioxus-[a-z-]+|wgpu|anyrender[a-z_]*|vello|vello_hybrid'

# B12 no crate but casement-intents reaches the agent tier or the shell.
TIERS='docket[a-z-]*|cua[a-z-]*|almanac[a-z-]*|sill[a-z-]*|shell-host|model-[a-z-]+|engine-supervisor|vision-prep|speech-provider'
for crate in $members; do
  [ "$crate" = casement-intents ] && continue
  reaches_none "$crate" "B12 $crate reaches no agent tier or shell crate" "$TIERS"
done

# B13 ds-settings only in casement-ipc and casement.
for crate in $members; do
  case "$crate" in casement-ipc|casement) continue ;; esac
  reaches_none "$crate" "B13 $crate reaches no ds-settings" 'ds-settings'
done

# B14 only casement-agent names reis, the role hand-out or EIS fds.
names_only "B14 only casement-agent names reis or the privileged hand-out" \
  '\breis::|ConnectionRole::(Shell|Agent)\b.*(hand_out|socket)|\bEisFd\b' 'crates/casement-agent/'

# B15 only casement-chrome reaches the text stack.
for crate in $members; do
  [ "$crate" = casement-chrome ] && continue
  reaches_none "$crate" "B15 $crate reaches no text stack" 'parley|fontique|vello_cpu'
done

# B16 the excluded crates are never members.
for excluded in wlcs-casement casement-scene-css; do
  if is_member "$excluded"; then bad "B16 $excluded is a workspace member"; else ok "B16 $excluded is not a member"; fi
done

# B17 no hash order reaches the wire or a render order.
names_only "B17 no HashMap/HashSet in wayland, render, layout, motion" '\bHash(Map|Set)\b' '$^' \
  crates/casement-wayland crates/casement-render crates/casement-layout crates/casement-motion

# B18 no unsafe outside the reserved module (the workspace also denies unsafe_code).
names_only "B18 no unsafe outside casement-render/src/gl_raw.rs" '\bunsafe\b' 'crates/casement-render/src/gl_raw\.rs'

# B19 casement-intents reaches no compositor stack.
reaches_none casement-intents "B19 casement-intents reaches no smithay, wayland or calloop" 'smithay|wayland-[a-z-]+|calloop'

# B20 clean-room tripwire: no use, file name or comment naming a GPL compositor.
TRIP='\b(niri|cosmic_comp|cosmic-comp|jay|pinnacle)\b'
hits=$(grep -rniE "$TRIP" crates dev 2>/dev/null; find crates dev -iregex ".*\(niri\|cosmic.comp\|pinnacle\).*" 2>/dev/null)
if [ -n "$hits" ]; then bad "B20 a GPL compositor is named in code"; printf '%s\n' "$hits" | head -20; else ok "B20 no GPL compositor named in code"; fi

exit "$fail"
