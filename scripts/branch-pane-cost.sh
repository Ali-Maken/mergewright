#!/usr/bin/env bash
# ease-pass 09 Cost: what the branch pane's read costs on the team fixture
# (scripts/41-scroll-team-fixture.sh's MONOREPO: 330 local, 900 remote-only
# branches). Release build of crates/mergewright-ops/examples/branch_pane_cost.rs.
# Writes docs/perf/branch-pane.txt (or $1). Summary lines:
#   branch-pane-cost: candidates=<n> local=<n> remote-only=<n> trunk=<n> integration=<n> other=<n>
#   branch-pane-cost: focus runs=10 p50_ms=… p95_ms=… spawns=<n>
#   branch-pane-cost: window rows=40 tips=<n> total_ms=… spawns=<n>
#   branch-pane-cost: breakdown <subcommand>=<n> …      (one focus)
#   branch-pane-cost: scaling others=10 spawns=<a> others=500 spawns=<b>
#   branch-pane-cost: budget ok | budget OVER <which>   (exit 1 on OVER)
# Times come from a run without the spawn shim; spawn counts from a second
# run with a `git` shim on PATH that logs one line per spawn.
set -u
. "$(cd "$(dirname "$0")" && pwd)/lib/machine-state.sh"  # battery / Low Power Mode guard
cd "$(dirname "$0")/.." || exit 1
OUT=${1:-docs/perf/branch-pane.txt}
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
machine_state_begin "$OUT"
EXAMPLE=(-p mergewright-ops --features heavy-tests --example branch_pane_cost)

cargo build --release --quiet "${EXAMPLE[@]}" 2>"$WORK/build.err" || {
  echo "release build failed:" >&2
  tail -20 "$WORK/build.err" >&2
  exit 1
}

REAL_GIT=$(command -v git)
mkdir -p "$WORK/shim"
# One line per spawn: the subcommand (git's own leading options skipped),
# and for rev-list its first option too, so `--count` (the measure) reads
# apart from the walks. The example's breakdown line tallies these.
cat >"$WORK/shim/git" <<'EOF'
#!/bin/sh
skip=0 sub= next=
for a do
  if [ "$skip" = 1 ]; then skip=0; continue; fi
  if [ -z "$sub" ]; then
    case $a in -C|-c) skip=1 ;; -*) ;; *) sub=$a ;; esac
  else
    next=$a; break
  fi
done
case $sub in rev-list) key="$sub:$next" ;; *) key=$sub ;; esac
echo "$key" >>"$MWR_SPAWN_LOG"
exec "$MWR_REAL_GIT" "$@"
EOF
chmod +x "$WORK/shim/git"

LOAD_BEFORE=$(uptime)
cargo run --release --quiet "${EXAMPLE[@]}" >"$WORK/times.out" 2>"$WORK/times.err"
status=$?
if [ "$status" -eq 0 ]; then
  PATH="$WORK/shim:$PATH" MWR_REAL_GIT="$REAL_GIT" MWR_SPAWN_LOG="$WORK/spawns.log" \
    cargo run --release --quiet "${EXAMPLE[@]}" >"$WORK/spawns.out" 2>"$WORK/spawns.err"
  status=$?
fi
LOAD_AFTER=$(uptime)

{
  echo "# ease-pass 09 branch-pane cost — $(date -u +%FT%TZ)"
  echo "# HEAD $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- crates || echo ' +uncommitted')"
  echo "# $(sysctl -n machdep.cpu.brand_string 2>/dev/null || uname -m), $(sysctl -n hw.ncpu 2>/dev/null) cpus, $(uname -sr), $(git --version)"
  echo "# release, team fixture MONOREPO; times without the spawn shim, spawns with it"
  echo "# uptime before: $LOAD_BEFORE"
  echo "# uptime after:  $LOAD_AFTER"
  # Times from the plain run; the spawn counts from the shimmed one.
  grep 'candidates=' "$WORK/times.out"
  paste -d' ' \
    <(grep -E 'focus|window' "$WORK/times.out" | sed 's/ spawns=.*//') \
    <(grep -E 'focus|window' "$WORK/spawns.out" 2>/dev/null | grep -o 'spawns=.*')
  # Spawn-only lines: from the shimmed run.
  grep -E 'breakdown|scaling' "$WORK/spawns.out" 2>/dev/null
  if [ "$status" -ne 0 ]; then
    echo "branch-pane-cost: FAILED (exit $status)"
    tail -20 "$WORK/times.err" "$WORK/spawns.err" 2>/dev/null
  fi
} 2>&1 | sed "s|$HOME|~|g" >"$OUT"

# Budget (ease-pass 09p): focus p50 <= 1 s, p95 <= 2 s, <= 60 spawns; the
# window <= 1 s and <= 60 spawns per tip; scaling spawns equal at both
# sizes. A missing number counts as over.
if [ "$status" -eq 0 ]; then
  BUDGET=$(awk '
    function val(line, key,   m) {
      if (match(line, " " key "=[0-9.]+")) { m = substr(line, RSTART + length(key) + 2, RLENGTH - length(key) - 2); return m }
      return ""
    }
    / focus / { p50 = val($0, "p50_ms"); p95 = val($0, "p95_ms"); fs = val($0, "spawns") }
    / window / { tips = val($0, "tips"); wms = val($0, "total_ms"); ws = val($0, "spawns") }
    / scaling / { n = split($0, f, " spawns="); a = f[2] + 0; b = f[3] + 0; sa = (n == 3) }
    END {
      over = ""
      if (p50 == "" || p50 + 0 > 1000) over = over " focus-p50"
      if (p95 == "" || p95 + 0 > 2000) over = over " focus-p95"
      if (fs == "" || fs + 0 > 60) over = over " focus-spawns"
      if (tips == "" || wms == "" || wms + 0 > 1000 * tips) over = over " window-ms"
      if (tips == "" || ws == "" || ws + 0 > 60 * tips) over = over " window-spawns"
      if (!sa || a != b) over = over " scaling"
      print (over == "" ? "branch-pane-cost: budget ok" : "branch-pane-cost: budget OVER" over)
    }' "$OUT")
  echo "$BUDGET" >>"$OUT"
  echo "$BUDGET"
  case $BUDGET in *OVER*) status=1 ;; esac
fi
echo "wrote $OUT"
exit "$status"
