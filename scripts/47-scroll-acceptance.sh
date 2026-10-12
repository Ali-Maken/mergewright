#!/usr/bin/env bash
# m7-power-tools/47: the scroll fix's acceptance. "On a repository his size,
# scrolling stops when he lets go of the key, and the next key acts at once —
# on both tiers" (Unicode; Pixel on Kitty with deflate and on iTerm2). Kitty
# without deflate misses by known gap M7/46-kitty-no-deflate and is printed,
# not judged.
#
#   scripts/47-scroll-acceptance.sh
#
# Writes docs/perf/47-scroll-acceptance-<UTC date>-<HEAD>.txt, a new file each
# run, and prints only its SUMMARY lines
# and paths. Raw test and pty output stay under $CARGO_TARGET_DIR/47-acceptance/
# (ignored; never committed). Exit 1 on any miss, each named in a
# `SUMMARY miss` line. Release builds only (ADR-0021).
#
# Steps:
#   1. release held_key::, two invocations: (a) every held_key test but the
#      raw Kitty pair, must exit 0; (b) the raw pair alone, judged by its
#      VERDICT lines against the known-gap list, never by exit status.
#   2. release frames:: (g_after_idle included) and heavy::budgets_hold.
#   3. pty walk (scripts/scroll-lag-pty.py) on a writable copy of the
#      TeamSpec::MONOREPO fixture, never his repo: 187x50, j at 16 ms for 3 s,
#      then J after 50 ms; text (free, 500 KB/s), kitty deflate=yes (free,
#      40 MB/s, judged), kitty deflate=no (same, known gap).
#   4. the journeys' step counts.
#   5. the captures: docs/shots/m7/fix-scroll-ghostty.png (ghostty_shot::,
#      compare mode) and docs/shots/m7/fix-scroll-release-unicode.txt (one
#      more text pty run, its release grid at the PNG's rows).
#
# docs/perf/47-scroll-acceptance.txt is the quiet baseline (a4d1f93b, load
# 1.1) that later runs are compared against; this script never writes it (m8
# 04 review, Major 3: an under-load run overwrote it once).
#
# CARGO_TARGET_DIR is honoured (default: target).
set -u
. "$(cd "$(dirname "$0")" && pwd)/lib/machine-state.sh"  # ticket 49: battery / Low Power Mode guard

ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT" || exit 2
TARGET=${CARGO_TARGET_DIR:-$ROOT/target}
case $TARGET in /*) ;; *) TARGET=$ROOT/$TARGET ;; esac
export CARGO_TARGET_DIR=$TARGET
OUT=docs/perf/47-scroll-acceptance-$(date -u +%Y-%m-%d)-$(git rev-parse --short HEAD).txt
machine_state_begin "$OUT"
WORK=$TARGET/47-acceptance
mkdir -p "$WORK"
rm -f "$WORK"/*.txt
: >"$OUT"
missed=0

# A line for the perf file only.
log() { echo "$1" >>"$OUT"; }
# A SUMMARY line: the perf file and the terminal, paths relative to the
# checkout (the perf file is committed; no home path goes in it).
say() { echo "SUMMARY $1" | sed "s|$ROOT/||g" | tee -a "$OUT"; }
miss() { say "miss $1"; missed=1; }

log "# 47-scroll-acceptance (m7-power-tools/47) — $(date -u +%Y-%m-%dT%H:%M:%SZ)"
log "# HEAD $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- crates scripts 2>/dev/null || echo ' (+ uncommitted changes)')"
log "# machine: $(sysctl -n machdep.cpu.brand_string 2>/dev/null || uname -m), $(sysctl -n hw.ncpu 2>/dev/null) cpus, $(uname -sr)"
log "# load before: $(uptime | sed 's/.*load averages*: //')"

TESTS=(cargo test --release --features heavy-tests --test it)
if ! "${TESTS[@]}" --no-run --quiet >"$WORK/build.txt" 2>&1 \
  || ! cargo build --release -q -p mergewright --bin mwr >>"$WORK/build.txt" 2>&1; then
  miss "build failed: $WORK/build.txt"
  exit 1
fi

# ── 1. held_key, release ────────────────────────────────────────────────────
# The raw Kitty pair (`kitty_deflate` off). Known gap M7/46-kitty-no-deflate:
# when 46d-scroll-kitty-without-deflate lands, this pair joins run (a), the
# known-gap list below empties, and the pty deflate=no runs become judged.
RAW=(held_key::held_j_stops_on_release_pixel held_key::j_then_journal_opens_at_once_pixel)
KNOWN_GAP=(
  "tier=Pixel scenario=A tail_ms<=100"
  "tier=Pixel scenario=B tail_ms<=100"
  "tier=Pixel scenario=B j_ms<=100"
)
"${TESTS[@]}" -- held_key:: --list 2>/dev/null | sed -n 's/: test$//p' >"$WORK/held-list.txt"
JUDGED=()
while read -r name; do
  case " ${RAW[*]} " in *" $name "*) ;; *) JUDGED+=("$name") ;; esac
done <"$WORK/held-list.txt"
log "# 1a held_key judged (${#JUDGED[@]}): ${JUDGED[*]}"
log "# 1b held_key raw pair: ${RAW[*]}"

"${TESTS[@]}" -- "${JUDGED[@]}" --exact --test-threads=1 --nocapture >"$WORK/held-a.txt" 2>&1
status_a=$?
"${TESTS[@]}" -- "${RAW[@]}" --exact --test-threads=1 --nocapture >"$WORK/held-b.txt" 2>&1
status_b=$?
for run in a b; do
  log "# 1$run"
  grep -E '^(SUMMARY|VERDICT) held_key|^test held_key::|^test result' "$WORK/held-$run.txt" >>"$OUT"
done
log "# 1a exit $status_a; 1b exit $status_b (1b judged by VERDICT lines, not exit)"

ok_a=$(grep -cE '^VERDICT held_key .*: ok$' "$WORK/held-a.txt")
failed_a=$(grep -E '^VERDICT held_key .*: FAILED$' "$WORK/held-a.txt")
unjudged=$(grep -hE '^VERDICT held_key .*: unjudged' "$WORK/held-a.txt" "$WORK/held-b.txt" | wc -l | tr -d ' ')
say "held_key run=a tests=${#JUDGED[@]} exit=$status_a verdict_ok=$ok_a verdict_failed=$(printf '%s' "$failed_a" | grep -c .)"
[ "$status_a" -eq 0 ] || miss "held_key run a exited $status_a: $WORK/held-a.txt"
while IFS= read -r line; do
  [ -n "$line" ] && miss "$line"
done <<<"$failed_a"
say "unjudged strip_transmits_held lines=$unjudged (fold_changes_held unread; guard: pixel_scroll::a_capped_graph_resends_strips_only_when_the_fold_changes)"

raw_lines=$(grep -cE '^VERDICT held_key ' "$WORK/held-b.txt")
[ "$raw_lines" -gt 0 ] || miss "held_key run b printed no VERDICT line: $WORK/held-b.txt"
while IFS= read -r line; do
  [ -z "$line" ] && continue
  item=${line#VERDICT held_key }
  item=${item%: *}
  verdict=${line##*: }
  listed=0
  for gap in "${KNOWN_GAP[@]}"; do [ "$item" = "$gap" ] && listed=1; done
  # The item's measured value, from its scenario's SUMMARY line.
  scenario=${item#tier=Pixel scenario=}
  scenario=${scenario%% *}
  key=${item##* }
  key=${key%%<=*}
  value=$(grep -E "^SUMMARY held_key tier=Pixel scenario=$scenario " "$WORK/held-b.txt" \
    | grep -oE " $key=[^ ]+" | head -1 | tr -d ' ')
  case "$verdict" in
    FAILED)
      if [ $listed -eq 1 ]; then say "known_gap M7/46-kitty-no-deflate $item $value"
      else miss "raw pair, not on the known-gap list: $line"; fi ;;
    ok)
      [ $listed -eq 1 ] && say "note known-gap item passed: $line ($value)" ;;
  esac
done < <(grep -E '^VERDICT held_key ' "$WORK/held-b.txt" | grep -v ': unjudged')
if grep -qh 'key list:' "$WORK/held-a.txt" "$WORK/held-b.txt"; then
  miss "a held_key key list differed from j x188 (then J): $WORK/held-*.txt"
fi

# ── before / after ──────────────────────────────────────────────────────────
python3 - "$WORK/held-a.txt" "$WORK/held-b.txt" docs/perf/fix-scroll-held-key.txt >"$WORK/before-after.txt" <<'PY'
import re, sys
a, b, red = sys.argv[1:4]
FIELDS = ["tail_ms", "j_ms", "frames_after_release", "bytes_per_frame_held",
          "strip_transmits_held", "fold_changes_held"]

def summaries(lines):
    out = {}
    for line in lines:
        if line.startswith("SUMMARY held_key "):
            kv = dict(re.findall(r"(\w+)=(\S+)", line))
            out[kv["scenario"] + "/" + kv["tier"]] = kv
    return out

def first_block(path):
    lines = []
    for line in open(path, encoding="utf-8"):
        if line.startswith("# exit"):
            break
        lines.append(line)
    return lines

red_lines = first_block(red)
head = next((l.split()[2] for l in red_lines if l.startswith("# HEAD")), "?")
before = summaries(red_lines)
after = summaries(open(a, encoding="utf-8").readlines() + open(b, encoding="utf-8").readlines())
ROWS = [  # row, after's A and B scenario keys, before's tier
    ("Unicode", "A/Unicode", "B/Unicode", "Unicode"),
    ("Pixel-deflate", "A-deflate/Pixel", "B-deflate/Pixel", "Pixel"),
    ("Pixel-iTerm2", "A-iterm2/Pixel", None, "Pixel"),
    ("Pixel-raw(known-gap)", "A/Pixel", "B/Pixel", "Pixel"),
]

def cell(table, a_key, b_key, field):
    src = table.get(b_key, {}) if field == "j_ms" else table.get(a_key, {})
    return src.get(field, "n/a") if (field != "j_ms" or b_key) else "n/a"

print(f"SUMMARY before_after source before=fix-scroll-held-key.txt first block (rev {head}, red); "
      "its Pixel row (raw Kitty, no deflate or iTerm2 variant then) is the before of all three Pixel rows; "
      "j_ms from scenario B, every other field from A; n/a = not measured")
for row, a_key, b_key, tier in ROWS:
    parts = []
    for field in FIELDS:
        was = cell(before, "A/" + tier, "B/" + tier, field)
        now = cell(after, a_key, b_key, field)
        parts.append(f"{field}={was}->{now}")
    print(f"SUMMARY before_after row={row} " + " ".join(parts))
PY
tee -a "$OUT" <"$WORK/before-after.txt"

# ── 2. no regression: frames::, budgets, g_after_idle ───────────────────────
"${TESTS[@]}" -- frames:: heavy::budgets_hold --test-threads=1 --nocapture >"$WORK/frames.txt" 2>&1
status_f=$?
grep -E '^frames: .*p99' "$WORK/frames.txt" | grep -v idle_prefetch >>"$OUT"
grep -E '^test (frames|heavy)::|^test result' "$WORK/frames.txt" >>"$OUT"
# --nocapture puts a test's output between its name and its `ok`, so the
# verdicts come from the result line and libtest's `failures:` list.
result_f=$(grep -E '^test result' "$WORK/frames.txt" | tail -1)
passed_f=$(echo "$result_f" | grep -oE '[0-9]+ passed' | cut -d' ' -f1)
failed_f=$(awk '/^failures:$/ {list = 1; next} list && /^    [a-z_:]+$/ {print $1} list && /^test result/ {list = 0}' "$WORK/frames.txt" | sort -u)
ran=$(grep -cE '^test (frames::after_idle::g_after_idle|heavy::budgets_hold)' "$WORK/frames.txt")
idle_failed=$(echo "$failed_f" | grep -c 'g_after_idle')
budgets_failed=$(echo "$failed_f" | grep -c 'budgets_hold')
say "frames exit=$status_f passed=${passed_f:-?} failed=$(printf '%s' "$failed_f" | grep -c .) g_after_idle_failed=$idle_failed budgets_hold_failed=$budgets_failed named_ran=$ran (p99 <= 16 ms asserted in release)"
[ "$status_f" -eq 0 ] || miss "frames/budgets exited $status_f: $WORK/frames.txt"
while IFS= read -r name; do
  [ -z "$name" ] && continue
  why=$(grep -A1 "'$name' .*panicked" "$WORK/frames.txt" | tail -1 | cut -c1-200)
  miss "$name: $why"
done <<<"$failed_f"
[ "$ran" -ge 6 ] || miss "g_after_idle or a heavy::budgets_hold test did not run ($ran of 6)"

# ── 3. pty walk ─────────────────────────────────────────────────────────────
FIXTURE=$(ls -dt "$TARGET"/fixtures/team-17700-* 2>/dev/null | head -1)
REPO=$WORK/repo
if [ -z "$FIXTURE" ]; then
  miss "no MONOREPO fixture under the target dir (step 1 builds it)"
else
  # Mergewright writes .git/mergewright/ in what it opens; the cache is sealed read-only.
  rm -rf "$REPO" && cp -R "$FIXTURE" "$REPO" && chmod -R u+w "$REPO"
fi
BIN=$TARGET/release/mwr
pty() { # label profile deflate rate [extra env...]
  local label=$1 profile=$2 deflate=$3 rate=$4
  shift 4
  env PROFILE="$profile" DEFLATE="$deflate" RATE="$rate" THEN_J=0.05 "$@" \
    python3 scripts/scroll-lag-pty.py "$BIN" "$REPO" 187 50 5 0.016 3 "$label" 2>"$WORK/pty-$label.err"
}
field() { echo "$1" | grep -oE " $2=[^ ]+" | head -1 | cut -d= -f2; }
judge_pty() { # line judged(1/0)
  local line=$1 judged=$2 label fa tail bpk rss deflate broken
  label=$(field "$line" label); fa=$(field "$line" frames_after_release); tail=$(field "$line" tail_ms)
  bpk=$(field "$line" bytes_per_key); rss=$(field "$line" rss_mb); deflate=$(field "$line" deflate)
  broken=$(field "$line" broken)
  [ "$broken" = no ] || miss "pty $label ended early, broken=$broken: not a measurement"
  if [ "$judged" -eq 0 ]; then
    say "known_gap M7/46-kitty-no-deflate pty label=$label deflate=$deflate frames_after_release=$fa tail_ms=$tail bytes_per_key=$bpk rss_mb=$rss"
    return
  fi
  [ -n "$fa" ] && [ "$fa" -le 2 ] || miss "pty $label frames_after_release=$fa > 2"
  [ -n "$tail" ] && [ "$tail" -le 100 ] || miss "pty $label tail_ms=$tail > 100"
  [ -n "$rss" ] && [ "$rss" -le 300 ] || miss "pty $label rss_mb=$rss > 300 (Mergewright process)"
  case $label in kitty-yes-*)
    [ "$deflate" = yes ] || miss "pty $label observed deflate=$deflate, judged run must send o=z"
    [ -n "$bpk" ] && [ "$bpk" -le 524288 ] || miss "pty $label bytes_per_key=$bpk > 512 KiB" ;;
  esac
}
if [ -d "$REPO" ]; then
  for spec in "text-free text - " "text-500k text - 500000" \
    "kitty-yes-free kitty yes " "kitty-yes-40m kitty yes 40000000" \
    "kitty-no-free kitty no " "kitty-no-40m kitty no 40000000"; do
    set -- $spec
    label=$1 profile=$2 deflate=$3 rate=${4:-}
    [ "$deflate" = - ] && deflate=
    line=$(pty "$label" "$profile" "$deflate" "$rate")
    if [ -z "$line" ]; then
      miss "pty $label printed no SUMMARY: $WORK/pty-$label.err"
      continue
    fi
    echo "$line" >>"$OUT"
    echo "$line" >"$WORK/pty-$label.txt"
    case $label in kitty-no-*) judge_pty "$line" 0 ;; *) judge_pty "$line" 1 ;; esac
  done
fi

# ── 4. journey step counts ──────────────────────────────────────────────────
# Journey 1 = 1 key (`j`, held), journey 2 = 1 key (`J`). Every held_key test
# asserts its key list is j x188 then J in B, nothing else (held_key.rs `run`,
# "47's walk count"); a differing list fails the test with "key list:", which
# step 1 turns into a miss. The pty walks sent the same script.
pty_keys=$(cat "$WORK"/pty-*.txt 2>/dev/null | grep -cE ' keys=188 then_J ')
pty_runs=$(cat "$WORK"/pty-*.txt 2>/dev/null | grep -c .)
held_runs=$(cat "$WORK/held-a.txt" "$WORK/held-b.txt" | grep -cE '^SUMMARY held_key .* keys=18[89] ')
say "journeys j_held=1_key J=1_key held_key_summaries_with_188/189_keys=$held_runs pty_runs_with_j188_then_J=$pty_keys/$pty_runs"
[ "$pty_keys" -eq "$pty_runs" ] || miss "a pty run's key script was not j x188 then J"

# ── 5. captures ─────────────────────────────────────────────────────────────
"${TESTS[@]}" -- ghostty_shot:: --test-threads=1 --nocapture >"$WORK/shot.txt" 2>&1
status_s=$?
shot=$(grep -E '^SUMMARY ghostty_png' "$WORK/shot.txt")
echo "$shot" >>"$OUT"
[ "$status_s" -eq 0 ] || miss "ghostty_shot:: exited $status_s (capture drifted or missing): $WORK/shot.txt"
say "capture docs/shots/m7/fix-scroll-ghostty.png exit=$status_s $(echo "$shot" | grep -oE 'bytes=[0-9]+ .*')"
# The PNG's rows, counted from the graph pane's first row; the pane starts on
# screen row 1, under the title bar.
pane=$(field "$shot" pane_rows)
GRID=docs/shots/m7/fix-scroll-release-unicode.txt
if [ -n "$pane" ] && [ -d "$REPO" ]; then
  rows="$((${pane%..*} + 1)):$((${pane#*..} + 1))"
  line=$(pty grid text "" "" GRID_OUT="$WORK/grid.txt" GRID_ROWS="$rows")
  echo "$line" >>"$OUT"
  {
    echo "# m7-power-tools/47: the release grid, pty text walk (Unicode), TeamSpec::MONOREPO 187x50,"
    echo "# held j x188 at 16 ms; every byte before J's frame, drawn by pyte. Screen rows $rows (the"
    echo "# PNG's rows), cut at the graph's divider. Characters only: colour, the lit thread and"
    echo "# the cursor's fill are not recorded (CLAUDE.md rule 5)."
    cat "$WORK/grid.txt"
  } >"$GRID"
  cargo run -q -p graft-fixtures -- redact "$GRID" >"$WORK/redact.txt" 2>&1 \
    || miss "redact failed on $GRID: $WORK/redact.txt"
  say "capture $GRID rows=$rows lines=$(grep -vc '^#' "$GRID")"
else
  miss "no release grid: the PNG test printed no pane_rows or the repo copy is missing"
fi

log "# load after: $(uptime | sed 's/.*load averages*: //')"
log "# exit $missed"
echo "$OUT"
[ "$missed" -eq 0 ] || exit 1
