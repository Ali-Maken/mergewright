#!/usr/bin/env bash
# m7-power-tools/45: deflate cost on a mostly-transparent 187-column strip
# (six thin lanes, 1870×400 px). Runs `kitty::tests::deflate_shrinks_a_sparse_strip`
# in release, five times. Writes docs/perf/45-scroll-kitty-layer-deflate.txt
# (or $1). Summary lines, one per run:
#   deflate_shrinks_a_sparse_strip: raw <B> B, deflated <B> B (<pct>%), encode <ms> ms
# The size bound (≤ 10% of raw) is asserted by the test; time is printed
# only (target ≤ 2 ms). "encode" is the whole `KittyImages::frame` call for
# that strip: straight alpha, deflate, base64 and chunking together.
set -u
. "$(cd "$(dirname "$0")" && pwd)/lib/machine-state.sh"  # ticket 49: battery / Low Power Mode guard
cd "$(dirname "$0")/.." || exit 1
OUT=${1:-docs/perf/45-scroll-kitty-layer-deflate.txt}
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
machine_state_begin "$OUT"
export CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-$PWD/target}

cargo test --release --quiet -p mergewright-term --lib --no-run 2>"$WORK/build.err" || {
  echo "release build failed:" >&2
  tail -20 "$WORK/build.err" >&2
  exit 1
}

LOAD_BEFORE=$(uptime)
status=0
for _ in 1 2 3 4 5; do
  cargo test --release --quiet -p mergewright-term --lib deflate_shrinks_a_sparse_strip -- --nocapture \
    >>"$WORK/run.out" 2>>"$WORK/run.err" || status=$?
done
LOAD_AFTER=$(uptime)

{
  echo "# m7-power-tools/45 deflate cost — $(date -u +%FT%TZ)"
  echo "# HEAD $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- crates || echo ' +uncommitted')"
  echo "# $(sysctl -n machdep.cpu.brand_string 2>/dev/null || uname -m), $(sysctl -n hw.ncpu 2>/dev/null) cpus, $(uname -sr)"
  echo "# release, 5 runs; miniz_oxide level 1"
  echo "# uptime before: $LOAD_BEFORE"
  echo "# uptime after:  $LOAD_AFTER"
  grep '^deflate_shrinks_a_sparse_strip:' "$WORK/run.out"
  if [ "$status" -ne 0 ]; then
    echo "deflate-cost: FAILED (exit $status)"
    tail -20 "$WORK/run.err"
  fi
} 2>&1 | sed "s|$HOME|~|g" >"$OUT"
echo "wrote $OUT"
exit "$status"
