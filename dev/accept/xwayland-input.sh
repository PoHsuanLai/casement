#!/usr/bin/env bash
# Acceptance script 'xwayland-input' (P1-X1): pointer and keyboard reach an X11 window through
# xwayland-satellite. A click inside the window is a ButtonPress with the position inside the
# window; moving is MotionNotify; after the click the window has focus, so typed keys arrive as
# KeyPress. Exit: 0 pass, 1 fail, 2 could not run, 3 environment.
#
# Owner: P1-X1.
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
# shellcheck source=dev/lib/accept.sh
source "$root/dev/lib/accept.sh"
# shellcheck source=dev/lib/x11probe.sh
source "$root/dev/lib/x11probe.sh"

CLASS=XProbeInput
x11_compositor --clock stepped
plug_devices
x11_start X
x11_do X map w "class=$CLASS" size=300x200
wait_for "mapped $CLASS" >/dev/null
settle
read -r x y w h <<<"$(win_rect $CLASS)"

inject "move $((x + 40)) $((y + 50))" >/dev/null
x11_do X sync
x11_wait X '^x11: event w (enter|motion)'
assert_ok "the pointer entered the window" x11_has X '^x11: event w enter'

inject "press left" "release left" >/dev/null
x11_wait X '^x11: event w button-press detail=1 at '
x11_do X sync
position=$(x11_log X | sed -n 's/^x11: event w button-press detail=1 at \([0-9]*,[0-9]*\).*/\1/p' | tail -1)
assert_ok "the click landed inside the window ($position)" test -n "$position"
assert_ok "the release arrived" x11_has X '^x11: event w button-release detail=1'

before=$(x11_count X '^x11: event w key-press')
inject "key a down" "key a up" >/dev/null
x11_wait X "^x11: event w key-press"
x11_do X sync
assert_eq "one key press arrived" "$((before + 1))" "$(x11_count X '^x11: event w key-press')"
assert_ok "the window got focus" x11_has X '^x11: event w focus-in'
assert_summary
