#!/usr/bin/env bash
# ease-pass/18: samples one release `frames::` test with macOS `sample` while
# it runs, so the loop's time is attributed to functions by observation.
#
#   scripts/18-sample.sh <test-filter> <out-file> [seconds]
#
# Needs the release heavy-tests `it` binary already built
# (`cargo test --release -p mergewright --features heavy-tests --test it --no-run`).
set -u
cd "$(dirname "$0")/.." || exit 1
filter="${1:?filter}"
out="${2:?out}"
secs="${3:-25}"
bin=$(ls -t "${CARGO_TARGET_DIR:-target}"/release/deps/it-* | grep -v '\.d$' | head -1)
"$bin" "$filter" --exact --test-threads=1 --nocapture >"$out.log" 2>&1 &
pid=$!
sleep 0.5
sample "$pid" "$secs" -f "$out" >/dev/null 2>&1
wait "$pid"
echo "exit=$? sample=$out log=$out.log"
