#!/usr/bin/env bash
# Acceptance script 'drag-selection' (20-FREEZE-PLAN §4.3, P0): the clipboard and drag-and-drop move
# real data between two clients. A copies "hello selection"; B, once it has the keyboard, pastes
# exactly that. A drags "dragged text" over B's window and lets go: B sees enter and drop and reads
# exactly that. Asserted on the clients' transcripts and the bytes they read. Exit: 0 pass, 1 fail,
# 2 could not run, 3 environment.
#
# Owner: P0-accept (fills the P0-H5 stub).
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
# shellcheck source=dev/lib/accept.sh
source "$root/dev/lib/accept.sh"

SRC=org.example.source
DST=org.example.target
start_compositor --clock stepped
plug_devices
probe_start A
probe_start B

# A is wide and low, B narrow and tall: each shows a part the other does not cover, whichever is
# on top, so a click can raise either one.
probe_do A toplevel src "app=$SRC" size=700x300 pattern=solid:a04020
probe_wait A '^probe: mapped src 700x300$'
wait_for "mapped $SRC" >/dev/null
probe_do B toplevel dst "app=$DST" size=300x500 pattern=solid:2040a0
probe_wait B '^probe: mapped dst 300x500$'
wait_for "mapped $DST" >/dev/null
settle
read -r ax ay aw ah <<<"$(win_rect $SRC)"
read -r bx by bw bh <<<"$(win_rect $DST)"
inside() { test "$1" -ge "$3" -a "$1" -lt $(($3 + $5)) -a "$2" -ge "$4" -a "$2" -lt $(($4 + $6)); }
# A point of A that B does not cover, and a point of B that A does not cover.
sx=$((ax + 10)) sy=$((ay + ah / 2))
tx=$((bx + bw / 2))
if [ $((by + 10)) -lt "$ay" ]; then ty=$((by + 10)); else ty=$((by + bh - 10)); fi
assert_not "the source point $sx,$sy is not under B" inside "$sx" "$sy" "$bx" "$by" "$bw" "$bh"
assert_not "the target point $tx,$ty is not under A" inside "$tx" "$ty" "$ax" "$ay" "$aw" "$ah"

# Selection: A copies while it has the keyboard; B takes the keyboard and pastes.
click "$sx" "$sy"
click "$sx" "$sy"
probe_sync A
probe_wait A '^< wl_keyboard#[0-9]+\.enter\('
probe_do A copy hello selection
probe_sync A
click "$tx" "$ty"
click "$tx" "$ty"
probe_wait B '^< wl_data_device#[0-9]+\.selection\('
probe_do B paste
probe_wait B '^probe: pasted '
assert_ok "B pasted what A copied" probe_has B '^probe: pasted "hello selection"$'

# Drag and drop: press on A (a click raises it), start the drag with that press, carry it to B.
click "$sx" "$sy"
inject "move $((sx + 5)) $sy" "press left" >/dev/null
probe_sync A
probe_do A drag src dragged text
probe_sync A
inject "move $((sx + 50)) $sy" "move $tx $ty" >/dev/null
probe_wait B '^< wl_data_device#[0-9]+\.enter\('
assert_ok "the drag entered B's window" probe_has B '^< wl_data_device#[0-9]+\.enter\('
inject "release left" >/dev/null
probe_wait B '^< wl_data_device#[0-9]+\.drop\('
probe_wait B '^probe: dropped '
assert_ok "B read exactly what A dragged" probe_has B '^probe: dropped "dragged text"$'
assert_ok "A was told the drop was performed" probe_has A '^< wl_data_source#[0-9]+\.dnd_drop_performed\('
assert_summary
