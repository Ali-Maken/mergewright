#!/usr/bin/env bash
# ease-pass 09 Cost: what the branch pane's read costs on the team fixture
# (scripts/41-scroll-team-fixture.sh's MONOREPO: 330 local, 900 remote-only
# branches). Release build of crates/mergewright-ops/examples/branch_pane_cost.rs.
# Writes docs/perf/branch-pane.txt (or $1). Summary lines:
#   branch-pane-cost: candidates=<n> local=<n> remote-only=<n>
#   branch-pane-cost: focus runs=10 p50_ms=… p95_ms=… spawns=<n>
#   branch-pane-cost: window rows=40 tips=<n> total_ms=… spawns=<n>
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
cat >"$WORK/shim/git" <<EOF
#!/bin/sh
echo spawn >>"$WORK/spawns.log"
exec "$REAL_GIT" "\$@"
EOF
chmod +x "$WORK/shim/git"

LOAD_BEFORE=$(uptime)
cargo run --release --quiet "${EXAMPLE[@]}" >"$WORK/times.out" 2>"$WORK/times.err"
status=$?
if [ "$status" -eq 0 ]; then
  PATH="$WORK/shim:$PATH" MWR_SPAWN_LOG="$WORK/spawns.log" \
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
  if [ "$status" -ne 0 ]; then
    echo "branch-pane-cost: FAILED (exit $status)"
    tail -20 "$WORK/times.err" "$WORK/spawns.err" 2>/dev/null
  fi
} 2>&1 | sed "s|$HOME|~|g" >"$OUT"
echo "wrote $OUT"
exit "$status"
