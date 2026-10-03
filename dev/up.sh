#!/usr/bin/env bash
# Starts one headless casement session inside the jail: a private dbus-daemon on
# dev/private-bus.conf (session and system bus), fake-logind on it, then
# `casement --backend headless --clock stepped` with --outputs from the arguments; waits for the
# socket and writes the run's env file (WAYLAND_DISPLAY, DBUS_SESSION_BUS_ADDRESS, pids) under
# $JAIL_SCRATCH/casement/<run>/ (A8: no state in the checkout, so jails run in parallel). Calls
# require_jail first. Prints the env file's path on stdout; source it to talk to the session.
#
#   dev/up.sh [--outputs SPEC[;SPEC...]] [--clock wall|stepped] [--dmabuf none|soft] [--renderer R]
#
# Owner: P0-T-dev. Exit codes: 0 up, 1 casement did not come up, 2 outside the jail, 3 a tool is
# missing (run dev/bootstrap.sh outside the jail).
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
# shellcheck source=dev/lib/compositor.sh
source "$root/dev/lib/compositor.sh"
require_jail
start_compositor "$@"
echo "$(casement_state_dir)/env"
