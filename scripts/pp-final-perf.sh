#!/usr/bin/env bash
# pre-publish-cleanup final run, quiet machine, serial: fast-tier wall, then the
# heavy tier under --release (frame budgets fail only there, ADR-0021).
# Summary to docs/perf/pp-final.txt; full output stays in a scratch log.
set -u
cd "$(dirname "$0")/.." || exit 1
out=docs/perf/pp-final.txt
log="$(mktemp "${TMPDIR:-/tmp}/pp-final.XXXXXX")"
{
  echo "load before: $(uptime)"
  s=$(date +%s); cargo nextest run --no-fail-fast >"$log" 2>&1; c=$?; e=$(date +%s)
  echo "FAST exit $c wall $((e - s)) s"; grep -E 'Summary' "$log"
  s=$(date +%s); cargo test --release --features heavy-tests --no-fail-fast -- --test-threads=2 >"$log" 2>&1; c=$?; e=$(date +%s)
  echo "HEAVY-RELEASE exit $c wall $((e - s)) s"
  grep -E '^test result:' "$log"
  echo "failed tests:"; grep -E '^    [a-z_0-9:]+$' "$log" | sort -u
  echo "SUMMARY lines from tests:"; grep -E '^SUMMARY' "$log" | head -60
  echo "load after: $(uptime)"
} >"$out" 2>&1
echo "log: $log"
