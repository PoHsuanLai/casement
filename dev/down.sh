#!/usr/bin/env bash
# Stops what dev/up.sh started, by the pids in dev/out/<run>/env (never by pattern),
# and removes the run's sockets under $JAIL_SCRATCH. Calls require_jail first.
#
# Owner: P0-T-dev. Frozen at P0-Z2 as a stub: the name and the command line are fixed, the body is not
# written yet. Exit 2 (could not run) until it is.
set -euo pipefail
root=$(cd "$(dirname "$0")" && pwd)
while [ ! -f "$root/dev/lib/jail.sh" ]; do root=$(dirname "$root"); [ "$root" != / ] || { echo "no dev/lib/jail.sh above $0" >&2; exit 3; }; done
# shellcheck source=dev/lib/jail.sh
source "$root/dev/lib/jail.sh"
require_jail
echo "down.sh: not written yet (P0-T-dev)" >&2
exit 2
