#!/usr/bin/env bash
# ease-pass/23: measure the fast tier. Usage: scripts/23-fast-tier.sh <label>
# Appends a section to docs/perf/23-fast-tier.txt; prints one SUMMARY line.
set -u
label=${1:?label}
cd "$(dirname "$0")/.."
out=docs/perf/23-fast-tier.txt
tdir=${CARGO_TARGET_DIR:-target}
load() { uptime | sed 's/.*load averages*: //'; }
now() { python3 -I -c 'import time;print(time.time())'; }
l0=$(load)
t0=$(now); cargo nextest run --no-run >/dev/null 2>&1; t1=$(now)
cargo nextest run --no-fail-fast >"$tdir/23-run.log" 2>&1; t2=$(now)
l1=$(load)
build_s=$(python3 -I -c "print(round($t1-$t0,1))")
run_s=$(python3 -I -c "print(round($t2-$t1,1))")
grep -E 'SLOW' "$tdir/23-run.log" >"$tdir/23-slow.log"
# nextest keeps its store under the workspace `target/`, which is not always
# $CARGO_TARGET_DIR; take the junit file this run wrote.
junit=$(ls -t "$tdir/nextest/default/junit.xml" target/nextest/default/junit.xml 2>/dev/null | head -1)
python3 -I - "$junit" "$label" "$build_s" "$run_s" "$l0" "$l1" "$(git rev-parse --short HEAD)" "$(cargo nextest --version | head -1)" "$(date +%F)" "$tdir/23-slow.log" >>"$out" <<'PY'
import sys, xml.etree.ElementTree as ET, collections
p, label, b, r, l0, l1, rev, ver, date, slowf = sys.argv[1:]
root = ET.parse(p).getroot()
cases = []
per = collections.defaultdict(float)
for ts in root.iter('testsuite'):
    for tc in ts.iter('testcase'):
        t = float(tc.get('time', 0)); cases.append((t, ts.get('name'), tc.get('name')))
        per[ts.get('name')] += t
tot = sum(c[0] for c in cases)
over = sum(1 for c in cases if c[0] > 10)
print(f"== {label} ==")
print(f"date {date}  rev {rev}  {ver}")
print(f"load before: {l0}   after: {l1}")
print(f"build_s {b}  run_wall_s {r}  tests {len(cases)}  sum_test_s {tot:.1f}  slow_over_10s {over}")
print("per-binary totals (s):")
for k, v in sorted(per.items(), key=lambda x: -x[1]): print(f"  {v:8.1f}  {k}")
print("slowest 20:")
for t, s, n in sorted(cases, reverse=True)[:20]: print(f"  {t:7.2f}  {s}  {n}")
slow = [l.rstrip() for l in open(slowf)]
print("SLOW lines:", len(slow))
for l in slow[:40]: print("  " + l)
print()
sys.stderr.write(f"SUMMARY label={label} tests={len(cases)} run_wall_s={r} build_s={b} sum_test_s={tot:.1f} slow_over_10s={over}\n")
PY
