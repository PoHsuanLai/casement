#!/usr/bin/env bash
# The acceptance runner: dev/accept.sh [--jobs N] [--build] [script...]. --build builds
# every artefact outside the jail first (A7: nothing inside the jail calls cargo or the network);
# each dev/accept/*.sh then runs in its own jail (parallel, N at a time, state under $JAIL_SCRATCH,
# A8) and the runner prints one PASS|SKIP|FAIL line per script with the plan's exit codes.
#
# Owner: P0-T-dev. Frozen at P0-Z2 as a stub: the name and the command line are fixed, the body is not
# written yet. Exit 2 (could not run) until it is.
set -euo pipefail
echo "accept.sh: not written yet (P0-T-dev)" >&2
exit 2
