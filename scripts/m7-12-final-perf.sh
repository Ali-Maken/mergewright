#!/usr/bin/env bash
# m7-power-tools/12e: the idle advance's final numbers, before and after.
#
#   before  this HEAD in a throwaway worktree with
#           scripts/m7-12-no-idle-advance.patch applied, every command run
#           with TRIB_FRAME_NO_IDLE_ADVANCE=1: the history worker waits for
#           the next request, as before m7/12b, and never says `LaidOut`.
#   after   this checkout as it is.
#
# Per side, interleaved before/after so both see the same machine:
#   g_after_idle  release frames::after_idle::g_after_idle, 3 runs (100k cg,
#                 50k refs, Unicode; 2 s idle, then G)
#   heap_slope    scripts/heap-slope.sh, 2 runs (live heap and footprint
#                 peak B/commit, 10k cg -> 100k cg)
#   budgets_G     examples/budgets on git/git (scripts/clone-git-git.sh),
#                 2 runs: first paint, G, g, peak RSS. Recorded, not asserted.
# After only:
#   heavy         heavy tier `heavy::` (debug: peak RSS ceilings, the slope
#                 guard, heavy.rs's promotion test) and release
#                 heavy::budgets_hold (first paint budgets).
#
# Appends one section to docs/perf/m7-12-idle-advance.txt (or $1). Read its
# SUMMARY lines. Every step records `uptime` before and after and any
# process over 40% CPU (claude excluded); a step beside one is marked BUSY
# and never rerun. KEEP=1 keeps the worktree.
set -u
. "$(cd "$(dirname "$0")" && pwd)/lib/machine-state.sh"  # ticket 49: battery / Low Power Mode guard
cd "$(dirname "$0")/.." || exit 1
ROOT=$(pwd)
OUT=${1:-docs/perf/m7-12-idle-advance.txt}
case $OUT in /*) ;; *) OUT=$ROOT/$OUT ;; esac
TMP=${TMPDIR:-/tmp}
TMP=${TMP%/}
WT=$TMP/graft-12e-before
WORK=$(mktemp -d)
GITGIT=$(scripts/clone-git-git.sh --where)
# Each tree builds in its own target/: heap-slope.sh reads target/release.
unset CARGO_TARGET_DIR

cleanup() {
  rm -rf "$WORK"
  if [ -z "${KEEP:-}" ]; then
    git -C "$ROOT" worktree remove --force "$WT" 2>/dev/null
    rm -rf "$WT"
  fi
}
trap cleanup EXIT
machine_state_begin "$OUT"

[ -d "$GITGIT" ] || { echo "no git/git at $GITGIT: run scripts/clone-git-git.sh" >&2; exit 1; }
if [ ! -d "$WT" ]; then
  git worktree add --detach "$WT" HEAD >/dev/null 2>"$WORK/wt.err" || {
    echo "worktree failed:" >&2
    cat "$WORK/wt.err" >&2
    exit 1
  }
fi
git -C "$WT" checkout --quiet -- . || exit 1
git -C "$WT" apply "$ROOT/scripts/m7-12-no-idle-advance.patch" || {
  echo "the no-idle-advance patch did not apply" >&2
  exit 1
}

build() { # dir
  (cd "$1" &&
    cargo test --release --quiet -p graft --features heavy-tests --test it --no-run &&
    cargo build --release --quiet -p graft --example budgets) 2>"$WORK/build.err" || {
    echo "release build in $1 failed:" >&2
    tail -20 "$WORK/build.err" >&2
    exit 1
  }
}
build "$WT"
build "$ROOT"

busy() {
  ps -Ao pcpu=,comm= | awk '$1 > 40 && $0 !~ /claude/ { print "BUSY: " $0 }'
}

KEEP_LINES='^frames: g_after_idle|^test result|^test .* (ok|FAILED)$|panicked|^first paint|^total/|^## |^budgets:|^heapslope:|failed|^error'

step() { # side name dir command...
  local side=$1 name=$2 dir=$3 status t0=$SECONDS
  shift 3
  {
    echo
    echo "### step side=$side name=$name $(date -u +%FT%TZ)"
    echo "uptime-before: $(uptime)"
    busy
  } >>"$WORK/log.txt"
  if [ "$side" = before ]; then
    (cd "$dir" && TRIB_FRAME_NO_IDLE_ADVANCE=1 "$@") >"$WORK/step.out" 2>&1
  else
    (cd "$dir" && "$@") >"$WORK/step.out" 2>&1
  fi
  status=$?
  {
    grep -E "$KEEP_LINES" "$WORK/step.out" | sed "s|$HOME|~|g" | cut -c1-600
    echo "exit $status, $((SECONDS - t0))s"
    echo "uptime-after: $(uptime)"
    busy
  } >>"$WORK/log.txt"
}

G_TEST=frames::after_idle::g_after_idle
for n in 1 2 3; do
  for side in before after; do
    dir=$ROOT
    [ "$side" = before ] && dir=$WT
    step "$side" g_after_idle "$dir" cargo test --release --quiet -p graft --features heavy-tests --test it \
      -- "$G_TEST" --exact --test-threads=1 --nocapture
  done
done
for n in 1 2; do
  for side in before after; do
    dir=$ROOT
    [ "$side" = before ] && dir=$WT
    step "$side" heap_slope "$dir" env HEAP_SLOPE_OUT="$WORK/heap-$side-$n.txt" scripts/heap-slope.sh "$side-$n"
    grep -E '^total/' "$WORK/heap-$side-$n.txt" >>"$WORK/log.txt" 2>/dev/null
  done
done
for n in 1 2; do
  for side in before after; do
    dir=$ROOT
    [ "$side" = before ] && dir=$WT
    step "$side" budgets_G "$dir" target/release/examples/budgets "$GITGIT"
  done
done
step after heavy_debug "$ROOT" cargo test -p graft --features heavy-tests --test it -- heavy:: --test-threads=2
step after heavy_release_budgets "$ROOT" cargo test --release -p graft --features heavy-tests --test it \
  -- heavy::budgets_hold --test-threads=1 --nocapture

{
  echo
  echo
  echo "# ===== m7-power-tools/12e final: idle advance before/after — $(date -u +%FT%TZ)"
  echo "# HEAD $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- crates || echo ' +uncommitted'); before = HEAD + scripts/m7-12-no-idle-advance.patch, TRIB_FRAME_NO_IDLE_ADVANCE=1"
  echo "# $(sysctl -n machdep.cpu.brand_string 2>/dev/null || uname -m), $(sysctl -n hw.ncpu 2>/dev/null) cpus, $(uname -sr), $(rustc --version)"
  echo "# git/git: $(git --git-dir="$GITGIT" rev-list --count --all 2>/dev/null) commits, commit-graph $([ -f "$GITGIT/objects/info/commit-graph" ] && echo present || echo absent)"
  python3 - "$WORK/log.txt" <<'PY'
import re, statistics, sys

steps, cur = [], None
for line in open(sys.argv[1]):
    line = line.rstrip("\n")
    m = re.match(r"### step side=(\w+) name=(\w+) (\S+)", line)
    if m:
        cur = {"side": m[1], "name": m[2], "at": m[3], "lines": [], "busy": [], "exit": None}
        steps.append(cur)
        continue
    if cur is None:
        continue
    m = re.match(r"exit (\d+), (\d+)s", line)
    if m:
        cur["exit"], cur["secs"] = int(m[1]), int(m[2])
    elif line.startswith("BUSY: "):
        cur["busy"].append(line[6:].strip())
    elif line.startswith("uptime-"):
        cur.setdefault("uptime", []).append(line)
    else:
        cur["lines"].append(line)

def fields(text):
    return dict(re.findall(r"(\w+)=(\S+)", text))

failed = [s for s in steps if s["exit"] != 0]
busy = [s for s in steps if s["busy"]]
print(f"SUMMARY steps: {len(steps) - len(failed)} of {len(steps)} exit 0" +
      ("" if not failed else "; FAILED: " + ", ".join(f"{s['side']} {s['name']} exit {s['exit']}" for s in failed)))
print(f"SUMMARY quiet: {len(steps) - len(busy)} of {len(steps)} steps had no process over 40% CPU (claude excluded)" +
      ("" if not busy else "; BUSY: " + "; ".join(f"{s['side']} {s['name']} at {s['at']}: {' | '.join(s['busy'])}" for s in busy)))

def med(values):
    return f"{int(statistics.median(values))}(max {max(values)})" if values else "-"

print()
print("## g_after_idle: G's send -> the frame showing the last row (µs), 2 s idle after LaidOut")
for side in ("before", "after"):
    runs = [fields(l) for s in steps if s["side"] == side and s["name"] == "g_after_idle"
            for l in s["lines"] if l.startswith("frames: g_after_idle")]
    lat = [int(r["latency_us"]) for r in runs]
    print(f"SUMMARY g_after_idle side={side} runs={len(runs)} latency_us={med(lat)} each={','.join(map(str, lat))} "
          f"asked={','.join(r['asked'] for r in runs)} wakes_between={','.join(r['wakes_between'] for r in runs)} "
          f"idle_gap_wakes={','.join(r['idle_gap_wakes'] for r in runs)} laid_out={','.join(r['laid_out'] for r in runs)}")

print()
print("## heap slope (B/commit, 10k cg -> 100k cg; scripts/heap-slope.sh)")
for side in ("before", "after"):
    # heap-slope's `total/` rows were appended to the log after each step.
    per = {}
    for s in steps:
        if s["side"] != side or s["name"] != "heap_slope":
            continue
        for l in s["lines"]:
            m = re.match(r"total/(\w+)\s+(\d+)\s+(\d+)\s+(\S+)", l)
            if m:
                per.setdefault(m[1], []).append(m[4])
    cells = " ".join(f"{k}={','.join(v)}" for k, v in per.items())
    print(f"SUMMARY heap_slope side={side} {cells or 'none'}")

print()
print("## examples/budgets on git/git (recorded, not asserted)")
for side in ("before", "after"):
    for s in steps:
        if s["side"] == side and s["name"] == "budgets_G":
            line = next((l for l in s["lines"] if l.startswith("first paint")), "no first paint line")
            print(f"SUMMARY budgets_G side={side} exit={s['exit']} {line}")

print()
print("## heavy tier (after only)")
for s in steps:
    if s["name"].startswith("heavy"):
        results = [l for l in s["lines"] if l.startswith("test result")]
        bad = [l for l in s["lines"] if l.endswith("FAILED") or "panicked" in l]
        promo = [l for l in s["lines"] if "the_table_and_the_order_stay_at_the_commit_count_after_g_and_after_promotion" in l]
        print(f"SUMMARY {s['name']} exit={s['exit']} {results[-1] if results else 'no test result'}"
              + (f"; heavy.rs:217 test: {promo[0]}" if promo else "")
              + (f"; failures: {' | '.join(bad)}" if bad else ""))
PY
  echo
  echo "## steps (raw lines kept)"
  sed "s|$HOME|~|g" "$WORK/log.txt"
} >>"$OUT"
echo "appended to $OUT"
# Non-zero when any step failed; its SUMMARY steps line names which.
if grep -qE '^exit [1-9]' "$WORK/log.txt"; then
  echo "m7-12-final-perf FAILED: see SUMMARY steps" >&2
  exit 1
fi
