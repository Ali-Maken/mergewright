#!/usr/bin/env bash
# ease-pass/24: opt-level probe. Cold = fresh target dir ($1), warm = rebuild after touching
# a ui source file. Appends BUILD lines to docs/perf/24-hints-fitting.txt.
# Usage: scripts/24-opt-probe.sh <label> <fresh-target-dir>
set -u
label=${1:?}; fresh=${2:?}
cd "$(dirname "$0")/.."
out=docs/perf/24-hints-fitting.txt
now() { python3 -I -c 'import time;print(time.time())'; }
t0=$(now); CARGO_TARGET_DIR=$fresh cargo nextest run --no-run >/dev/null 2>&1; t1=$(now)
echo "BUILD $label cold_build_s $(python3 -I -c "print(round($t1-$t0,1))")  load $(uptime | sed 's/.*averages*: //')" >>$out
touch crates/mergewright-ui/src/lib.rs
t0=$(now); cargo nextest run --no-run >/dev/null 2>&1; t1=$(now)
echo "BUILD $label warm_build_s (touch ui lib.rs) $(python3 -I -c "print(round($t1-$t0,1))")" >>$out
