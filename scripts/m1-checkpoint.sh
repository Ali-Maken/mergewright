#!/usr/bin/env bash
#
# The M1 checkpoint: one command per scenario the user judges by eye.
#
# M1's promise (PROMPT.md §7.2, §9; CLAUDE.md) is that the shape of history
# is obvious at a glance. This script does not judge that — only a person
# can — it locates or builds the repository, prints what right looks like,
# and opens it in whatever terminal the user is sitting in.
#
#   scripts/m1-checkpoint.sh list          # the scenarios and what each shows
#   scripts/m1-checkpoint.sh git-git       # real, messy, merge-heavy history
#   scripts/m1-checkpoint.sh criss-cross   # two branches that merged each other
#   scripts/m1-checkpoint.sh foxtrot       # a merge whose first parent leaves main
#   scripts/m1-checkpoint.sh octopus       # one merge, four parents
#
# Every scenario but `list` ends by handing control to `cargo run -p mergewright --`,
# so `q` quits it the same way as any other Tributary session. `-p mergewright` is
# required at the workspace root now that `graft-fixtures` also builds a
# binary (its corpus-report `main.rs`) — bare `cargo run --` is ambiguous
# there (KNOWN_GAPS.md, m1-the-graph/28).
#
# A missing prerequisite (no network for git-git, a fixture that will not
# build) is an error, never a silent skip: this script exits non-zero and
# says exactly what is missing.

set -euo pipefail
cd "$(dirname "$0")/.."

usage() {
	cat >&2 <<USAGE
usage: $0 <git-git|criss-cross|foxtrot|octopus|list>
USAGE
}

require() {
	# require <command> <what it's for>
	if ! command -v "$1" >/dev/null 2>&1; then
		printf 'error: missing prerequisite: %s (%s)\n' "$1" "$2" >&2
		exit 1
	fi
}

open_shape() {
	# open_shape <fixture-shape-name> <what to look at, one line>
	local shape=$1 look=$2
	require cargo "builds and runs Tributary and the fixtures crate"

	printf 'building the fixture corpus (cargo run -p graft-fixtures)...\n' >&2
	if ! cargo run --quiet -p graft-fixtures >&2; then
		printf 'error: the fixture corpus could not be built — see the cargo output above.\n' >&2
		exit 1
	fi

	local path
	if ! path=$(cargo run --quiet -p graft-fixtures --example shape_path -- "$shape"); then
		printf 'error: fixture shape "%s" is not available on this machine — see the error above.\n' "$shape" >&2
		exit 1
	fi

	printf '\n%s\n  %s\n\nlook at: %s\n\n' "$shape" "$path" "$look" >&2
	printf 'running: cargo run -p mergewright -- %s\n\n' "$path" >&2
	exec cargo run -p mergewright -- "$path"
}

open_git_git() {
	require cargo "builds and runs Tributary"
	require git "clones and inspects git/git"

	printf 'locating or cloning git/git (scripts/clone-git-git.sh)...\n' >&2
	if ! scripts/clone-git-git.sh >&2; then
		cat >&2 <<MISSING

error: git/git could not be cloned. scripts/clone-git-git.sh failed above —
most likely no network reached github.com from this machine. This is a real
prerequisite for the git-git checkpoint, not something to skip: PROMPT.md
§7.2 names git/git as the human-proof corpus, so the checkpoint has nothing
honest to show without it. Get network access (or a pre-existing clone at
\$MWR_GIT_GIT_DIR / \${XDG_CACHE_HOME:-\$HOME/.cache}/tributary/git-git.git) and
run this again.
MISSING
		exit 1
	fi

	local path
	path=$(scripts/clone-git-git.sh --where)

	cat >&2 <<LOOK

git-git
  $path

look at: master is lane 0 and stays the leftmost column end to end (G to
the bottom and back); merge rows draw \`◉\` with a curve coming in from the
merged branch's own colour; lanes stay distinguishable at every width; at
80 columns the header ends \`+N lanes\` (N should be double digits on the
current clone) and no curve reaches the \`⋮\` rail. TESTING.md's "git-git"
gallery stop and its "Pixel/Unicode tier" pass conditions say the rest.

LOOK
	printf 'running: cargo run -p mergewright -- %s\n\n' "$path" >&2
	exec cargo run -p mergewright -- "$path"
}

list_scenarios() {
	cat <<LIST
git-git      real, messy, merge-heavy history (~85k commits, PROMPT.md §7.2's
             human-proof corpus). Needs a clone: scripts/clone-git-git.sh.
             Look at: default branch (master) stays a straight line; merges
             read as merges; lanes stay distinguishable; nothing shifts on
             resize.
criss-cross  two branches that merged each other, so their curves cross.
             Look at: the two merge curves actually cross, and each branch's
             own colour is traceable through the crossing.
foxtrot      a merge on main whose *first* parent is the feature branch, so
             first-parent view leaves main. Look at: the merge node sits in
             the feature's column, not lane 0; pressing 1 drops main's own
             commit and keeps the feature one — the opposite of an ordinary
             merge (stop 3/8 in the gallery).
octopus      one merge, four parents. Look at: one curve per parent, each in
             that branch's own colour, all landing on the same merge node.
LIST
}

case "${1:-}" in
git-git)
	open_git_git
	;;
criss-cross)
	open_shape "criss-cross-merge" "the two merge curves (alpha into beta, beta into alpha) actually cross, each staying in its own branch colour through the crossing"
	;;
foxtrot)
	open_shape "foxtrot-merge" "the merge node sits in feature's column, not lane 0 — main's own line curves INTO it; press 1: main's commit disappears, feature's stays (the opposite of an ordinary merge)"
	;;
octopus)
	open_shape "octopus-merge" "the top merge has three curves coming in, one per topic branch, each in its own colour, all landing on one node"
	;;
list)
	list_scenarios
	;;
*)
	usage
	exit 2
	;;
esac
