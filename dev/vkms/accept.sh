#!/usr/bin/env bash
# USER ONLY. Never run by an agent, never part of any gate, never with sudo (20-FREEZE-PLAN C3).
# P1's exit criterion 1: casement's tty backend on a vkms (virtual KMS) device. Read README.md
# first: the person's one-time setup (typed by the person, with sudo) comes before this.
#
# It refuses unless: a terminal is attached, vkms is the only DRM driver it will touch (the card
# node is picked by driver name; any other card is never opened), exactly one vkms card exists
# and the user can open it, the tools it needs are installed, the casement binary has the vkms
# hooks (see "casement contract"), and the person types the confirmation.
#
# Steps (each prints STEP-PASS or STEP-FAIL; the run ends 0 only if all pass):
#   modeset    casement lights the vkms output and flips pages
#   idle       10 minutes of an idle scene: flips advance at the refresh rate, zero missed vblanks
#   hotplug    a configfs connector connected appears as a second output, disconnected it goes
#   writeback  a frame read back through the writeback connector matches the pixman golden
#   dpms       output power off stops the flips, on restarts them
#   vt         session pause stops the flips and the seat is lent, resume lights it again
#
# Exit: 0 all passed; 1 a step failed; 2 refused (not a terminal, not confirmed, wrong driver,
# run as root); 3 the vkms device or a tool is absent (the gate convention: SKIP-VKMS);
# 4 the casement binary lacks the hooks below; 5 a short run (CASEMENT_VKMS_IDLE_SECONDS under 600).
#
# casement contract (all four flags only with --backend tty; none is a general feature):
#   --drm-node PATH         open only this node, directly (no libseat, no logind), after checking
#                           that its driver is vkms; the real GPU is never opened or enumerated.
#   --kms-report FILE       once a second, append one JSON line:
#       {"flips":N,"missed":M,"seat":"held"|"lent",
#        "outputs":[{"name":"Virtual-1","mode":"1024x768@60000","power":"on"|"off"}]}
#       flips and missed count since start, summed over outputs.
#   --writeback-raw PATH    on SIGHUP, queue the next frame with a writeback job and write the
#                           buffer to PATH and "WIDTH HEIGHT PITCH FOURCC" to PATH.meta.
#   signals                 SIGUSR1 = session Paused, SIGUSR2 = session Activated (the VT switch
#                           simulation; no real VT is touched).
#
# Nothing is written outside the scratch directory this creates (kept when the run fails),
# except the status of the one connector (hotplug1) it connects and disconnects in the vkms
# configfs device the person made. Processes are signalled by the
# PID this script started, never by name.
set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
tools="${CASEMENT_TOOLS:-$root/dev/out/tools}/bin"
idle_seconds="${CASEMENT_VKMS_IDLE_SECONDS:-600}"
device_name="${CASEMENT_VKMS_DEVICE:-casement}"
cfs="/sys/kernel/config/vkms/$device_name"
golden="$root/crates/casement-testkit/tests/golden/scenes/empty-desktop-1024x768"
extra_connector="hotplug1"

refuse() { echo "vkms/accept.sh: refusing: $*" >&2; exit 2; }
absent() { echo "vkms/accept.sh: $*" >&2; exit 3; }
failed=0
pass() { echo "STEP-PASS $1"; }
fail() { echo "STEP-FAIL $1: $2"; failed=1; }

[ "$(id -u)" -ne 0 ] || refuse "do not run as root or with sudo; the one-time setup gives your user access"
if [ ! -t 0 ] || [ ! -t 1 ]; then refuse "stdin and stdout must be a terminal"; fi

# ---- the vkms node, by driver name; nothing else is ever opened ---------------------------
vkms_cards=()
for dir in /sys/class/drm/card[0-9]*; do
  [ -d "$dir" ] || continue
  case "$(basename "$dir")" in card[0-9]*-*) continue ;; esac
  driver=$(basename "$(readlink -f "$dir/device/driver" 2>/dev/null || echo none)")
  if [ "$driver" = vkms ]; then vkms_cards+=("$(basename "$dir")"); fi
done
[ "${#vkms_cards[@]}" -gt 0 ] || absent "no vkms DRM device (README: one-time setup)"
[ "${#vkms_cards[@]}" -eq 1 ] || refuse "more than one vkms card (${vkms_cards[*]}); unload the extra ones"
card="${vkms_cards[0]}"
node="/dev/dri/$card"
[ -c "$node" ] || absent "$node is not a device node"
if [ ! -r "$node" ] || [ ! -w "$node" ]; then
  absent "you cannot open $node read-write (README: the udev rule and the group)"
fi
[ -d "$cfs" ] || absent "no configfs device $cfs (README: one-time setup)"
[ -w "$cfs/connectors/$extra_connector/status" ] || absent "no writable $cfs/connectors/$extra_connector/status (README: one-time setup, chown step)"

