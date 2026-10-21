#!/usr/bin/env bash
# ease-pass/19: the release heavy-tier runs behind the fold rules (ADR-0065):
# at-rest wakes (the four frames tests), the budgets in both graph modes (five
# `budgets_hold_*` plus every-branch-shown) and `g_after_idle`. Writes
# docs/perf/19-fold-rules.txt (or $OUT): one labelled block with the
# `frames:` and `SUMMARY budget` lines, `woke=` lines, p99 lines and panics.
# Full output goes to a scratch log, never the repo.
#
#   scripts/19-fold-rules.sh <label>
#
# `--test-threads=1`, release, one cargo run: no other cargo helper may run.
set -u
cd "$(dirname "$0")/.." || exit 1
label="${1:?label}"
out="${OUT:-docs/perf/19-fold-rules.txt}"
filters=(
  frames::scrolling_the_100k_fixture_keeps_input_to_frame_under_16_ms_at_the_pixel_tier
  frames::scrolling_the_100k_fixture_keeps_input_to_frame_under_16_ms_at_the_unicode_tier
  frames::fifty_thousand_refs_keep_input_to_frame_under_16_ms_at_the_pixel_tier
  frames::fifty_thousand_refs_keep_input_to_frame_under_16_ms_at_the_unicode_tier
  heavy::budgets_hold_on_the_100k_fixture_without_a_commit_graph
  heavy::budgets_hold_on_the_100k_fixture_with_a_commit_graph
  heavy::budgets_hold_on_the_100k_realistic_fixture_without_a_commit_graph
  heavy::budgets_hold_on_the_100k_realistic_fixture_with_a_commit_graph
  heavy::budgets_hold_on_the_100k_fixture_with_50k_refs
  heavy::budgets_hold_on_the_100k_fixture_with_every_branch_shown
  frames::after_idle::g_after_idle
)
log="$(mktemp "${TMPDIR:-/tmp}/19-fold-rules.XXXXXX")"
start=$(date +%s)
cargo test --release -p mergewright --features heavy-tests --test it -- \
  "${filters[@]}" --exact --test-threads=1 --nocapture >"$log" 2>&1
code=$?
end=$(date +%s)
{
  echo "## $label — $(git rev-parse --short HEAD)$(git diff --quiet || echo '+dirty') release exit=$code $((end - start))s load: $(sysctl -n vm.loadavg 2>/dev/null)"
  grep -E '^test (frames|heavy)|^frames: .*(p99_us|idle_gap_wakes)|^SUMMARY budget|^thread .* panicked|^p99 |^first paint|^[0-9]+ wakes|woke=' "$log" | cut -c1-400
  grep -E 'keys over 16ms' "$log" | cut -c1-400
  echo
} >>"$out"
echo "log: $log"
exit $code
