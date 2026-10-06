#!/usr/bin/env bash
# Acceptance script 'exclusive-layer-focus' (20-FREEZE-PLAN §4.3, P0): a layer surface under a
# resting pointer gets `enter` without the pointer moving; an exclusive-keyboard overlay layer takes
# the keyboard from the focused window as it maps (A16), keys reach it and not the window; when it
# goes the window has the keyboard back. Asserted on the clients' transcripts. Exit: 0 pass,
# 1 fail, 2 could not run, 3 environment.
#
# Owner: P0-accept (fills the P0-H3 stub).
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
# shellcheck source=dev/lib/accept.sh
source "$root/dev/lib/accept.sh"

APP=org.example.exclusive
KEY='^< wl_keyboard#[0-9]+\.key\('
ENTER='^< wl_keyboard#[0-9]+\.enter\('
LEAVE='^< wl_keyboard#[0-9]+\.leave\('
POINTER_ENTER='^< wl_pointer#[0-9]+\.enter\('
start_compositor --clock stepped
plug_devices
probe_start A
probe_start B

# 1. A layer mapped under a pointer that does not move gets pointer `enter` (the pointer rests at
# the centre of the output, where it starts).
probe_do A layer p level=top kbd=none size=200x100
probe_wait A '^probe: layer-mapped p '
probe_sync A
assert_ok "a layer under a still pointer got enter" probe_has A "$POINTER_ENTER"
probe_do A layer-close p
probe_sync A

# 2. A window with the keyboard.
probe_do A toplevel w "app=$APP" size=400x300
probe_wait A '^probe: mapped w 400x300$'
wait_for "mapped $APP" >/dev/null
settle
read -r x y w h <<<"$(win_rect $APP)"
click $((x + 100)) $((y + 100))
click $((x + 100)) $((y + 100))
probe_wait A "$ENTER"
inject "key a down" "key a up" >/dev/null
probe_wait A "$KEY"
probe_sync A
window_keys=$(probe_count A "$KEY")
assert_ok "before the layer the window has the keyboard" test "$window_keys" -ge 2
a_leaves=$(probe_count A "$LEAVE")

# 3. An exclusive overlay layer: it has the keyboard when it is mapped, before any input.
probe_do B layer l level=overlay kbd=exclusive size=300x100
probe_wait B '^probe: layer-mapped l '
probe_sync B
probe_sync A
assert_ok "the exclusive layer got keyboard enter on map" probe_has B "$ENTER"
assert_ok "the window lost the keyboard to it" test "$(probe_count A "$LEAVE")" -gt "$a_leaves"
inject "key c down" "key c up" >/dev/null
probe_wait B "$KEY"
probe_sync B
probe_sync A
assert_ok "keys go to the layer" test "$(probe_count B "$KEY")" -ge 2
assert_eq "no key reached the window behind it" "$window_keys" "$(probe_count A "$KEY")"

# 4. The layer goes: the window has the keyboard again.
a_enters=$(probe_count A "$ENTER")
probe_do B layer-close l
probe_sync B
probe_sync A
assert_ok "the window got keyboard enter back" test "$(probe_count A "$ENTER")" -gt "$a_enters"
inject "key d down" "key d up" >/dev/null
probe_wait A "$KEY" >/dev/null
probe_sync A
assert_ok "keys reach the window again" test "$(probe_count A "$KEY")" -gt "$window_keys"
assert_summary
