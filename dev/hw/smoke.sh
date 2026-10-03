#!/usr/bin/env bash
# USER ONLY. Never run by an agent, never part of any gate (20-FREEZE-PLAN §3.2, C3).
# The optional real-hardware smoke: records KMS, input and GPU fixtures for the headless fakes.
# It refuses unless stdin is a terminal and the person types the confirmation.
#
# Owner: P1-D2. Frozen at P0-Z2 as a stub: even when confirmed it exits 2 until it is written.
set -euo pipefail
if [ ! -t 0 ]; then
  echo "smoke.sh: refusing: stdin is not a terminal (this script is for a person at the machine)" >&2
  exit 2
fi
printf 'This touches real hardware. Type "I am at the machine" to continue: '
read -r answer
if [ "$answer" != "I am at the machine" ]; then
  echo "smoke.sh: not confirmed" >&2
  exit 2
fi
echo "smoke.sh: not written (P1-D2)" >&2
exit 2
