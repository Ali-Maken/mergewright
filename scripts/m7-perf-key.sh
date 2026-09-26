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
#
# Revisions (m7-power-tools/04b): when the perf file exists, the run is
# appended as the next `# ===== rev N` section, headed by `SUMMARY revN`
# lines comparing it with the section before (first key after idle, p99,
# max). REV_NOTE says what changed. Rev 1 is the file as 04 left it.
#
# Since 04b the keys come from an input thread (`KeyFeed`) and a key's
# latency starts at its send; `oversleep` is that thread's own wake, kept
# visible but outside the latency. Control D (`machine:` lines) still
# shows the OS timer slack with no Graft code at all.
#
# Failed runs (m7-power-tools/12d, review 04 Major 1): `SUMMARY runs:` counts
# each run's exit; a spread verdict is FAIL(<run> failed) when a plain run
# failed, MISSING(k of 3) when a scenario has fewer than 3 cells, then OVER
# or ok. The script exits 1 unless every run passed and every spread is ok.
# `scripts/m7-perf-key-selftest.sh` checks this on canned outputs.
set -u
cd "$(dirname "$0")/.."
OUT=${1:-docs/perf/m7-04-slow-key.txt}
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
FEATURES=heavy-tests

CANNED=${M7_PERF_KEY_CANNED:-}
if [ -z "$CANNED" ]; then
  cargo test --release --features "$FEATURES" --no-run --quiet 2>"$WORK/build.err" || {
    echo "release build failed:" >&2
    tail -20 "$WORK/build.err" >&2
    exit 1
  }
fi

# M7_PERF_KEY_CANNED=<dir> (scripts/m7-perf-key-selftest.sh only): each run
# prints <dir>/<label>.out and exits <dir>/<label>.exit instead of testing.
canned() { # label
  cat "$CANNED/$1.out" || return 1
  return "$(cat "$CANNED/$1.exit")"
}

run() { # label control
  local label=$1 control=$2
  local t0=$SECONDS
  if [ -n "$CANNED" ]; then
    canned "$label" >"$WORK/$label.out" 2>&1
  else
    GRAFT_FRAME_TRACE="$WORK/trace.txt" GRAFT_FRAME_RUN="$label" GRAFT_FRAME_CONTROL="$control" \
      cargo test --release --features "$FEATURES" --quiet -- frames:: frame_trace:: --nocapture --test-threads=1 \
      >"$WORK/$label.out" 2>&1
  fi
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
    grep -oE '(frames: .*(p99_us=|idle_wake_us|keys over)|machine: |frame_trace: ).*|^test result.*|panicked.*' "$WORK/$label.out" |
      grep -v '0 passed; 0 failed' |
      awk '{ if (length($0) > 600) print substr($0, 1, 600) " (cut)"; else print }'
  done
  echo
  python3 - "$WORK" <<'PY'
import collections, re, statistics, sys
work = sys.argv[1]

def fields(line):
    return dict(re.findall(r'(\w+)=(\S+)', line))

# Every run's exit, from `## runs`: a failed run is never read as a pass
# (review 04 Major 1). A failure after a run's `frames:` line (rows on hand,
# pixel images drawn) leaves the line looking fine; one before it drops the
# scenario's cell.
exits = collections.OrderedDict()
for line in open(f"{work}/runs.txt"):
    m = re.match(r'run (\S+) control=\S+ exit (\d+),', line)
    if m:
        exits[m.group(1)] = int(m.group(2))
failed = [label for label, status in exits.items() if status != 0]
if failed:
    named = ", ".join(f"{label} exit {exits[label]}" for label in failed)
    print(f"SUMMARY runs: FAILED {len(failed)} of {len(exits)}: {named}")
else:
    print(f"SUMMARY runs: {len(exits)} of {len(exits)} exit 0")
plain_failed = [label for label in failed if label.startswith("plain")]

# Run-to-run spread over the three plain runs (KNOWN_GAPS: p99 noisy).
spread = collections.defaultdict(list)
for n in (1, 2, 3):
    for line in open(f"{work}/plain{n}.out", errors="replace"):
        if line.startswith("frames: ") and "p99_us=" in line:
            f = fields(line)
            key = f"tier={f['tier']} refs={f['refs']} pane={f['pane']}"
            spread[key].append((int(f["p50_us"]), int(f["p99_us"]), int(f["max_us"])))
print("## spread over 3 plain runs (p50 / p99 / max, µs)")
if not spread:
    print("SUMMARY spread none: no frames: p99 line in any plain run gate(p99<=16ms)=MISSING(0 of 3)")
for key, runs in spread.items():
    p99s = [r[1] for r in runs]
    maxes = [r[2] for r in runs]
    cells = "  ".join(f"{a}/{b}/{c}" for a, b, c in runs)
    # A failed run first: its cell, or the cell it never printed, proves nothing.
    if plain_failed:
        verdict = f"FAIL({','.join(plain_failed)} failed)"
    elif len(runs) < 3:
        verdict = f"MISSING({len(runs)} of 3)"
    elif max(p99s) > 16000:
        verdict = "OVER"
    else:
        verdict = "ok"
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
} 2>&1 | sed "s|$HOME|~|g" >"$WORK/new.txt"
SUMMARISED=("${PIPESTATUS[@]}")

