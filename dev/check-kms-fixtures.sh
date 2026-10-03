#!/usr/bin/env bash
# Phase tier: recorded KMS fixtures parse and replay through FakeKms. Never touches hardware.
# There are no fixtures before P1-Z.
set -euo pipefail
echo "no KMS fixtures before P1-Z"
exit 0
