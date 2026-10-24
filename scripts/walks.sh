#!/usr/bin/env bash
# ease-pass/23: the walk tier. Usage: scripts/walks.sh [--check] [-- <extra nextest args>]
# 1. Partition check: the default and walks profiles of .config/nextest.toml
#    split every test (no overlap, no gap, counts add up). Exit 1 otherwise.
# 2. Unless --check: cargo nextest run --profile walks.
# stdout: one SUMMARY line. Failure output only on failure.
set -u
cd "$(dirname "$0")/.."
check_only=0
if [ "${1:-}" = "--check" ]; then check_only=1; shift; fi
[ "${1:-}" = "--" ] && shift
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
list() { cargo nextest list "$@" --message-format oneline 2>"$tmp/err" | sort; }
list >"$tmp/fast"; list --profile walks >"$tmp/walk"; list --ignore-default-filter >"$tmp/all"
if [ ! -s "$tmp/all" ]; then cat "$tmp/err" >&2; echo "SUMMARY walks check=error" >&2; exit 1; fi
cat "$tmp/fast" "$tmp/walk" | sort >"$tmp/union"
overlap=$(comm -12 "$tmp/fast" "$tmp/walk")
gap=$(comm -13 "$tmp/union" "$tmp/all")
extra=$(comm -23 "$tmp/union" "$tmp/all")
nf=$(wc -l <"$tmp/fast" | tr -d ' '); nw=$(wc -l <"$tmp/walk" | tr -d ' '); na=$(wc -l <"$tmp/all" | tr -d ' ')
echo "partition: fast=$nf walks=$nw all=$na"
bad=0
if [ -n "$overlap" ]; then echo "IN BOTH PROFILES:"; echo "$overlap"; bad=1; fi
if [ -n "$gap" ]; then echo "IN NEITHER PROFILE:"; echo "$gap"; bad=1; fi
if [ -n "$extra" ]; then echo "IN A PROFILE BUT NOT IN THE SUITE:"; echo "$extra"; bad=1; fi
if [ "$((nf + nw))" -ne "$na" ]; then echo "counts do not add up: $nf + $nw != $na"; bad=1; fi
if [ "$bad" -ne 0 ]; then echo "SUMMARY walks check=FAILED"; exit 1; fi
[ "$check_only" -eq 1 ] && { echo "SUMMARY walks check=ok fast=$nf walks=$nw all=$na"; exit 0; }
start=$(python3 -I -c 'import time;print(time.time())')
cargo nextest run --profile walks --no-fail-fast "$@" >"$tmp/run" 2>&1
rc=$?
end=$(python3 -I -c 'import time;print(time.time())')
[ "$rc" -ne 0 ] && cat "$tmp/run"
wall=$(python3 -I -c "print(round($end-$start,1))")
failed=$(grep -E '^ +Summary' "$tmp/run" | grep -oE '[0-9]+ failed' | grep -oE '[0-9]+' | head -1)
failed=${failed:-0}
[ "$rc" -ne 0 ] && [ "$failed" = 0 ] && failed="unknown(rc=$rc)"
echo "SUMMARY walks tests=$nw failed=$failed wall_s=$wall"
exit "$rc"
