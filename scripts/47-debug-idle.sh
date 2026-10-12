#!/usr/bin/env bash
# 47 debugger: measures `frames::after_idle::g_after_idle` (release) at a rev.
#
#   scripts/47-debug-idle.sh <rev|HEAD-worktree> [runs]
#
# A rev other than `.` is checked out detached into one reusable bisect
# worktree (scratch dir, never committed); `.` measures this worktree as it
# stands. Fixtures are shared through MWR_FIXTURES_DIR. Appends raw lines to
# target/47-debug/<rev>.txt and SUMMARY lines to docs/perf/47-debug.txt;
# prints the SUMMARY line only.
set -uo pipefail
. "$(cd "$(dirname "$0")" && pwd)/lib/machine-state.sh"  # ticket 49: battery / Low Power Mode guard
ROOT=$(cd "$(dirname "$0")/.." && pwd)
REV=${1:?rev}
RUNS=${2:-3}
OUT="$ROOT/docs/perf/47-debug.txt"
machine_state_begin "$OUT"
WORK="$ROOT/target/47-debug"
mkdir -p "$WORK"
export MWR_FIXTURES_DIR="$ROOT/target/fixtures"
FILTER=${FILTER:-frames::after_idle::g_after_idle}

if [ "$REV" = "." ]; then
  SRC="$ROOT"; export CARGO_TARGET_DIR="$ROOT/target"; HASH="worktree"
else
  SRC="${BISECT_WT:?BISECT_WT: the bisect worktree path}"
  export CARGO_TARGET_DIR="$ROOT/target/bisect"
  if [ ! -d "$SRC" ]; then git -C "$ROOT" worktree add -q --detach "$SRC" "$REV" || exit 1; fi
  git -C "$SRC" checkout -q --detach "$REV" || exit 1
  HASH=$(git -C "$SRC" rev-parse --short HEAD)
fi
LOG="$WORK/$HASH.txt"
( cd "$SRC" && cargo test --release --features graft/heavy-tests -p mergewright --test it --no-run ) >"$LOG.build" 2>&1 \
  || { echo "SUMMARY idle rev=$HASH build=FAILED ($LOG.build)" | tee -a "$OUT"; exit 1; }
BIN=$(grep -oE '\(target/[^)]*it-[0-9a-f]+\)|\([^)]*/deps/it-[0-9a-f]+\)' "$LOG.build" | tail -1 | tr -d '()')
case "$BIN" in /*) ;; *) BIN="$CARGO_TARGET_DIR/${BIN#target/}";; esac
each=()
for _ in $(seq 1 "$RUNS"); do
  ( cd "$SRC/crates/graft" && "$BIN" "$FILTER" --exact --test-threads=1 --nocapture ) >>"$LOG" 2>&1
  us=$(grep -oE 'g_after_idle build=idle-advance .*latency_us=[0-9]+' "$LOG" | tail -1 | grep -oE '[0-9]+$')
  each+=("${us:-?}")
done
echo "SUMMARY idle rev=$HASH runs=$RUNS latency_us=$(IFS=,; echo "${each[*]}") $(date -u +%FT%TZ)" | tee -a "$OUT"
