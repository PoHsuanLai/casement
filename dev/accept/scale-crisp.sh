#!/usr/bin/env bash
# Acceptance script 'scale-crisp' (20-FREEZE-PLAN §4.3, P0 exit criterion 4): at output scale 1.5 a
# window maps, resizes and renders crisp. The client draws a 2-logical-pixel checkerboard at the
# output's scale (a 3-pixel checkerboard in a buffer 1.5 times the size, under a viewport
# destination in logical pixels): in the captured frame every square must be exactly 3 physical
# pixels, opaque black or white, before and after a maximise and an unmaximise. The first frame also
# equals the golden crates/casement-testkit/tests/golden/scenes/one-window-1.5x.png (the same scene
# the testkit shoots: a 400x300 gradient window, the pointer in the corner) byte for byte.
# The client is casement's own test client, the artefact a jail has; the quire and shell-host demo
# windows are the consumer scripts' (shell-host's scripts/dev/accept-*.sh, run with
# COMPOSITOR=casement). Exit: 0 pass, 1 fail, 2 could not run, 3 environment.
#
# Owner: P0-accept (fills the P0-H1 stub).
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
# shellcheck source=dev/lib/accept.sh
source "$root/dev/lib/accept.sh"

golden="$root/crates/casement-testkit/tests/golden/scenes/one-window-1.5x.png"
[ -f "$golden" ] || { echo "scale-crisp.sh: the golden $golden is missing" >&2; exit 2; }
crisp="$root/dev/lib/crisp.py"
APP=org.example.scene
OUT=$JAIL_SCRATCH/crisp
mkdir -p "$OUT"

# The testkit's scene: one output of 2880x1620 at scale 1.5 (logical 1920x1080).
start_compositor --clock stepped --outputs "HEADLESS-1:2880x1620@60000:180:0,0"
plug_devices
assert_eq "the output is 1920x1080 logical at scale 1.5" "1920,1080,180" \
  "$(output_json | jq -r '[.logical[0], .logical[1], .scale] | join(",")')"
probe_start A

# 1. The golden scene, byte for byte: a 400x300 gradient window, the pointer in the corner.
inject "move 0 0" >/dev/null
probe_do A toplevel w "app=$APP" size=400x300 pattern=gradient scale=1.5
probe_wait A '^probe: mapped w 400x300$'
wait_for "mapped $APP" >/dev/null
settle
step 3
shot output "$OUT/scene.png" >/dev/null
assert_eq "the capture is the output's physical size" "2880x1620" "$(python3 "$crisp" size "$OUT/scene.png")"
assert_ok "the window equals the one-window-1.5x golden" python3 "$crisp" same "$OUT/scene.png" "$golden"
probe_do A close w
wait_for "gone $APP" >/dev/null

# 2. Crisp: a 2-logical-pixel checkerboard, then after maximise and unmaximise.
# crisp_check LABEL: the window's buffer starts at its frame's origin, in physical pixels.
crisp_check() {
  local label=$1 x y w h
  settle
  step 3
  shot output "$OUT/$label.png" >/dev/null
  read -r x y w h <<<"$(win_rect "$APP")"
  # logical -> physical at 1.5: a window on an odd logical pixel would sit on a half pixel.
  assert_eq "$label: the frame is on whole physical pixels" "0 0" "$((x * 3 % 2)) $((y * 3 % 2))"
  assert_ok "$label: every square is exactly 3 physical pixels, black or white (${w}x${h} logical at $x,$y)" \
    python3 "$crisp" checker "$OUT/$label.png" $((x * 3 / 2)) $((y * 3 / 2)) $((w * 3 / 2)) $((h * 3 / 2)) 3
}
probe_do A toplevel c "app=$APP" size=400x300 pattern=checker:3 scale=1.5
probe_wait A '^probe: mapped c 400x300$'
wait_for "mapped $APP" >/dev/null
crisp_check mapped

probe_do A maximise c
probe_wait A '^probe: configure c [0-9]+x[0-9]+ \[.*maximized'
configure=$(probe_log A | sed -n 's/^probe: configure c \([0-9]*\)x\([0-9]*\) \[.*maximized.*/\1x\2/p' | tail -1)
probe_wait A "^probe: drawn c ${configure}\$"
probe_sync A
crisp_check maximised

probe_do A unmaximise c
probe_wait A '^probe: drawn c 400x300$'
probe_sync A
crisp_check restored
assert_summary
