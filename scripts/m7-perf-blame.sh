#!/usr/bin/env bash
# m7-power-tools/18: how long blame and the two traces take on the heavy
# synthetic repository's busiest file. Release build, 10 runs, from
# `crates/graft-git/examples/trace_perf.rs`. Writes
# docs/perf/trace-readers.txt (or $1); its last line is the summary:
#   trace-readers: blame p95=<ms> line_history p95=<ms> file_history p95=<ms>
set -u
cd "$(dirname "$0")/.."
OUT=${1:-docs/perf/trace-readers.txt}
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT

cargo build --release --quiet -p graft-git --features heavy-tests --example trace_perf 2>"$WORK/build.err" || {
  echo "release build failed:" >&2
  tail -20 "$WORK/build.err" >&2
  exit 1
}

cargo run --release --quiet -p graft-git --features heavy-tests --example trace_perf >"$WORK/run.out" 2>"$WORK/run.err"
status=$?

{
  echo "# m7-power-tools/18 trace readers — $(date -u +%FT%TZ)"
  echo "# HEAD $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- crates || echo ' +uncommitted')"
  echo "# $(sysctl -n machdep.cpu.brand_string 2>/dev/null || uname -m), $(sysctl -n hw.ncpu 2>/dev/null) cpus, $(uname -sr), $(git --version)"
  echo "# release, 10 runs; p95 is nearest-rank (with 10 runs, the slowest)"
  cat "$WORK/run.out"
  if [ "$status" -ne 0 ]; then
    echo "trace-readers: FAILED (exit $status)"
    tail -20 "$WORK/run.err"
  fi
} 2>&1 | sed "s|$HOME|~|g" >"$OUT"
echo "wrote $OUT"
exit "$status"
