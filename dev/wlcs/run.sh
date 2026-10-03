#!/usr/bin/env bash
# Runs wlcs against the wlcs-casement shim inside the jail (built outside first, by
# dev/wlcs/build.sh).
#   --merge  smithay's four CI filters (SelfTest*:FrameSubmission*, XdgOutputV1Test*,
#            */SurfacePointerMotionTest*:RelativePointer*, BadBufferTest*) plus every test in
#            dev/wlcs/parity.txt (the anvil-parity list, A6).
#   --phase  the full suite minus dev/wlcs/expected-failures.txt, excluded through
#            --gtest_filter=-... (never run and ignored: each wlcs timeout costs 10 s, A6).
#
# Exit: 0 pass; 1 a test failed or none ran; 2 could not run safely; 3 artefacts missing (the
# fix is printed); 64 usage.
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
# shellcheck source=dev/lib/jail.sh
source "$root/dev/lib/jail.sh"
require_jail

tier="${1:-}"
case "$tier" in
  --merge|--phase) ;;
  *) echo "usage: dev/wlcs/run.sh --merge|--phase" >&2; exit 64 ;;
esac

wlcs="$root/dev/out/tools/wlcs/wlcs"
shim="$root/dev/out/tools/libwlcs_casement.so"
for f in "$wlcs" "$shim"; do
  [ -x "$f" ] || [ -f "$f" ] || { echo "run.sh: $f is missing" >&2; echo "fix: run dev/wlcs/build.sh outside the jail" >&2; exit 3; }
done

# The names in a list file: one per line, `#` comments and blank lines dropped, joined with ':'.
names() {
  sed -e 's/#.*//' -e 's/[[:space:]]*$//' -e '/^$/d' "$1" | paste -sd: -
}

ci='SelfTest*:FrameSubmission*:XdgOutputV1Test*:*/SurfacePointerMotionTest*:RelativePointer*:BadBufferTest*'
case "$tier" in
  --merge)
    parity=$(names "$root/dev/wlcs/parity.txt")
    [ -n "$parity" ] || { echo "run.sh: dev/wlcs/parity.txt is empty" >&2; exit 2; }
    filter="$ci:$parity" ;;
  --phase)
    skip=$(names "$root/dev/wlcs/expected-failures.txt")
    filter="*${skip:+-$skip}" ;;
esac

# The log goes to a file, is shown whole, and is checked: wlcs exits 0 when a filter matches
# nothing, which would be a green gate that ran no test.
log=$(mktemp)
status=0
"$wlcs" "$shim" --gtest_filter="$filter" >"$log" 2>&1 || status=$?
cat "$log"
if [ "$status" -ne 0 ]; then
  echo "run.sh: wlcs exited $status" >&2
  rm -f "$log"
  exit 1
fi
if ! grep -Eq '^\[  PASSED  \] [1-9][0-9]* test' "$log"; then
  echo "run.sh: wlcs ran no test with filter $filter" >&2
  rm -f "$log"
  exit 1
fi
rm -f "$log"
