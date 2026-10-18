#!/usr/bin/env bash
# Cost of every local branch's ahead count in one refs read (ease-pass/07a):
# 2,000 local branches, 500 tracked, release build. Writes
# docs/perf/07a-ahead-counts.txt; a reader takes the `summary` line.
# Budget: <=100 ms per cold refs read (it runs on a job runner worker).
#
# Never run inside a helper's context.
#
#   scripts/ease-ahead-read.sh
set -euo pipefail
. "$(cd "$(dirname "$0")" && pwd)/lib/machine-state.sh"  # battery / Low Power Mode guard

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$ROOT/docs/perf/07a-ahead-counts.txt"
machine_state_begin "$OUT"

LOAD_BEFORE=$(sysctl -n vm.loadavg)
# Build first, so the build's load is not in the numbers.
cargo test --release --manifest-path "$ROOT/Cargo.toml" -p mergewright --features heavy-tests --lib --no-run 2>/dev/null
LINES=$(cargo test --release --manifest-path "$ROOT/Cargo.toml" -p mergewright --features heavy-tests --lib \
  -- --nocapture --test-threads=1 --exact refs::perf_ahead::perf_ahead_read_2000_branches 2>/dev/null | grep '^ease-07a')
LOAD_AFTER=$(sysctl -n vm.loadavg)

{
  echo "# ahead counts in one refs read — $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "# $(git --version), $(uname -sm), $(sysctl -n hw.model 2>/dev/null || true), release build"
  echo "# load before: $LOAD_BEFORE   after: $LOAD_AFTER"
  echo "# fixture: 60 linear commits; 2000 local branches, 500 with upstreams, each <=50 commits apart"
  echo "# measured: refs::read, cold ahead/behind cache (5 runs), then a warm re-read"
  echo "# budget: 100 ms per cold refs read, off the UI thread"
  echo
  echo "$LINES" | grep -v summary
  echo
  echo "$LINES" | grep summary | sed "s|\$|; load $LOAD_BEFORE/$LOAD_AFTER|"
} > "$OUT"
grep 'summary' "$OUT"
