#!/usr/bin/env bash
# Debug: scroll lag on a big real repository (field notes 2026-10-01, "held
# j kept going after I let go"). Writes docs/perf/scroll-lag-big-repo.txt
# (or $2). Lines a reader needs first start with SUMMARY.
#
#   $1  a COPY or clone of the repository (never the user's own: Tributary
#       writes .git/tributary/ in what it opens). This debug used
#       `rsync -a` of his work repo into target/scroll-lag/, so the
#       working tree, node_modules and reflogs match his.
#
# Part 1, in-process: the release `frames::a_real_repository_...` test (the
# real loop, real jobs, an in-memory terminal with no back-pressure), traced
# per key (TRIB_FRAME_TRACE): update, carry, draw (view + raster + encode)
# and write, at 120x40 and 200x50, Unicode and Pixel tiers.
#
# Part 2, a real pty: the release binary, `j` held at 30 Hz for 3 s
# (scripts/scroll-lag-pty.py), the probe answered like VS Code/Zed (text)
# or like Ghostty (kitty), at open (0.3 s) and settled (5 s). The reader
# costs nothing, so `tail_ms` is Tributary's own backlog, before any terminal
# parses a byte.
#
# Part 3, a model: the text profile read at a fixed rate, to show what the
# loop does when the terminal drains slower than keys arrive.
set -u
. "$(cd "$(dirname "$0")" && pwd)/lib/machine-state.sh"  # ticket 49: battery / Low Power Mode guard
cd "$(dirname "$0")/.." || exit 1
REPO=${1:?usage: scroll-lag-big-repo.sh <repo copy> [out]}
OUT=${2:-docs/perf/scroll-lag-big-repo.txt}
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
machine_state_begin "$OUT"
ROOT=$PWD

cargo build --release --quiet --bin mwr 2>"$WORK/build.err" || { tail -20 "$WORK/build.err" >&2; exit 1; }
cargo test --release --features heavy-tests --no-run --quiet 2>"$WORK/build.err" || { tail -20 "$WORK/build.err" >&2; exit 1; }

{
  echo "# debug scroll-lag-big-repo — $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "# HEAD $(git rev-parse --short HEAD)$(git diff --quiet || echo ' +uncommitted')"
  echo "# $(sysctl -n machdep.cpu.brand_string), $(sysctl -n hw.ncpu) cpus; $(uptime | sed 's/.*load/load/')"
  echo "# repo: $(git -C "$REPO" rev-list --count --all) commits, $(git -C "$REPO" for-each-ref | wc -l | tr -d ' ') refs, commit-graph chain: $(test -f "$REPO/.git/objects/info/commit-graphs/commit-graph-chain" && echo yes || echo no)"
} >"$OUT"

summarise() { # label trace-file test-output
  python3 - "$1" "$2" "$3" <<'EOF' >>"$OUT"
import re, sys
label, trace, out = sys.argv[1:4]
rows = []
for line in open(trace):
    if "kind=scroll" not in line or "code=Char('j')" not in line:
        continue
    rows.append({k: int(v) for k, v in re.findall(r"(\w+)_us=(\d+)", line)})
def q(key, p):
    xs = sorted(r[key] for r in rows if key in r)
    return xs[min(len(xs) - 1, int(len(xs) * p))] if xs else -1
text = open(out).read()
j = re.search(r"trigger=Char\('j'\) frames=(\d+) totals: bytes=(\d+) transmits=(\d+)", text)
summary = re.search(r"frames: real .*", text)
parts = " ".join(f"{k}_p50={q(k, .5)} {k}_p99={q(k, .99)}" for k in ("latency", "update", "carry", "draw", "write"))
per = int(j.group(2)) // int(j.group(1)) if j else -1
print(f"SUMMARY inproc {label} j_keys={len(rows)} {parts} bytes_per_j_frame={per} j_transmits={j.group(3) if j else '-'}")
print(f"  {summary.group(0) if summary else 'NO SUMMARY LINE'}")
for line in text.splitlines():
    if (line.startswith("frame_trace: real startup burst") or "trigger=" in line
            or "drawn before their rows" in line or "panicked" in line):
        print("  " + line[:260])
EOF
}

for tier in unicode pixel; do
  for size in 120x40 200x50; do
    label="tier=$tier size=$size"
    rm -f "$WORK/trace.txt"
    TRIB_REAL_REPO="$REPO" TRIB_REAL_TIER=$tier TRIB_REAL_SIZE=$size TRIB_FRAME_TRACE="$WORK/trace.txt" \
      cargo test --release --features heavy-tests --quiet -p mergewright --test it -- --ignored --exact \
      frames::a_real_repository_keeps_input_to_frame_under_16_ms --nocapture \
      >"$WORK/out.txt" 2>&1
    echo "# inproc $label exit $?" >>"$OUT"
    summarise "$label" "$WORK/trace.txt" "$WORK/out.txt"
  done
done

pty() { # profile cols rows wait [rate]
  PROFILE=$1 RATE=${5:-} python3 scripts/scroll-lag-pty.py "$ROOT/target/release/mwr" "$REPO" "$2" "$3" "$4" 0.033 3 pty >>"$OUT"
}
for profile in text kitty; do
  for size in "120 40" "200 50"; do
    for wait in 0.3 5; do
      pty $profile $size $wait
    done
  done
done
# Part 3: the model. A terminal draining 150 KB/s, 300 KB/s.
for rate in 150000 300000; do
  pty text 120 40 0.3 $rate
done
echo "# done $(date -u +%H:%M:%SZ)" >>"$OUT"
grep '^SUMMARY' "$OUT"
