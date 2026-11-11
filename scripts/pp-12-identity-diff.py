#!/usr/bin/env python3
"""pre-publish-cleanup/12: judge every changed capture, snapshot and golden.

Usage: python3 -I scripts/pp-12-identity-diff.py [BASE]   (BASE default 183460a11)

Old text (BASE) gets the fixture identity swapped for the new one, then hex
runs of 7-40 chars are masked in both. A file is `equal` (only identity and
oids moved), `reordered` (same multiset of lines, other order: an oid
tie-break moved a row) or `other` (anything else: a stop). PNGs: dimensions
and bytes are compared and every changed one is listed. The lane-slot probe
files are diffed by (shape, subject#n) key. Writes
docs/perf/pp-12-identity-diff.txt and docs/shots/pp-12-identity-review.md.
"""
import re
import struct
import subprocess
import sys
from collections import Counter

BASE = sys.argv[1] if len(sys.argv) > 1 else "183460a11"
SWAPS = [
    ("Graft Fixture", "Maker Fixture"),
    ("Graft Test", "Maker Test"),
    ("Graft Perf", "Maker Perf"),
    ("@graft.invalid", "@maker.invalid"),
    ("graft.git", "maker.git"),
]
INITIALS = [("GF", "MF"), ("GT", "MT"), ("GP", "MP")]
HEX = re.compile(rb"[0-9a-f]{7,40}")


def git(*args, check=True):
    return subprocess.run(["git", *args], capture_output=True, check=check).stdout


def swap(data: bytes) -> bytes:
    for old, new in SWAPS:
        data = data.replace(old.encode(), new.encode())
    data = data.replace("Graf…".encode(), "Make…".encode())
    data = re.sub(rb"\bGraft\b", b"Maker", data)
    for old, new in INITIALS:
        data = re.sub(rb"\b" + old.encode() + rb"\b", new.encode(), data)
    return data


# The author column shows the first word of the name, cut to the column:
# "Graft" fits, "Fixture" becomes "Fixtu…". The cell's width moves with it, so
# on a line holding the cell, runs of spaces are collapsed on both sides.
AUTHOR = re.compile("(Graf…|Fixt…|Graft|Fixtu…|Fixture|Test|Perf)".encode())
AUTHOR_S = re.compile("(Graf…|Fixt…|Graft|Fixtu…|Fixture|Test|Perf)")
# A lane-slot letter run (a..l = palette slot 0..11, '.' = no lane), as the
# raster and graph-column snapshots print it. Approximate by design: the
# lane-slot probe is the real colour check.
LANES = re.compile(rb"(?<![\w.])(?=[0-9a-l]*\.)[0-9a-l.]{3,}(?!\w)")


def mask(data: bytes) -> bytes:
    data = HEX.sub(b"#", data)
    # An oid cut by the column: up to six hex digits, then the ellipsis.
    return re.sub(rb"(?<![\w])[0-9a-f]{1,6}(?=\xe2\x80\xa6)", b"#", data)


GLYPHS = re.compile("[│┃┊●◆◇■▸◉╮╯╰╭─━┬┴├┤┼║╫*|/\\\\ ]+")


def strip_glyphs(data: bytes) -> bytes:
    """Graph and cursor glyphs and spacing removed: what is left is the text."""
    return GLYPHS.sub(" ", data.decode("utf-8", "replace")).encode()


def author_norm(data: bytes) -> bytes:
    lines = []
    for line in data.decode("utf-8", "replace").split("\n"):
        if AUTHOR_S.search(line):
            line = AUTHOR_S.sub("<A>", line)
            line = re.sub(r" {2,}", " ", line)
            # A wider author cell moves where a long detail line is cut.
            line = re.sub(r"(<A>.{0,12})[^│]*…", r"\1…", line)
        lines.append(line)
    return "\n".join(lines).encode()


def lanes(data: bytes) -> bytes:
    return LANES.sub(b"<L>", data)


def png_size(data: bytes):
    return struct.unpack(">II", data[16:24]) if data[:8] == b"\x89PNG\r\n\x1a\n" else None


def classify(old: bytes, new: bytes):
    a, b = mask(swap(old)), mask(new)
    if a == b:
        return "equal", None
    a2, b2 = lanes(a), lanes(b)
    a3, b3 = mask(lanes(swap(old))), mask(lanes(new))
    if a3 == b3 and a3 != a2:
        a2, b2 = a3, b3
    if a2 == b2:
        first = next(i for i, (x, y) in enumerate(zip(a.split(b"\n"), b.split(b"\n"))) if x != y)
        return "lane-slot", (first + 1, a.split(b"\n")[first], b.split(b"\n")[first])
    if Counter(a2.split(b"\n")) == Counter(b2.split(b"\n")):
        a, b = a2, b2
    la, lb = a.split(b"\n"), b.split(b"\n")
    sa, sb = strip_glyphs(a).split(b"\n"), strip_glyphs(b).split(b"\n")
    if Counter(la) == Counter(lb) or Counter(sa) == Counter(sb):
        first = next(i for i, (x, y) in enumerate(zip(la, lb)) if x != y)
        return "reordered", (first + 1, la[first], lb[first])
    if len(la) == len(lb):
        first = next(i for i, (x, y) in enumerate(zip(la, lb)) if x != y)
    else:
        first = next((i for i, (x, y) in enumerate(zip(la, lb)) if x != y), min(len(la), len(lb)))
    x = la[first] if first < len(la) else b"<missing>"
    y = lb[first] if first < len(lb) else b"<missing>"
    return "other", (first + 1, x, y)


