#!/usr/bin/env bash
# m7-power-tools/04: where a slow key's time goes. Release `frames::` tests,
# 3 plain runs and one run per control, each key's phases traced
# (GRAFT_FRAME_TRACE). Writes docs/perf/m7-04-slow-key.txt (or $1).
#
#   plain  the scroll, then 5 keys each after a 600 ms idle pause
#   a      control A: the same keys with no idle pause
#   b      control B: idle pauses, a thread spinning on one core throughout
#   c      control C: idle pauses, no idle prefetch
#
# The trace itself stays in a temp dir; the perf file keeps the summary
# lines, the run-to-run spread, and the phase medians. Lines a reader
# (or a helper) needs first start with SUMMARY.
set -u
cd "$(dirname "$0")/.."
OUT=${1:-docs/perf/m7-04-slow-key.txt}
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
FEATURES=heavy-tests

cargo test --release --features "$FEATURES" --no-run --quiet 2>"$WORK/build.err" || {
  echo "release build failed:" >&2
  tail -20 "$WORK/build.err" >&2
  exit 1
}

run() { # label control
  local label=$1 control=$2
  local t0=$SECONDS
  GRAFT_FRAME_TRACE="$WORK/trace.txt" GRAFT_FRAME_RUN="$label" GRAFT_FRAME_CONTROL="$control" \
    cargo test --release --features "$FEATURES" --quiet -- frames:: --nocapture --test-threads=1 \
    >"$WORK/$label.out" 2>&1
  local status=$?
  echo "run $label control=$control exit $status, $((SECONDS - t0))s" >>"$WORK/runs.txt"
}

for n in 1 2 3; do run "plain$n" plain; done
run controlA a
run controlB b
run controlC c

{
  echo "# m7-power-tools/04 slow key — $(date -u +%FT%TZ)"
  echo "# HEAD $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- crates || echo ' +uncommitted')"
  echo "# $(sysctl -n machdep.cpu.brand_string 2>/dev/null || uname -m), $(sysctl -n hw.ncpu 2>/dev/null) cpus, $(uname -sr)"
  echo "# release, --features $FEATURES, --test-threads=1; harness: synthetic keys, in-memory terminal"
  echo
  echo "## runs"
  cat "$WORK/runs.txt"
  for label in plain1 plain2 plain3 controlA controlB controlC; do
    echo
    echo "## $label: summary lines"
    # `--nocapture` prints a test's first line after its name: keep from
    # the marker on. Lists of late keys are cut at 600 characters.
    grep -oE '(frames: .*(p99_us=|idle_wake_us|keys over)|machine: ).*|^test result.*|panicked.*' "$WORK/$label.out" |
      grep -v '0 passed; 0 failed' |
      awk '{ if (length($0) > 600) print substr($0, 1, 600) " (cut)"; else print }'
  done
  echo
  python3 - "$WORK" <<'PY'
import collections, re, statistics, sys
work = sys.argv[1]

def fields(line):
    return dict(re.findall(r'(\w+)=(\S+)', line))

# Run-to-run spread over the three plain runs (KNOWN_GAPS: p99 noisy).
spread = collections.defaultdict(list)
for n in (1, 2, 3):
    for line in open(f"{work}/plain{n}.out", errors="replace"):
        if line.startswith("frames: ") and "p99_us=" in line:
            f = fields(line)
            key = f"tier={f['tier']} refs={f['refs']} pane={f['pane']}"
            spread[key].append((int(f["p50_us"]), int(f["p99_us"]), int(f["max_us"])))
print("## spread over 3 plain runs (p50 / p99 / max, µs)")
for key, runs in spread.items():
    p99s = [r[1] for r in runs]
    maxes = [r[2] for r in runs]
    cells = "  ".join(f"{a}/{b}/{c}" for a, b, c in runs)
    verdict = "ok" if max(p99s) <= 16000 else "OVER"
    print(f"SUMMARY spread {key}: {cells}  p99 range {min(p99s)}-{max(p99s)} max range {min(maxes)}-{max(maxes)} gate(p99<=16ms)={verdict}")

rows = []
try:
    for line in open(f"{work}/trace.txt"):
        if line.startswith("trace "):
            rows.append(fields(line))
except FileNotFoundError:
    print("SUMMARY no trace written")
    sys.exit(0)

PHASES = ["latency", "oversleep", "queued", "drain", "update", "carry", "draw", "write", "tail"]
def med(values):
    values = [int(v) for v in values if v != "-"]
    return (int(statistics.median(values)), max(values), len(values)) if values else None

def table(title, pick, group):
    groups = collections.OrderedDict()
    for row in rows:
        if pick(row):
            groups.setdefault(group(row), []).append(row)
    print()
    print(f"## {title}")
    print("median(max) µs per phase; n = keys")
    for name, members in groups.items():
        cells = []
        for phase in PHASES:
            m = med([r[f"{phase}_us"] for r in members])
            cells.append(f"{phase}={m[0]}({m[1]})" if m else f"{phase}=-")
        asked = sum(r["asked"] == "1" for r in members)
        out = sum(r["rows_out"] == "1" for r in members)
        print(f"SUMMARY {title.split()[0]} {name} n={len(members)} " + " ".join(cells) + f" asked={asked}/{len(members)} rows_out={out}/{len(members)}")

by = lambda r: f"control={r['control']} tier={r['tier']} refs={r['refs']} pane={r['pane']}"
table("idle keys (first key after each pause)", lambda r: r["kind"] == "idle", by)
table("scroll keys (all, for comparison)", lambda r: r["kind"] == "scroll", by)
table("slow scroll keys (latency over 8 ms)", lambda r: r["kind"] == "scroll" and int(r["latency_us"]) > 8000, by)

print()
print("## key 37 (M1's slow key: j #37 of the Closed-pane scroll), every run")
for r in rows:
    if r["pane"] == "Closed" and r["key"] == "37":
        print("SUMMARY key37 " + " ".join(f"{k}={r[k]}" for k in ["control", "run", "tier", "refs", "latency_us", "oversleep_us", "queued_us", "update_us", "carry_us", "draw_us", "asked", "rows_out", "behind"]))

print()
print("## every idle key, and every scroll key over 10 ms (raw trace)")
for r in rows:
    if r["kind"] == "idle" or int(r["latency_us"]) > 10000:
        print(" ".join(f"{k}={v}" for k, v in r.items()))
PY
} 2>&1 | sed "s|$HOME|~|g" >"$OUT"
echo "wrote $OUT"
