#!/usr/bin/env bash
# What the dependents check costs `a` on HEAD when another local branch
# exists (m7-power-tools/09, "Decided": under one frame on the fixture).
# Runs the ignored measurement walk in graft, release build, and writes its
# lines to docs/perf/09.txt; a reader takes the `summary` line.
#
# Never run inside a helper's context.
#
#   scripts/m7-09-perf.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$ROOT/docs/perf/09.txt"

LINES=$(cargo test --release --manifest-path "$ROOT/Cargo.toml" -p graft \
  --test it -- --ignored --nocapture --test-threads=1 \
  m7_walk::amend_into::perf_dependents_check 2>/dev/null | grep '^m7-09')

{
  echo "# a on HEAD through the dependents check — $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "# $(git --version), $(uname -sm), release build"
  echo "# fixture: m7_walk::amend_into's oauth repo (main, feature-a of 2 commits), one file staged"
  echo "# measured: a pressed to the amend editor open — the check, then HEAD's message read"
  echo "# budget: one frame, 16.7 ms"
  echo
  echo "$LINES"
} > "$OUT"
grep 'summary' "$OUT"
