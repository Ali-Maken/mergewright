"""pre-publish-cleanup/01: diff the junit files of an A/B fast-tier run.
Usage: pp-junit-diff.py <old1> <old2> <new1> <new2>   (junit paths; mean per test over the two runs)"""
import sys, xml.etree.ElementTree as ET, collections


def load(p):
    d = {}
    for ts in ET.parse(p).getroot().iter('testsuite'):
        for tc in ts.iter('testcase'):
            d[(ts.get('name'), tc.get('name'))] = float(tc.get('time', 0))
    return d


def mean(ps):
    ds = [load(p) for p in ps]
    keys = set().union(*ds)
    return {k: sum(d[k] for d in ds if k in d) / sum(1 for d in ds if k in d) for k in keys}


old, new = mean(sys.argv[1:3]), mean(sys.argv[3:5])
both = [k for k in new if k in old]
bin_o, bin_n = collections.defaultdict(float), collections.defaultdict(float)
for k, v in old.items(): bin_o[k[0]] += v
for k, v in new.items(): bin_n[k[0]] += v
print("== per binary (mean s over 2 runs): old, new, delta ==")
for b in sorted(set(bin_o) | set(bin_n), key=lambda b: -(bin_n[b] - bin_o[b])):
    print(f"  {bin_o[b]:8.1f} {bin_n[b]:8.1f} {bin_n[b]-bin_o[b]:+8.1f}  {b}")
print("== top 30 per-test growth (in both): old, new, delta ==")
for k in sorted(both, key=lambda k: -(new[k] - old[k]))[:30]:
    print(f"  {old[k]:7.2f} {new[k]:7.2f} {new[k]-old[k]:+7.2f}  {k[0]} {k[1]}")
only = [k for k in new if k not in old]
print(f"== top 30 tests only in new, by time ({len(only)} such, sum {sum(new[k] for k in only):.1f} s) ==")
for k in sorted(only, key=lambda k: -new[k])[:30]:
    print(f"  {new[k]:7.2f}  {k[0]} {k[1]}")
gone = [k for k in old if k not in new]
print(f"== tests gone: {len(gone)} (sum {sum(old[k] for k in gone):.1f} s) ==")
for k in gone[:10]: print(f"  {old[k]:7.2f}  {k[0]} {k[1]}")
print(f"TOTALS both_old={sum(old[k] for k in both):.1f} both_new={sum(new[k] for k in both):.1f} "
      f"new_only={sum(new[k] for k in only):.1f} gone={sum(old[k] for k in gone):.1f}")
