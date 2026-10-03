#!/usr/bin/env bash
# Runs wlcs against the wlcs-casement shim inside the jail (built outside first).
#   --merge  smithay's four CI filters (SelfTest*:FrameSubmission*, XdgOutputV1Test*,
#            */SurfacePointerMotionTest*:RelativePointer*, BadBufferTest*) plus every test in
#            dev/wlcs/parity.txt (the anvil-parity list, A6).
#   --phase  the full suite minus dev/wlcs/expected-failures.txt, excluded through
#            --gtest_filter=-... (never run and ignored: each wlcs timeout costs 10 s, A6).
#
# Owner: P0-T-wlcs. Frozen at P0-Z2 as a stub: exit 2 (could not run) until it is written.
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
# shellcheck source=dev/lib/jail.sh
source "$root/dev/lib/jail.sh"
require_jail
case "${1:-}" in
  --merge|--phase) ;;
  *) echo "usage: dev/wlcs/run.sh --merge|--phase" >&2; exit 64 ;;
esac
echo "run.sh: not written yet (P0-T-wlcs)" >&2
exit 2
