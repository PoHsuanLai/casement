#!/usr/bin/env bash
# USER ONLY. Never run by an agent, never part of any gate (20-FREEZE-PLAN §3.2, C3).
# The optional real-hardware smoke: it copies what this machine's display, input and GPU
# stacks report into a directory, as raw material for KMS, input and GPU fixtures for the
# headless fakes. It refuses unless stdin and stdout are a terminal and the person types the
# confirmation. It only reads: /sys/class/drm, `drm_info`, `lspci` and `libinput list-devices`
# when they exist; it opens no DRM node, takes no master and changes nothing.
#
# Usage: dev/hw/smoke.sh [OUTPUT_DIR]   (default: ./casement-hw-<date>)
# Nothing here is hardcoded to one machine: every card and connector found is recorded.
set -euo pipefail
if [ ! -t 0 ] || [ ! -t 1 ]; then
  echo "smoke.sh: refusing: stdin and stdout must be a terminal (this script is for a person at the machine)" >&2
  exit 2
fi
printf 'This reads your real display, input and GPU state. Type "I am at the machine" to continue: '
read -r answer
if [ "$answer" != "I am at the machine" ]; then
  echo "smoke.sh: not confirmed" >&2
  exit 2
fi

out="${1:-./casement-hw-$(date +%Y%m%d-%H%M%S)}"
mkdir -p "$out/drm"

# Every connector the kernel knows: its status, modes and raw EDID.
for dir in /sys/class/drm/card*-*; do
  [ -d "$dir" ] || continue
  name="$(basename "$dir")"
  mkdir -p "$out/drm/$name"
  for file in status enabled modes edid; do
    [ -r "$dir/$file" ] && cp "$dir/$file" "$out/drm/$name/$file" 2>/dev/null || true
  done
done
ls -l /sys/class/drm > "$out/drm/cards.txt" 2>&1 || true

# The richer view when the tool is installed (planes, formats, properties).
if command -v drm_info >/dev/null 2>&1; then
  drm_info -j > "$out/drm_info.json" 2> "$out/drm_info.err" || true
fi
if command -v lspci >/dev/null 2>&1; then
  lspci -nn -k | grep -A3 -Ei 'vga|3d|display' > "$out/gpus.txt" || true
fi
if command -v libinput >/dev/null 2>&1; then
  libinput list-devices > "$out/libinput.txt" 2> "$out/libinput.err" || true
fi

echo "smoke.sh: recorded into $out"
echo "Hand the directory to a casement lane: it turns drm/ and drm_info.json into"
echo "crates/casement-testkit/fixtures/kms/*.json (the shape of KmsResources)."
