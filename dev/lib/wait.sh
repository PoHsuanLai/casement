# wait_for PREDICATE [TIMEOUT_MS]: `casementctl debug wait` (mapped APP, focused APP, gone APP,
# frames N [OUTPUT], settled), in session time: on the stepped clock the compositor steps itself
# until the predicate holds or the bound passes. PREDICATE is one argument and may hold spaces:
# `wait_for "mapped foot" 5000`. TIMEOUT_MS defaults to 10000. Exit 1 on timeout, with the scene
# dump on stderr.
#
# Owner: P0-T-dev. Sourceable.

# shellcheck source=dev/lib/inject.sh
source "$(dirname "${BASH_SOURCE[0]}")/inject.sh"

wait_for() {
  require_jail
  [ $# -ge 1 ] && [ $# -le 2 ] || { echo "usage: wait_for PREDICATE [TIMEOUT_MS]" >&2; return 64; }
  _session || return 1
  local ctl predicate=$1 timeout=${2:-10000}
  ctl=$(casement_tool casementctl) || return 3
  # The predicate is deliberately word-split: `mapped foot` is two words to casementctl.
  # shellcheck disable=SC2086
  "$ctl" debug wait $predicate --timeout "$timeout"
}
