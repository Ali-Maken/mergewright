#!/usr/bin/env bash
# m8/22: run every fuzz target for a bounded time. Milestone reviews only
# (fuzz/README.md: never from builders' gates or CI).
#
#   scripts/fuzz-run.sh [seconds-per-target=60] [target ...]
#
# Why -timeout: libFuzzer's default per-input timeout is 1200 s, and
# -max_total_time is only checked between inputs, so one slow input let
# line_log run 590 s, parse_file_list 397 s and parse_porcelain_v2 1001 s
# against a 60 s budget (M7 review). -timeout=10 makes a slow input a
# reported failure instead. libFuzzer writes it as fuzz/slow/<target>-timeout-*;
# file each one found (it is a performance defect in the parser, not noise).
#
# Writes docs/perf/fuzz-run.txt; summary lines:
#   fuzz: <target> exit=<n> wall=<s>s budget=<s>s [slow=<file>...]
set -u
cd "$(dirname "$0")/.." || exit 1
SECS=${1:-60}
[ $# -gt 0 ] && shift
TARGETS=("$@")
if [ ${#TARGETS[@]} -eq 0 ]; then
  for f in fuzz/fuzz_targets/*.rs; do TARGETS+=("$(basename "$f" .rs)"); done
fi
OUT=docs/perf/fuzz-run.txt
mkdir -p fuzz/slow docs/perf
LOGS=$(mktemp -d)
trap 'rm -rf "$LOGS"' EXIT
worst=0
echo "# fuzz run — $(date -u +%FT%TZ), HEAD $(git rev-parse --short HEAD), ${SECS}s per target, -timeout=10" >"$OUT"
for t in "${TARGETS[@]}"; do
  start=$SECONDS
  (cd fuzz && cargo +nightly fuzz run "$t" -- \
    -max_total_time="$SECS" -timeout=10 -artifact_prefix="slow/$t-") >"$LOGS/$t.log" 2>&1
  status=$?
  wall=$((SECONDS - start))
  slow=$(find fuzz/slow -type f -name "$t-*" | sort | tr '\n' ' ')
  echo "fuzz: $t exit=$status wall=${wall}s budget=${SECS}s${slow:+ slow=$slow}" | tee -a "$OUT"
  if [ "$status" -ne 0 ]; then
    worst=$status
    tail -15 "$LOGS/$t.log" | sed 's/^/#   /' >>"$OUT"
  fi
done
exit "$worst"
