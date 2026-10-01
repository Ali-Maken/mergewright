#!/usr/bin/env bash
# m7-power-tools/44: what one cursor move costs per strip at the Pixel tier
# once the selection's paint is its own layer (ADR-0053), against today's
# full strip with the thread baked in. Release build, from the ignored test
# `emphasis_layer::emphasis_cost` in crates/graft-graph/tests/it. Recorded,
# not asserted (the gate is the held-key test, ticket 42).
#
# Writes docs/perf/44-scroll-emphasis-layer.txt (or $1); summary lines:
#   emphasis-cost: <window> <cells> cells 32 rows 17x37 <layer>: n=<n> p50=<µs> p99=<µs> max=<µs>
#   emphasis-cost: <window> emphasis width p50=<cells> p99=<cells> of <cells> cells
# <layer> is today-full-strip (what every `j` rasterises today, per strip),
# backdrop (once per strip, never per `j`), emphasis (per strip per `j` on a
# miss) and composite (iTerm2 only). Windows: `branchy` (the tests' window,
# 10 lanes plus the rail: 22 cells) and `wide-187` (93 lanes plus the rail,
# ~188 cells: a graph as wide as his 187-column terminal). The cell count
# measured is printed on each line.
set -u
cd "$(dirname "$0")/.."
OUT=${1:-docs/perf/44-scroll-emphasis-layer.txt}
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT

cargo test --release --quiet -p graft-graph --test it --no-run 2>"$WORK/build.err" || {
  echo "release build failed:" >&2
  tail -20 "$WORK/build.err" >&2
  exit 1
}

LOAD_BEFORE=$(uptime)
cargo test --release --quiet -p graft-graph --test it emphasis_layer::emphasis_cost \
  -- --ignored --nocapture --test-threads=1 >"$WORK/run.out" 2>"$WORK/run.err"
status=$?
LOAD_AFTER=$(uptime)

{
  echo "# m7-power-tools/44 scroll emphasis layer cost — $(date -u +%FT%TZ)"
  echo "# HEAD $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- crates || echo ' +uncommitted')"
  echo "# $(sysctl -n machdep.cpu.brand_string 2>/dev/null || uname -m), $(sysctl -n hw.ncpu 2>/dev/null) cpus, $(uname -sr)"
  echo "# release; per strip, 32 rows at 17x37; 4 rounds over the strip's 32 cursors, OffThread; p99 nearest-rank"
  echo "# uptime before: $LOAD_BEFORE"
  echo "# uptime after:  $LOAD_AFTER"
  ps -Ao pcpu=,comm= | awk '$1 > 50 { print "# busy after: " $0 }'
  grep '^emphasis-cost:' "$WORK/run.err"
  if [ "$status" -ne 0 ]; then
    echo "emphasis-cost: FAILED (exit $status)"
    tail -20 "$WORK/run.err"
  fi
} 2>&1 | sed "s|$HOME|~|g" >"$OUT"
echo "wrote $OUT"
exit "$status"