# The script's own verdict (review 04 Major 1), after the perf file is
# written: non-zero when any run failed, any spread is not `ok` (FAIL,
# MISSING, OVER), or summarising the runs itself failed.
verdict() {
  local why=() failed bad
  if [ "${SUMMARISED[0]}" != 0 ] || [ "${SUMMARISED[1]}" != 0 ]; then
    why+=("summarising the runs failed (status ${SUMMARISED[*]})")
  fi
  failed=$(grep '^SUMMARY runs: ' "$WORK/new.txt")
  case $failed in
    "") why+=("no SUMMARY runs line") ;;
    *FAILED*) why+=("${failed#SUMMARY }") ;;
  esac
  bad=$(grep '^SUMMARY spread ' "$WORK/new.txt" | grep -vc 'gate(p99<=16ms)=ok$')
  [ "$bad" -eq 0 ] || why+=("$bad spread line(s) not ok")
  if [ "${#why[@]}" -gt 0 ]; then
    printf 'm7-perf-key FAILED: %s\n' "${why[@]}" >&2
    return 1
  fi
}

if [ ! -s "$OUT" ]; then
  cp "$WORK/new.txt" "$OUT"
  echo "wrote $OUT"
  verdict
  exit
fi

REV=$(($(grep -c '^# ===== rev ' "$OUT") + 2))
# Read the sections before from a copy: the block below appends to $OUT
# while it runs.
cp "$OUT" "$WORK/old.txt"
{
  echo
  echo
  echo "# ===== rev $REV: ${REV_NOTE:-no note} ====="
  python3 - "$WORK/old.txt" "$WORK/new.txt" "$REV" <<'PY'
import re, sys
old_path, new_path, rev = sys.argv[1], sys.argv[2], sys.argv[3]
# The section before: everything after the last rev marker (all of it for rev 1).
before = re.split(r'^# ===== rev .*$', open(old_path).read(), flags=re.M)[-1]
after = open(new_path).read()

def idle(text):
    # First key after idle, plain runs: latency median(max) per scenario.
    out = {}
    for line in text.splitlines():
        m = re.match(r'SUMMARY idle control=Plain (tier=\S+ refs=\S+ pane=\S+) n=\d+ latency=(\S+) oversleep=(\S+)', line)
        if m:
            out[m.group(1)] = (m.group(2), m.group(3))
    return out

def spread(text):
    out = {}
    for line in text.splitlines():
        m = re.match(r'SUMMARY spread (tier=\S+ refs=\S+ pane=\S+): .* p99 range (\S+) max range (\S+) gate\(p99<=16ms\)=(.+)$', line)
        if m:
            out[m.group(1)] = (m.group(2), m.group(3), m.group(4))
    return out

b, a = idle(before), idle(after)
print(f"## rev {rev} against the section before (µs; idle latency median(max) over plain runs)")
for key in a:
    lat_b, over_b = b.get(key, ("-", "-"))
    lat_a, over_a = a[key]
    print(f"SUMMARY rev{rev} idle {key}: latency before={lat_b} after={lat_a}; oversleep (after: outside the latency) before={over_b} after={over_a}")
b, a = spread(before), spread(after)
for key in a:
    p99_b, max_b, _ = b.get(key, ("-", "-", "-"))
    p99_a, max_a, gate = a[key]
    print(f"SUMMARY rev{rev} spread {key}: p99 before={p99_b} after={p99_a}; max before={max_b} after={max_a}; gate(p99<=16ms)={gate}")
PY
  echo
  cat "$WORK/new.txt"
} >>"$OUT"
echo "appended rev $REV to $OUT"
verdict
