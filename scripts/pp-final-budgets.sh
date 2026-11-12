#!/usr/bin/env bash
# pre-publish-cleanup/14: run the three raised-budget heavy tests N times in release; counts to docs/perf/pp-final-budgets.txt.
set -u
cd "$(dirname "$0")/.." || exit 1
out=docs/perf/pp-final-budgets.txt; n=${1:-5}
cargo test --release --features heavy-tests --no-run >/dev/null 2>&1
{
  echo "load before: $(uptime)"
  for t in held_key::held_j_stops_on_release_pixel_free piece_gate::no_frame_rasterises_more frames::scrolling_the_100k_fixture_keeps_input_to_frame_under_16_ms_at_the_pixel_tier; do
    pass=0
    for i in $(seq "$n"); do
      cargo test --release --features heavy-tests -p mergewright --test it -- --exact "$t" >/tmp/ppb.log 2>&1 && pass=$((pass+1))
      grep -E 'p99 .* over|VERDICT.*FAILED|blocked:' /tmp/ppb.log | head -2
      grep -E '^frames: tier=Pixel refs=Fixture pane=Closed control=Plain keys' /tmp/ppb.log
    done
    echo "$t: $pass/$n passed"
  done
  echo "load after: $(uptime)"
} >"$out" 2>&1
