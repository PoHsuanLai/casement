#!/usr/bin/env bash
# Acceptance script 'lock-blocks-input' (20-FREEZE-PLAN §4.3, P0): while a client holds the session
# lock, keys and pointer reach only its lock surface, never the window behind it; unlocking gives
# the window the keyboard back; and a locker that dies leaves the session locked under the
# compositor's own cover (a frame with nothing but black and the cursor). Asserted on transcripts
# and on a captured frame. Exit: 0 pass, 1 fail, 2 could not run, 3 environment.
#
# Owner: P0-accept (fills the P0-H3 stub).
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
# shellcheck source=dev/lib/accept.sh
source "$root/dev/lib/accept.sh"

APP=org.example.locked
KEY='^< wl_keyboard#[0-9]+\.key\('
ENTER='^< wl_keyboard#[0-9]+\.enter\('
LEAVE='^< wl_keyboard#[0-9]+\.leave\('
BUTTON='^< wl_pointer#[0-9]+\.button\('
start_compositor --clock stepped
plug_devices
probe_start A
probe_start L

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
assert_ok "before the lock the window has the keys" test "$(probe_count A "$KEY")" -ge 2
a_leaves=$(probe_count A "$LEAVE")

# Lock.
probe_do L lock lk
probe_wait L '^probe: locked lk$'
probe_sync L
probe_sync A
assert_ok "the window lost the keyboard to the lock" test "$(probe_count A "$LEAVE")" -gt "$a_leaves"
assert_ok "the lock surface got keyboard enter" probe_has L "$ENTER"
a_keys=$(probe_count A "$KEY")
a_buttons=$(probe_count A "$BUTTON")
inject "key a down" "key a up" "move $((x + 120)) $((y + 120))" "press left" "release left" >/dev/null
probe_wait L "$KEY"
probe_wait L "$BUTTON"
probe_sync L
probe_sync A
assert_ok "the lock surface hears the keys" test "$(probe_count L "$KEY")" -ge 2
assert_eq "no key reaches the window while locked" "$a_keys" "$(probe_count A "$KEY")"
assert_eq "no button reaches the window while locked" "$a_buttons" "$(probe_count A "$BUTTON")"

# Unlock: the window gets the keyboard back.
a_enters=$(probe_count A "$ENTER")
probe_do L unlock lk
probe_sync L
probe_sync A
assert_ok "the window got keyboard enter on unlock" test "$(probe_count A "$ENTER")" -gt "$a_enters"
inject "key b down" "key b up" >/dev/null
probe_wait A "$KEY" >/dev/null
probe_sync A
assert_ok "keys reach the window again after unlock" test "$(probe_count A "$KEY")" -gt "$a_keys"

# A locker that dies leaves the session locked under the compositor's cover.
probe_start L2
probe_do L2 lock lk2
probe_wait L2 '^probe: locked lk2$'
probe_stop L2
a_keys=$(probe_count A "$KEY")
inject "key e down" "key e up" >/dev/null
step 5
probe_sync A
assert_eq "a dead locker's lock still blocks keys" "$a_keys" "$(probe_count A "$KEY")"
shot output "$JAIL_SCRATCH/dead-locker.png" >/dev/null
assert_ok "the cover leaves nothing of the session showing (only the cursor, where it rests)" \
  python3 "$root/dev/lib/crisp.py" black "$JAIL_SCRATCH/dead-locker.png" $((x + 120)) $((y + 120)) 40
assert_summary
