#!/usr/bin/env bash
# Starts one headless casement session inside the jail: a private dbus-daemon on
# dev/private-bus.conf (session and system bus), fake-logind on it, then
# `casement --backend headless --clock stepped` with --outputs from the arguments; waits for the
# socket and writes dev/out/<run>/env (WAYLAND_DISPLAY, DBUS_SESSION_BUS_ADDRESS, pids). All state
# lives under $JAIL_SCRATCH (A8), so jails run in parallel. Calls require_jail first.
#
# Owner: P0-T-dev. Frozen at P0-Z2 as a stub: the name and the command line are fixed, the body is not
# written yet. Exit 2 (could not run) until it is.
set -euo pipefail
root=$(cd "$(dirname "$0")" && pwd)
while [ ! -f "$root/dev/lib/jail.sh" ]; do root=$(dirname "$root"); [ "$root" != / ] || { echo "no dev/lib/jail.sh above $0" >&2; exit 3; }; done
# shellcheck source=dev/lib/jail.sh
source "$root/dev/lib/jail.sh"
require_jail
echo "up.sh: not written yet (P0-T-dev)" >&2
exit 2
