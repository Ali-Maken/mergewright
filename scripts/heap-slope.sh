#!/usr/bin/env bash
# housekeeping/02: where the heavy tier's heap slope per commit goes.
#
#   scripts/heap-slope.sh [label]     default label: "before"
#
# Runs `prototypes/heap-slope-hk02` (a counting allocator plus a
# drop-one-structure-at-a-time accounting) on the same two fixtures the
# guard uses — 10k and 100k, both with a commit-graph — one process at a
# time, and writes the per-structure B/commit table to
# docs/perf/housekeeping-02.txt. Nothing here runs in an agent's context
# (CLAUDE.md: measurement never does).
#
# Live heap is not the gated number: the guard asserts on the footprint
# peak. Both are printed, so the table says how much of the rise the live
# accounting explains.
set -u
cd "$(dirname "$0")/.."
label=${1:-before}
# HEAP_SLOPE_OUT: another perf file to append to (m7-power-tools/12e); an
# absolute path, or relative to the repo root.
out=${HEAP_SLOPE_OUT:-docs/perf/housekeeping-02.txt}
raw=target/heap-slope-hk02
mkdir -p "$raw" docs/perf

cargo build --release -p graft-fixtures --features heavy-tests --example synthetic || exit 1
( cd prototypes/heap-slope-hk02 && cargo build --release ) || exit 1
bin=prototypes/heap-slope-hk02/target/release/heapslope

small=$(target/release/examples/synthetic 10000 --commit-graph | cut -f1) || exit 1
large=$(target/release/examples/synthetic 100000 --commit-graph | cut -f1) || exit 1

"$bin" "$small" > "$raw/$label-10k.tsv" 2> "$raw/$label-10k.err" || {
  echo "10k run failed: $(cat "$raw/$label-10k.err")"; exit 1; }
"$bin" "$large" > "$raw/$label-100k.tsv" 2> "$raw/$label-100k.err" || {
  echo "100k run failed: $(cat "$raw/$label-100k.err")"; exit 1; }

{
  echo "## $label — heap accounting, $(date '+%Y-%m-%d %H:%M'), release, one process at a time"
  echo "# 10k cg: $small"
  echo "# 100k cg: $large"
  echo "# B/commit = (100k - 10k) / 90000. Live heap is measured by a counting"
  echo "# global allocator; each 'hold' row is the live bytes that structure freed"
  echo "# when dropped, in the order listed."
  echo
  printf '%-46s %12s %12s %10s\n' "structure" "10k B" "100k B" "B/commit"
  awk -F'\t' '
    FILENAME == ARGV[1] { small[$2] = $3; order[++n] = $2; kind[$2] = $1; next }
    { large[$2] = $3; if (!($2 in small)) { order[++n] = $2; kind[$2] = $1 } }
    END {
      for (i = 1; i <= n; i++) {
        k = order[i]
        s = (k in small) ? small[k] : 0
        l = (k in large) ? large[k] : 0
        per = (l - s) / 90000
        printf "%-46s %12d %12d %10.2f\n", kind[k] "/" k, s, l, per
      }
    }
  ' "$raw/$label-10k.tsv" "$raw/$label-100k.tsv"
  echo
} >> "$out"
echo "written: $out (raw: $raw, untracked)"
