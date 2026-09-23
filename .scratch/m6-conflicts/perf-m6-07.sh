#!/usr/bin/env bash
# M6 acceptance (m6-conflicts/07): the M6 walk rows' fast-tier time, then the
# milestone's release + heavy tier. Writes docs/perf/m6-07.txt; the builder
# reads the summary lines only.
#
# Usage (repo root): CARGO_TARGET_DIR=<dir> bash .scratch/m6-conflicts/perf-m6-07.sh
set -u
cd "$(dirname "$0")/../.."
OUT=docs/perf/m6-07.txt
ROWS='merge_resolved_with_the_keys|every_whole_file_shape|whole_file_card_glimpses_a_large_file|binary_card_shows_its_size|crlf_and_no_final_newline_survive|zdiff3_and_base_on_demand|n_past_the_last_block_says_no_more|start_over_brings_the_markers_back|editor_edits_are_never_clobbered|mergetool_hand_over_and_back|rebase_stops_twice_resolved_twice|stash_pop_conflict_resolves_without_a_banner|undo_a_resolution_in_a_stash_pop_conflict|kill_9_mid_rebase_then_relaunch|kill_9_mid_rebase_then_abort|u_right_after_continue_is_accepted|repush_lease_is_the_entrys_tip|resolver_is_modal_and_esc_returns|tiny_frame_says_so'
LOG=$(mktemp)
{
  echo "# M6 acceptance perf — $(date -u +%FT%TZ)"
  echo "# HEAD $(git rev-parse --short HEAD) (plus the uncommitted work of m6-conflicts/07)"
  echo "# machine: $(uname -sm), $(sysctl -n hw.ncpu 2>/dev/null || nproc) cpus"
  echo
  echo "## M6 walk rows, fast tier (warm build; the ticket's bar is < 60 s)"
  cargo build -q -p graft --tests >/dev/null 2>&1
  t0=$(date +%s)
  cargo nextest run -p graft --test it --no-fail-fast -E "test(/every_scenario::($ROWS)\$/)" >"$LOG" 2>&1
  code=$?
  t1=$(date +%s)
  grep -E "Summary" "$LOG" | sed 's/^ */  /'
  grep -E "^\s+FAIL \[" "$LOG" | sort -u | sed 's/^ */  /'
  echo "  wall $((t1 - t0)) s, exit $code (red rows are the ticket's findings F1-F5)"
  echo
  echo "## release + heavy tier (ADR-0021; frame budgets fail only under --release)"
  t0=$(date +%s)
  cargo test --release --features heavy-tests --workspace --no-fail-fast --quiet >"$LOG" 2>&1
  code=$?
  t1=$(date +%s)
  passed=$(grep -Eo "[0-9]+ passed" "$LOG" | awk '{s+=$1} END {print s+0}')
  failed=$(grep -Eo "[0-9]+ failed" "$LOG" | awk '{s+=$1} END {print s+0}')
  ignored=$(grep -Eo "[0-9]+ ignored" "$LOG" | awk '{s+=$1} END {print s+0}')
  echo "  cargo test --release --features heavy-tests --workspace --no-fail-fast"
  echo "  $passed passed, $failed failed, $ignored ignored, exit $code, wall $((t1 - t0)) s"
  grep -E "^test .* FAILED|panicked at|^---- " "$LOG" | head -40 | sed 's/^/  /'
} >"$OUT" 2>&1
rm -f "$LOG"
