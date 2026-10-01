#!/usr/bin/env bash
# m7-power-tools/19: how long finding lost commits takes — 500 reflog
# candidates on the heavy synthetic repository with 50 000 refs, three
# methods, 10 warm runs each. Release build, from
# `crates/graft-git/examples/lost_cost.rs`. Writes
# docs/perf/lost-commit-cost.txt (or $1); its last line is the summary:
#   lost-commit-cost: method=<m> p95=<ms> cands=500 refs=<n> · budget 1000 ms · <held|missed>
# Any further arguments go to the example (`<commits> <refs>`: a smaller
# repository, to check the harness quickly).
set -u
. "$(cd "$(dirname "$0")" && pwd)/lib/machine-state.sh"  # ticket 49: battery / Low Power Mode guard
cd "$(dirname "$0")/.."
OUT=${1:-docs/perf/lost-commit-cost.txt}
shift || true
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
machine_state_begin "$OUT"

cargo build --release --quiet -p graft-git --features heavy-tests --example lost_cost 2>"$WORK/build.err" || {
  echo "release build failed:" >&2
  tail -20 "$WORK/build.err" >&2
  exit 1
}

cargo run --release --quiet -p graft-git --features heavy-tests --example lost_cost -- "$@" >"$WORK/run.out" 2>"$WORK/run.err"
status=$?

{
  echo "# m7-power-tools/19 lost-commit cost — $(date -u +%FT%TZ)"
  echo "# HEAD $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- crates || echo ' +uncommitted')"
  echo "# $(sysctl -n machdep.cpu.brand_string 2>/dev/null || uname -m), $(sysctl -n hw.ncpu 2>/dev/null) cpus, $(uname -sr), $(git --version)"
  echo "# release, 10 warm runs per method; p95 is nearest-rank (with 10 runs, the slowest)"
  cat "$WORK/run.out"
  if [ "$status" -ne 0 ]; then
    echo "lost-commit-cost: FAILED (exit $status)"
    tail -20 "$WORK/run.err"
  fi
} 2>&1 | sed "s|$HOME|~|g" >"$OUT"
echo "wrote $OUT"
exit "$status"
