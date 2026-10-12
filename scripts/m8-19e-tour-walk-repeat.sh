#!/usr/bin/env bash
# m8-polish-and-ship/19e (R2): the whole-tour walk, RUNS times in a row (10 by
# default), to show its captures never shoot an unsettled screen. The race
# showed under load, so this is meant to run beside a full `cargo nextest run`
# in the same worktree. To keep that to one cargo command at a time, it builds
# the workspace's test binaries once (`cargo test --no-run`, the same build
# the gate makes, so the gate rebuilds nothing under it), then runs the graft package's
# `it` binary directly, without cargo.
#
# Writes docs/perf/19e-tour-walk-repeat.txt (or $1): HEAD, the load, one line
# per run (and the panic's place for a failed one), then `pass=<n>/<runs>`.
# Exit 0 only when every run passed.
set -u
cd "$(dirname "$0")/.." || exit 1
OUT=${1:-docs/perf/19e-tour-walk-repeat.txt}
RUNS=${RUNS:-10}
TEST=tour_walk::the_whole_tour_in_a_pty_ticks_all_eight_and_comes_home
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT

load() { uptime | sed 's/.*load averages*: //'; }

cargo test --no-run --message-format=json >"$WORK/build.json" 2>"$WORK/build.err" || {
  echo "build failed:" >&2
  tail -20 "$WORK/build.err" >&2
  exit 1
}
# The integration-test binary of the graft package: the `it` target of the package at
# crates/graft (graft-ops has an `it` too).
BIN=$(python3 -c '
import json, sys
for line in open(sys.argv[1]):
    m = json.loads(line)
    if (m.get("reason") == "compiler-artifact" and m.get("executable")
            and m["target"]["name"] == "it" and "/crates/graft#" in m["package_id"]):
        print(m["executable"])
' "$WORK/build.json")
[ -x "$BIN" ] || { echo "no it binary of package graft in the build" >&2; exit 1; }

{
  echo "# 19e-tour-walk-repeat (m8-polish-and-ship/19e R2): $TEST, $RUNS runs in a row"
  echo "# HEAD $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- crates || echo ' (+ uncommitted changes under crates/)')"
  echo "# load at start: $(load)"
} >"$OUT"
pass=0
for n in $(seq 1 "$RUNS"); do
  start=$(date +%s)
  # As cargo runs it: from the package's folder, never regenerating captures.
  if (cd crates/graft && env -u MWR_UPDATE_SHOTS "$BIN" "$TEST" --exact >"$WORK/run-$n.out" 2>&1); then
    pass=$((pass + 1))
    verdict=pass
  else
    verdict=fail
  fi
  echo "run $n: $verdict $(($(date +%s) - start))s load $(load)" >>"$OUT"
  if [ "$verdict" = fail ]; then
    grep -m1 -o "panicked at [^ ]*" "$WORK/run-$n.out" | sed 's/^/  /' >>"$OUT"
  fi
done
echo "pass=$pass/$RUNS" >>"$OUT"
echo "pass=$pass/$RUNS"
[ "$pass" -eq "$RUNS" ]
