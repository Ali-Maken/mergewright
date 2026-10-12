#!/usr/bin/env bash
# How often the evil-merge probe fires, before and after the M6 fix.
#
# The old check called a merge evil whenever `git show --remerge-diff` named
# any file. The new one calls it evil only when a named file was changed by
# at most one side since the merge base, because a conflict — and so a hand
# resolution — needs both sides to have touched the path.
#
# The rule below mirrors `split_remerge_files` in graft-ops and must be kept
# in step with it. Since review round 1e that includes renames: each side is
# read with `-M --name-status`, both ends of a rename count as touched, and a
# rename carries "both sides changed it" from the old name to the new one.
# Round 1f: a copy (`C`) does not. A rename is one file under a new name; a
# copy is a new file, and carrying the property across one exempted every file
# copied from a file both sides had changed. Paths are read from
# tab-separated `--name-status` rather than `-z`,
# which is safe for git/git's own paths (no tabs, no newlines).
#
# This walks real merges in the git/git clone (scripts/clone-git-git.sh) with
# plain git, independently of Mergewright's Rust, and writes the counts to
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

# One side of a merge, as `C<path>` lines (every path it touched, both ends
# of a rename included) and `R<old><TAB><new>` lines (its renames only).
# A copy's source is not touched by the copy, and its pair never carries.
side_changes() {
	git -C "$repo" -c core.quotePath=false \
		diff-tree -r -M --name-status "$1" "$2" |
		awk -F'\t' '
			$1 ~ /^R/ { print "C" $2; print "C" $3; print "R" $2 "\t" $3; next }
			$1 ~ /^C/ { print "C" $3; next }
			{ print "C" $2 }'
}

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
	ours="$(side_changes "$base" "$p1")"
	theirs="$(side_changes "$base" "$p2")"
	both="$(comm -12 \
		<(printf '%s\n' "$ours" | sed -n 's/^C//p' | sort -u) \
		<(printf '%s\n' "$theirs" | sed -n 's/^C//p' | sort -u))"
	while IFS="$(printf '\t')" read -r old new; do
		[ -n "$old" ] || continue
		if printf '%s\n' "$both" | grep -Fxq -- "$old"; then
			both="$both
$new"
		fi
	done < <(printf '%s\n%s\n' "$ours" "$theirs" | sed -n 's/^R//p')
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