# ---- tools --------------------------------------------------------------------------------
for tool in jq wlopm; do
  command -v "$tool" >/dev/null || absent "$tool is not installed (sudo dnf install $tool)"
done
casement="$tools/casement"
compare="$tools/writeback-compare"
[ -x "$casement" ] && [ -x "$compare" ] || absent "build the tools first: dev/bootstrap.sh (needs $casement and $compare)"
usage=$("$casement" --help 2>&1 || true)
for hook in --drm-node --kms-report --writeback-raw; do
  case "$usage" in *"$hook"*) ;; *)
    echo "vkms/accept.sh: this casement has no $hook; the vkms hooks are not built in yet (see the header, casement contract)" >&2
    exit 4 ;;
  esac
done
[ -f "$golden.png" ] || absent "the writeback golden $golden.png is missing (bless it: CASEMENT_BLESS=1 cargo test, see FINDINGS p1-v)"

printf 'This drives casement on the vkms device %s (%s) only. Your real displays are not touched.\nType "vkms only" to continue: ' "$card" "$node"
read -r answer
[ "$answer" = "vkms only" ] || refuse "not confirmed"

# ---- scratch, and the cleanup that kills only what this script started --------------------
scratch=$(mktemp -d "${TMPDIR:-/tmp}/casement-vkms.XXXXXX")
chmod 700 "$scratch"
mkdir -p "$scratch/run" "$scratch/config" "$scratch/state"
chmod 700 "$scratch/run"
casement_pid=""
added_connector=no
cleanup() {
  local code=$?
  if [ -n "$casement_pid" ] && kill -0 "$casement_pid" 2>/dev/null; then
    kill -TERM "$casement_pid" 2>/dev/null || true
    for _ in 1 2 3 4 5 6 7 8 9 10; do kill -0 "$casement_pid" 2>/dev/null || break; sleep 0.5; done
    kill -0 "$casement_pid" 2>/dev/null && kill -KILL "$casement_pid" 2>/dev/null || true
  fi
  if [ "$added_connector" = yes ]; then remove_connector || true; fi
  # A failed run keeps its scratch directory (log, report, writeback images) for a look.
  if [ "$code" -eq 0 ]; then rm -rf "$scratch"; else echo "vkms/accept.sh: kept $scratch" >&2; fi
  exit "$code"
}
trap cleanup EXIT

report="$scratch/report.jsonl"
raw="$scratch/writeback.raw"
log="$scratch/casement.log"

# ---- helpers ------------------------------------------------------------------------------
last() { tail -n 1 "$report" 2>/dev/null || echo '{}'; }
field() { last | jq -r "$1 // empty"; }
alive() { kill -0 "$casement_pid" 2>/dev/null; }

# wait_for SECONDS DESCRIPTION JQ_TEST: the last report line satisfies the test.
wait_for() {
  local limit=$1 what=$2 test=$3 n=0
  while [ "$n" -lt $((limit * 2)) ]; do
    alive || { echo "  casement exited while waiting for: $what" >&2; return 1; }
    if last | jq -e "$test" >/dev/null 2>&1; then return 0; fi
    sleep 0.5; n=$((n + 1))
  done
  echo "  timed out after ${limit}s waiting for: $what; last report: $(last)" >&2
  return 1
}

# stalled SECONDS: the flip counter does not move for SECONDS.
stalled() {
  local before after
  before=$(field .flips); sleep "$1"; after=$(field .flips)
  [ "$before" = "$after" ]
}

# running SECONDS: the flip counter moves.
running() {
  local before after
  before=$(field .flips); sleep "$1"; after=$(field .flips)
  [ "${after:-0}" -gt "${before:-0}" ]
}

# Hotplug is the connector's configfs status: the setup made connector "hotplug1" (with its
# links) and left it disconnected (2); 1 = connected. The kernel makes the files of a connector
# created later root-owned, so the script cannot mkdir one itself without sudo.
set_status() { echo "$1" >"$cfs/connectors/$extra_connector/status"; }
add_connector() { set_status 1; added_connector=yes; }
remove_connector() { set_status 2; added_connector=no; }

# ---- start casement on the vkms node ------------------------------------------------------
echo "vkms/accept.sh: scratch $scratch; node $node"
env -i PATH="$PATH" HOME="$scratch" XDG_RUNTIME_DIR="$scratch/run" XDG_STATE_HOME="$scratch/state" \
  XDG_CONFIG_HOME="$scratch/config" \
  "$casement" --backend tty --drm-node "$node" --config-home "$scratch/config" \
  --kms-report "$report" --writeback-raw "$raw" >"$log" 2>&1 </dev/null &
casement_pid=$!
echo "vkms/accept.sh: casement pid $casement_pid, log $log"

socket=""
for _ in $(seq 1 60); do
  alive || break
  for s in "$scratch"/run/wayland-*; do
    case "$s" in *.lock) continue ;; esac
    [ -S "$s" ] && socket=$(basename "$s")
  done
  [ -z "$socket" ] || break
  sleep 0.5
