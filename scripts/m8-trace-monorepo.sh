#!/usr/bin/env bash
# m8-polish-and-ship G-trace: the builder's GRAFT_PERF_TRACE run on the
# MONOREPO fixture (TeamSpec::MONOREPO, 17 700 commits, 1 330 refs) through a
# real pty, never his repo. 50 `j` presses at 33 ms after a 5 s load, 187x50,
# Unicode (the pty answers no probe). Writes docs/perf/m8-trace-monorepo.txt:
# the checks below, then scripts/m8-trace-summary.sh's lines.
#
# Checks (each `ok` or `FAIL`; exit 1 on any FAIL):
#   lines      every line is `header ...` or `frame ...`, every frame field
#              a number or `-` (header: tier/term words allowed)
#   frames     trace frame lines = frames the pty saw begin (DEC 2026)
#   presses    frames with motions>0 is 1..50, and total motions = keys sent
#   names      no subject word, author, ref name or the repo path in the trace
#
# Needs: the fixture under $CARGO_TARGET_DIR/fixtures (built by any heavy
# `held_key::` test) and the release binary (`cargo build --release -p graft`).
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
TARGET=${CARGO_TARGET_DIR:-$ROOT/target}
FIXTURE=$(ls -dt "$TARGET"/fixtures/team-17700-* 2>/dev/null | head -1)
[ -n "$FIXTURE" ] || { echo "no MONOREPO fixture under $TARGET/fixtures" >&2; exit 1; }
BIN=$TARGET/release/trib
[ -x "$BIN" ] || { echo "no release binary at $BIN" >&2; exit 1; }
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
REPO=$WORK/repo
# Graft writes .git/graft/ in what it opens; the cache is sealed read-only.
cp -R "$FIXTURE" "$REPO" && chmod -R u+w "$REPO"
TRACE=$WORK/trace.txt

line=$(GRAFT_PERF_TRACE=$TRACE python3 "$ROOT/scripts/scroll-lag-pty.py" \
  "$BIN" "$REPO" 187 50 5 0.033 1.65 m8-trace 2>"$WORK/pty.err" | grep SUMMARY || true)
sent=$(echo "$line" | grep -oE ' keys=[0-9]+' | cut -d= -f2)

fails=0
check() { # name ok? detail
  if [ "$2" = 1 ]; then echo "check $1 ok $3"; else echo "check $1 FAIL $3"; fails=$((fails + 1)); fi >>"$WORK/checks.txt"
}

bad=$(awk '
  $1 == "header" { for (i = 2; i <= NF; i++) { split($i, kv, "="); if (kv[1] != "tier" && kv[1] != "term" && kv[2] !~ /^[0-9]+$/) n++ } next }
  $1 == "frame"  { for (i = 2; i <= NF; i++) { split($i, kv, "="); if (kv[2] !~ /^([0-9]+|-)$/) n++ } next }
  { n++ }
  END { print n + 0 }' "$TRACE")
check lines "$([ "$bad" -eq 0 ] && echo 1 || echo 0)" "bad_fields=$bad"

# One line per drawn frame: the pty counted DEC 2026 begin markers.
seen=0
for k in frames_before_keys frames_while_held frames_after_release; do
  n=$(echo "$line" | grep -oE " $k=[0-9]+" | cut -d= -f2)
  seen=$((seen + ${n:-0}))
done
lines=$(grep -c '^frame ' "$TRACE" || true)
check frames "$([ "$lines" -eq "$seen" ] && [ "$seen" -gt 0 ] && echo 1 || echo 0)" "trace_lines=$lines pty_frames=$seen"

moved=$(grep -c ' motions=[1-9]' "$TRACE" || true)
motions=$(grep -oE ' motions=[0-9]+' "$TRACE" | cut -d= -f2 | awk '{ s += $1 } END { print s + 0 }')
ok=0; [ "$moved" -ge 1 ] && [ "$moved" -le 50 ] && [ "$motions" = "${sent:-x}" ] && ok=1
check presses "$ok" "keys_sent=${sent:-?} frames_with_motions=$moved motions=$motions"

# Words that are on his screen and must never be in the file.
words=$(git -C "$REPO" log -20 --format='%s%n%an' | tr -cs 'A-Za-z0-9' '\n' | awk 'length >= 5' | sort -u)
refs=$(git -C "$REPO" for-each-ref --count=20 --format='%(refname:short)')
hits=0
for w in $words $refs "$REPO" "$(basename "$FIXTURE")"; do
  if grep -qF -- "$w" "$TRACE"; then hits=$((hits + 1)); echo "name in trace: (word $hits)" >&2; fi
done
check names "$([ "$hits" -eq 0 ] && echo 1 || echo 0)" "words_checked=$(echo $words $refs | wc -w | tr -d ' ') hits=$hits"

OUT=$ROOT/docs/perf/m8-trace-monorepo.txt
{
  echo "# m8 G-trace builder run: MONOREPO fixture, real pty 187x50, release, Unicode (no probe answers),"
  echo "# 50 j at 33 ms after 5 s; TERM_PROGRAM=vscode (set by the pty script). $(date -u +%Y-%m-%dT%H:%MZ) $(uname -sm)"
  echo "${line#*SUMMARY }"
  cat "$WORK/checks.txt"
  "$ROOT/scripts/m8-trace-summary.sh" "$TRACE" "$WORK/summary.txt" >/dev/null
  cat "$WORK/summary.txt"
} >"$OUT"
cat "$OUT"
[ "$fails" -eq 0 ]
