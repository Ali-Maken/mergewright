#!/usr/bin/env bash
# M7 acceptance (m7-power-tools/26): "you can rewrite, trace and recover
# history without leaving the app". One command walks every scenario.
#
#   scripts/m7-accept.sh
#
# Raw output goes to .scratch/m7-power-tools/accept/; the summary to
# docs/perf/m7-accept.txt. This prints paths and one line per step, never
# test output or frames. Exits non-zero on any failure.
#
# Steps:
#   1. cargo nextest, filtered to the M7 walks (m7_walk::*, which holds every
#      journey walk, every_m7_journey_id_has_a_walk and the promise
#      scenarios), journey_counts_match_the_note and the hint-bar tests.
#   2. Every m3–m6 pty walk in compare mode (GRAFT_UPDATE_SHOTS unset), each
#      at every size its captures exist at. M7 has no pty walk: its captures
#      are drawn in-process and compared by step 1's tests (check_shot).
#      A failing run's log (it names the file, the first differing line and
#      the command that re-takes it) goes to accept/pty-diffs/.
#   3. The contact sheet docs/shots/m7/m7-contact-sheet*.txt, from its
#      captions file in row order (the journal's captures first), under the
#      "Unjudged in rev 2" heading.
#   4. The perf summary lines of 04, 19 and 18, copied verbatim.
#   5. The journey tally.
#
# CARGO_TARGET_DIR is honoured (default: target).
set -u

ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT" || exit 2
ACCEPT=.scratch/m7-power-tools/accept
SUMMARY=docs/perf/m7-accept.txt
TARGET=${CARGO_TARGET_DIR:-target}
mkdir -p "$ACCEPT/pty-diffs" "$(dirname "$SUMMARY")"
rm -f "$ACCEPT"/pty-diffs/*.txt "$ACCEPT"/pty-*.txt
failed=0
out() { echo "$1" | tee -a "$SUMMARY"; }

{
  echo "# m7-acceptance — $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "# HEAD $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- . 2>/dev/null || echo ' +uncommitted')"
  echo "# $(uname -sr), $(git --version)"
} > "$SUMMARY"

# ── 1. the walks ────────────────────────────────────────────────────────────
FILTER='test(/^m7_walk::/) | test(/journey_counts_match_the_note$/) | test(/^hint_bar::/)'
cargo nextest run --workspace --no-fail-fast --status-level pass -E "$FILTER" > "$ACCEPT/nextest.txt" 2>&1
rc=$?
line=$(grep -E '^ +Summary ' "$ACCEPT/nextest.txt" | tail -1 | sed -E 's/^ +//')
out "1 nextest: ${line:-no summary (build failed?)} · rc=$rc · $ACCEPT/nextest.txt"
[ "$rc" -eq 0 ] || failed=1

# ── 2. pty walks, compare mode ──────────────────────────────────────────────
cargo build -q -p graft --example m3_walk --example m4_walk --example m5_walk --bin trib \
  > "$ACCEPT/pty-build.txt" 2>&1 \
  && cargo build -q -p graft-fixtures --bin graft-fixtures >> "$ACCEPT/pty-build.txt" 2>&1
if [ $? -ne 0 ]; then
  out "2 pty: build failed · $ACCEPT/pty-build.txt"
  failed=1
else
  pty_runs=0
  pty_bad=0
  # milestone  script  example  stem-prefix  shots-dir
  while read -r m script example prefix dir; do
    # stem<TAB>scenario for this milestone's captures.
    "$TARGET/debug/examples/$example" --stems | awk -F'\t' -v p="$prefix" 'index($1, p) == 1 { print $1 "\t" $2 }' \
      > "$ACCEPT/pty-$m-stems.tsv"
    # size<TAB>scenario for every capture file that exists.
    : > "$ACCEPT/pty-$m-plan.tsv"
    while IFS="$(printf '\t')" read -r stem scenario; do
      for file in "$dir/$stem"-*x*.txt; do
        [ -e "$file" ] || continue
        size=${file##*-}
        size=${size%.txt}
        case "$size" in *[!0-9x]*) continue ;; esac
        printf '%s\t%s\n' "$size" "$scenario" >> "$ACCEPT/pty-$m-plan.tsv"
      done
    done < "$ACCEPT/pty-$m-stems.tsv"
    for size in $(cut -f1 "$ACCEPT/pty-$m-plan.tsv" | sort -u); do
      scenarios=$(awk -F'\t' -v s="$size" '$1 == s { print $2 }' "$ACCEPT/pty-$m-plan.tsv" | sort -u | tr '\n' ' ')
      log="$ACCEPT/pty-$m-$size.txt"
      # shellcheck disable=SC2086 # the scenario names are words
      env -u GRAFT_UPDATE_SHOTS GRAFT_TARGET="$TARGET" python3 "$script" "$size" $scenarios > "$log" 2>&1
      prc=$?
      pty_runs=$((pty_runs + 1))
      if [ "$prc" -ne 0 ]; then
        pty_bad=$((pty_bad + 1))
        cp "$log" "$ACCEPT/pty-diffs/$m-$size.txt"
      fi
    done
  done <<'WALKS'
m3 .scratch/m3-working-tree-undo-and-stash/pty-walk-m3.py m3_walk m3- docs/shots
m4 .scratch/m4-remotes/pty-walk-m4.py m4_walk m4- docs/shots
m5 .scratch/m5-history-and-ghost-preview/pty-walk-m5.py m5_walk m5- docs/shots
m6 .scratch/m6-conflicts/pty-walk-m6.py m5_walk m6- docs/shots/m6
WALKS
  out "2 pty: $pty_runs runs (m3–m6 × sizes), $pty_bad drifted or failed · $ACCEPT/pty-*.txt, $ACCEPT/pty-diffs/"
  [ "$pty_bad" -eq 0 ] || failed=1
fi

# ── 3. the contact sheet ────────────────────────────────────────────────────
if scripts/contact-sheet.sh --captions-order m7 docs/shots/m7 > "$ACCEPT/sheet.txt" 2>&1; then
  heading='# Unjudged in rev 2: the journal, and every no-colour carrier. Every frame below is colourless: these are what you\n# see with colour off.\n#'
  first=$(ls docs/shots/m7/m7-contact-sheet.txt docs/shots/m7/m7-contact-sheet-1.txt 2>/dev/null | head -1)
  # The heading goes above the first capture (its `====` rule).
  awk -v h="$heading" '!done && /^====/ { gsub(/\\n/, "\n", h); print h; done = 1 } { print }' "$first" > "$ACCEPT/sheet-head.tmp" \
    && mv "$ACCEPT/sheet-head.tmp" "$first"
  big=0
  for sheet in docs/shots/m7/m7-contact-sheet*.txt; do
    [ "$(wc -c < "$sheet")" -le 204800 ] || big=1
  done
  sheets=$(ls docs/shots/m7/m7-contact-sheet*.txt | tr '\n' ' ')
  out "3 sheet: ${sheets}· every file ≤ 200 KB: $([ $big -eq 0 ] && echo yes || echo NO)"
  [ "$big" -eq 0 ] || failed=1
else
  out "3 sheet: contact-sheet.sh failed · $ACCEPT/sheet.txt"
  failed=1
fi

# ── 4. perf summary lines, verbatim ─────────────────────────────────────────
# 04's file is docs/perf/m7-04-slow-key.txt (the ticket's `m7-key.txt`).
perf_missing=0
perf_missed=0
{
  echo "## perf summary lines (verbatim)"
} >> "$SUMMARY"
for spec in 'docs/perf/m7-04-slow-key.txt|^SUMMARY ' \
            'docs/perf/lost-commit-cost.txt|^lost-commit-cost: ' \
            'docs/perf/trace-readers.txt|^(trace-readers: |blame-longfile p95=)'; do
  file=${spec%%|*}
  pattern=${spec#*|}
  if [ ! -f "$file" ]; then
    echo "MISSING $file" >> "$SUMMARY"
    perf_missing=$((perf_missing + 1))
    continue
  fi
  echo "### $file" >> "$SUMMARY"
  grep -E "$pattern" "$file" >> "$SUMMARY"
  n=$(grep -E "$pattern" "$file" | grep -ciE '(^|[^a-z])missed([^a-z]|$)')
  perf_missed=$((perf_missed + n))
done
echo "4 perf: 3 files, $perf_missing missing, $perf_missed lines say missed · $SUMMARY"
[ "$perf_missing" -eq 0 ] || failed=1

# ── 5. the journey tally ────────────────────────────────────────────────────
# Each id of the note's two fences and its walk, from m7_walk.rs's WALKS.
NOTE=docs/design-notes/m7-power-tools.md
ids=$(awk '/^```journey-counts$/ { f = 1; next } /^```/ { f = 0 } f && !/^#/ && NF { print $1 }' "$NOTE")
walked=0
under=0
over=0
other=0
for id in $ids; do
  test=$(ID="$id" perl -0ne 'print $1 if /\(\s*"\Q$ENV{ID}\E",\s*"[^"]+",\s*"([^"]+)"/' crates/graft/tests/it/m7_walk.rs)
  [ -n "$test" ] || { other=$((other + 1)); continue; }
  hits=$(grep -E "(PASS|FAIL) .*m7_walk::.*::$test$|(PASS|FAIL) .*m7_walk::$test$" "$ACCEPT/nextest.txt" | head -1)
  case "$hits" in
    *PASS*) walked=$((walked + 1)); under=$((under + 1)) ;;
    *FAIL*)
      walked=$((walked + 1))
      if grep -qE "$id.*(the note allows|allows [0-9])" "$ACCEPT/nextest.txt"; then
        over=$((over + 1))
      else
        other=$((other + 1))
      fi ;;
    *) other=$((other + 1)) ;;
  esac
done
out "accept: $walked journeys walked, $under at or under count, $over over$([ $other -gt 0 ] && echo ", $other failed or not run")"
[ "$over" -eq 0 ] && [ "$other" -eq 0 ] || failed=1

echo "summary: $SUMMARY"
exit "$failed"
