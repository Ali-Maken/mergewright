#!/usr/bin/env bash
# m7-power-tools/12g: what M1's slow key 37 (17–18 ms release) was made of.
#
# Builds M1's code at b5380399 (m1/30c part b seam 1, the commit that recorded
# ADR-0021's key 37 numbers; its crates/*/src equal its parent's) in a
# throwaway detached worktree with its own CARGO_TARGET_DIR, applies
# scripts/m7-key37.patch (harness only: tests/it/frames.rs; nothing under
# crates/*/src), and runs M1's release
# Unicode 50k-refs scroll — the case ADR-0021's key 37 numbers came from —
# PLAIN times as it was and SPIN times with ADR-0021's control (one thread
# spinning on another core), interleaved. Every run's key 37 is split into
# parts that add up to its latency:
#   late       the loop still drawing the previous wake past the key's due tick
#   oversleep  the harness's recv_timeout waking after the due tick
#   frames     Tributary's update + draw, summed over the key's wakes
#   worker     the waits between those wakes (the row worker's answer)
#
# Writes docs/perf/m7-12g-key-37.txt (or $1, relative to the repo root); a
# second pass appends a new section. Read its SUMMARY lines. Each run records
# `uptime` before and after and any process over 40% CPU (claude excluded); a run at
# 1-minute load >= 3 or beside such a process is marked `*` and left out of
# the conclusions, never rerun.
#
# PLAIN=6 SPIN=3 by default. KEEP=1 keeps the worktree and target dir.
set -u
. "$(cd "$(dirname "$0")" && pwd)/lib/machine-state.sh"  # ticket 49: battery / Low Power Mode guard
cd "$(dirname "$0")/.." || exit 1
ROOT=$(pwd)
OUT=${1:-docs/perf/m7-12g-key-37.txt}
PLAIN=${PLAIN:-6}
SPIN=${SPIN:-3}
M1=${M1:-b5380399}
# Not 10138177 (ticket 12g's commit): 21 src files differ from b5380399
# (30b, 30d, 30c part a, 35c), and there the first key to ask for rows is 75.
TMP=${TMPDIR:-/tmp}
TMP=${TMP%/}
WT=$TMP/trib-12g-wt
# Target inside this checkout's gitignored target/, never the shared one.
export CARGO_TARGET_DIR=${KEY37_TARGET:-$ROOT/target/m7-key37}
TEST=frames::fifty_thousand_refs_keep_input_to_frame_under_16_ms_at_the_unicode_tier
WORK=$(mktemp -d)

cleanup() {
  rm -rf "$WORK"
  if [ -z "${KEEP:-}" ]; then
    git -C "$ROOT" worktree remove --force "$WT" 2>/dev/null
    rm -rf "$WT" "$CARGO_TARGET_DIR"
  fi
}
trap cleanup EXIT
machine_state_begin "$OUT"

if [ ! -d "$WT" ]; then
  git worktree add --detach "$WT" "$M1" >/dev/null 2>"$WORK/wt.err" || {
    echo "worktree at $M1 failed:" >&2
    cat "$WORK/wt.err" >&2
    exit 1
  }
fi
git -C "$WT" checkout --quiet -- . || exit 1
git -C "$WT" apply "$ROOT/scripts/m7-key37.patch" || {
  echo "patch did not apply at $M1" >&2
  exit 1
}
(cd "$WT" && cargo test --release --quiet -p graft --features heavy-tests --test it --no-run) \
  2>"$WORK/build.err" || {
  echo "release build at $M1 failed:" >&2
  tail -20 "$WORK/build.err" >&2
  exit 1
}

# Processes over 40% CPU other than claude (the conductor's quiet rule).
busy() {
  ps -Ao pcpu=,comm= | awk -v w="$1" '$1 > 40 && $0 !~ /claude/ { print "busy-" w ": " $0 }'
}

# One run: header, uptime before/after, busy processes, the test's lines.
run() {
  local n=$1 mode=$2 status
  {
    echo "# run $n mode=$mode $(date -u +%FT%TZ)"
    echo "uptime-before: $(uptime)"
    busy before
  } >>"$WORK/runs.txt"
  if [ "$mode" = spin ]; then
    (cd "$WT" && KEY37_SPIN=1 cargo test --release --quiet -p graft --features heavy-tests --test it \
      -- "$TEST" --exact --test-threads=1 --nocapture) >"$WORK/run.out" 2>&1
  else
    (cd "$WT" && cargo test --release --quiet -p graft --features heavy-tests --test it \
      -- "$TEST" --exact --test-threads=1 --nocapture) >"$WORK/run.out" 2>&1
  fi
  status=$?
  {
    grep -E '^(frames:|KEY37|NEAR|KEYS|refs:)' "$WORK/run.out"
    grep -oE '[0-9]+ of [0-9]+ keys missed[^:]*' "$WORK/run.out" | sed 's/^/missed: /'
    echo "exit: $status"
    echo "uptime-after: $(uptime)"
    busy after
  } >>"$WORK/runs.txt"
  grep -q '^KEY37' "$WORK/run.out" || {
    echo "run $n printed no KEY37 line:" >>"$WORK/runs.txt"
    tail -20 "$WORK/run.out" >>"$WORK/runs.txt"
  }
}

n=0
p=0
s=0
# Interleaved: two plain runs, then one spinner run, until both counts are met.
while [ "$p" -lt "$PLAIN" ] || [ "$s" -lt "$SPIN" ]; do
  for mode in plain plain spin; do
    if [ "$mode" = plain ] && [ "$p" -lt "$PLAIN" ]; then
      p=$((p + 1))
    elif [ "$mode" = spin ] && [ "$s" -lt "$SPIN" ]; then
      s=$((s + 1))
    else
      continue
    fi
    n=$((n + 1))
    run "$n" "$mode"
  done
