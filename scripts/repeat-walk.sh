#!/usr/bin/env bash
# pre-publish-cleanup/02: repeat a walk-tier filter under nextest, count fails.
# Usage: scripts/repeat-walk.sh <label> <runs> <nextest filterset> [out-file] [-- <extra nextest args>]
# Appends "== <label> ==", per-run lines, the first failure's tail and one
# SUMMARY line to out-file (default docs/perf/pp-02-fork-rows.txt). Prints
# only the SUMMARY line. Machine paths are replaced by placeholders.
set -u
cd "$(dirname "$0")/.."
label=$1 runs=$2 filter=$3 out=${4:-docs/perf/pp-02-fork-rows.txt}
shift 3; [ $# -gt 0 ] && [ "$1" != "--" ] && shift
[ "${1:-}" = "--" ] && shift
tmp=$(mktemp -d)
trap 'rm -rf "$tmp" "${canary:-}"' EXIT
redact() { sed -e "s#$PWD#<repo>#g" -e "s#${TMPDIR%/}#<tmp>#g" -e "s#$HOME#<home>#g"; }
{ echo "== $label =="; echo "filter: $filter $*"; echo "load: $(uptime | sed 's/.*load/load/')"; } >>"$out"
fails=0 tests=0 first="" lost=0
# Canaries: a repository-shaped folder in the shared temp dir, named as
# tempfile names its own. A sweep of the temp dir would take it too.
canary="${TMPDIR%/}/.tmppp02cn"
plant() { rm -rf "$canary"; mkdir -p "$canary/.git/objects/ab" "$canary/.git/refs"; echo x >"$canary/.git/HEAD"; echo x >"$canary/.git/objects/ab/cd"; }
plant
start=$(date +%s)
for i in $(seq 1 "$runs"); do
  cargo nextest run --profile walks --no-fail-fast -E "$filter" "$@" >"$tmp/run" 2>&1
  rc=$?
  line=$(grep -E '^ +Summary' "$tmp/run" | tail -1)
  n=$(echo "$line" | grep -oE '[0-9]+ tests? run' | grep -oE '[0-9]+' | head -1)
  f=$(echo "$line" | grep -oE '[0-9]+ failed' | grep -oE '[0-9]+' | head -1)
  tests=$((tests + ${n:-0})); f=${f:-0}
  [ "$rc" -ne 0 ] && [ "$f" = 0 ] && f=1
  if [ ! -f "$canary/.git/objects/ab/cd" ]; then lost=$((lost + 1)); echo "run $i: canary lost" >>"$out"; plant; fi
  if [ "$f" -ne 0 ]; then
    fails=$((fails + 1))
    names=$(grep -E '^ +(FAIL|SIGSEGV|SIGABRT|TIMEOUT)' "$tmp/run" | sed 's/.*\] *//' | sort -u | tr '\n' ' ')
    echo "run $i: FAIL $names" >>"$out"
    if [ -z "$first" ]; then
      first=$i
      { echo "-- first failure (run $i), stderr tail --"; grep -E 'panicked|error:|fatal:|failed \(exit' "$tmp/run" | head -40 | redact; echo "--"; } >>"$out"
    fi
  fi
done
wall=$(( $(date +%s) - start ))
summary="SUMMARY $label runs=$runs fails=$fails tests_run=$tests first_fail_run=${first:-none} canary_lost=$lost wall_s=$wall"
echo "$summary" >>"$out"
echo "$summary"
