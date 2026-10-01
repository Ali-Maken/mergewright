#!/bin/sh
# m7-power-tools/41: build time and disk size of the team fixture, both specs.
# Builds each fresh (heavy tier, debug test profile) and writes the summary
# lines to docs/perf/41-scroll-team-fixture.txt.
set -eu
cd "$(dirname "$0")/.."
out=docs/perf/41-scroll-team-fixture.txt
log=$(mktemp)
cargo test -q -p graft-fixtures --features heavy-tests --test it -- \
  --nocapture --test-threads=1 team:: >"$log" 2>&1
{
  echo "# 41-scroll-team-fixture: fresh build into a temp dir, debug test profile"
  echo "# $(date -u +%Y-%m-%dT%H:%MZ) $(uname -sm) $(git --version)"
  grep -o 'TEAM-MEASURE.*' "$log"
  grep -E '^test result' "$log"
} >"$out"
rm -f "$log"
cat "$out"
