#!/usr/bin/env bash
# M8 acceptance (m8-polish-and-ship/28): "it stays smooth on your biggest
# repo, looks like the mock-up, installs with one command, and teaches
# itself from inside the app". One command walks every scenario.
#
#   scripts/m8-accept.sh            steps 1, 3, 4, 5
#   scripts/m8-accept.sh --perf     and step 2, the release scroll run
#
# Raw output goes to .scratch/m8-polish-and-ship/accept/; the summary to
# docs/perf/m8-accept.txt. Prints paths and one line per step, never test
# output or frames. Exits non-zero on any failure.
#
# Steps:
#   1. The fast tier, workspace-wide (cargo nextest run --workspace). It
#      holds every M8 journey walk (WALKS in tests/it/m8_walk/mod.rs, held
#      to the note's fence by every_m8_journey_id_has_a_walk), the §9
#      "teaches itself" tests (m8_walk::teaches), the tour walked to
#      `practice done ✓` with the `mwr` binary (tour_walk), and every
#      capture under docs/shots compared with what the code draws
#      (check_shot; MWR_UPDATE_SHOTS unset), the look re-shoots included.
#   2. (--perf only) the heavy tier's held-key runs in release: held `j` at
#      Unicode and Pixel (raw and deflate), `G` after idle
#      (frames::after_idle::g_after_idle), RSS after held `j` at Pixel
#      (heavy::budgets_hold_at_pixel_*). Their summary lines go to
#      docs/perf/m8-accept.txt. Measurement never runs in a helper's context (CLAUDE.md Gates): run this from a
#      terminal, then read the summary.
#   3. The contact sheet: scripts/contact-sheet.sh --captions-order m8
#      docs/shots → docs/shots/m8-contact-sheet*.txt.
#   4. The release facts the install clause rests on, quoted from
#      docs/release/*.txt (not re-run here: dist needs the network).
#   5. The journey tally: fence ids against WALKS entries.
#
# CARGO_TARGET_DIR is honoured (default: target).
set -u
ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT" || exit 2
ACCEPT=.scratch/m8-polish-and-ship/accept
SUMMARY=docs/perf/m8-accept.txt
perf=false
[ "${1:-}" = "--perf" ] && perf=true
mkdir -p "$ACCEPT" "$(dirname "$SUMMARY")"
failed=0
out() { echo "$1" | tee -a "$SUMMARY"; }

{
  echo "# m8-acceptance — $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "# HEAD $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- . 2>/dev/null || echo ' +uncommitted')"
  echo "# $(uname -sr), $(git --version)"
} > "$SUMMARY"

# ── 1. the fast tier: walks, teaches, tour, captures ────────────────────────
env -u MWR_UPDATE_SHOTS cargo nextest run --workspace --no-fail-fast > "$ACCEPT/nextest.txt" 2>&1
rc=$?
line=$(grep -E '^ +Summary ' "$ACCEPT/nextest.txt" | tail -1 | sed -E 's/^ +//')
out "1 fast tier: ${line:-no summary (build failed?)} · rc=$rc · $ACCEPT/nextest.txt"
[ "$rc" -eq 0 ] || failed=1

# ── 2. scroll, release (heavy filters) ──────────────────────────────────────
if $perf; then
  # No held-wheel heavy test exists: J2's stop is held in the fast tier
  # (m8_walk::journeys::wheel_flick, headless), its release tail unmeasured.
  FILTER='test(/held_j_stops_on_release_/) | test(/g_after_idle/) | test(/budgets_hold_at_pixel_/)'
  cargo nextest run --release -p mergewright --features heavy-tests --no-fail-fast \
    --no-capture -E "$FILTER" > "$ACCEPT/perf.txt" 2>&1
  prc=$?
  pline=$(grep -E '^ +Summary ' "$ACCEPT/perf.txt" | tail -1 | sed -E 's/^ +//')
  out "2 scroll (release): ${pline:-no summary (build failed?)} · rc=$prc · $ACCEPT/perf.txt"
  grep -E 'tail|after release|RSS|rss|budget' "$ACCEPT/perf.txt" | sed 's/^/    /' >> "$SUMMARY"
  [ "$prc" -eq 0 ] || failed=1
else
  out "2 scroll (release): not run — pass --perf from a terminal (CLAUDE.md: never in a helper's context)"
fi

# ── 3. the contact sheet ────────────────────────────────────────────────────
if scripts/contact-sheet.sh --captions-order m8 docs/shots > "$ACCEPT/sheet.txt" 2>&1; then
  out "3 contact sheet: $(head -1 "$ACCEPT/sheet.txt")"
  sed -n '2,$p' "$ACCEPT/sheet.txt" | tee -a "$SUMMARY"
else
  out "3 contact sheet: FAILED · $ACCEPT/sheet.txt"
  failed=1
fi

# ── 4. install facts, quoted ────────────────────────────────────────────────
out "4 install (quoted from docs/release/, dated in each file's first line):"
{
  echo "    plan targets: $(grep -cE '^    graft-[a-z0-9_]+-[a-z-]+\.tar\.xz$' docs/release/plan.txt) archives · $(head -1 docs/release/plan.txt)"
  echo "    host archive: $(grep -m1 -E '\.1$' docs/release/build.txt | sed -E 's/^ +//') (man page in the archive)"
  echo "    formula style: $(grep -m1 -E '^brew style' docs/release/build.txt)"
  echo "    cargo install: $(grep -m1 -E '^install:' docs/release/cargo-install.txt)"
} | tee -a "$SUMMARY"

# ── 5. journey tally ────────────────────────────────────────────────────────
fence=$(awk '/^```journey-counts/{on=1; next} /^```/{on=0} on && $1 !~ /^#/ && NF {print $1}' \
  docs/design-notes/m8-polish-and-ship.md | sort -u)
walked=$(grep -oE '^\s+\("[a-z-]+",|^\s+"[a-z-]+",$' crates/graft/tests/it/m8_walk/mod.rs \
  | grep -oE '"[a-z-]+"' | tr -d '"' | sort -u)
missing=$(comm -23 <(echo "$fence") <(echo "$walked") | tr '\n' ' ')
out "5 journeys: $(echo "$fence" | wc -l | tr -d ' ') in the fence · missing a walk: ${missing:-none}"
[ -z "$missing" ] || failed=1

out "summary: $SUMMARY"
exit "$failed"
