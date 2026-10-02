#!/usr/bin/env bash
# m7-power-tools/48: re-take 47's two captures with the cursor fix in.
# Step 5 of scripts/47-scroll-acceptance.sh, alone (the perf steps are not
# this ticket's): the Ghostty-path PNG (ghostty_shot::, heavy, release) and
# the pty `text` walk's release grid over the same rows. Summary lines go to
# docs/perf/scroll-cursor-under-uncommitted-row.txt.
#
# Usage: .scratch/m7-power-tools/48-recapture.sh   (CARGO_TARGET_DIR honoured)
set -u
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
cd "$ROOT" || exit 2
TARGET=${CARGO_TARGET_DIR:-$ROOT/target}
export CARGO_TARGET_DIR=$TARGET
OUT=docs/perf/scroll-cursor-under-uncommitted-row.txt
WORK=$TARGET/48-recapture
mkdir -p "$WORK"
: >"$OUT"
missed=0
say() { echo "SUMMARY $1" | sed "s|$ROOT/||g" | tee -a "$OUT"; }
miss() { say "miss $1"; missed=1; }
echo "# 48-recapture — $(date -u +%Y-%m-%dT%H:%M:%SZ), HEAD $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- crates 2>/dev/null || echo ' (+ uncommitted changes)')" >>"$OUT"

TESTS=(cargo test --release --features heavy-tests --test it)
if ! "${TESTS[@]}" --no-run --quiet >"$WORK/build.txt" 2>&1 \
  || ! cargo build --release -q -p graft --bin graft >>"$WORK/build.txt" 2>&1; then
  miss "build failed: $WORK/build.txt"
  exit 1
fi

GRAFT_UPDATE_SHOTS=1 "${TESTS[@]}" -- ghostty_shot:: --test-threads=1 --nocapture >"$WORK/shot.txt" 2>&1
status_s=$?
shot=$(grep -E '^SUMMARY ghostty_png' "$WORK/shot.txt")
echo "$shot" >>"$OUT"
[ "$status_s" -eq 0 ] || miss "ghostty_shot:: exited $status_s: $WORK/shot.txt"
say "capture docs/shots/m7/fix-scroll-ghostty.png exit=$status_s $(echo "$shot" | grep -oE 'bytes=[0-9]+ .*')"

field() { echo "$1" | grep -oE " $2=[^ ]+" | head -1 | cut -d= -f2; }
FIXTURE=$(ls -dt "$TARGET"/fixtures/team-17700-* 2>/dev/null | head -1)
REPO=$WORK/repo
pane=$(field "$shot" pane_rows)
GRID=docs/shots/m7/fix-scroll-release-unicode.txt
if [ -z "$FIXTURE" ] || [ -z "$pane" ]; then
  miss "no MONOREPO fixture under $TARGET/fixtures or no pane_rows in the PNG summary"
else
  rm -rf "$REPO" && cp -R "$FIXTURE" "$REPO" && chmod -R u+w "$REPO"
  rows="$((${pane%..*} + 1)):$((${pane#*..} + 1))"
  line=$(env PROFILE=text DEFLATE="" RATE="" THEN_J=0.05 GRID_OUT="$WORK/grid.txt" GRID_ROWS="$rows" \
    python3 scripts/scroll-lag-pty.py "$TARGET/release/graft" "$REPO" 187 50 5 0.016 3 grid 2>"$WORK/pty.err")
  echo "$line" >>"$OUT"
  {
    echo "# m7-power-tools/47, re-taken by 48: the release grid, pty text walk (Unicode), TeamSpec::MONOREPO"
    echo "# 187x50, held j x188 at 16 ms; every byte before J's frame, drawn by pyte. Screen rows $rows (the"
    echo "# PNG's rows), cut at the graph's divider. Characters only: colour, the lit thread and"
    echo "# the cursor's fill are not recorded (CLAUDE.md rule 5)."
    cat "$WORK/grid.txt"
  } >"$GRID"
  cargo run -q -p graft-fixtures -- redact "$GRID" >"$WORK/redact.txt" 2>&1 \
    || miss "redact failed on $GRID: $WORK/redact.txt"
  say "capture $GRID rows=$rows lines=$(grep -vc '^#' "$GRID")"
fi
echo "# exit $missed" >>"$OUT"
[ "$missed" -eq 0 ] || exit 1
