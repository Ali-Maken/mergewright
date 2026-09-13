#!/bin/bash
# The memory curve (ticket m1-the-graph/46): peak RSS against history size.
#
#   scripts/memory-curve.sh [sizes...]        default: 10000 100000 1000000
#
# Heavy tier. Builds each synthetic repository once (cached under
# target/fixtures/, 1M ≈ 350 MiB), then runs the release `budgets` example
# (open → first paint → G → g) three times per cell, one process at a time.
# Variants per size: small commits without and with a commit-graph, and
# git/git-sized commits without one. Results: target/46/curve.txt.
# RSS = getrusage peak (the gate); footprint = /usr/bin/time -l.
set -euo pipefail
cd "$(dirname "$0")/.."
SIZES=("$@"); [ ${#SIZES[@]} -eq 0 ] && SIZES=(10000 100000 1000000)
OUT=target/46/curve.txt; mkdir -p target/46
cargo build --release -p graft --example budgets
cargo build --release -p graft-fixtures --features heavy-tests --example synthetic
for n in "${SIZES[@]}"; do
  for variant in "" "--commit-graph" "--realistic"; do
    fixture=$(target/release/examples/synthetic "$n" $variant)
    repo=$(cut -f1 <<<"$fixture")
    echo "# $n ${variant:-small} fixture: $(cut -f2- <<<"$fixture")" | tee -a "$OUT"
    for run in 1 2 3; do
      out=$( { /usr/bin/time -l target/release/examples/budgets "$repo"; } 2>&1 )
      fp=$(awk '/peak memory footprint/ {printf "%.1f", $1/1048576}' <<<"$out")
      line=$(grep '^first paint' <<<"$out")
      echo "$n ${variant:-small} run$run: $line, footprint $fp MiB" | tee -a "$OUT"
    done
  done
done
