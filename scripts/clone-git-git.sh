#!/usr/bin/env bash
#
# Fetches the `git/git` repository for manual, large-repo testing.
#
# PROMPT.md §7.1 puts a real repository of roughly 80,000 commits in the
# corpus, and §7.2 wants the graph tried against one. It is not a fixture: it
# comes off the network, its contents change, and it costs hundreds of
# megabytes. So this is a script a person runs, never a test:
#
#   * no test calls it, and none ever should — a test that reaches the
#     network fails on a train;
#   * nothing runs it automatically, not on build and not on first use;
#   * it is idempotent: run it again and it fetches what is new.
#
# `--filter=tree:0` asks the server for commits and nothing else. Trees and
# blobs arrive only if something asks for them, which for reading history is
# never. That turns a ~250 MB clone into a small fraction of it, and it is
# also the honest shape to test against: Tributary reads history, so a history-only
# clone is the working set.
#
# Usage:
#   scripts/clone-git-git.sh            # clone or update
#   scripts/clone-git-git.sh --where    # print the path and exit
#
# Where it goes:
#   $TRIB_GIT_GIT_DIR, or ${XDG_CACHE_HOME:-$HOME/.cache}/tributary/git-git.git
#
# To delete it:
#   rm -rf "$(scripts/clone-git-git.sh --where)"
#
# It is outside the repository and outside `target/` on purpose: `cargo clean`
# must not throw away a clone that took minutes to fetch, and `git status`
# must not see it.

set -euo pipefail

UPSTREAM="https://github.com/git/git.git"
CACHE="${TRIB_GIT_GIT_DIR:-${XDG_CACHE_HOME:-$HOME/.cache}/tributary/git-git.git}"

if [ "${1:-}" = "--where" ]; then
	printf '%s\n' "$CACHE"
	exit 0
fi

if [ "${1:-}" != "" ]; then
	printf 'usage: %s [--where]\n' "$0" >&2
	exit 2
fi

if [ -d "$CACHE" ]; then
	printf 'updating %s\n' "$CACHE"
	git -C "$CACHE" fetch --quiet --filter=tree:0 --prune origin '+refs/heads/*:refs/heads/*'
else
	printf 'cloning %s into %s\n' "$UPSTREAM" "$CACHE"
	mkdir -p "$(dirname "$CACHE")"
	# A bare clone: there is no working tree to check out, and checking one
	# out would defeat `--filter=tree:0` by asking for every blob in HEAD.
	git clone --quiet --bare --filter=tree:0 "$UPSTREAM" "$CACHE"
fi

# A commit-graph makes a walk of 80,000 commits a different measurement
# (ADR-0008). Written here so that what is being measured is stated rather
# than inherited from whatever `git` happened to do in the background.
git -C "$CACHE" commit-graph write --reachable --quiet 2>/dev/null ||
	git -C "$CACHE" commit-graph write --reachable

commits="$(git -C "$CACHE" rev-list --count --all)"
refs="$(git -C "$CACHE" for-each-ref --format='%(refname)' | wc -l | tr -d ' ')"
size="$(du -sh "$CACHE" | cut -f1)"

cat <<SUMMARY

$CACHE
  commits  $commits
  refs     $refs
  on disk  $size

  Open it with:  cargo run -p mergewright -- $CACHE
  Delete it with: rm -rf $CACHE
SUMMARY
