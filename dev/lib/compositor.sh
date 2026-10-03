# start_compositor [--outputs SPEC] [--clock wall|stepped] [--dmabuf none|soft] [--renderer R]
#                  [casement ARGS...]
#   A fresh run: the private bus (bus_up), fake-logind on it as the system bus, then
#   `casement --backend headless` in the current jail, waiting for its socket. Exports and records
#   in the run's env file: WAYLAND_DISPLAY (an absolute path in the jail's /run/user/$UID),
#   DBUS_SESSION_BUS_ADDRESS, DBUS_SYSTEM_BUS_ADDRESS, XDG_RUNTIME_DIR, CASEMENT_PID,
#   CASEMENT_LOGIND_PID, CASEMENT_BUS_PID, CASEMENT_STATE. Defaults: one 1920x1080 output at
#   scale 1, the stepped clock (virtual time: `casementctl debug clock`), no dma-buf.
# stop_compositor: stops what start_compositor started, each by the pid in the env file (never by
#   name), the bus last.
#
# Owner: P0-T-dev. Sourceable. Exit codes follow 20-FREEZE-PLAN 6.1: 2 outside the jail, 3 a
# tool is missing (run dev/bootstrap.sh), 1 the compositor did not come up (its log is printed).

# shellcheck source=dev/lib/bus.sh
source "$(dirname "${BASH_SOURCE[0]}")/bus.sh"

# How long start_compositor waits for the socket, in tenths of a second.
CASEMENT_START_TENTHS="${CASEMENT_START_TENTHS:-150}"

# _wayland_sockets DIR: the wayland-* sockets (not their .lock files) in DIR, one per line.
_wayland_sockets() {
  local path
  for path in "$1"/wayland-*; do
    case "$path" in *.lock) continue ;; esac
    if [ -S "$path" ]; then echo "$path"; fi
  done
  return 0
}

start_compositor() {
  require_jail
  local outputs="" clock=stepped dmabuf=none renderer="" extra=() state bin logind sockets_before
  while [ $# -gt 0 ]; do
    case "$1" in
      --outputs) outputs=${2:?--outputs needs a value}; shift 2 ;;
      --clock) clock=${2:?--clock needs a value}; shift 2 ;;
      --dmabuf) dmabuf=${2:?--dmabuf needs a value}; shift 2 ;;
      --renderer) renderer=${2:?--renderer needs a value}; shift 2 ;;
      *) extra+=("$1"); shift ;;
    esac
  done
  bin=$(casement_tool casement) || return 3
  logind=$(casement_tool fake-logind) || return 3
  if [ -z "${CASEMENT_STATE:-}" ]; then casement_new_state; _CASEMENT_STATE_OWNED=1; fi
  state=$(casement_state_dir)
  : "${XDG_RUNTIME_DIR:?start_compositor: XDG_RUNTIME_DIR is not set}"
  env_set XDG_RUNTIME_DIR "$XDG_RUNTIME_DIR"
  bus_up || return $?

  # fake-logind sits on the private bus as the system bus (Inhibit, Lock, TakeDevice).
  "$logind" >"$state/fake-logind.log" 2>&1 </dev/null &
  env_set CASEMENT_LOGIND_PID "$!"

  sockets_before=$(_wayland_sockets "$XDG_RUNTIME_DIR")
  local args=(--backend headless --clock "$clock" --dmabuf "$dmabuf" --config-home "$state/config")
  [ -z "$outputs" ] || args+=(--outputs "$outputs")
  [ -z "$renderer" ] || args+=(--renderer "$renderer")
  "$bin" "${args[@]}" "${extra[@]}" >"$state/casement.log" 2>&1 </dev/null &
  env_set CASEMENT_PID "$!"
  env_set CASEMENT_CLOCK "$clock"

  local tenths=0 socket=""
  while [ "$tenths" -lt "$CASEMENT_START_TENTHS" ]; do
    if ! _alive "$CASEMENT_PID"; then
      echo "start_compositor: casement exited before its socket appeared; $state/casement.log:" >&2
      tail -20 "$state/casement.log" >&2
      stop_compositor
      return 1
    fi
    socket=$(comm -13 <(printf '%s\n' "$sockets_before" | sort) <(_wayland_sockets "$XDG_RUNTIME_DIR" | sort) | head -1)
    [ -z "$socket" ] || break
    sleep 0.1
    tenths=$((tenths + 1))
  done
  if [ -z "$socket" ]; then
    echo "start_compositor: no wayland socket after $((CASEMENT_START_TENTHS / 10)) s; $state/casement.log:" >&2
    tail -20 "$state/casement.log" >&2
    stop_compositor
    return 1
  fi
  env_set WAYLAND_DISPLAY "$socket"
  echo "casement is up: WAYLAND_DISPLAY=$socket (env: $state/env)" >&2
}

# _alive PID: the process exists and is not a zombie waiting to be reaped.
_alive() {
  [ -n "$1" ] && kill -0 "$1" 2>/dev/null && [ "$(awk '{print $3}' "/proc/$1/stat" 2>/dev/null)" != Z ]
}

# _stop_pid PID: a polite TERM, then KILL after two seconds. Only ever the pid it is given.
_stop_pid() {
  local pid=$1 tenths=0
  _alive "$pid" || return 0
  kill "$pid" 2>/dev/null || return 0
  while _alive "$pid"; do
    if [ "$tenths" -ge 20 ]; then kill -9 "$pid" 2>/dev/null || true; break; fi
    sleep 0.1
    tenths=$((tenths + 1))
  done
  wait "$pid" 2>/dev/null || true
}

stop_compositor() {
  require_jail
  local file state
  state=$(casement_state_dir)
  file="$state/env"
  if [ -f "$file" ]; then
    casement_env_load "$file"
    _stop_pid "${CASEMENT_PID:-}"
    _stop_pid "${CASEMENT_LOGIND_PID:-}"
    _stop_pid "${CASEMENT_BUS_PID:-}"
    [ -z "${WAYLAND_DISPLAY:-}" ] || rm -f "$WAYLAND_DISPLAY" "$WAYLAND_DISPLAY.lock"
    : >"$state/stopped"
  fi
  unset CASEMENT_PID CASEMENT_LOGIND_PID CASEMENT_BUS_PID
  # A run dir start_compositor made is finished with; one the caller named stays theirs.
  if [ -n "${_CASEMENT_STATE_OWNED:-}" ]; then unset CASEMENT_STATE _CASEMENT_STATE_OWNED; fi
}
