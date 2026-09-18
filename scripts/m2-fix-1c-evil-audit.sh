#!/usr/bin/env bash
# How often the evil-merge probe fires, before and after the M6 fix.
#
# The old check called a merge evil whenever `git show --remerge-diff` named
# any file. The new one calls it evil only when a named file was changed by
# at most one side since the merge base, because a conflict — and so a hand
# resolution — needs both sides to have touched the path.
#
# This walks real merges in the git/git clone (scripts/clone-git-git.sh) with
# plain git, independently of Graft's Rust, and writes the counts to
# docs/perf/m2-review-fix-1c.txt. Never run from an agent's context: it
# spawns four git processes per merge.
#
# Usage: scripts/m2-fix-1c-evil-audit.sh [merge-count]
set -euo pipefail

count="${1:-60}"
root="$(cd "$(dirname "$0")/.." && pwd)"
repo="$("$root/scripts/clone-git-git.sh" --where)"
out="$root/docs/perf/m2-review-fix-1c.txt"
mkdir -p "$(dirname "$out")"

old_evil=0
new_evil=0
resolved=0
clean=0
not_checkable=0

merges="$(git -C "$repo" log --merges --max-parents=2 --min-parents=2 \
	--format=%H -n "$count" HEAD)"

for merge in $merges; do
	files="$(git -C "$repo" show --remerge-diff --format= --name-only "$merge" 2>/dev/null || true)"
	if [ -z "$files" ]; then
		clean=$((clean + 1))
		continue
	fi
	old_evil=$((old_evil + 1))

	p1="$(git -C "$repo" rev-parse "$merge^1")"
	p2="$(git -C "$repo" rev-parse "$merge^2")"
	base="$(git -C "$repo" merge-base "$p1" "$p2" 2>/dev/null || true)"
	if [ -z "$base" ]; then
		not_checkable=$((not_checkable + 1))
		continue
	fi
	both="$(comm -12 \
		<(git -C "$repo" diff-tree -r --no-renames --name-only "$base" "$p1" | sort) \
		<(git -C "$repo" diff-tree -r --no-renames --name-only "$base" "$p2" | sort))"
	invented="$(comm -23 <(printf '%s\n' "$files" | sort -u) <(printf '%s\n' "$both" | sort -u))"
	if [ -n "$invented" ]; then
		new_evil=$((new_evil + 1))
	else
		resolved=$((resolved + 1))
	fi
done

{
	echo "m2 review fix 1c — evil-merge probe, before and after (M6)"
	echo "repository: $repo"
	echo "merges walked (two-parent, newest first): $count"
	echo "date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
	echo
	echo "clean (empty remerge diff, unchanged by the fix): $clean"
	echo "old check: EVIL MERGE warnings                  : $old_evil"
	echo "new check: EVIL MERGE warnings                  : $new_evil"
	echo "new check: hand-resolved conflicts, no warning  : $resolved"
	echo "new check: not checkable (no merge base)        : $not_checkable"
} >"$out"

cat "$out"
