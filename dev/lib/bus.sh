# Shared state for the jailed dev scripts, and bus_up: a private dbus-daemon on
# dev/private-bus.conf as both session and system bus, under $JAIL_SCRATCH, exporting
# DBUS_SESSION_BUS_ADDRESS and DBUS_SYSTEM_BUS_ADDRESS.
#
# Owner: P0-T-dev. Sourceable; every other dev/lib file sources this one first.
#
# State (A8): everything a script writes goes under $JAIL_SCRATCH/casement/<run>/ (a jail's
# scratch is its own, so jails run in parallel and the checkout is never written). The run dir
# holds env (a sourceable KEY=VALUE file), the daemons' logs and shots/. `latest` beside the run
# dirs names the newest. $CASEMENT_STATE overrides the run dir.
#
# Tools (A7): built outside the jail by dev/bootstrap.sh into dev/out/tools/bin
# ($CASEMENT_TOOLS overrides the tools dir); nothing here calls cargo or the network.

[ -z "${_CASEMENT_BUS_SH:-}" ] || return 0
_CASEMENT_BUS_SH=1

# shellcheck source=dev/lib/jail.sh
source "$(dirname "${BASH_SOURCE[0]}")/jail.sh"

# casement_root: the checkout.
casement_root() { (cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd); }

# casement_tool NAME: the path of a bootstrapped tool; exit 3 (environment incomplete) says how
# to build it.
casement_tool() {
  local tools="${CASEMENT_TOOLS:-$(casement_root)/dev/out/tools}" path
  for path in "$tools/bin/$1" "$tools/$1"; do
    if [ -x "$path" ]; then echo "$path"; return 0; fi
  done
  echo "casement: tool '$1' is missing from $tools/bin: run dev/bootstrap.sh outside the jail" >&2
  return 3
}

# casement_state_dir: this run's state directory (created). $CASEMENT_STATE wins; else the
# newest under the jail's scratch; else a new one.
casement_state_dir() {
  local base dir
  if [ -n "${CASEMENT_STATE:-}" ]; then mkdir -p "$CASEMENT_STATE"; echo "$CASEMENT_STATE"; return 0; fi
  : "${JAIL_SCRATCH:?casement: JAIL_SCRATCH is not set: run inside ~/desktop/harness/jail.sh}"
  base="$JAIL_SCRATCH/casement"
  mkdir -p "$base"
  if [ -L "$base/latest" ]; then dir=$(readlink "$base/latest"); else
    dir="$base/run-$(date +%Y%m%dT%H%M%S)-$RANDOM"
    mkdir -p "$dir" && ln -sfn "$dir" "$base/latest"
  fi
  mkdir -p "$dir"
  echo "$dir"
}

# casement_new_state: starts a fresh run dir (the next casement_state_dir returns it).
casement_new_state() {
  : "${JAIL_SCRATCH:?casement: JAIL_SCRATCH is not set: run inside ~/desktop/harness/jail.sh}"
  local base="$JAIL_SCRATCH/casement" dir
  mkdir -p "$base"
  dir="$base/run-$(date +%Y%m%dT%H%M%S)-$RANDOM"
  mkdir -p "$dir" && ln -sfn "$dir" "$base/latest"
  CASEMENT_STATE=$dir
  export CASEMENT_STATE
}

# env_set KEY VALUE: records KEY in the run's env file (replacing an earlier value) and exports it.
env_set() {
  local file tmp
  file="$(casement_state_dir)/env"
  tmp="$file.tmp.$$"
  { grep -v "^$1=" "$file" 2>/dev/null || true; printf '%s=%q\n' "$1" "$2"; } >"$tmp"
  mv "$tmp" "$file"
  export "$1=$2"
}

# casement_env_load [ENV_FILE]: exports the run's env (the newest run's, by default).
casement_env_load() {
  local file="${1:-$(casement_state_dir)/env}"
  [ -f "$file" ] || { echo "casement: no session env at $file: start one with dev/up.sh" >&2; return 1; }
  set -a
  # shellcheck source=/dev/null
  source "$file"
  set +a
}

# bus_up: starts the private bus (once per run). Needs the jail.
bus_up() {
  require_jail
  if [ -n "${CASEMENT_BUS_PID:-}" ] && kill -0 "$CASEMENT_BUS_PID" 2>/dev/null; then return 0; fi
  command -v dbus-daemon >/dev/null || { echo "bus_up: dbus-daemon is not installed" >&2; return 3; }
  local state out address pid
  state=$(casement_state_dir)
  out=$(dbus-daemon --config-file "$(casement_root)/dev/private-bus.conf" --fork --print-address=1 --print-pid=1 \
    2>"$state/dbus.log") || { echo "bus_up: dbus-daemon failed: $(cat "$state/dbus.log")" >&2; return 1; }
  address=$(printf '%s\n' "$out" | sed -n 1p)
  pid=$(printf '%s\n' "$out" | sed -n 2p)
  [ -n "$address" ] && [ -n "$pid" ] || { echo "bus_up: dbus-daemon printed no address" >&2; return 1; }
  env_set CASEMENT_BUS_PID "$pid"
  env_set DBUS_SESSION_BUS_ADDRESS "$address"
  env_set DBUS_SYSTEM_BUS_ADDRESS "$address"
}

# bus_down: stops the bus by pid.
bus_down() {
  [ -z "${CASEMENT_BUS_PID:-}" ] || kill "$CASEMENT_BUS_PID" 2>/dev/null || true
}
