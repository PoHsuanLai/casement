#!/usr/bin/env bash
# Acceptance script 'map-resize' (20-FREEZE-PLAN §4.3, P0): runs against dev/up.sh's jailed casement,
# prints PASS|SKIP|FAIL per step and exits with the plan's codes. Calls require_jail first.
#
# Owner: P0-H2. Frozen at P0-Z2 as a stub: the name and the command line are fixed, the body is not
# written yet. Exit 2 (could not run) until it is.
set -euo pipefail
root=$(cd "$(dirname "$0")" && pwd)
while [ ! -f "$root/dev/lib/jail.sh" ]; do root=$(dirname "$root"); [ "$root" != / ] || { echo "no dev/lib/jail.sh above $0" >&2; exit 3; }; done
# shellcheck source=dev/lib/jail.sh
source "$root/dev/lib/jail.sh"
require_jail
echo "map-resize.sh: not written yet (P0-H2)" >&2
exit 2
