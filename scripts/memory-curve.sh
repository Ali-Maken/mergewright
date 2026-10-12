#!/bin/bash
# The memory curve (tickets m1-the-graph/46, 56): peak RSS against history size.
#
#   scripts/memory-curve.sh [sizes...]            default: 10000 100000 1000000
#   scripts/memory-curve.sh --repo <path> <label> one repository, as one cell
#   APPEND=1 scripts/memory-curve.sh 1000000        add sizes to an existing file
#
# Heavy tier. Builds each synthetic repository once (cached under
# target/fixtures/, 1M ≈ 340–400 MiB), then runs the release `budgets`
# example's gated path (open → first paint → G → g) three times per cell,
# one process at a time, then once more for the counters (order bytes, what
# a cold G reads, one far jump). Variants per size:
# small and git/git-sized commits, each without and with a commit-graph.
# Results: docs/shots/m1-memory-curve.txt (the sizes run) or
# target/56/<label>.txt (--repo). Per cell: the fixture's pack bytes on
# disk; per run: every `budgets` line plus the footprint.
# RSS = getrusage peak (the gate); footprint = /usr/bin/time -l's
# "peak memory footprint" (anonymous pages; excludes clean file-backed ones).
set -euo pipefail
. "$(cd "$(dirname "$0")" && pwd)/lib/machine-state.sh"  # ticket 49: battery / Low Power Mode guard
cd "$(dirname "$0")/.."
OUT=docs/shots/m1-memory-curve.txt
[ "${1:-}" = "--repo" ] && OUT=target/56/$3.txt
machine_state_begin "$OUT"
cargo build --release -p mergewright --example budgets

cell() { # cell <repo> <label> <out>
  local repo=$1 label=$2 out=$3
  local objects=$repo/.git/objects
  [ -d "$objects" ] || objects=$repo/objects
  local pack cg=absent
  pack=$(du -sk "$objects/pack" | cut -f1)
  [ -e "$objects/info/commit-graph" ] && cg=present
  echo "# $label: $repo" | tee -a "$out"
  echo "# $label: pack $((pack / 1024)) MiB on disk, commit-graph $cg" | tee -a "$out"
  # The gated path alone, three times: its own peak RSS (in-process
  # getrusage, the gate), the process-wide max RSS and footprint from time -l.
  for run in 1 2 3; do
    local raw fp maxrss
    raw=$( { /usr/bin/time -l target/release/examples/budgets --gate "$repo"; } 2>&1 )
    fp=$(awk '/peak memory footprint/ {printf "%.1f", $1/1048576}' <<<"$raw")
    maxrss=$(awk '/maximum resident set size/ {printf "%.1f", $1/1048576}' <<<"$raw")
    grep -E '^first paint' <<<"$raw" | sed "s/^/$label run$run: /" | tee -a "$out"
    echo "$label run$run: footprint $fp MiB, process max RSS $maxrss MiB" | tee -a "$out"
  done
  # The counters, once: order bytes, what a cold G reads, one far jump.
  target/release/examples/budgets "$repo" | grep -E '^(EmittedOrder|cold G|far jump|peak RSS)' \
    | sed "s/^/$label once: /" | tee -a "$out"
}

if [ "${1:-}" = "--repo" ]; then
  mkdir -p target/56
  OUT=target/56/$3.txt; : > "$OUT"
  cell "$2" "$3" "$OUT"
  exit 0
fi

SIZES=("$@"); [ ${#SIZES[@]} -eq 0 ] && SIZES=(10000 100000 1000000)
OUT=docs/shots/m1-memory-curve.txt
cargo build --release -p graft-fixtures --features heavy-tests --example synthetic
{
  echo "# The memory curve: scripts/memory-curve.sh ${SIZES[*]} ($(date +%F), release, 3 runs per cell, one process at a time)"
  echo "# Gate: peak RSS < 300 MB with a commit-graph (ADR-0026); without one at 1M: reported, footprint < 300 MB."
} | if [ "${APPEND:-0}" = 1 ]; then cat >> "$OUT"; else cat > "$OUT"; fi
for n in "${SIZES[@]}"; do
  for variant in "" "--commit-graph" "--realistic" "--realistic --commit-graph"; do
    # shellcheck disable=SC2086
    fixture=$(target/release/examples/synthetic "$n" $variant)
    repo=$(cut -f1 <<<"$fixture")
    label="$n ${variant:-small}"
    cell "$repo" "$label" "$OUT"
  done
done
echo "written: $OUT"
