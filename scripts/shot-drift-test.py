"""m8/56: shot-drift.py counts a rename from any earlier name as name-only.

Builds a throwaway repo with one capture per earlier name (graft, Tributary,
trib) plus one plain line, renames each to the new name and changes the plain
line, runs shot-drift.py and asserts `name-only 3 · drift-files 1`. Prints
`shot-drift-test: ok` or exits 1. Run by scripts/check-scripts.sh.
"""

import os
import subprocess
import sys
import tempfile

SCRIPT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "shot-drift.py")
OLD = {"a.txt": "title graft x\n", "b.txt": "Tributary  title\n", "c.txt": "$ trib --help\n", "d.txt": "row one\n"}
NEW = {"a.txt": "title mwr x\n", "b.txt": "Mergewright  title\n", "c.txt": "$ mwr --help\n", "d.txt": "row two\n"}
WANT = "changed 4 · name-only 3 · drift-files 1 · png 0"


def write_all(shots, files):
    for name, text in files.items():
        with open(os.path.join(shots, name), "w", encoding="utf-8") as handle:
            handle.write(text)


def main():
    with tempfile.TemporaryDirectory() as repo:
        git = ["git", "-C", repo, "-c", "user.name=t", "-c", "user.email=t@t"]
        subprocess.run(git + ["init", "-q"], check=True)
        shots = os.path.join(repo, "docs", "shots")
        os.makedirs(shots)
        write_all(shots, OLD)
        subprocess.run(git + ["add", "."], check=True)
        subprocess.run(git + ["commit", "-qm", "base"], check=True)
        write_all(shots, NEW)
        out = os.path.join(repo, "drift.txt")
        got = subprocess.run(
            [sys.executable, SCRIPT, "--base", "HEAD", "--out", out],
            cwd=repo, check=True, capture_output=True, text=True,
        ).stdout.strip()
    if got != WANT:
        print(f"shot-drift-test: FAIL want {WANT!r} got {got!r}")
        return 1
    print("shot-drift-test: ok")
    return 0


if __name__ == "__main__":
    sys.exit(main())
