#!/usr/bin/env bash
# What the dependents check costs `a` on HEAD when another local branch
# exists (m7-power-tools/09, "Decided": under one frame on the fixture;
# 09b A1: the phases). Runs the heavy tier's measurement walks in the graft package
# (review m7-09..21c Minor 12: `heavy-tests`, no longer ignored), release
# build, and writes their lines to docs/perf/09b.txt; a reader takes the
# `summary` line.
#
# Never run inside a helper's context.
#
#   scripts/m7-09-perf.sh
set -euo pipefail
. "$(cd "$(dirname "$0")" && pwd)/lib/machine-state.sh"  # ticket 49: battery / Low Power Mode guard

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$ROOT/docs/perf/09b.txt"
TRACE="$(mktemp -d)"
trap 'rm -rf "$TRACE"' EXIT
machine_state_begin "$OUT"

run() {
  cargo test --release --manifest-path "$ROOT/Cargo.toml" -p graft --features heavy-tests \
    --test it -- --nocapture --test-threads=1 --exact "$1" 2>/dev/null | grep '^m7-09b'
}

LOAD_BEFORE=$(sysctl -n vm.loadavg)
# Build first, so the build's load is not in the numbers.
cargo test --release --manifest-path "$ROOT/Cargo.toml" -p graft --features heavy-tests --test it --no-run 2>/dev/null
LINES=$(run m7_walk::amend_into::perf::perf_dependents_check)
SPAWNS=$(GIT_TRACE2_PERF="$TRACE" run m7_walk::amend_into::perf::perf_dependents_spawns)
LOAD_AFTER=$(sysctl -n vm.loadavg)

{
  echo "# a on HEAD through the dependents check — $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "# $(git --version), $(uname -sm), $(sysctl -n hw.model 2>/dev/null || true), release build"
  echo "# load before: $LOAD_BEFORE   after: $LOAD_AFTER"
  echo "# fixture: m7_walk::amend_into's oauth repo (main, feature-a of 2 commits, HEAD), plus spike (off main, own commit); one file staged"
  echo "# measured: a pressed to the amend editor open — the check, then HEAD's message read"
  echo "# budget: one frame, 16.7 ms"
  echo
  echo "$LINES" | grep -v summary
  echo
  echo "$SPAWNS"
  echo
  echo "$LINES" | grep summary | sed "s|\$|; load $LOAD_BEFORE/$LOAD_AFTER|"
} > "$OUT"
grep 'summary' "$OUT"
