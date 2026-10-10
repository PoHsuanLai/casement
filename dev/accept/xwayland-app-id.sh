#!/usr/bin/env bash
# Acceptance script 'xwayland-app-id' (P1-X1): X11 clients through xwayland-satellite started on
# demand. casement exports DISPLAY and holds the sockets with no satellite running; the first X11
# client starts it; the client's window is a Wayland window whose app id is its WM_CLASS class
# (not satellite's own name); closing it ends the window. Exit: 0 pass, 1 fail, 2 could not run,
# 3 environment. Run by the user or the jailed accept runner, never by the gate.
#
# Owner: P1-X1.
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
# shellcheck source=dev/lib/accept.sh
source "$root/dev/lib/accept.sh"
# shellcheck source=dev/lib/x11probe.sh
source "$root/dev/lib/x11probe.sh"

CLASS=XProbeAppId
x11_compositor --clock stepped
plug_devices
assert_ok "casement exported an X11 display ($X11_DISPLAY)" test -n "$X11_DISPLAY"
assert_eq "no satellite runs before the first X11 client" "" "$(x11_satellite_pid)"

x11_start X
assert_ok "the first client started satellite" test -n "$(x11_satellite_pid)"
x11_do X map w "class=$CLASS" instance=xprobe size=300x200
x11_wait X '^x11: event w mapped'
wait_for "mapped $CLASS" >/dev/null
settle
assert_eq "the Wayland window's app id is the WM_CLASS class" "$CLASS" \
  "$(ctl debug windows | jq -r --arg c "$CLASS" '[.[] | select(.class == $c)][0].class // "none"')"
assert_eq "one window, none named after satellite" "0" \
  "$(ctl debug windows | jq '[.[] | select(.class == "xwayland-satellite")] | length')"
read -r _ _ w h <<<"$(win_rect $CLASS)"
assert_ok "the window has a size ($w x $h)" test "$w" -gt 0 -a "$h" -gt 0

x11_do X close w
wait_for "gone $CLASS" >/dev/null
assert_eq "closed: no window is left" "0" "$(ctl debug windows | jq length)"
assert_summary
