#!/usr/bin/env bash
# m7-power-tools/18e: blame on the long file with an ignore list in effect,
# from `crates/graft-git/examples/blame_longfile.rs --with-list`. On git
# before 2.50 this includes the human-format pass for the marks. Release
# build, 10 runs. Usage:
#   scripts/m7-perf-blame-list.sh [<dir holding the git to use>] [<out file>]
# The first argument goes first on PATH (e.g. a 2.38.5 build under $TMPDIR).
# Appends a block to docs/perf/18e.txt (or $2) whose last line is:
#   blame-longfile-list p95=<ms>
set -u
. "$(cd "$(dirname "$0")" && pwd)/lib/machine-state.sh"  # ticket 49: battery / Low Power Mode guard
cd "$(dirname "$0")/.."
GITDIR=${1:-}
OUT=${2:-docs/perf/18e.txt}
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
machine_state_begin "$OUT"

cargo build --release --quiet -p graft-git --example blame_longfile 2>"$WORK/build.err" || {
  echo "release build failed:" >&2
  tail -20 "$WORK/build.err" >&2
  exit 1
}
BIN=${CARGO_TARGET_DIR:-target}/release/examples/blame_longfile

if [ -n "$GITDIR" ]; then
  export PATH="$GITDIR:$PATH"
fi
"$BIN" --with-list >"$WORK/run.out" 2>"$WORK/run.err"
status=$?

{
  echo
  echo "# m7-power-tools/18e blame on a long file, ignore list in effect — $(date -u +%FT%TZ)"
  echo "# HEAD $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- crates || echo ' +uncommitted')"
  echo "# $(sysctl -n machdep.cpu.brand_string 2>/dev/null || uname -m), $(sysctl -n hw.ncpu 2>/dev/null) cpus, $(uname -sr), $(git --version)"
  echo "# release, 10 runs; p95 is nearest-rank (with 10 runs, the slowest); default options (-M -w)"
  cat "$WORK/run.out"
  if [ "$status" -ne 0 ]; then
    echo "blame-longfile-list: FAILED (exit $status)"
    tail -20 "$WORK/run.err"
  fi
} 2>&1 | sed -e "s|$HOME|~|g" -e "s|${TMPDIR:-/nonexistent}|\$TMPDIR/|g" >>"$OUT"
echo "appended to $OUT"
exit "$status"
