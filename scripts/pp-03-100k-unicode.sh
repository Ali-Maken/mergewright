#!/usr/bin/env bash
# pre-publish-cleanup/03: the 100k Unicode frames test under a fixed load,
# repeated. Builds the release heavy test binary once (one cargo command),
# then runs the one test RUNS times by itself while LOAD busy-loop processes
# (`sh -c 'while :; do :; done'`, pure user CPU, one core each) spin. Writes one block to docs/perf/pp-03-100k-unicode.txt
# (or $OUT): a line per run (exit, load average, p99 line, the rows-on-hand
# count and panic if any; `-` when that check passed or was not reached)
# and a SUMMARY line. Full output goes to scratch logs.
#
#   scripts/pp-03-100k-unicode.sh <label> [runs] [load]
#
#   TEST=<filter>   another frames test (default: the 100k Unicode scroll)
#   TRACE=1         also write MWR_FRAME_TRACE per run (kept beside the logs)
#   POLICY="<args>" run the test under `taskpolicy <args>`, e.g. "-c utility"
#                   (the slow-CPU case: the test clamped to efficiency cores)
set -u
cd "$(dirname "$0")/.." || exit 1
label="${1:?label}"
runs="${2:-100}"
load="${3:-10}"
out="${OUT:-docs/perf/pp-03-100k-unicode.txt}"
test="${TEST:-frames::scrolling_the_100k_fixture_keeps_input_to_frame_under_16_ms_at_the_unicode_tier}"
logs="$(mktemp -d "${TMPDIR:-/tmp}/pp-03.XXXXXX")"

bin="$(cargo test --release -p mergewright --features heavy-tests --test it --no-run \
  --message-format=json 2>"$logs/build.log" |
  python3 -c 'import json,sys
for line in sys.stdin:
    m = json.loads(line)
    if m.get("reason") == "compiler-artifact" and m.get("executable") and m["target"]["name"] == "it":
        print(m["executable"])')"
[ -x "$bin" ] || { echo "no test binary; see $logs/build.log" >&2; exit 1; }

spinners=()
stop_load() { [ ${#spinners[@]} -gt 0 ] && kill "${spinners[@]}" 2>/dev/null; spinners=(); }
trap stop_load EXIT
for _ in $(seq "$load"); do
  sh -c 'while :; do :; done' &
  spinners+=("$!")
done
sleep 2 # let the spinners reach their cores before the first run

fails=0
p99s=()
{
  echo "== $label == $(git rev-parse --short HEAD)$(git diff --quiet || echo '+dirty') release runs=$runs load=$load (spin x$load, $(sysctl -n hw.ncpu) cpus) policy=[${POLICY:-default}] test=$test"
} >>"$out"
wrap=()
# shellcheck disable=SC2206 # POLICY is a word list on purpose
[ -n "${POLICY:-}" ] && wrap=(taskpolicy $POLICY)
for run in $(seq "$runs"); do
  log="$logs/run-$run.log"
  if [ "${TRACE:-0}" = 1 ]; then
    MWR_FRAME_TRACE="$logs/trace-$run.txt" MWR_FRAME_RUN="$run" ${wrap[@]+"${wrap[@]}"} "$bin" "$test" --exact --test-threads=1 --nocapture >"$log" 2>&1
  else
    ${wrap[@]+"${wrap[@]}"} "$bin" "$test" --exact --test-threads=1 --nocapture >"$log" 2>&1
  fi
  code=$?
  p99="$(grep -oE 'p99_us=[0-9]+' "$log" | head -1 | cut -d= -f2)"
  [ -n "$p99" ] && p99s+=("$p99")
  [ "$code" -ne 0 ] && fails=$((fails + 1))
  # The verdict's rows-on-hand panic (frames/verdict.rs) names the count.
  misses="$(grep -oE '^[0-9]+ keys were drawn before their rows arrived' "$log" | head -1 | cut -d' ' -f1)"
  panic="$(grep -A1 -E '^thread .* panicked' "$log" | tail -1 | cut -c1-200)"
  echo "run=$run exit=$code load=\"$(sysctl -n vm.loadavg)\" p99_us=${p99:--} rows_on_hand_misses=${misses:--} ${panic:+panic=\"$panic\"}" >>"$out"
done
stop_load
worst="$(printf '%s\n' "${p99s[@]:-0}" | sort -n | tail -1)"
# The logs' scratch path names this machine: the terminal gets it, the
# perf file (committed) does not.
echo "SUMMARY pp-03 $label runs=$runs fails=$fails load=$load worst_p99_us=$worst" | tee -a "$out"
echo "logs: $logs"
echo >>"$out"