done
if [ -z "$socket" ]; then
  echo "vkms/accept.sh: casement did not come up; $log:" >&2; tail -20 "$log" >&2
  exit 1
fi
wl() { env -i PATH="$PATH" XDG_RUNTIME_DIR="$scratch/run" WAYLAND_DISPLAY="$socket" "$@"; }

# ---- 1 modeset ----------------------------------------------------------------------------
if wait_for 30 "an output lit with flips" '.outputs | length >= 1 and (.[0].mode | test("^[0-9]+x[0-9]+@")) and .flips > 0'; then
  out_name=$(field '.outputs[0].name'); mode=$(field '.outputs[0].mode')
  if [ "$mode" = "1024x768@60000" ]; then pass "modeset ($out_name $mode)"; else
    echo "  note: mode is $mode, the writeback golden is 1024x768@60000"; pass "modeset ($out_name $mode)"; fi
else
  fail modeset "no lit output with flips"
  exit 1
fi
refresh_hz=$(( $(echo "$mode" | sed 's/.*@//') / 1000 ))

# ---- 2 idle -------------------------------------------------------------------------------
echo "vkms/accept.sh: idle for ${idle_seconds}s ..."
flips0=$(field .flips); missed0=$(field .missed); t0=$(date +%s)
sleep "$idle_seconds"
t1=$(date +%s)
flips1=$(field .flips); missed1=$(field .missed)
elapsed=$((t1 - t0)); expected=$((elapsed * refresh_hz))
if ! alive; then fail idle "casement died"
elif [ $((missed1 - missed0)) -ne 0 ]; then fail idle "$((missed1 - missed0)) missed vblanks in ${elapsed}s"
elif [ $(( (flips1 - flips0) * 100 )) -lt $((expected * 90)) ]; then
  fail idle "only $((flips1 - flips0)) flips in ${elapsed}s, expected about $expected at ${refresh_hz} Hz"
else pass "idle (${elapsed}s, $((flips1 - flips0)) flips, 0 missed)"; fi

# ---- 3 hotplug ----------------------------------------------------------------------------
if add_connector && wait_for 15 "a second output" '.outputs | length == 2'; then
  if remove_connector && wait_for 15 "back to one output" '.outputs | length == 1' && alive; then
    pass "hotplug (add, then remove)"
  else fail hotplug "the output did not go away"; fi
else fail hotplug "the connected connector never appeared"; remove_connector 2>/dev/null || true; fi

# ---- 4 writeback --------------------------------------------------------------------------
rm -f "$raw" "$raw.meta"
kill -HUP "$casement_pid"
n=0; while [ ! -s "$raw.meta" ] && [ "$n" -lt 40 ]; do sleep 0.5; n=$((n + 1)); done
if [ ! -s "$raw.meta" ]; then fail writeback "no writeback frame within 20s"
else
  read -r wb_w wb_h wb_pitch wb_fourcc <"$raw.meta"
  if "$compare" "$raw" "$wb_w" "$wb_h" "$wb_pitch" "$wb_fourcc" "$golden" "$scratch/golden-out" 3 2000; then
    pass "writeback (${wb_w}x${wb_h} $wb_fourcc matches the pixman golden)"
  else
    fail writeback "differs from the golden; actual and diff images in $scratch/golden-out"
  fi
fi

# ---- 5 dpms -------------------------------------------------------------------------------
out_name=$(field '.outputs[0].name')
if wl wlopm --off "$out_name" >/dev/null && wait_for 10 "power off" '.outputs[0].power == "off"' && stalled 3; then
  if wl wlopm --on "$out_name" >/dev/null && wait_for 10 "power on" '.outputs[0].power == "on"' && running 3; then
    pass "dpms (off stops the flips, on restarts them)"
  else fail dpms "on did not restart the flips"; fi
else fail dpms "off did not stop the flips"; wl wlopm --on "$out_name" >/dev/null 2>&1 || true; fi

# ---- 6 vt (session pause and resume, simulated) ---------------------------------------------
kill -USR1 "$casement_pid"
if wait_for 10 "seat lent" '.seat == "lent"' && stalled 3; then
  kill -USR2 "$casement_pid"
  if wait_for 15 "seat held again with output lit" '.seat == "held" and (.outputs | length == 1) and .outputs[0].power == "on"' && running 3 && alive; then
    pass "vt (pause stops the flips, resume lights the output)"
  else fail vt "resume did not light the output"; fi
else fail vt "pause did not lend the seat and stop the flips"; kill -USR2 "$casement_pid" 2>/dev/null || true; fi

echo "vkms/accept.sh: final report: $(last)"
if [ "$failed" -ne 0 ]; then echo "VKMS ACCEPT FAILED (log and images are kept in $scratch)"; exit 1; fi
if [ "$idle_seconds" -lt 600 ]; then echo "VKMS PARTIAL: idle was ${idle_seconds}s, the criterion is 600s"; exit 5; fi
echo "VKMS ACCEPT PASSED"
