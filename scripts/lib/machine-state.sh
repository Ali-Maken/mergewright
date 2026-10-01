#!/usr/bin/env bash
# Machine-state guard for release perf runs (m7 ticket 49).
#
# Why: a run on battery or in Low Power Mode is slower for reasons that have
# nothing to do with the code (47's g_after_idle miss: a CPU-only control ran
# 2.07x slower, docs/perf/47-debug.txt). Such a run is recorded, never judged.
#
# Use, from a script that runs a --release measurement, once OUT is known and
# after the script's own `trap ... EXIT` (the guard chains onto it):
#
#   . "$(dirname "$0")/lib/machine-state.sh"
#   machine_state_begin "$OUT"
#
# What it does:
#   - prints `SUMMARY machine ac=<1|0|unknown> lowpower=<1|0|unknown> os=<darwin|other>`
#   - on exit, puts that line first in what this run wrote to OUT: at the head
#     of the file if the run rewrote it, at the head of the appended block if
#     the run appended. No paths in the line (redaction guard).
#   - macOS with ac=0 or lowpower=1: prints `SUMMARY not_judged machine` and
#     turns a verdict exit (0 pass / 1 miss) into 3: neither pass nor miss,
#     "re-run on mains". Other exit codes (setup errors) pass through.
#   - ALLOW_LOW_POWER=1 (diagnosis runs only): prints `SUMMARY machine override`
#     and exits 3 all the same.
#   - Other systems: ac=unknown lowpower=unknown, judged as normal.
#   No retry, no sleep, no waiting for power.
#
# Dry run: MACHINE_STATE_DRY_RUN=<file> makes machine_state_begin write the
# line to <file> instead of OUT and exit at once, so a script's wiring can be
# checked without a release build.
#
# Self-test: bash scripts/lib/machine-state.sh --self-test
# Test seam: MACHINE_STATE_PMSET_BATT / MACHINE_STATE_PMSET_G name files that
# stand in for `pmset -g batt` / `pmset -g` output (forces os=darwin).

# Sets MS_AC, MS_LOWPOWER, MS_OS, MS_LINE.
machine_state_read() {
  local batt g
  if [ -n "${MACHINE_STATE_PMSET_BATT:-}" ]; then
    MS_OS=darwin
    batt=$(cat "$MACHINE_STATE_PMSET_BATT") || return 2
    g=$(cat "$MACHINE_STATE_PMSET_G") || return 2
  elif [ "$(uname -s)" = Darwin ]; then
    MS_OS=darwin
    batt=$(pmset -g batt) || return 2
    g=$(pmset -g) || return 2
  else
    MS_OS=other
  fi
  if [ "$MS_OS" = darwin ]; then
    case "$batt" in
      *"'AC Power'"*) MS_AC=1 ;;
      *"'Battery Power'"*) MS_AC=0 ;;
      *) MS_AC=unknown ;;
    esac
    MS_LOWPOWER=$(printf '%s\n' "$g" | awk '$1 == "lowpowermode" { print $2; exit }')
    case "$MS_LOWPOWER" in 0 | 1) ;; *) MS_LOWPOWER=unknown ;; esac
  else
    MS_AC=unknown
    MS_LOWPOWER=unknown
  fi
  MS_LINE="SUMMARY machine ac=$MS_AC lowpower=$MS_LOWPOWER os=$MS_OS"
}

# 0 when the run may be judged, 1 when it must not be.
machine_state_judgeable() {
  [ "$MS_OS" = darwin ] || return 0
  [ "$MS_AC" != 0 ] && [ "$MS_LOWPOWER" != 1 ]
}

machine_state_begin() {
  # Absolute, so a later `cd` in the script does not move it.
  case "$1" in /*) MS_OUT=$1 ;; *) MS_OUT="$PWD/$1" ;; esac
  machine_state_read || { echo "machine-state: pmset failed" >&2; exit 2; }
  echo "$MS_LINE"
  if [ -n "${MACHINE_STATE_DRY_RUN:-}" ]; then
    echo "$MS_LINE" >"$MACHINE_STATE_DRY_RUN"
    exit 0
  fi
  MS_SIZE0=0
  MS_HEAD0=
  if [ -f "$MS_OUT" ]; then
    MS_SIZE0=$(wc -c <"$MS_OUT" | tr -d ' ')
    MS_HEAD0=$(cksum <"$MS_OUT")
  fi
  # Chain onto the script's own EXIT trap, if any.
  # A guarded script run by another guarded script (m7-12-final-perf.sh runs
  # heap-slope.sh) records its line but leaves the verdict to the outer one,
  # whose exit-code contract the caller reads.
  if [ -n "${MACHINE_STATE_OUTER:-}" ]; then MS_NESTED=1; else MS_NESTED=; fi
  export MACHINE_STATE_OUTER=1
  local prev
  prev=$(trap -p EXIT | sed -e "s/^trap -- '//" -e "s/' EXIT\$//")
  MS_PREV_TRAP=$prev
  trap machine_state__exit EXIT
}

# Puts MS_LINE first in what this run wrote to MS_OUT.
machine_state__insert() {
  local tmp size
  mkdir -p "$(dirname "$MS_OUT")" || return 1
  if [ ! -f "$MS_OUT" ]; then
    echo "$MS_LINE" >"$MS_OUT"
    return
  fi
  tmp=$(mktemp) || return 1
  size=$(wc -c <"$MS_OUT" | tr -d ' ')
  if [ "$MS_SIZE0" -gt 0 ] && [ "$size" -ge "$MS_SIZE0" ] &&
    [ "$(head -c "$MS_SIZE0" "$MS_OUT" | cksum)" = "$MS_HEAD0" ]; then
    # Appended: the line heads the new block.
    { head -c "$MS_SIZE0" "$MS_OUT"; echo "$MS_LINE"; tail -c +"$((MS_SIZE0 + 1))" "$MS_OUT"; } >"$tmp"
  else
    # Rewritten (or new): the line heads the file.
    { echo "$MS_LINE"; cat "$MS_OUT"; } >"$tmp"
  fi
  cat "$tmp" >"$MS_OUT" && rm -f "$tmp"
}

machine_state__exit() {
  local rc=$?
  if [ -n "$MS_PREV_TRAP" ]; then eval "$MS_PREV_TRAP"; fi
  machine_state__insert || echo "machine-state: could not write the machine line to the perf file" >&2
  if [ -n "$MS_NESTED" ]; then
    :
  elif [ "${ALLOW_LOW_POWER:-0}" = 1 ]; then
    echo "SUMMARY machine override"
    [ "$rc" -le 1 ] && rc=3
  elif ! machine_state_judgeable; then
    echo "SUMMARY not_judged machine"
    [ "$rc" -le 1 ] && rc=3
  fi
  exit "$rc"
}

machine_state__self_test() {
  local dir case want got fail=0
  dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/machine-state-fixtures"
  for case in ac-lpm-off:0 battery:3 ac-lpm-on:3; do
    want=${case#*:}
    case=${case%%:*}
    MACHINE_STATE_PMSET_BATT="$dir/$case.batt" MACHINE_STATE_PMSET_G="$dir/$case.g" machine_state_read
    if machine_state_judgeable; then got=0; else got=3; fi
    echo "$case: $MS_LINE exit=$got (want $want)"
    [ "$got" = "$want" ] || fail=1
  done
  return "$fail"
}

if [ "${BASH_SOURCE[0]}" = "$0" ] && [ "${1:-}" = --self-test ]; then
  machine_state__self_test
  exit $?
fi
