#!/usr/bin/env bash
# m7-power-tools/18b (review Major 3): how long blame takes on a long file
# with interleaved history, from `crates/mergewright-git/examples/blame_longfile.rs`.
# Release build, 10 runs. Appends a block to docs/perf/trace-readers.txt (or
# $1) whose last line is the summary:
#   blame-longfile p95=<ms>
set -u
. "$(cd "$(dirname "$0")" && pwd)/lib/machine-state.sh"  # ticket 49: battery / Low Power Mode guard
cd "$(dirname "$0")/.." || exit 1
OUT=${1:-docs/perf/trace-readers.txt}
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
machine_state_begin "$OUT"

cargo build --release --quiet -p mergewright-git --example blame_longfile 2>"$WORK/build.err" || {
  echo "release build failed:" >&2
  tail -20 "$WORK/build.err" >&2
  exit 1
}

cargo run --release --quiet -p mergewright-git --example blame_longfile >"$WORK/run.out" 2>"$WORK/run.err"
status=$?

{
  echo
  echo "# m7-power-tools/18b blame on a long file — $(date -u +%FT%TZ)"
  echo "# HEAD $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- crates || echo ' +uncommitted')"
  echo "# $(sysctl -n machdep.cpu.brand_string 2>/dev/null || uname -m), $(sysctl -n hw.ncpu 2>/dev/null) cpus, $(uname -sr), $(git --version)"
  echo "# release, 10 runs; p95 is nearest-rank (with 10 runs, the slowest); default options (-M -w)"
  cat "$WORK/run.out"
  if [ "$status" -ne 0 ]; then
    echo "blame-longfile: FAILED (exit $status)"
    tail -20 "$WORK/run.err"
  fi
} 2>&1 | sed "s|$HOME|~|g" >>"$OUT"
echo "appended to $OUT"
exit "$status"
