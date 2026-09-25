#!/usr/bin/env bash
# M6 look-and-finish acceptance (m6-conflicts/63): the resolver's three-pane
# frame at 187x50 and 200x50 against the 16 ms input-to-frame budget, plus the
# release heavy::budgets run (ADR-0021: time budgets fail only under
# --release). Writes docs/perf/acceptance-look-and-finish.txt; the builder
# reads the summary lines only.
#
# Budgets: input to frame < 16 ms per key (crates/graft/tests/it/heavy.rs
# INPUT_TO_FRAME); first paint <= 150 ms and peak RSS < 300 MiB on the 100k
# fixtures.
#
# Usage (repo root): CARGO_TARGET_DIR=<dir> bash .scratch/m6-conflicts/perf-m6-63.sh
set -u
cd "$(dirname "$0")/../.."
OUT=docs/perf/acceptance-look-and-finish.txt
LOG=$(mktemp)
{
  echo "# M6 look-and-finish perf — $(date -u +%FT%TZ)"
  echo "# HEAD $(git rev-parse --short HEAD)"
  echo "# machine: $(uname -sm), $(sysctl -n hw.ncpu 2>/dev/null || nproc) cpus"
  echo
  echo "## release resolver::frame_budget + heavy::budgets"
  t0=$(date +%s)
  cargo test --release --features heavy-tests -p graft --test it --no-fail-fast -- resolver::frame_budget heavy::budgets --nocapture --test-threads=1 >"$LOG" 2>&1
  code=$?
  t1=$(date +%s)
  echo
  echo "## three-pane resolver frame (budget: every key < 16000 us, app.key + terminal.draw)"
  grep -E "resolver_frames:" "$LOG" | sed 's/^/  /'
  echo
  echo "## first paint (budget: <= 150000 us)"
  grep -E "first_paint_us=" "$LOG" | sed 's/^/  /'
  echo
  echo "## summary"
  grep -E "^test result:" "$LOG" | sed 's/^/  /'
  grep -E "^test .* FAILED|panicked at|over the" "$LOG" | head -20 | sed 's/^/  /'
  awk -F'max_us=' 'NF>1 {split($2,a," "); us=a[1]+0; n++; if (us>max) max=us; if (us>=16000) bad++} END {printf "  resolver_frames: %d sizes, max %d us, %d at/over the 16000 us budget\n", n, max, bad+0}' "$LOG"
  awk -F'first_paint_us=' 'NF>1 {split($2,a," "); us=a[1]+0; n++; if (us>max) max=us; if (us>150000) bad++} END {printf "  first_paint_us: %d runs, max %d us, %d over the 150000 us budget\n", n, max, bad+0}' "$LOG"
  awk -F'peak_rss=' 'NF>1 {split($2,a," "); r=a[1]+0; if (r>max) max=r; if (r>=314572800) bad++} END {printf "  peak_rss: max %d MiB, %d at/over the 300 MiB budget\n", max/1048576, bad+0}' "$LOG"
  echo "  exit $code, wall $((t1 - t0)) s"
} >"$OUT" 2>&1
rm -f "$LOG"
