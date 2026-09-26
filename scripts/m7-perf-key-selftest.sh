#!/usr/bin/env bash
# Self-test for scripts/m7-perf-key.sh (m7-power-tools/12d, review 04 Major 1):
# the perf script must never report a failed run as a pass. Runs it on canned
# test outputs (M7_PERF_KEY_CANNED: no cargo, no measuring) and checks its
# exit status and SUMMARY lines for each case. Takes seconds.
#
#   bash scripts/m7-perf-key-selftest.sh      # exit 0: every case held
set -u
cd "$(dirname "$0")/.."
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
FAILURES=0

# One `frames:` summary line, as frames.rs prints it.
frames_line() { # pane p99_us
  echo "frames: tier=Unicode refs=Fixture pane=$1 control=Plain keys=420 p50_us=900 p99_us=$2 max_us=$(($2 + 500)) over_budget=0 kitty_commands=0 bytes=1000"
}

# A canned run: `<label>.out` holds the test output, `<label>.exit` its status.
canned_run() { # dir label exit [panes...]
  local dir=$1 label=$2 status=$3
  shift 3
  mkdir -p "$dir"
  : >"$dir/$label.out"
  for pane in "$@"; do
    frames_line "$pane" 4000 >>"$dir/$label.out"
  done
  if [ "$status" = 0 ]; then
    echo "test result: ok. 2 passed; 0 failed" >>"$dir/$label.out"
  else
    echo "thread 'frames::x' panicked at frames.rs:1: 3 keys were drawn before their rows arrived" >>"$dir/$label.out"
    echo "test result: FAILED. 1 passed; 1 failed" >>"$dir/$label.out"
  fi
  echo "$status" >"$dir/$label.exit"
}

# Every run exits 0 and prints both scenarios, except as the case changes.
canned_set() { # dir
  for label in plain1 plain2 plain3 controlA controlB controlC; do
    canned_run "$1" "$label" 0 Closed Open
  done
}

# Runs the perf script on a canned set; its stdout+stderr go to <case>.log.
perf() { # case dir out
  M7_PERF_KEY_CANNED="$2" REV_NOTE="selftest $1" bash scripts/m7-perf-key.sh "$3" >"$WORK/$1.log" 2>&1
}

expect() { # case what actual wanted
  if [ "$3" = "$4" ]; then
    echo "ok   $1: $2"
  else
    echo "FAIL $1: $2: got '$3', wanted '$4'"
    FAILURES=$((FAILURES + 1))
  fi
}

has() { # case file pattern
  if grep -qE -- "$3" "$2"; then
    echo "ok   $1: has /$3/"
  else
    echo "FAIL $1: no line matches /$3/ in $2:"
    grep '^SUMMARY' "$2" | sed 's/^/       /'
    FAILURES=$((FAILURES + 1))
  fi
}

lacks() { # case file pattern
  if grep -qE -- "$3" "$2"; then
    echo "FAIL $1: a line matches /$3/:"
    grep -E -- "$3" "$2" | sed 's/^/       /'
    FAILURES=$((FAILURES + 1))
  else
    echo "ok   $1: lacks /$3/"
  fi
}

# 1. Every run passes: exit 0, every spread ok.
canned_set "$WORK/pass"
perf pass "$WORK/pass" "$WORK/pass.txt"
expect pass "exit status" "$?" 0
has pass "$WORK/pass.txt" '^SUMMARY runs: 6 of 6 exit 0$'
has pass "$WORK/pass.txt" '^SUMMARY spread tier=Unicode refs=Fixture pane=Closed: .*gate\(p99<=16ms\)=ok$'
lacks pass "$WORK/pass.txt" 'FAIL|MISSING'

# 2. plain2 prints its p99 line under budget, then fails an assert after it
#    (rows on hand): the case review 04 found reported as `gate=ok`, exit 0.
canned_set "$WORK/late"
canned_run "$WORK/late" plain2 101 Closed Open
perf late "$WORK/late" "$WORK/late.txt"
expect late "exit status" "$?" 1
has late "$WORK/late.txt" '^SUMMARY runs: FAILED 1 of 6: plain2 exit 101$'
has late "$WORK/late.txt" '^SUMMARY spread tier=Unicode refs=Fixture pane=Closed: .*gate\(p99<=16ms\)=FAIL\(plain2 failed\)$'
lacks late "$WORK/late.txt" 'gate\(p99<=16ms\)=ok'

# 3. plain3 panics before its Open line: that spread has 2 cells, not 3.
canned_set "$WORK/early"
canned_run "$WORK/early" plain3 101 Closed
perf early "$WORK/early" "$WORK/early.txt"
expect early "exit status" "$?" 1
has early "$WORK/early.txt" '^SUMMARY runs: FAILED 1 of 6: plain3 exit 101$'
has early "$WORK/early.txt" 'pane=Open: .*gate\(p99<=16ms\)=FAIL\(plain3 failed\)$'

# 4. Every run exits 0 but plain1 has no Open line (say a filter left it
#    out): a spread short of 3 cells is MISSING, and the script fails.
canned_set "$WORK/short"
canned_run "$WORK/short" plain1 0 Closed
perf short "$WORK/short" "$WORK/short.txt"
expect short "exit status" "$?" 1
has short "$WORK/short.txt" '^SUMMARY runs: 6 of 6 exit 0$'
has short "$WORK/short.txt" 'pane=Open: .*gate\(p99<=16ms\)=MISSING\(2 of 3\)$'
has short "$WORK/short.txt" 'pane=Closed: .*gate\(p99<=16ms\)=ok$'

# 5. A control run fails: the spread is only plain runs, but the script fails.
canned_set "$WORK/control"
canned_run "$WORK/control" controlB 101 Closed Open
perf control "$WORK/control" "$WORK/control.txt"
expect control "exit status" "$?" 1
has control "$WORK/control.txt" '^SUMMARY runs: FAILED 1 of 6: controlB exit 101$'

# 6. A missing canned output is a failed run, not a skipped one.
canned_set "$WORK/gone"
rm "$WORK/gone/controlC.out"
perf gone "$WORK/gone" "$WORK/gone.txt"
expect gone "exit status" "$?" 1
has gone "$WORK/gone.txt" '^SUMMARY runs: FAILED 1 of 6: controlC exit '

# 7. A revision appended after a passing one carries the whole verdict.
cp "$WORK/pass.txt" "$WORK/revs.txt"
perf rev "$WORK/late" "$WORK/revs.txt"
expect rev "exit status" "$?" 1
has rev "$WORK/revs.txt" '^SUMMARY rev2 spread tier=Unicode refs=Fixture pane=Closed: .*gate\(p99<=16ms\)=FAIL\(plain2 failed\)$'

echo
if [ "$FAILURES" -gt 0 ]; then
  echo "m7-perf-key selftest: $FAILURES check(s) failed"
  exit 1
fi
echo "m7-perf-key selftest: every check held"
