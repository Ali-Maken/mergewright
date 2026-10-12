#!/usr/bin/env bash
# 47 debugger: Tributary-process RSS on the MONOREPO fixture over a real pty, at a rev.
#
#   BISECT_WT=<dir> scripts/47-debug-rss.sh <rev|.> [hold_s]
#
# Builds the release binary at <rev> (`.`: this worktree) and runs this
# worktree's scripts/scroll-lag-pty.py on a copy of the team-17700 fixture,
# 187x50, `j` at 16 ms for <hold_s> (default 3; 0 = open and idle only),
# profiles text and kitty (deflate answered yes), reader free. Appends
# SUMMARY lines to docs/perf/47-debug.txt; raw stderr under target/47-debug/.
set -uo pipefail
. "$(cd "$(dirname "$0")" && pwd)/lib/machine-state.sh"  # ticket 49: battery / Low Power Mode guard
ROOT=$(cd "$(dirname "$0")/.." && pwd)
REV=${1:?rev}
HOLD=${2:-3}
OUT="$ROOT/docs/perf/47-debug.txt"
machine_state_begin "$OUT"
WORK="$ROOT/target/47-debug"
mkdir -p "$WORK"
export MWR_FIXTURES_DIR="$ROOT/target/fixtures"
if [ "$REV" = "." ]; then
  SRC="$ROOT"; export CARGO_TARGET_DIR="$ROOT/target"; HASH="worktree"
else
  SRC="${BISECT_WT:?BISECT_WT}"
  export CARGO_TARGET_DIR="$ROOT/target/bisect"
  if [ ! -d "$SRC" ]; then git -C "$ROOT" worktree add -q --detach "$SRC" "$REV" || exit 1; fi
  git -C "$SRC" checkout -q --detach "$REV" || exit 1
  HASH=$(git -C "$SRC" rev-parse --short HEAD)
fi
( cd "$SRC" && cargo build --release -q -p mergewright --bin mwr ) >"$WORK/rss-$HASH.build" 2>&1 \
  || { echo "SUMMARY rss rev=$HASH build=FAILED" | tee -a "$OUT"; exit 1; }
BIN="$CARGO_TARGET_DIR/release/mwr"
FIXTURE=$(ls -dt "$ROOT"/target/fixtures/team-17700-* | head -1)
REPO="$WORK/repo"
for profile in ${PROFILES:-text kitty}; do
  rm -rf "$REPO" && cp -R "$FIXTURE" "$REPO" && chmod -R u+w "$REPO"
  line=$(env PROFILE="$profile" DEFLATE=yes ${EXTRA:-} python3 "$ROOT/scripts/scroll-lag-pty.py" \
    "$BIN" "$REPO" 187 50 5 0.016 "$HOLD" "$profile-$HASH" 2>"$WORK/rss-$HASH-$profile.err" | grep '^SUMMARY')
  rss=$(echo "$line" | grep -oE ' rss_mb=[^ ]+' | cut -d= -f2)
  broken=$(echo "$line" | grep -oE ' broken=[^ ]+' | cut -d= -f2)
  echo "SUMMARY rss rev=$HASH profile=$profile hold_s=$HOLD rss_mb=${rss:-?} broken=${broken:-?} $(date -u +%FT%TZ)" | tee -a "$OUT"
done
