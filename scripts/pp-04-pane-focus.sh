#!/usr/bin/env bash
# pre-publish-cleanup/04 Part A cost: git spawns and settle time per branch-pane
# focus change, through the app path (the update loop, debug test build), on
# the main screen's stacked repository (graph_rows_tests::stacked_focus).
# A `git` shim on PATH logs one line per spawn; the test writes a marker per
# `j`. Writes docs/perf/pp-04-pane-focus.txt (or $1). Summary line:
#   SUMMARY pane-focus changes=<n> spawns_max=<n> spawns_p50=<n> p50_ms=… p95_ms=… budget ok|OVER
set -u
cd "$(dirname "$0")/.." || exit 1
OUT=${1:-docs/perf/pp-04-pane-focus.txt}
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
REAL_GIT=$(command -v git)
mkdir -p "$WORK/shim"
cat >"$WORK/shim/git" <<'EOF'
#!/bin/sh
echo "$*" >>"$MWR_SPAWN_LOG"
exec "$MWR_REAL_GIT" "$@"
EOF
chmod +x "$WORK/shim/git"
cargo nextest run -p mergewright --test it stacked_focus --no-capture >/dev/null 2>"$WORK/build.err" || {
  echo "warm build failed" >&2; tail -20 "$WORK/build.err" >&2; exit 1; }
PATH="$WORK/shim:$PATH" MWR_REAL_GIT="$REAL_GIT" MWR_SPAWN_LOG="$WORK/spawns.log" \
  MWR_FOCUS_TIMES="$WORK/times.log" \
  cargo nextest run -p mergewright --test it stacked_focus >"$WORK/run.out" 2>&1
status=$?
{
  echo "# pre-publish-cleanup/04 pane focus cost — $(date -u +%FT%TZ)"
  echo "# HEAD $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- crates || echo ' +uncommitted')"
  echo "# $(sysctl -n machdep.cpu.brand_string 2>/dev/null || uname -m), $(uname -sr), $(git --version)"
  echo "# debug test build, stacked repo (7 rows); spawns between focus markers, step 0 = open"
  awk '/^# focus/ { if (n) print step, c; step=$3; c=0; n=1; next } n { c++ } END { if (n) print step, c }' \
    "$WORK/spawns.log" | sed 's/^/focus-step /'
  echo "settle-ms $(tr '\n' ' ' <"$WORK/times.log")"
  awk '/^# focus/ { if (n && step > 0) print c; step=$3; c=0; n=1; next } n { c++ } END { if (n && step > 0) print c }' \
    "$WORK/spawns.log" | sort -n >"$WORK/per"
  sed 1d "$WORK/times.log" | sort -n >"$WORK/ms"
  awk -v status="$status" 'FNR==NR { s[++ns]=$1; next } { m[++nm]=$1 }
    END {
      p95 = int(0.95 * nm + 0.999); if (p95 < 1) p95 = 1
      over = (status != 0 || ns == 0 || s[ns] > 60)
      printf "SUMMARY pane-focus changes=%d spawns_max=%d spawns_p50=%d p50_ms=%s p95_ms=%s budget %s\n",
        ns, s[ns], s[int((ns + 1) / 2)], m[int((nm + 1) / 2)], m[p95], (over ? "OVER" : "ok")
    }' "$WORK/per" "$WORK/ms"
} >"$OUT"
grep SUMMARY "$OUT"
exit "$status"
