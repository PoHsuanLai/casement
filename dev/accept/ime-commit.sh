#!/usr/bin/env bash
# Acceptance script 'ime-commit' (20-FREEZE-PLAN §4.3, P0): a focused text field enables text input;
# the input method is activated and grabs the keyboard; the first key goes to the input method and
# not to the document (A16); the method's preedit and commit_string reach the field, each closed by
# `done`. Asserted on the client's transcript (a text field and an input method on one connection,
# as the protocol tests have them). Exit: 0 pass, 1 fail, 2 could not run, 3 environment.
#
# Owner: P0-accept (fills the P0-H7 stub).
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
# shellcheck source=dev/lib/accept.sh
source "$root/dev/lib/accept.sh"

APP=org.example.ime
start_compositor --clock stepped
plug_devices
probe_start A
probe_do A toplevel w "app=$APP" size=400x300
probe_wait A '^probe: mapped w 400x300$'
wait_for "mapped $APP" >/dev/null
settle
# The field and the method bind before the window has the keyboard, so the field gets `enter` with it.
probe_do A ime-start
probe_sync A
read -r x y w h <<<"$(win_rect $APP)"
click $((x + 100)) $((y + 100))
click $((x + 100)) $((y + 100))
probe_wait A '^< zwp_text_input_v3#[0-9]+\.enter\('
assert_ok "the field got enter with the keyboard focus" probe_has A '^< zwp_text_input_v3#[0-9]+\.enter\('

# Enable: the compositor activates the input method, which grabs the keyboard.
probe_do A ime-enable
probe_wait A '^< zwp_input_method_v2#[0-9]+\.activate\('
assert_ok "the input method was activated" probe_has A '^< zwp_input_method_v2#[0-9]+\.activate\('
probe_do A ime-grab
probe_sync A
doc_keys=$(probe_count A '^< wl_keyboard#[0-9]+\.key\(')

# The first key goes to the method's grab, not to the document.
inject "key a down" "key a up" >/dev/null
probe_wait A '^< zwp_input_method_keyboard_grab_v2#[0-9]+\.key\('
probe_sync A
assert_ok "the grab saw the key press and release" test "$(probe_count A '^< zwp_input_method_keyboard_grab_v2#[0-9]+\.key\(')" -ge 2
assert_eq "the document's keyboard saw no key" "$doc_keys" "$(probe_count A '^< wl_keyboard#[0-9]+\.key\(')"

# Preedit, then commit: the field is told, each batch closed by done.
probe_do A ime-preedit ni
probe_wait A '^< zwp_text_input_v3#[0-9]+\.preedit_string\('
assert_ok "preedit reached the field" probe_has A '^< zwp_text_input_v3#[0-9]+\.preedit_string\(.*ni'
probe_do A ime-commit あ
probe_wait A '^< zwp_text_input_v3#[0-9]+\.commit_string\('
probe_sync A
assert_ok "commit_string carried the text" probe_has A '^< zwp_text_input_v3#[0-9]+\.commit_string\(.*あ'
last_two=$(probe_log A | grep -E '^< zwp_text_input_v3#[0-9]+\.' | tail -2 | sed -E 's/^< zwp_text_input_v3#[0-9]+\.([a-z_]+)\(.*/\1/' | paste -sd' ')
assert_eq "the commit is followed by done" "commit_string done" "$last_two"
assert_summary
