#!/usr/bin/env bash
# M4 milestone review: release perf tier + heavy tier. Writes docs/perf/m4.txt.
set -u
cd "$(dirname "$0")/.."
OUT=docs/perf/m4.txt
{
  echo "# M4 milestone review — $(date -u +%FT%TZ)"
  echo "# HEAD $(git rev-parse --short HEAD)"
  echo
  echo "## release perf tier (frame budgets fail only under --release, ADR-0021)"
  t0=$SECONDS
  cargo test --release --features heavy-tests --quiet 2>&1 | tail -40
  echo "release+heavy exit ${PIPESTATUS[0]:-?}, ${((SECONDS-t0))}s"
} > "$OUT" 2>&1
