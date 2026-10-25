#!/usr/bin/env bash
# pre-publish-cleanup/01: A/B the fast tier, interleaved A B A B, on one machine.
# Usage: scripts/pp-fast-tier-ab.sh <old-rev> <new-rev>
# Both revs are checked out under target/ (ignored, not scanned by published_repo),
# each with its own target dir. Appends to docs/perf/pp-01-fast-tier-ab.txt;
# stdout: the SUMMARY line only. Run alone: the numbers are wall times.
set -eu
here=$(cd "$(dirname "$0")/.." && pwd)
cd "$here"
old=${1:?old-rev}; new=${2:?new-rev}
ab=$here/target/pp-ab
out=docs/perf/pp-01-fast-tier-ab.txt
mkdir -p "$ab"
for side in old new; do
  rev=$old; [ $side = new ] && rev=$new
  [ -d "$ab/$side" ] || git worktree add --detach "$ab/$side" "$rev" >&2
done
run() { # <side> <n>
  FT_ROOT=$ab/$1 CARGO_TARGET_DIR=$ab/$1-target FT_OUT=$ab/runs.txt FT_JUNIT=$ab/$1-$2.xml \
    "$here/scripts/23-fast-tier.sh" "ab-$1-$2" 2>>"$ab/summaries.txt" || { echo "run $1 $2 failed" >&2; exit 1; }
}
: >"$ab/runs.txt"; : >"$ab/summaries.txt"
# warm both builds
for side in old new; do
  (cd "$ab/$side" && CARGO_TARGET_DIR=$ab/$side-target cargo nextest run --no-run >/dev/null 2>&1) || { echo "build $side failed" >&2; exit 1; }
done
run old 1; run new 1; run old 2; run new 2
{
  echo "pp-01 A/B: old=$(git rev-parse --short "$old") new=$(git rev-parse --short "$new"), order old new old new"
  cat "$ab/runs.txt"
  python3 -I scripts/pp-junit-diff.py "$ab/old-1.xml" "$ab/old-2.xml" "$ab/new-1.xml" "$ab/new-2.xml"
  grep -h '^SUMMARY' "$ab/summaries.txt"
} >>"$out"
python3 -I - "$ab/summaries.txt" "$out" <<'PY'
import sys, re
rows = {}
for l in open(sys.argv[1]):
    m = re.search(r'label=ab-(old|new)-(\d) .*run_wall_s=([\d.]+)', l)
    if m: rows[(m[1], m[2])] = float(m[3])
o = [rows[('old', '1')], rows[('old', '2')]]; n = [rows[('new', '1')], rows[('new', '2')]]
om, nm = sum(o) / 2, sum(n) / 2
aa = max(abs(o[0] - o[1]), abs(n[0] - n[1]))
txt = open(sys.argv[2]).read().splitlines()
tot = [l for l in txt if l.startswith('TOTALS')][-1]
f = dict(x.split('=') for x in tot.split()[1:])
line = (f"SUMMARY old_wall={om:.1f} new_wall={nm:.1f} delta_s={nm-om:.1f} grown_s={float(f['both_new'])-float(f['both_old']):.1f} "
        f"new_tests_s={f['new_only']} aa_spread_s={aa:.1f} (old {o[0]}/{o[1]}, new {n[0]}/{n[1]}) load=see run sections")
open(sys.argv[2], 'a').write(line + "\n")
print(line)
PY
