#!/usr/bin/env bash
# ease-pass/24: hints_for cost per call (dev profile) and the 8 hint sweeps' seconds.
# Usage: scripts/24-hints-fitting.sh <label>  -> appends to docs/perf/24-hints-fitting.txt
set -u
label=${1:?label}
cd "$(dirname "$0")/.."
out=docs/perf/24-hints-fitting.txt
h=crates/mergewright-ui/src/graph_screen/hints.rs
cp scripts/24-hints-bench.rs crates/mergewright-ui/src/graph_screen/hints/bench24.rs
cp $h "$h.orig"
printf '\n#[cfg(test)]\nmod bench24;\n' >>$h
cargo test -p mergewright-ui --lib bench_hints_for -- --nocapture --test-threads=1 >${TMPDIR:-/tmp}/24-bench.log 2>&1
cargo test -p mergewright-ui --lib bench_hints_for -- --nocapture --test-threads=1 2>&1 | grep BENCH >${TMPDIR:-/tmp}/24-bench2.log
mv "$h.orig" $h; rm crates/mergewright-ui/src/graph_screen/hints/bench24.rs
{
echo "== $label ==  rev $(git rev-parse --short HEAD) $(date +%F)"
grep BENCH ${TMPDIR:-/tmp}/24-bench.log; cat ${TMPDIR:-/tmp}/24-bench2.log
cargo nextest run -p mergewright-ui --no-fail-fast -E 'test(/graph_screen::hints::tests::(the_bar_never_draws_a_fragment_of_itself|a_tail_hint_gains_its_verb|no_hint_loses_ground|the_bare_keys_are_a_tail|a_words_only_bar_draws|no_context_hides_every_key|the_three_most_relevant)/) | test(/bar_keys/)' --status-level pass 2>&1 | grep -E 'PASS|FAIL' | sed 's/^ *//'
echo
} >>$out
tail -14 $out
