# What every dev/accept script starts with: `source "$root/dev/lib/accept.sh"` inside the jail.
# Sources the other libraries, installs the exit trap (test clients first, then the compositor),
# and gives the small helpers the scripts share. Owner: P0-accept. Sourceable.
#
#   ctl ARGS...              casementctl
#   step N                   N vblanks of the stepped clock
#   settle                   step until no window is moving
#   win FIELD CLASS          one field of a window in `debug windows`: frame (x y w h), active, ...
#   win_rect CLASS           the window's frame as `x y w h`
#   output_json              the first output as JSON (`casementctl output list`)
#   plug_devices             plug a virtual keyboard and pointer (clients bind seats after this)
#   click X Y                move there, press and release the left button
#   expect_line NAME CMD...  see assert_ok; the same with a reason printed on failure

# shellcheck source=dev/lib/jail.sh
source "$root/dev/lib/jail.sh"
require_jail
# shellcheck source=dev/lib/packages.sh
source "$root/dev/lib/packages.sh"
# shellcheck source=dev/lib/compositor.sh
source "$root/dev/lib/compositor.sh"
# shellcheck source=dev/lib/assert.sh
source "$root/dev/lib/assert.sh"
# shellcheck source=dev/lib/inject.sh
source "$root/dev/lib/inject.sh"
# shellcheck source=dev/lib/wait.sh
source "$root/dev/lib/wait.sh"
# shellcheck source=dev/lib/probe.sh
source "$root/dev/lib/probe.sh"
# shellcheck source=dev/lib/shot.sh
source "$root/dev/lib/shot.sh"

accept_cleanup() {
  probe_stop_all
  stop_compositor
}
trap accept_cleanup EXIT

command -v jq >/dev/null || { echo "accept: jq is not installed: $(pkg_hint jq)" >&2; exit 2; }

ctl() { "$(casement_tool casementctl)" "$@"; }

step() { ctl debug clock step "${1:-1}"; }

settle() { ctl debug wait settled --timeout "${1:-10000}" >/dev/null; }

win() {
  ctl debug windows | jq -r --arg class "$2" '[.[] | select(.class == $class)][0] | .'"$1"' | if type == "array" then join(" ") else . end'
}

win_rect() { win frame "$1"; }

output_json() { ctl output list | jq -c '.[0]'; }

plug_devices() { inject "device plug keyboard" "device plug pointer" >/dev/null; }

click() { inject "move $1 $2" "press left" "release left" >/dev/null; }
