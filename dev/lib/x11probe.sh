# X11 clients for the XWayland suite: `casement-x11probe session` processes driven like
# `casement-probe` ones (dev/lib/probe.sh; probe_wait, probe_has, probe_count, probe_log and
# probe_stop work on their names, which carry the prefix x11-). Run only by the user or the
# jailed accept runner, never by the gate.
#
#   x11_compositor [start_compositor ARGS...]
#                         start_compositor with xwayland-satellite configured (through a wrapper
#                         that adds `-glamor none`, SP6) and the display it chose in X11_DISPLAY
#   x11_start NAME        start an X11 client named NAME on X11_DISPLAY and wait until it is ready
#   x11_do NAME COMMAND...  send one command line (grammar: x11probe/command.rs), wait until run;
#                         fails when the client reported an error
#   x11_wait NAME REGEX [MS] | x11_has NAME REGEX | x11_count NAME REGEX | x11_log NAME
#                         probe_wait/has/count/log on the client
#   x11_satellite_pid     the pid of casement's satellite child, or nothing
#
# Owner: P1-X1. Sourceable after dev/lib/accept.sh.

x11_compositor() {
  local state wrapper satellite
  satellite=$(casement_tool xwayland-satellite) || return 3
  if [ -z "${CASEMENT_STATE:-}" ]; then casement_new_state; _CASEMENT_STATE_OWNED=1; fi
  state=$(casement_state_dir)
  wrapper="$state/xwayland-satellite-wrapper"
  # The wrapper keeps the inherited listening descriptors (exec) and adds the software flag.
  printf '#!/bin/sh\nexec %q "$@" -glamor none\n' "$satellite" >"$wrapper"
  chmod +x "$wrapper"
  start_compositor "$@" --xwayland satellite --xwayland-satellite "$wrapper" || return $?
  X11_DISPLAY=$(sed -n 's/^casement: X11 display //p' "$state/casement.log" | head -1)
  if [ -z "$X11_DISPLAY" ]; then
    echo "x11_compositor: casement did not name an X11 display; $state/casement.log:" >&2
    tail -20 "$state/casement.log" >&2
    return 1
  fi
  export X11_DISPLAY
}

x11_start() {
  require_jail
  local name=${1:?x11_start NAME} dir tool pid
  dir=$(_probe_dir)
  tool=$(casement_tool casement-x11probe) || return 3
  rm -f "$dir/x11-$name.in" "$dir/x11-$name.log"
  mkfifo "$dir/x11-$name.in"
  DISPLAY=$X11_DISPLAY "$tool" session <>"$dir/x11-$name.in" >"$dir/x11-$name.log" 2>&1 &
  pid=$!
  echo "$pid" >"$dir/x11-$name.pid"
  probe_wait "x11-$name" '^x11: ready$' 15000
}

x11_do() {
  local name=${1:?x11_do NAME COMMAND} dir done_before errors_before
  shift
  dir=$(_probe_dir)
  done_before=$(probe_count "x11-$name" '^x11: done ')
  errors_before=$(probe_count "x11-$name" '^x11: error ')
  printf '%s\n' "$*" >"$dir/x11-$name.in"
  probe_wait "x11-$name" "^x11: done $((done_before + 1))\$" 10000 || return 1
  if [ "$(probe_count "x11-$name" '^x11: error ')" != "$errors_before" ]; then
    echo "x11_do: $name refused '$*':" >&2
    grep '^x11: error ' "$(_probe_log "x11-$name")" | tail -1 >&2
    return 1
  fi
}

x11_wait() { probe_wait "x11-$1" "${@:2}"; }
x11_has() { probe_has "x11-$1" "$2"; }
x11_count() { probe_count "x11-$1" "$2"; }
x11_log() { probe_log "x11-$1"; }

x11_satellite_pid() { pgrep -P "$CASEMENT_PID" -f xwayland-satellite | head -1 || true; }