done

PASS=1
if [ -f "$OUT" ]; then
  PASS=$(($(grep -c '^# ===== pass' "$OUT") + 1))
fi
{
  echo "# ===== pass $PASS"
  echo "# m7-power-tools/12g key 37 at M1 ($M1 + scripts/m7-key37.patch) — $(date -u +%FT%TZ)"
  echo "# from $(git rev-parse --short HEAD), $(sysctl -n machdep.cpu.brand_string 2>/dev/null || uname -m), $(sysctl -n hw.ncpu 2>/dev/null) cpus, $(uname -sr), $(rustc --version)"
  echo "# release, $TEST, plain=$PLAIN spin=$SPIN"
  python3 - "$PASS" "$WORK/runs.txt" <<'PY'
import re, statistics, sys

pass_no, path = sys.argv[1], sys.argv[2]
runs, cur = [], None
for line in open(path):
    line = line.rstrip("\n")
    m = re.match(r"# run (\d+) mode=(\w+)", line)
    if m:
        cur = {"n": int(m[1]), "mode": m[2], "busy": 0, "load": [], "key": None, "exit": None}
        runs.append(cur)
    elif cur is None:
        continue
    elif line.startswith("uptime-"):
        cur["load"].append(float(re.search(r"load averages?: ([\d.]+)", line)[1]))
    elif line.startswith("busy-"):
        cur["busy"] += 1
    elif line.startswith("KEY37 "):
        cur["key"] = dict(re.findall(r"(\w+)=(\d+)", line.split(" detail=")[0]))
        cur["detail"] = line.split(" detail=")[1]
    elif line.startswith("KEYS "):
        cur["keys"] = dict(re.findall(r"(\w+)=(\d+)", line))
    elif line.startswith("exit: "):
        cur["exit"] = int(line[6:])

def quiet(r):
    return r["key"] is not None and max(r["load"]) < 3 and r["busy"] == 0

def ms(us):
    return f"{int(us) / 1000:.1f}"

def spread(values):
    if not values:
        return "MISSING"
    values = sorted(values)
    return f"min/median/max {ms(values[0])}/{ms(statistics.median(values))}/{ms(values[-1])} ms"

def tag(r):
    return "" if quiet(r) else "*"

S = f"SUMMARY pass{pass_no}"
plain = [r for r in runs if r["mode"] == "plain"]
spin = [r for r in runs if r["mode"] == "spin"]
qp = [r for r in plain if quiet(r)]
qs = [r for r in spin if quiet(r)]
failed = [r["n"] for r in runs if r["key"] is None]
print(f"{S} runs: plain={len(plain)} (quiet {len(qp)}) spin={len(spin)} (quiet {len(qs)}); "
      f"quiet = 1-min load < 3 before and after and no process > 40% CPU but claude; * marks a run left out"
      + (f"; NO KEY37 LINE in runs {failed}" if failed else ""))
lat = lambda rs: [int(r["key"]["latency_us"]) for r in rs]
print(f"{S} repro: key index {sorted({r['key']['key'] for r in runs if r['key']})}; plain quiet latency {spread(lat(qp))}; "
      f"over 12 ms in {sum(1 for v in lat(qp) if v > 12000)} of {len(qp)}; per run "
      + " ".join(f"r{r['n']}{tag(r)}={ms(r['key']['latency_us'])}" for r in plain if r["key"]))
slack = lambda r: int(r["key"]["late_us"]) + int(r["key"]["oversleep_us"])
print(f"{S} H1 harness slack (late + oversleep before key 37): plain quiet {spread([slack(r) for r in qp])}; per run "
      + " ".join(f"r{r['n']}{tag(r)}={ms(r['key']['late_us'])}+{ms(r['key']['oversleep_us'])}" for r in runs if r["key"])
      + "; all keys oversleep p50/max per run "
      + " ".join(f"r{r['n']}={ms(r['keys']['oversleep_us_p50'])}/{ms(r['keys']['oversleep_us_max'])}" for r in runs if "keys" in r))
print(f"{S} H2 spinner: key 37 latency spin quiet {spread(lat(qs))} vs plain quiet {spread(lat(qp))}; "
      f"frames spin {spread([int(r['key']['frames_us']) for r in qs])} vs plain {spread([int(r['key']['frames_us']) for r in qp])}; "
      f"worker spin {spread([int(r['key']['worker_wait_us']) for r in qs])} vs plain {spread([int(r['key']['worker_wait_us']) for r in qp])}")
print(f"{S} H3 Tributary's frames (update+draw summed) and worker waits, per run: "
      + " ".join(f"r{r['n']}{tag(r)}:{r['mode']}:frames={ms(r['key']['frames_us'])},worker={ms(r['key']['worker_wait_us'])},wakes={r['key']['wakes']}" for r in runs if r["key"])
      + "; all keys' own frame p50/max per run "
      + " ".join(f"r{r['n']}={ms(r['keys']['own_frame_us_p50'])}/{ms(r['keys']['own_frame_us_max'])}" for r in runs if "keys" in r))
gaps = [abs(int(r["key"]["latency_us"]) - int(r["key"]["sum_us"])) for r in runs if r["key"]]
print(f"{S} sum: max |latency - (late+oversleep+frames+worker)| = {max(gaps) if gaps else 'MISSING'} us over {len(gaps)} runs (1 ms allowed)")
PY
  cat "$WORK/runs.txt"
} 2>&1 | sed "s|$HOME|~|g" >>"$OUT"
echo "wrote $OUT (pass $PASS)"
