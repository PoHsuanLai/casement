# Test clients for the acceptance scripts: `casement-probe session` processes, each driven through
# a fifo (commands in, one line each) and read through its log (the client's protocol transcript,
# `< object#id.event(args)`, and its own `probe: ...` notes). Scripts wait on log lines, with a
# bound, never on sleeps: see crates/casement-testclient/src/bin/probe/command.rs for the commands.
#
#   probe_start NAME            start a client named NAME and wait until it has connected
#   probe_do NAME COMMAND...    send one command line and wait until the client has run it;
#                               fails (and prints the log) when the client reported an error
#   probe_wait NAME REGEX [MS]  wait (default 5000 ms) for a log line matching the ERE; on a
#                               timeout print the log's tail and fail
#   probe_has NAME REGEX        succeed when some line matches
#   probe_count NAME REGEX      print how many lines match
#   probe_sync NAME             a wl_display.sync round trip: every event the compositor sent
#                               this client before now is in the log when it returns, so a
#                               `! probe_has` after it is a real "nothing arrived"
#   probe_log NAME              print the log
#   probe_stop NAME | probe_stop_all
#
# Owner: P0-accept. Sourceable; needs dev/lib/bus.sh (casement_state_dir, casement_tool).

# shellcheck source=dev/lib/bus.sh
source "$(dirname "${BASH_SOURCE[0]}")/bus.sh"

_probe_dir() { local dir; dir="$(casement_state_dir)/probe"; mkdir -p "$dir"; echo "$dir"; }
_probe_log() { echo "$(_probe_dir)/$1.log"; }

probe_start() {
  require_jail
  local name=${1:?probe_start NAME} dir tool pid
  dir=$(_probe_dir)
  tool=$(casement_tool casement-probe) || return 3
  rm -f "$dir/$name.in" "$dir/$name.log"
  mkfifo "$dir/$name.in"
  # <> keeps the fifo open read-write: the client never sees an end of file between commands.
  "$tool" session <>"$dir/$name.in" >"$dir/$name.log" 2>&1 &
  pid=$!
  echo "$pid" >"$dir/$name.pid"
  probe_wait "$name" '^probe: ready$' 10000
}

probe_wait() {
  local name=${1:?probe_wait NAME REGEX} regex=${2:?probe_wait NAME REGEX} bound=${3:-5000} log tenths=0
  log=$(_probe_log "$name")
  local limit=$((bound / 50))
  while [ "$tenths" -le "$limit" ]; do
    if grep -Eq -- "$regex" "$log" 2>/dev/null; then return 0; fi
    sleep 0.05
    tenths=$((tenths + 1))
  done
  echo "probe_wait: no line matching /$regex/ in $name's log after ${bound} ms; its tail:" >&2
  tail -25 "$log" 2>/dev/null | sed 's/^/    /' >&2
  return 1
}

probe_has() { grep -Eq -- "${2:?probe_has NAME REGEX}" "$(_probe_log "${1:?probe_has NAME REGEX}")" 2>/dev/null; }

probe_count() { grep -Ec -- "${2:?probe_count NAME REGEX}" "$(_probe_log "${1:?probe_count NAME REGEX}")" 2>/dev/null || true; }

probe_do() {
  local name=${1:?probe_do NAME COMMAND} dir done_before errors_before
  shift
  dir=$(_probe_dir)
  done_before=$(probe_count "$name" '^probe: done ')
  errors_before=$(probe_count "$name" '^probe: error ')
  printf '%s\n' "$*" >"$dir/$name.in"
  probe_wait "$name" "^probe: done $((done_before + 1))\$" 10000 || return 1
  if [ "$(probe_count "$name" '^probe: error ')" != "$errors_before" ]; then
    echo "probe_do: $name refused '$*':" >&2
    grep '^probe: error ' "$(_probe_log "$name")" | tail -1 >&2
    return 1
  fi
}

probe_sync() {
  local name=${1:?probe_sync NAME} before
  before=$(probe_count "$name" '^probe: synced ')
  probe_do "$name" sync || return 1
  probe_wait "$name" "^probe: synced $((before + 1))\$" 10000
}

probe_log() { cat "$(_probe_log "${1:?probe_log NAME}")"; }

probe_stop() {
  local name=${1:?probe_stop NAME} dir pid
  dir=$(_probe_dir)
  [ -f "$dir/$name.pid" ] || return 0
  pid=$(cat "$dir/$name.pid")
  rm -f "$dir/$name.pid"
  if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
    kill "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
  fi
}

probe_stop_all() {
  local dir file
  dir=$(_probe_dir)
  for file in "$dir"/*.pid; do
    [ -e "$file" ] || continue
    probe_stop "$(basename "$file" .pid)"
  done
}
