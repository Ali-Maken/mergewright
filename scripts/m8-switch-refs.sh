#!/usr/bin/env bash
# m8-polish-and-ship/12c: `switch_refs` (BRANCHES `reading…`, the `C` picker)
# split by phase on the piled fixture of scripts/perf-palette.sh (40 000
# branches + 10 000 tags on 3 commits), release. Measurement only, no fix.
# Runs crates/mergewright-git/examples/switch_refs_cost.rs; writes
# docs/perf/m8-switch-refs.txt, summary lines only, numbers only.
# The fixture must exist: run scripts/perf-palette.sh piled first.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
TARGET=${CARGO_TARGET_DIR:-$ROOT/target}
SCRATCH=${SCRATCH:-$TARGET/perf-palette}
FIXTURE=$SCRATCH/refs-50000
[ -d "$FIXTURE/.git" ] || { echo "no piled fixture at $FIXTURE: run scripts/perf-palette.sh piled" >&2; exit 1; }
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
cp -R "$FIXTURE" "$WORK/repo"
chmod -R u+w "$WORK/repo"

cd "$ROOT"
cargo build --release --quiet -p mergewright-git --example switch_refs_cost
started_load=$(uptime | sed 's/.*load average[s]*: //')
"$TARGET/release/examples/switch_refs_cost" "$WORK/repo" >"$WORK/run.out"

OUT=$ROOT/docs/perf/m8-switch-refs.txt
{
  echo "# m8/12c switch_refs by phase: piled fixture (40 000 branches + 10 000 tags, 3 commits), release."
  echo "# $(date -u +%Y-%m-%dT%H:%MZ) $(uname -sm); load at start $started_load; git $(git --version | cut -d' ' -f3)"
  grep '^switch-refs-cost:' "$WORK/run.out"
} >"$OUT"
cat "$OUT"
