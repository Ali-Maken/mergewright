#!/usr/bin/env bash
# ease-pass/20: what a held pixel frame writes. Runs the three held_key pixel
# tests the 512 KiB per-held-frame check failed in (debug build, heavy tier),
# one thread, and writes their SUMMARY / VERDICT / BYTES lines and results to
# docs/perf/20-held-pixel-bytes.txt (or $OUT). Full output stays in a temp file.
# `LABEL` names the run in the file's header (e.g. "before", "after").
set -u
cd "$(dirname "$0")/.." || exit 1
out="${OUT:-docs/perf/20-held-pixel-bytes.txt}"
label="${LABEL:-run}"
log="$(mktemp "${TMPDIR:-/tmp}/ease-20-held.XXXXXX")"
start=$(date +%s)
cargo test --features heavy-tests --test it -- --exact \
  held_key::held_j_stops_on_release_pixel \
  held_key::held_j_stops_on_release_pixel_free \
  held_key::j_then_journal_opens_at_once_pixel \
  --test-threads=1 --nocapture >"$log" 2>&1
code=$?
end=$(date +%s)
{
  echo "# ease-20 held pixel bytes [$label]: HEAD $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- crates || echo ' (+ uncommitted changes under crates/)'), debug, exit $code, $((end - start)) s"
  grep -E '^(SUMMARY|VERDICT|BYTES) held_key' "$log"
  grep -E '^test held_key::' "$log"
  grep -E '^test result' "$log"
} >>"$out"
grep -E '^(test held_key|test result)' "$log"
echo "log: $log"
[ "$code" -eq 0 ] || exit 1
