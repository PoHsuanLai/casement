#!/usr/bin/env bash
# Acceptance script 'globals' (20-FREEZE-PLAN §4.3, P0): the globals a public client sees on
# casement's headless session are exactly the golden list
# crates/casement-testkit/tests/golden/globals.txt: one output, two outputs (wl_output once each),
# and with --dmabuf soft (the extra zwp_linux_dmabuf_v1 row). Prints PASS|SKIP|FAIL per step and
# exits with the plan's codes. Calls require_jail first.
#
# Owner: P0-T-dev.
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
while [ ! -f "$root/dev/lib/jail.sh" ]; do root=$(dirname "$root"); [ "$root" != / ] || { echo "no dev/lib/jail.sh above $0" >&2; exit 3; }; done
# shellcheck source=dev/lib/jail.sh
source "$root/dev/lib/jail.sh"
require_jail
# shellcheck source=dev/lib/compositor.sh
source "$root/dev/lib/compositor.sh"
# shellcheck source=dev/lib/assert.sh
source "$root/dev/lib/assert.sh"

golden="$root/crates/casement-testkit/tests/golden/globals.txt"
[ -f "$golden" ] || { echo "globals.sh: $golden is missing" >&2; exit 3; }
trap stop_compositor EXIT

# expected OUTPUTS [EXTRA_ROW...]: the golden's `interface version` rows, wl_output once per output,
# plus the extra rows, sorted.
expected() {
  local outputs=$1; shift
  {
    awk '!/^#/ && NF >= 2 { print $1, $2 }' "$golden" | awk -v n="$outputs" '$1 == "wl_output" { for (i = 0; i < n; i++) print; next } { print }'
    printf '%s\n' "$@"
  } | sed '/^$/d' | sort
}

# actual: the `interface version` rows the session advertises, sorted. casement-probe's JSON lines
# when it answers, else wayland-info's text.
actual() {
  local rows=""
  if probe=$(casement_tool casement-probe 2>/dev/null) && out=$(timeout 20 "$probe" globals 2>/dev/null); then
    rows=$(printf '%s\n' "$out" | python3 -c '
import json, sys
for line in sys.stdin:
    line = line.strip()
    if not line:
        continue
    try:
        row = json.loads(line)
    except ValueError:
        continue
    name = row.get("interface") or row.get("iface")
    version = row.get("version")
    if name and version is not None:
        print(name, version)
' 2>/dev/null || true)
  fi
  if [ -z "$rows" ]; then
    command -v wayland-info >/dev/null || { echo "globals.sh: neither casement-probe nor wayland-info answered" >&2; return 3; }
    rows=$(timeout 20 wayland-info 2>/dev/null | sed -n "s/^interface: '\([^']*\)', *version: *\([0-9]*\).*/\1 \2/p")
  fi
  printf '%s\n' "$rows" | sed '/^$/d' | sort
}

# check NAME OUTPUTS [EXTRA_ROW...] -- [start_compositor args]: starts a session, compares, stops it.
check() {
  local name=$1 outputs=$2 extra=() args=()
  shift 2
  while [ $# -gt 0 ] && [ "$1" != -- ]; do extra+=("$1"); shift; done
  [ $# -eq 0 ] || shift
  args=("$@")
  if ! start_compositor "${args[@]}"; then assert_eq "$name: session starts" up down; stop_compositor; return 0; fi
  local want got
  want=$(expected "$outputs" "${extra[@]}")
  got=$(actual) || { assert_skip "$name" "no tool could list the globals"; stop_compositor; return 0; }
  if [ "$want" = "$got" ]; then
    assert_eq "$name" same same
  else
    assert_eq "$name" "same" "different:
$(diff <(printf '%s\n' "$want") <(printf '%s\n' "$got") | sed 's/^/    /')"
  fi
  stop_compositor
}

check "one output matches the golden" 1 -- --clock stepped
check "two outputs: wl_output once each" 2 -- --clock stepped \
  --outputs "HEADLESS-1:1920x1080@60000:120:0,0;HEADLESS-2:1280x720@60000:120:1920,0"
check "--dmabuf soft adds zwp_linux_dmabuf_v1" 1 "zwp_linux_dmabuf_v1 6" -- --clock stepped --dmabuf soft
assert_summary
