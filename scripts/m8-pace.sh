#!/usr/bin/env bash
# m8-polish-and-ship/02 (G-pace, ADR-0054): the paced loop under held `j`.
# Release build. Runs every `held_key::` test (the drain now answers the
# loop's fences, so they measure one frame in flight), plus J3's
# `g_after_idle`. Writes docs/perf/m8-pace.txt (or $1): HEAD, machine, the
# SUMMARY / VERDICT lines, J1's tail check per tier and drain, J3's latency,
# and the verdicts for KNOWN_GAPS rows 1089 (kitty without deflate) and 1095
# (scroll residual lag). Full test output stays in a temp dir.
set -u
. "$(cd "$(dirname "$0")" && pwd)/lib/machine-state.sh"
cd "$(dirname "$0")/.."
OUT=${1:-docs/perf/m8-pace.txt}
WORK=$(mktemp -d)
LOAD=$(uptime | sed "s/.*load averages*: //")
trap 'rm -rf "$WORK"' EXIT
machine_state_begin "$OUT"

cargo test --release --features heavy-tests --test it --no-run --quiet 2>"$WORK/build.err" || {
  echo "release build failed:" >&2
  tail -20 "$WORK/build.err" >&2
  exit 1
}

cargo test --release --features heavy-tests --test it -- held_key:: \
  --test-threads=1 --nocapture >"$WORK/held.out" 2>&1
held=$?
cargo test --release --features heavy-tests --test it -- frames::after_idle::g_after_idle \
  --exact --test-threads=1 --nocapture >"$WORK/idle.out" 2>&1
idle=$?

# J1: tail_ms of the scenario-A runs at 40 MB/s and free, per tier.
tail_of() {  # tier drain scenario
  grep -E "^SUMMARY held_key tier=$1 scenario=$3 .*drain=$2B/s" "$WORK/held.out" |
    sed -E 's/.* tail_ms=([0-9]+).*/\1/' | head -1
}
j1_line() {  # tier drain label scenario
  local ms
  ms=$(tail_of "$1" "$2" "$4")
  if [ -z "$ms" ]; then
    echo "J1 tier=$1 scenario=$4 drain=$3 tail_ms=- verdict=no-reading"
  elif [ "$ms" -le 100 ]; then
    echo "J1 tier=$1 scenario=$4 drain=$3 tail_ms=$ms verdict=pass"
  else
    echo "J1 tier=$1 scenario=$4 drain=$3 tail_ms=$ms verdict=fail"
  fi
}

{
  echo "# m8-pace (m8-polish-and-ship/02): held j through the paced loop, release"
  echo "# HEAD $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- crates || echo ' (+ uncommitted changes under crates/)')"
  echo "# machine: $(sysctl -n machdep.cpu.brand_string 2>/dev/null || uname -m), $(sysctl -n hw.ncpu 2>/dev/null) cpus, $(uname -sr)"
  echo "# load before: $LOAD"
  echo "# the DrainSink answers each CSI 5 n once drained past; capabilities.answers_dsr = true"
  grep -E '^(SUMMARY|VERDICT) held_key' "$WORK/held.out"
  grep -E '^test held_key::' "$WORK/held.out"
  grep -A1 'panicked at' "$WORK/held.out" | grep -vE 'panicked at|^--' | cut -c1-400 | sed 's/^/# panic: /'
  grep -E 'frames: g_after_idle' "$WORK/idle.out" | cut -c1-400
  grep -A1 'panicked at' "$WORK/idle.out" | grep -vE 'panicked at|^--' | cut -c1-400 | sed 's/^/# idle panic: /'
  j1_line Unicode 40000000 40MB/s A
  j1_line Unicode 18446744073709551615 free A
  j1_line Pixel 40000000 40MB/s A-deflate
  j1_line Pixel 18446744073709551615 free A-deflate
  # m8-03 (G-settle): the pane shown over merges, the path 03 changed. Its
  # tail is the graph's; the pane's catch-up is rest_ms on its SUMMARY line.
  j1_line Unicode 40000000 40MB/s A-pane
  j1_line Pixel 40000000 40MB/s A-deflate-pane
  idle_us=$(grep -E 'frames: g_after_idle' "$WORK/idle.out" | sed -E 's/.* latency_us=([0-9]+).*/\1/' | head -1)
  if [ -z "$idle_us" ]; then
    echo "J3 g_after_idle latency_us=- verdict=no-reading (exit $idle)"
  elif [ "$idle_us" -le 16000 ] && [ "$idle" -eq 0 ]; then
    echo "J3 g_after_idle latency_us=$idle_us verdict=pass"
  else
    echo "J3 g_after_idle latency_us=$idle_us verdict=fail (exit $idle)"
  fi
  # Row 1089: Kitty without deflate, Pixel scenario A, at 40 MB/s and free.
  raw=$(tail_of Pixel 40000000 A)
  raw_free=$(tail_of Pixel 18446744073709551615 A)
  if [ -n "$raw" ] && [ "$raw" -le 100 ] && [ -n "$raw_free" ] && [ "$raw_free" -le 100 ]; then
    echo "ROW 1089 kitty-no-deflate tail_ms=$raw free_tail_ms=$raw_free verdict=closed-by-fixture (his terminal unverified)"
  else
    echo "ROW 1089 kitty-no-deflate tail_ms=${raw:--} free_tail_ms=${raw_free:--} verdict=still-open"
  fi
  # Row 1095 is his feel in his terminals: the fixture can only say whether
  # the loop still writes ahead of the drain.
  echo "ROW 1095 scroll-residual-lag verdict=question-stays (fixture tails above; feel in Ghostty/VS Code/Zed is his)"
  echo "# exit held=$held idle=$idle"
} >"$OUT"

grep -E '^(J1|J3|ROW|test held_key)' "$OUT"
[ "$held" -eq 0 ] && [ "$idle" -eq 0 ] || exit 1
