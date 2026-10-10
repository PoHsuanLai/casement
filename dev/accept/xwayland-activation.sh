#!/usr/bin/env bash
# Acceptance script 'xwayland-activation' (P1-X1): activation across the X11 bridge. Clicking an
# X11 window focuses it (FocusIn); clicking a Wayland window takes the focus (FocusOut for the X11
# window); an X11 client's _NET_ACTIVE_WINDOW request, sent while the other window has focus,
# is a request the compositor may refuse (focus-stealing prevention), so the script records the
# outcome and asserts only that the compositor and X agree about who is active afterwards.
# Exit: 0 pass, 1 fail, 2 could not run, 3 environment.
#
# Owner: P1-X1.
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
# shellcheck source=dev/lib/accept.sh
source "$root/dev/lib/accept.sh"
# shellcheck source=dev/lib/x11probe.sh
source "$root/dev/lib/x11probe.sh"

X=XProbeActive
W=org.example.native
x11_compositor --clock stepped
plug_devices
x11_start X
probe_start A
x11_do X map w "class=$X" size=300x200
wait_for "mapped $X" >/dev/null
probe_do A toplevel n "app=$W" size=300x200 pattern=gradient
probe_wait A '^probe: mapped n '
wait_for "mapped $W" >/dev/null
settle

read -r xx xy _ _ <<<"$(win_rect $X)"
click $((xx + 20)) $((xy + 20))
x11_wait X '^x11: event w focus-in'
settle
assert_eq "clicking the X11 window makes it active" "true" "$(win active $X)"

read -r wx wy _ _ <<<"$(win_rect $W)"
click $((wx + 20)) $((wy + 20))
x11_wait X '^x11: event w focus-out'
settle
assert_eq "clicking the Wayland window makes it active" "true" "$(win active $W)"
assert_eq "the X11 window is not active" "false" "$(win active $X)"

x11_do X activate w
x11_do X sync
settle
active_x=$(win active $X)
echo "fact: after _NET_ACTIVE_WINDOW from the X11 client, X11 window active=$active_x"
if [ "$active_x" = true ]; then
  x11_wait X '^x11: event w focus-in'
  assert_eq "the Wayland window lost it" "false" "$(win active $W)"
else
  assert_eq "refused: the Wayland window is still active" "true" "$(win active $W)"
fi
assert_summary
