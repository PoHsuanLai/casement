#!/usr/bin/env bash
# Acceptance script 'map-resize' (20-FREEZE-PLAN §4.3, P0): a toplevel maps centred at the size it
# drew, follows the compositor's maximise and unmaximise configures (the frame in `debug windows`
# and the buffer the client draws agree each time), grows by 100x80 under an interactive
# bottom-right resize, and leaves when closed. Every step waits on the client's transcript or on
# a Debug1 predicate, not on time. Exit: 0 pass, 1 fail, 2 could not run, 3 environment.
#
# Owner: P0-accept (fills the P0-H2 stub).
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
# shellcheck source=dev/lib/accept.sh
source "$root/dev/lib/accept.sh"

APP=org.example.mapresize
start_compositor --clock stepped
plug_devices
probe_start A

# Map: 400x300, centred on the 1920x1080 output.
probe_do A toplevel w "app=$APP" size=400x300 pattern=gradient
probe_wait A '^probe: mapped w 400x300$'
wait_for "mapped $APP" >/dev/null
settle
assert_eq "maps centred at the size it drew" "760 390 400 300" "$(win_rect $APP)"
assert_ok "the first configure suggested no size" probe_has A '^probe: configure w 0x0 \['

# Maximise: the compositor's configure carries the maximised state and the usable area; the client
# redraws at that size and the window's frame becomes it.
probe_do A maximise w
probe_wait A '^probe: configure w [0-9]+x[0-9]+ \[.*maximized'
configure=$(probe_log A | sed -n 's/^probe: configure w \([0-9]*\)x\([0-9]*\) \[.*maximized.*/\1 \2/p' | tail -1)
read -r mw mh <<<"$configure"
probe_wait A "^probe: drawn w ${mw}x${mh}\$"
probe_sync A
settle
assert_eq "maximised: the frame is the configured size" "$mw $mh" "$(win_rect $APP | awk '{print $3, $4}')"
assert_ok "maximised fills the output's width" test "$mw" -eq 1920
assert_ok "maximised is no taller than the output" test "$mh" -le 1080 -a "$mh" -gt 300

# Unmaximise: back to the size it had.
probe_do A unmaximise w
probe_wait A '^probe: drawn w 400x300$'
probe_sync A
settle
assert_eq "unmaximised: the frame is 400x300 again" "400 300" "$(win_rect $APP | awk '{print $3, $4}')"

# Interactive resize: focus, press on the bottom-right corner, ask for the resize, drag, release.
read -r x y w h <<<"$(win_rect $APP)"
click $((x + 200)) $((y + 150))
click $((x + 200)) $((y + 150))
probe_sync A
probe_wait A '^< wl_pointer#[0-9]+\.button\('
corner_x=$((x + w - 2)) corner_y=$((y + h - 2))
inject "move $corner_x $corner_y" "press left" >/dev/null
probe_sync A
presses=$(probe_count A '^< wl_pointer#[0-9]+\.button\(')
probe_do A resize-request w bottom-right
inject "move $((corner_x + 100)) $((corner_y + 80))" >/dev/null
probe_wait A '^probe: drawn w 500x380$'
inject "release left" >/dev/null
probe_sync A
settle
assert_eq "resized: the frame follows the drag" "500 380" "$(win_rect $APP | awk '{print $3, $4}')"
assert_eq "resized: the top-left corner stayed" "$x $y" "$(win_rect $APP | awk '{print $1, $2}')"
assert_ok "the client saw its presses ($presses)" test "$presses" -ge 1

# Close.
probe_do A close w
wait_for "gone $APP" >/dev/null
assert_eq "closed: no window is left" "0" "$(ctl debug windows | jq length)"
assert_summary
