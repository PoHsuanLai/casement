# inject LINE...: sends inject lines (11 section 4.1 grammar, a superset of shell-host's
# fake-input.sh: wheel value120 and axis_source explicit) through `casementctl debug inject`, and
# prints the `ok <line>` each one is answered with. With no arguments the lines come from stdin,
# so shell-host's `printf 'move 300 200\npress left\n' | fake-input.sh` becomes
# `printf ... | inject`. Exit 1 when casement refused a line (nothing was injected then).
# The grammar is documented in crates/casement-ipc/src/inject.rs.
#
# Owner: P0-T-dev. Sourceable. Reads the session from $DBUS_SESSION_BUS_ADDRESS, or the newest
# run's env file.

# shellcheck source=dev/lib/bus.sh
source "$(dirname "${BASH_SOURCE[0]}")/bus.sh"

# _session: makes sure the bus address is in the environment.
_session() {
  [ -n "${DBUS_SESSION_BUS_ADDRESS:-}" ] || casement_env_load
}

inject() {
  require_jail
  _session || return 1
  local ctl
  ctl=$(casement_tool casementctl) || return 3
  if [ $# -gt 0 ]; then
    printf '%s\n' "$@" | "$ctl" debug inject
  else
    "$ctl" debug inject
  fi
}
