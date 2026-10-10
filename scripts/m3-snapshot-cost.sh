#!/usr/bin/env bash
# What one backup snapshot costs on a big, untracked-heavy tree
# (KNOWN_GAPS R2-3, hard choice 3 in the M3 design note).
#
# Never run inside a helper's context: it builds a large repository and
# writes its numbers to docs/perf/m3-snapshot.txt, which is what a reviewer
# reads. Run it at the milestone review.
#
#   scripts/m3-snapshot-cost.sh [tracked-files] [untracked-files]
set -euo pipefail

TRACKED=${1:-20000}
UNTRACKED=${2:-2000}
OUT="$(cd "$(dirname "$0")/.." && pwd)/docs/perf/m3-snapshot.txt"
WORK=$(mktemp -d)
# git makes its loose objects read-only, so a plain rm -rf leaves the tree behind
# on macOS and the script exits non-zero after a good run.
trap 'chmod -R u+w "$WORK" 2>/dev/null; rm -rf "$WORK"' EXIT

export GIT_CONFIG_GLOBAL=/dev/null
export GIT_CONFIG_SYSTEM=/dev/null
export GIT_AUTHOR_NAME=tributary GIT_AUTHOR_EMAIL=tributary@backup
export GIT_COMMITTER_NAME=tributary GIT_COMMITTER_EMAIL=tributary@backup

REPO="$WORK/repo"
SCRATCH="$WORK/scratch"
mkdir -p "$REPO" "$SCRATCH"
git -C "$REPO" init -q -b main .

echo "building $TRACKED tracked files…" >&2
mkdir -p "$REPO/src"
for i in $(seq 1 "$TRACKED"); do
  printf 'line one of file %s\nline two\n' "$i" > "$REPO/src/f$i.txt"
done
git -C "$REPO" add -A >/dev/null
git -C "$REPO" commit -qm "the tracked tree"

echo "adding $UNTRACKED untracked files…" >&2
mkdir -p "$REPO/scratchpad"
for i in $(seq 1 "$UNTRACKED"); do
  printf 'untracked %s\n' "$i" > "$REPO/scratchpad/u$i.txt"
done
# One stat-dirty tracked file, the ordinary case before a stage.
printf 'edited\n' > "$REPO/src/f1.txt"

milli() { python3 -c 'import time;print(int(time.time()*1000))'; }

run_snapshot() {
  local label=$1 index="$SCRATCH/idx-$1"
  rm -f "$index"
  local start end
  start=$(milli)
  cp "$REPO/.git/index" "$index"
  GIT_INDEX_FILE="$index" git -C "$REPO" write-tree >/dev/null
  GIT_INDEX_FILE="$index" git -C "$REPO" --literal-pathspecs add -A >/dev/null
  local worktree
  worktree=$(GIT_INDEX_FILE="$index" git -C "$REPO" write-tree)
  end=$(milli)
  echo "$label: $((end - start)) ms (worktree tree $worktree)"
}

{
  echo "# snapshot cost — $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "# $(git --version), $(uname -s) $(uname -m)"
  echo "# $TRACKED tracked files, $UNTRACKED untracked, 1 stat-dirty"
  echo
  run_snapshot cold
  run_snapshot warm-1
  run_snapshot warm-2
  echo
  echo "# for comparison, the route the design rejected: a fresh index"
  start=$(milli)
  rm -f "$SCRATCH/fresh"
  GIT_INDEX_FILE="$SCRATCH/fresh" git -C "$REPO" read-tree HEAD
  GIT_INDEX_FILE="$SCRATCH/fresh" git -C "$REPO" --literal-pathspecs add -A >/dev/null
  GIT_INDEX_FILE="$SCRATCH/fresh" git -C "$REPO" write-tree >/dev/null
  end=$(milli)
  echo "fresh read-tree: $((end - start)) ms"
} > "$OUT"

echo "wrote $OUT" >&2
cat "$OUT" >&2
