#!/usr/bin/env bash
# ease-pass/18: the release `frames::` runs behind review Major 6 (50 000 refs)
# and 16's at-rest wakes (100k scroll). Appends a labelled block to
# docs/perf/18-lag.txt (or $OUT): the `frames:` summary lines, each key over
# budget cut to 400 chars, and the panic lines. Full output goes to a scratch
# log, never the repo.
#
#   scripts/18-lag.sh <label> [filter...]
#
# Filters default to the five tests the ticket names. `--test-threads=1`:
# the tests share one MEASURING lock anyway, and one runs at a time on a
# laptop. MWR_FRAME_TRACE, if set, is passed through (per-key phase lines).
# BUILD=debug runs the debug build (16's heavy run was debug).
set -u
cd "$(dirname "$0")/.." || exit 1
label="${1:?label}"
shift
out="${OUT:-docs/perf/18-lag.txt}"
filters=("$@")
if [ ${#filters[@]} -eq 0 ]; then
  filters=(
    frames::fifty_thousand_refs_keep_input_to_frame_under_16_ms
    frames::scrolling_the_100k_fixture_keeps_input_to_frame_under_16_ms
  )
fi
profile=(--release)
[ "${BUILD:-release}" = debug ] && profile=()
log="$(mktemp "${TMPDIR:-/tmp}/18-lag.XXXXXX")"
start=$(date +%s)
cargo test ${profile[@]+"${profile[@]}"} -p mergewright --features heavy-tests --test it -- \
  "${filters[@]}" --test-threads=1 --nocapture >"$log" 2>&1
code=$?
end=$(date +%s)
{
  echo "## $label — $(git rev-parse --short HEAD)$(git diff --quiet || echo '+dirty') build=${BUILD:-release} exit=$code $((end - start))s load: $(sysctl -n vm.loadavg 2>/dev/null)"
  grep -E '^test frames|^frames: .*(p99_us|idle_gap_wakes|idle_prefetch|idle_wake_us)|^thread .* panicked|^p99 |^[0-9]+ wakes|^SUMMARY18' "$log" | cut -c1-400
  grep -E 'keys over 16ms' "$log" | cut -c1-400
  echo
} >>"$out"
echo "log: $log"
exit $code