def dec(b):
    return b.decode("utf-8", "replace")


def slots(path):
    out = {}
    for line in swap(open(path, "rb").read()).decode("utf-8").splitlines(True):
        shape, key, lanes = line.rstrip("\n").split("\t")
        out[(shape, key)] = lanes
    return out


changed = git("diff", "--name-only", "-z", BASE).decode().split("\0")
changed = [p for p in changed if p]
wanted = [
    p for p in changed
    if p.startswith("docs/shots/") or p.endswith(".snap") or p.endswith(".golden")
]
counts = Counter()
report, reordered, others, pngs, lanefiles = [], [], [], [], []
for path in sorted(wanted):
    if path == "docs/shots/pp-12-identity-review.md" or "contact-sheet" in path:
        continue
    try:
        new = open(path, "rb").read()
    except FileNotFoundError:
        counts["removed"] += 1
        report.append(f"removed {path}")
        continue
    old = git("show", f"{BASE}:{path}", check=False)
    if path.endswith(".png"):
        counts["png"] += 1
        if old != new:
            pngs.append((path, png_size(old), png_size(new)))
        continue
    if not old:
        counts["added"] += 1
        report.append(f"added {path}")
        continue
    counts["compared"] += 1
    cls, detail = classify(old, new)
    counts[cls] += 1
    if cls != "equal":
        line, x, y = detail
        report.append(f"{cls} {path}:{line}\n    old: {dec(x)[:140]}\n    new: {dec(y)[:140]}")
        {"reordered": reordered, "lane-slot": lanefiles}.get(cls, others).append((path, line, dec(x), dec(y)))

before = slots("docs/perf/pp-12-lane-slots-before.txt")
after = slots("docs/perf/pp-12-lane-slots-after.txt")
lane_changes = [
    (k, before.get(k), after.get(k))
    for k in sorted(set(before) | set(after))
    if before.get(k) != after.get(k)
]
size_changes = [p for p in pngs if p[1] != p[2]]

summary = (
    f"SUMMARY compared={counts['compared']} equal={counts['equal']} "
    f"reordered={counts['reordered']} lane-slot-files={counts['lane-slot']} other={counts['other']} "
    f"png_changed={len(pngs)} png_size_changed={len(size_changes)} "
    f"added={counts['added']} removed={counts['removed']} "
    f"lane-slot changes={len(lane_changes)}"
)
with open("docs/perf/pp-12-identity-diff.txt", "w", encoding="utf-8") as f:
    f.write(f"base {BASE}\n")
    for line in report:
        f.write(line + "\n")
    for path, a, b in pngs:
        f.write(f"png {path} {a} -> {b}\n")
    for k, a, b in lane_changes:
        f.write(f"lane-slot {k[0]} {k[1]}: {a} -> {b}\n")
    f.write(summary + "\n")

with open("docs/shots/pp-12-identity-review.md", "w", encoding="utf-8") as f:
    f.write("# pp-12 identity rename: what moved, for the user's eye\n\n")
    f.write("Colour is unverified (a capture holds no colour). Lane-slot lines are the only colour evidence: a changed slot is a changed palette entry.\n\n")
    f.write(f"`{summary}`\n\n")
    f.write("## Reordered captures (same lines, other order)\n\n")
    f.write("Cause site: oid tie-break in `mergewright-git/src/history/node.rs` (equal commit time, ties on oid) or `mergewright/src/map/walk.rs:92` (branch order). Fixture commits sharing a timestamp are the inputs.\n\n")
    for path, line, x, y in reordered:
        f.write(f"- `{path}` line {line}\n  - was: `{x[:100]}`\n  - now: `{y[:100]}`\n")
    f.write("\n## Snapshot files whose lane-slot letters moved (approximate class)\n\n")
    for path, line, x, y in lanefiles:
        f.write(f"- `{path}` line {line}\n  - was: `{x[:100]}`\n  - now: `{y[:100]}`\n")
    f.write("\n## Lane-slot changes (probe, by corpus fixture row)\n\n")
    f.write("Cause site: `BranchIdentity::of_short_sha` (`mergewright-graph/src/identity.rs`) for an unnamed lane; named lanes hash the name and cannot move.\n\n")
    for k, a, b in lane_changes:
        f.write(f"- `{k[0]}` row `{k[1]}`: `{a}` -> `{b}`\n")
    f.write("\n## Changed PNGs\n\n")
    for path, a, b in pngs:
        f.write(f"- `{path}` {a} -> {b}\n")
    if not pngs:
        f.write("none\n")
    f.write("\n## Other (stop; causes)\n\n")
    f.write("Every entry here is a stop. Attributed by reading each pair (2026-11-11 run): the six `docs/shots/m6/*` files are rows that swapped place (two fixture commits with equal times: tie-break on oid, `mergewright-git/src/history/node.rs`), so they also show a moved cursor or lane glyph; `it__raster__raster-criss-cross-merge.snap` is lane-slot letters (unnamed lane, `of_short_sha`) hit by the hex mask; `m8-look-help-graph-187x50.txt` is the tail of the author word (`aft` from Graft, now the tail of Maker) showing past an overlay edge.\n\n")
    for path, line, x, y in others:
        f.write(f"- `{path}` line {line}\n")
    if not others:
        f.write("none\n")
print(summary)
