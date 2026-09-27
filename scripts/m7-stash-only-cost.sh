#!/usr/bin/env bash
# m7-power-tools/14a: how long `only_in_stash` takes for 1 and 10 stash
# entries on the fixture `lost_cost` builds (heavy synthetic, 50 000 refs,
# no commit-graph). Release build, from
# `crates/graft-git/examples/stash_only_cost.rs`. Writes
# docs/perf/stash-only-cost.txt (or $1); summary lines:
#   stash-only-cost: unreached cands=500 p50=<ms> p95=<ms> refs=<n> · the bound, this run
#   stash-only-cost: entries=<n> p50=<ms> p95=<ms> per-entry=<ms> refs=<n> · bound <ms> ms per entry (unreached p95, same run) · <held|missed>
# The bound is `unreached` timed in the same run, interleaved with the stash
# cases (review m7-20k-to-20l). `uptime` is recorded before and after.
# Further arguments go to the example (`<commits> <refs>`).
set -u
cd "$(dirname "$0")/.."
OUT=${1:-docs/perf/stash-only-cost.txt}
shift || true
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT

cargo build --release --quiet -p graft-git --features heavy-tests --example stash_only_cost 2>"$WORK/build.err" || {
  echo "release build failed:" >&2
  tail -20 "$WORK/build.err" >&2
  exit 1
}

LOAD_BEFORE=$(uptime)
cargo run --release --quiet -p graft-git --features heavy-tests --example stash_only_cost -- "$@" >"$WORK/run.out" 2>"$WORK/run.err"
status=$?
LOAD_AFTER=$(uptime)

{
  echo "# m7-power-tools/14a stash-only cost — $(date -u +%FT%TZ)"
  echo "# HEAD $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- crates || echo ' +uncommitted')"
  echo "# $(sysctl -n machdep.cpu.brand_string 2>/dev/null || uname -m), $(sysctl -n hw.ncpu 2>/dev/null) cpus, $(uname -sr), $(git --version)"
  echo "# release, 10 warm runs per case, cases interleaved; p95 is nearest-rank (with 10 runs, the slowest)"
  echo "# uptime before: $LOAD_BEFORE"
  echo "# uptime after:  $LOAD_AFTER"
  ps -Ao pcpu=,comm= | awk '$1 > 50 { print "# busy after: " $0 }'
  cat "$WORK/run.out"
  if [ "$status" -ne 0 ]; then
    echo "stash-only-cost: FAILED (exit $status)"
    tail -20 "$WORK/run.err"
  fi
} 2>&1 | sed "s|$HOME|~|g" >"$OUT"
echo "wrote $OUT"
exit "$status"
