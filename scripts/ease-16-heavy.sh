#!/usr/bin/env bash
# ease-pass/16: the heavy tier, run once, summary written to docs/perf/16-heavy.txt
# (or $OUT). Full output goes to a scratch log, never the repo; the file keeps
# the SUMMARY line, wall time, and one line per failed test.
set -u
cd "$(dirname "$0")/.." || exit 1
out="${OUT:-docs/perf/16-heavy.txt}"
log="$(mktemp "${TMPDIR:-/tmp}/ease-16-heavy.XXXXXX")"
start=$(date +%s)
cargo test --features heavy-tests --no-fail-fast -- --test-threads=2 >"$log" 2>&1
code=$?
end=$(date +%s)
{
  echo "SUMMARY ease-16 heavy tier: exit $code, $((end - start)) s, --test-threads=2"
  grep -E '^test result:' "$log"
  echo "failed tests:"
  grep -E '^    [a-z_0-9:]+$' "$log" | sort -u
  echo "SUMMARY lines from tests:"
  grep -E '^SUMMARY' "$log" | head -60
} >"$out"
echo "log: $log"
