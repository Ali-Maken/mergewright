#!/usr/bin/env python3
"""Which capture changes are only the name, and which moved anything else.

    scripts/shot-drift.py --base <rev> --out <file>

For each text file under docs/shots/ changed since <rev> (working tree
included), the changed lines are paired: difflib's `replace` blocks of equal
size, line by line. A pair is *name-only* when the old line says graft (any
case) — the new name may move the rest of the line along with it. Anything
else (an unpaired line, or an old line without the word) is *drift*,
written as `path:line` (the new file's line). PNGs are listed apart: they
cannot be paired by line. Last line:
`changed <n> · name-only <m> · drift-files <k> · png <p>`.
"""

import argparse
import difflib
import re
import subprocess
import sys

OLD = re.compile(r"(?i)graft")


def git(*args):
    return subprocess.run(["git", *args], check=True, capture_output=True, text=True).stdout


def old_text(rev, path):
    shown = subprocess.run(["git", "show", f"{rev}:{path}"], capture_output=True)
    if shown.returncode != 0:
        return None  # new file
    return shown.stdout.decode("utf-8", "replace")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--base", required=True)
    parser.add_argument("--out", required=True)
    args = parser.parse_args()

    changed = git("diff", "--name-only", args.base, "--", "docs/shots/").split()
    untracked = git("ls-files", "--others", "--exclude-standard", "--", "docs/shots/").split()
    paths = sorted(set(changed) | set(untracked))

    lines, pngs, drift_files = [], [], set()
    name_only = 0
    for path in paths:
        if path.endswith(".png"):
            pngs.append(path)
            continue
        try:
            with open(path, encoding="utf-8", errors="replace") as handle:
                new = handle.read().splitlines()
        except FileNotFoundError:
            lines.append(f"{path}: deleted")
            drift_files.add(path)
            continue
        before = old_text(args.base, path)
        old = before.splitlines() if before is not None else []
        matcher = difflib.SequenceMatcher(a=old, b=new, autojunk=False)
        for tag, a0, a1, b0, b1 in matcher.get_opcodes():
            if tag == "equal":
                continue
            if tag == "replace" and a1 - a0 == b1 - b0:
                for offset in range(a1 - a0):
                    if OLD.search(old[a0 + offset]):
                        name_only += 1
                    else:
                        lines.append(f"{path}:{b0 + offset + 1}")
                        drift_files.add(path)
            else:
                for at in range(b0, b1) if b1 > b0 else [b0]:
                    lines.append(f"{path}:{at + 1}")
                drift_files.add(path)

    summary = (
        f"changed {len(paths)} · name-only {name_only} · "
        f"drift-files {len(drift_files)} · png {len(pngs)}"
    )
    with open(args.out, "w", encoding="utf-8") as out:
        for line in lines:
            out.write(line + "\n")
        for png in pngs:
            out.write(f"png {png}\n")
        out.write(summary + "\n")
    print(summary)
    return 0


if __name__ == "__main__":
    sys.exit(main())
