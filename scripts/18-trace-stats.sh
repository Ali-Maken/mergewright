#!/usr/bin/env bash
# ease-pass/18: per test (tier, refs, pane) in an MWR_FRAME_TRACE file, the
# scroll keys' latency / update / draw / queued: n, p50, p99 (nearest rank,
# as `Gate::of`), max. Text tiers' `draw_us=-` (a frame joined a later turn)
# is left out of draw_us.
#
#   scripts/18-trace-stats.sh <trace-file>
set -u
f="${1:?trace file}"
grep -o 'tier=[A-Za-z]* refs=[A-Za-z]* pane=[A-Za-z]*' "$f" | sort -u | while read -r tier refs pane; do
  sel="$tier $refs $pane "
  for field in latency_us update_us draw_us queued_us; do
    grep -F "$sel" "$f" | grep 'kind=scroll' | grep -o "$field=[0-9]*" | cut -d= -f2 | sort -n |
      awk -v t="$sel" -v f="$field" '{a[NR]=$1} END{if (NR) printf "SUMMARY18 %s%s n=%d p50=%d p99=%d max=%d\n", t, f, NR, a[int((NR-1)/2)+1], a[int((NR-1)*99/100)+1], a[NR]}'
  done
done
