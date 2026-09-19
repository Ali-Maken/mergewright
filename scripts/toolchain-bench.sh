#!/usr/bin/env bash
# Before/after for cargo-nextest and sccache on the three quiet gates.
#
# Never run inside a helper's context, and never while another cargo job is
# running: the numbers are wall-clock on a 10-core laptop throttled to
# jobs = 4, so anything else competing makes them meaningless.
#
#   scripts/toolchain-bench.sh          # warm gates only (~minutes)
#   scripts/toolchain-bench.sh --cold   # also a from-scratch build, in a
#                                       # scratch target dir (~tens of minutes)
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
OUT="$ROOT/docs/perf/toolchain.txt"
COLD=${1:-}
cd "$ROOT"

# Wall-clock seconds for a command, output discarded unless it fails.
time_it() {
  local label=$1; shift
  local log start end
  log=$(mktemp)
  start=$(python3 -c 'import time;print(time.time())')
  if ! "$@" >"$log" 2>&1; then
    echo "FAILED: $label" >&2
    tail -30 "$log" >&2
    rm -f "$log"
    return 1
  fi
  end=$(python3 -c 'import time;print(time.time())')
  rm -f "$log"
  printf '%-34s %7.1f s\n' "$label" "$(python3 -c "print($end-$start)")"
}

gates_with_test_runner() {
  local tag=$1 runner=$2
  time_it "$tag build"  cargo build --quiet
  time_it "$tag clippy" cargo clippy --all-targets --message-format=short -- -D warnings
  if [ "$runner" = nextest ]; then
    time_it "$tag nextest" cargo nextest run --status-level=fail
    # nextest does not run doctests; the gate keeps them separately.
    time_it "$tag doctests" cargo test --doc --quiet
  else
    time_it "$tag test" cargo test --quiet
  fi
}

{
  echo "# toolchain before/after — $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "# $(rustc --version)"
  echo "# $(uname -s) $(uname -m), $(sysctl -n hw.ncpu) cores, build jobs = 4"
  echo "# nextest $(cargo nextest --version 2>/dev/null | head -1 | awk '{print $2}'), sccache $(sccache --version | awk '{print $2}')"
  echo

  echo "## warm, before (cargo test, no sccache)"
  unset RUSTC_WRAPPER
  cargo build --quiet >/dev/null 2>&1 || true   # settle the cache first
  gates_with_test_runner "before" cargo
  echo

  echo "## warm, after (cargo nextest + sccache)"
  export RUSTC_WRAPPER="$HOME/.cargo/bin/sccache"
  sccache --zero-stats >/dev/null 2>&1 || true
  cargo build --quiet >/dev/null 2>&1 || true
  gates_with_test_runner "after" nextest
  echo
  echo "# sccache after the warm run:"
  sccache --show-stats | grep -Ei 'requests|hits|misses|cache size' | sed 's/^/#   /'
  echo

  if [ "$COLD" = "--cold" ]; then
    echo "## cold build, scratch target dir (what a new worktree pays)"
    SCRATCH=$(mktemp -d)
    trap 'chmod -R u+w "$SCRATCH" 2>/dev/null; rm -rf "$SCRATCH"' EXIT

    unset RUSTC_WRAPPER
    CARGO_TARGET_DIR="$SCRATCH/no-sccache" time_it "cold, no sccache" cargo build --quiet

    export RUSTC_WRAPPER="$HOME/.cargo/bin/sccache"
    sccache --zero-stats >/dev/null 2>&1 || true
    # Populated by the warm run above, so this is the hit case a second
    # worktree actually sees.
    CARGO_TARGET_DIR="$SCRATCH/sccache" time_it "cold, sccache warm" cargo build --quiet
    echo "# sccache on the cold build:"
    sccache --show-stats | grep -Ei 'requests|hits|misses' | sed 's/^/#   /'
  else
    echo "## cold build: not measured (pass --cold)"
  fi
} | tee "$OUT"

echo "wrote $OUT" >&2
