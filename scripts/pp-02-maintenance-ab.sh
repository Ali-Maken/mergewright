#!/usr/bin/env bash
# pre-publish-cleanup/02: the fork_rows fixture in plain git, A/B on auto
# maintenance. Usage: scripts/pp-02-maintenance-ab.sh <runs per loop> [loops]
# Arm on: git as the fixture runner ran it before the fix (auto maintenance
# on). Arm off: `maintenance.auto=false`, as the runner runs it now. Each arm
# runs <loops> (default 4) loops in parallel, each building the fixture
# <runs> times in a fresh temp dir. Appends both arms' SUMMARY lines and the
# first failure's git stderr to docs/perf/pp-02-fork-rows.txt; prints them.
set -u
cd "$(dirname "$0")/.."
runs=$1 loops=${2:-4} out=${PP02_OUT:-docs/perf/pp-02-fork-rows.txt}
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

export GIT_TERMINAL_PROMPT=0 GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null \
  GIT_CONFIG_SYSTEM=/dev/null GIT_ATTR_NOSYSTEM=1 LC_ALL=C \
  GIT_AUTHOR_NAME=t GIT_AUTHOR_EMAIL=t@t GIT_COMMITTER_NAME=t GIT_COMMITTER_EMAIL=t@t

# One loop: build `two_forks_repo` (fork_rows.rs) <runs> times; print fails.
loop() {
  local log=$1 fails=0 d ok
  g() { local h=$1; shift
    GIT_AUTHOR_DATE="2020-01-01T0$h:00:00+00:00" GIT_COMMITTER_DATE="2020-01-01T0$h:00:00+00:00" \
      git -C "$d" "$@" 2>>"$log" || { echo "FAILED: git $*" >>"$log"; return 1; }; }
  step() { printf "$3" >"$d/$2" && g "$1" add -A && g "$1" commit -q -m "$4"; }
  for _ in $(seq 1 "$runs"); do
    d=$(mktemp -d); ok=1
    g 0 init -q -b main || ok=0
    step 0 readme.txt 'm1\n' m1 || ok=0
    g 1 switch -q -c feature-a || ok=0
    step 1 one.txt 'part 1\n' p1 || ok=0
    step 2 two.txt 'part 2\n' p2 || ok=0
    step 3 three.txt 'part 3\n' p3 || ok=0
    g 4 switch -q -c feature-b feature-a~2 || ok=0
    step 4 b.txt 'b1\n' b1 || ok=0
    g 5 switch -q -c feature-c feature-a~1 || ok=0
    step 5 one.txt 'part 1, by c\n' c1 || ok=0
    rm -rf "$d"
    [ "$ok" = 0 ] && fails=$((fails + 1))
  done
  echo "$fails"
}

arm() {
  local name=$1 total=0 k
  for k in $(seq 1 "$loops"); do loop "$work/$name-$k.log" >"$work/$name-$k.n" & done
  wait
  for k in $(seq 1 "$loops"); do total=$((total + $(cat "$work/$name-$k.n"))); done
  echo "SUMMARY maintenance-ab arm=$name fixtures=$((runs * loops)) fails=$total"
  if [ "$total" -gt 0 ]; then
    cat "$work/$name"-*.log | grep -m4 -E 'error|fatal|FAILED' | sed -e 's#/var/folders/[^ ]*#<tmp>#g'
  fi
}

{
  echo "== maintenance A/B (plain git, $loops loops in parallel) =="
  echo "load: $(uptime | sed 's/.*load/load/')"
  arm on
  GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=maintenance.auto GIT_CONFIG_VALUE_0=false arm off
} | tee -a "$out"
