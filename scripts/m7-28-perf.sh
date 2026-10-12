#!/usr/bin/env bash
# What the in-the-way read (ADR-0049, `in_the_way::blocking`) costs a switch
# on a big repository (review 28, m5). Runs the ignored measurement test in
# graft-ops, release build, and writes its lines to docs/perf/m7-28.txt; a
# reader takes the `summary` line.
#
# Never run inside a helper's context. The 100k-commit fixture is built once
# into the fixture cache if it is not there yet (a few minutes).
#
#   scripts/m7-28-perf.sh [tracked-files]     (default 20000)
set -euo pipefail
. "$(cd "$(dirname "$0")" && pwd)/lib/machine-state.sh"  # ticket 49: battery / Low Power Mode guard

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$ROOT/docs/perf/m7-28.txt"
machine_state_begin "$OUT"
export MWR_M728_FILES=${1:-20000}

LINES=$(cargo test --release --manifest-path "$ROOT/Cargo.toml" -p graft-ops \
  --features heavy-tests --test it -- --ignored --nocapture --test-threads=1 \
  in_the_way::perf 2>/dev/null | grep '^m7-28')

{
  echo "# in_the_way::blocking for a switch — $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "# $(git --version), $(uname -sm), release build"
  echo "# budget: none is written for this read; 100 ms assumed (it runs at the preview and again at y)"
  echo
  echo "$LINES"
} > "$OUT"
grep 'summary' "$OUT"
