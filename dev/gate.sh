#!/usr/bin/env bash
# casement's gate (20-FREEZE-PLAN §6.2): `dev/gate.sh fast -p <crate>… | merge | phase`.
# Every exit code checked; never piped, never tailed.
set -euo pipefail
cd "$(dirname "$0")/.."
: "${CARGO_TARGET_DIR:=$PWD/target}"; export CARGO_TARGET_DIR
mkdir -p "$CARGO_TARGET_DIR"
tier="${1:?fast|merge|phase}"; shift
export GATE_TIER="$tier"
run=$(dev/lib/run-id.sh)
mkdir -p "dev/out/$run"
step() { dev/lib/step.sh "$run" "$@"; }       # times, writes summary.json, applies budgets.toml
# §6.2 wrote `jail` as a shell function, which step.sh (its own process) cannot call; the same
# command line as an array.
jail=("${CASEMENT_JAIL:-../desktop/harness/jail.sh}" --fabricate-gpus 1 --bind "$PWD" --bind "$CARGO_TARGET_DIR" --)
step preflight  dev/preflight.sh
step fmt        cargo fmt --all --check
step boundary   ./scripts/check-boundary.sh
step coverage   ./scripts/check-protocol-coverage.sh
step doc-paths  ./scripts/check-doc-paths.sh
case "$tier" in
  fast)
    step clippy   cargo clippy "$@" --all-targets --all-features -- -D warnings
    step archive  cargo nextest archive "$@" --all-features --archive-file "dev/out/$run/tests.tar.zst"
    step test     "${jail[@]}" cargo-nextest nextest run --profile fast --archive-file "dev/out/$run/tests.tar.zst" --workspace-remap "$PWD" ;;
  merge|phase)
    step clippy   cargo clippy --workspace --all-targets --all-features -- -D warnings
    step deny     cargo deny check licenses
    step doc      cargo test --doc --workspace --all-features
    step archive  cargo nextest archive --workspace --all-features --archive-file "dev/out/$run/tests.tar.zst"
    step test     "${jail[@]}" cargo-nextest nextest run --profile "$tier" --archive-file "dev/out/$run/tests.tar.zst" --workspace-remap "$PWD"
    step accept   "${jail[@]}" dev/accept.sh --jobs 8 ;;
  *) echo "gate: unknown tier '$tier' (fast|merge|phase)" >&2; exit 64 ;;
esac
if [ "$tier" = phase ]; then
  step wlcs        "${jail[@]}" dev/wlcs/run.sh --phase
  step hw-fixtures dev/check-kms-fixtures.sh       # fixtures parse and replay; never touches hardware
fi
echo "GATE GREEN ($tier) $run"
