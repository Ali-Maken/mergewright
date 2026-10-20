#!/usr/bin/env bash
# Ease-pass acceptance (ease-pass/14): "the three mocks are what the app draws
# and how it behaves". One command walks every scenario.
#
#   scripts/ease-accept.sh
#
# Raw output goes to .scratch/ease-pass/accept/; the summary to
# docs/perf/ease-accept.txt. Prints paths and one line per step, never test
# output or frames. Exits non-zero on any failure.
#
# What it walks: the `ease_walk` tests of the mergewright crate. They drive the
# real app in-process (update + view + the real job pool, over real git, in
# fixture repos shaped like the mocks: `mock` and `team`) and read the drawn
# frame; they do not run the binary in a pty. Colour is unverified (rule 5):
# every check is of words and shapes.
#
# Steps:
#   1. The ease_walk tests, workspace build, MWR_UPDATE_SHOTS unset (captures
#      are compared, never rewritten).
#   2. The nine journeys of the design note's `journeys` fence, one line each:
#      `journey <id> <keys walked>/<keys allowed>`; fails if any walk is over,
#      or if commit-all-subject is not 3/3 or merge-one-clash is not 5/5.
#   3. The behaviour ids: G1-G14, C1-C15, M1-M19 and Q1-Q17, the three g12_,
#      the three fold_ and picks_shell_merge: each must have a PASS line.
#   4. The contact sheet: scripts/contact-sheet.sh --captions-order ease-pass
#      docs/shots/ease-pass, then every file under 200 KB, and no NOT RECORDED.
#   5. The pixel-lag summary, quoted from docs/perf/ease-pixel.txt (not
#      judged here).
#
# CARGO_TARGET_DIR is honoured (default: target).
set -u
ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT" || exit 2
ACCEPT=.scratch/ease-pass/accept
SUMMARY=docs/perf/ease-accept.txt
SHOTS=docs/shots/ease-pass
mkdir -p "$ACCEPT" "$(dirname "$SUMMARY")"
failed=0
out() { echo "$1" | tee -a "$SUMMARY"; }

{
  echo "# ease-pass acceptance — $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "# HEAD $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- . 2>/dev/null || echo ' +uncommitted')"
  echo "# $(uname -sr), $(git --version)"
} > "$SUMMARY"

# ── 1. the ease_walk tests ──────────────────────────────────────────────────
env -u MWR_UPDATE_SHOTS cargo nextest run -p mergewright --test it --no-fail-fast \
  --status-level pass --success-output immediate -E 'test(/^ease_walk::/)' \
  > "$ACCEPT/walks.txt" 2>&1
rc=$?
line=$(grep -E '^ +Summary ' "$ACCEPT/walks.txt" | tail -1 | sed -E 's/^ +//')
out "1 walks (in-process, real git): ${line:-no summary (build failed?)} · rc=$rc · $ACCEPT/walks.txt"
[ "$rc" -eq 0 ] || failed=1

# ── 2. the nine journeys ────────────────────────────────────────────────────
out "2 journeys (keys walked/allowed, the note's fence):"
grep -E '^ *journey [a-z-]+ [0-9]+/[0-9]+ ' "$ACCEPT/walks.txt" | sed -E 's/^ +//' | sort -u > "$ACCEPT/journeys.txt"
over=0
while read -r _ id counts _; do
  walked=${counts%/*}
  allowed=${counts#*/}
  mark=""
  if [ "$walked" -gt "$allowed" ]; then
    mark=" OVER"
    over=$((over + 1))
  fi
  out "    $id $counts$mark"
done < "$ACCEPT/journeys.txt"
n=$(wc -l < "$ACCEPT/journeys.txt" | tr -d ' ')
lines_ok=true
[ "$n" -eq 9 ] || lines_ok=false
grep -qE '^journey commit-all-subject 3/3 ' "$ACCEPT/journeys.txt" || lines_ok=false
grep -qE '^journey merge-one-clash 5/5 ' "$ACCEPT/journeys.txt" || lines_ok=false
if [ "$over" -eq 0 ] && $lines_ok; then
  out "    $n journeys, none over; commit-all-subject 3/3, merge-one-clash 5/5"
else
  out "    FAILED: $n journey lines (want 9), $over over, or the two counted ones are not 3/3 and 5/5"
  failed=1
fi

# ── 3. the behaviour ids ────────────────────────────────────────────────────
passed() { # <test name prefix>: a PASS line for it
  grep -E "^ +PASS " "$ACCEPT/walks.txt" | grep -qE "mergewright::it ease_walk::[a-z_]+::([a-z][0-9]+_)*$1"
}
tally() { # <label> <prefix letter> <count>
  local label=$1 letter=$2 count=$3 miss="" i
  for i in $(seq 1 "$count"); do
    passed "${letter}${i}_" || miss="$miss ${letter}${i}"
  done
  if [ -z "$miss" ]; then
    out "    $label: $count of $count"
  else
    out "    $label: FAILED, no passing test for:$miss"
    failed=1
  fi
}
out "3 behaviours (a passing test named by each id):"
tally "G1-G14 graph" g 14
tally "C1-C15 commit" c 15
tally "M1-M19 merge" m 19
tally "Q1-Q17 questions" q 17
named=""
for t in g12_sibling_never_built_on g12_teammate_sync_is_not_a_merge g12_dependent_before_the_move \
  fold_sync_merge_has_no_mark fold_reveal_parent fold_same_without_commit_graph picks_shell_merge; do
  passed "$t\$" || named="$named $t"
done
if [ -z "$named" ]; then
  out "    g12_ x3, fold_ x3, picks_shell_merge: 7 of 7"
else
  out "    named walks FAILED, no passing test for:$named"
  failed=1
fi

# ── 4. captures and the contact sheet ───────────────────────────────────────
if scripts/contact-sheet.sh --captions-order ease-pass "$SHOTS" > "$ACCEPT/sheet.txt" 2>&1; then
  out "4 contact sheet: $(head -1 "$ACCEPT/sheet.txt")"
  sed -n '2,$p' "$ACCEPT/sheet.txt" | tee -a "$SUMMARY"
else
  out "4 contact sheet: FAILED · $ACCEPT/sheet.txt"
  failed=1
fi
big=$(find "$SHOTS" -type f -size +200000c | tr '\n' ' ')
nr=$(cat "$SHOTS"/ease-pass-contact-sheet*.txt | grep -c 'NOT RECORDED')
out "    over 200 KB: ${big:-none} · NOT RECORDED: $nr · colour unverified (rule 5)"
{ [ -z "$big" ] && [ "$nr" -eq 0 ]; } || failed=1

# ── 5. pixel lag, quoted ────────────────────────────────────────────────────
out "5 pixel lag (quoted from docs/perf/ease-pixel.txt, not judged; the Ghostty run is the user's):"
if [ -f docs/perf/ease-pixel.txt ]; then
  head -n 2 docs/perf/ease-pixel.txt | tail -n 1 | sed 's/^/    /' | tee -a "$SUMMARY"
  grep -E '^SUMMARY bin=' docs/perf/ease-pixel.txt \
    | sed -E 's/ (input_age_us_p50|draw_us_p95|bytes_p95|keyed_frames|transmits|broken|input_age_us_p99|pty_tail_ms)=[^ ]+//g; s/^SUMMARY /    /' \
    | tee -a "$SUMMARY"
else
  out "    docs/perf/ease-pixel.txt missing"
  failed=1
fi

out "summary: $SUMMARY"
exit "$failed"
