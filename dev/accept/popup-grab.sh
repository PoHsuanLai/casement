#!/usr/bin/env bash
# Acceptance script 'popup-grab' (20-FREEZE-PLAN §4.3, P0): a popup that grabs (xdg_popup.grab, on a
# button press the client received) takes the keyboard, a click inside it leaves it open, a click
# outside dismisses it (`popup_done`), and the keyboard goes back to its window. Asserted on the
# client's transcript. Exit: 0 pass, 1 fail, 2 could not run, 3 environment.
#
# Owner: P0-accept (fills the P0-H2 stub).
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
# shellcheck source=dev/lib/accept.sh
source "$root/dev/lib/accept.sh"

APP=org.example.popupgrab
KEY_EVENT='^< wl_keyboard#[0-9]+\.key\('
ENTER_EVENT='^< wl_keyboard#[0-9]+\.enter\('
start_compositor --clock stepped
plug_devices
probe_start A
probe_do A toplevel w "app=$APP" size=400x300
probe_wait A '^probe: mapped w 400x300$'
wait_for "mapped $APP" >/dev/null
settle
read -r x y w h <<<"$(win_rect $APP)"

# Focus the window and let the client see a press (the grab answers its serial).
click $((x + 200)) $((y + 200))
click $((x + 200)) $((y + 200))
probe_sync A
probe_wait A '^< wl_pointer#[0-9]+\.button\(.*, 1\)$'
probe_wait A "$ENTER_EVENT"
enters_before=$(probe_count A "$ENTER_EVENT")

# The popup: 100x50 at (20,20) of the window, grabbing.
probe_do A popup m w grab at=20,20 size=100x50
probe_wait A '^probe: popup-mapped m 20,20 100x50$'
probe_sync A
px=$((x + 20)) py=$((y + 20))
probe_wait A "$ENTER_EVENT" >/dev/null
assert_ok "the grab gave the popup the keyboard (a fresh enter)" test "$(probe_count A "$ENTER_EVENT")" -gt "$enters_before"
keys_before=$(probe_count A "$KEY_EVENT")
inject "key a down" "key a up" >/dev/null
probe_wait A "$KEY_EVENT"
probe_sync A
assert_ok "keys reach the client while the popup grabs" test "$(probe_count A "$KEY_EVENT")" -gt "$keys_before"

# A click inside the popup does not dismiss it.
click $((px + 50)) $((py + 25))
probe_sync A
assert_not "a click inside leaves the popup open" probe_has A '^probe: popup-done m$'

# A click outside (on the bare desktop) dismisses it.
enters_open=$(probe_count A "$ENTER_EVENT")
click 40 40
probe_wait A '^probe: popup-done m$'
assert_ok "a click outside dismissed the popup" probe_has A '^< xdg_popup#[0-9]+\.popup_done\('
probe_do A popup-close m
probe_sync A
assert_ok "the keyboard went back to the window (another enter)" test "$(probe_count A "$ENTER_EVENT")" -gt "$enters_open"
assert_eq "the window is the active one again" "true" "$(win active $APP)"
keys_before=$(probe_count A "$KEY_EVENT")
inject "key b down" "key b up" >/dev/null
probe_wait A "$KEY_EVENT"
probe_sync A
assert_ok "keys reach the window after the popup is gone" test "$(probe_count A "$KEY_EVENT")" -gt "$keys_before"
assert_summary
