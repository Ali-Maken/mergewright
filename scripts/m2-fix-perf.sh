#!/usr/bin/env bash
# M2 review fix 1a: the heavy tier and the release perf run, one at a time,
# with their timings, into docs/perf/m2-review-fix-1a.txt. Nothing here runs
# in an agent's context (CLAUDE.md: measurement never does).
set -u
. "$(cd "$(dirname "$0")" && pwd)/lib/machine-state.sh"  # ticket 49: battery / Low Power Mode guard
cd "$(dirname "$0")/.." || exit 1
out=docs/perf/m2-review-fix-1a.txt
machine_state_begin "$out"
raw=target/m2-fix-perf
mkdir -p "$raw" docs/perf
: > "$out"
echo "M2 review fix 1a (C1) gates, $(date '+%Y-%m-%d %H:%M')" >> "$out"
echo >> "$out"

run() { # name, then the command
  local name="$1"; shift
  local log="$raw/${name}.log"
  local start=$SECONDS
  "$@" > "$log" 2>&1
  local code=$?
  echo "$name: exit $code, $((SECONDS - start)) s" >> "$out"
  grep -E '^test result:' "$log" | tail -3 >> "$out"
  echo >> "$out"
}

run fast-tier cargo test --quiet
run heavy-tier cargo test --features heavy-tests -- --test-threads=2
run release-perf cargo test --release -p mergewright --features heavy-tests --test it -- \
  frames:: heavy::budgets_hold --test-threads=1 --nocapture
grep -h '^frames: ' "$raw/release-perf.log" | grep -v ' trigger=' >> "$out"
echo "(full logs: $raw, untracked)" >> "$out"
