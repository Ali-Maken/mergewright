#!/usr/bin/env bash
#
# Runs Tributary's git-version-sensitive tests against other gits than the one on
# this machine (m8-polish-and-ship/24).
#
# Tributary finds git on PATH and nowhere else: every spawn is `Command::new("git")`
# (graft-git/src/run.rs, the fixtures, the tests). So this script builds each
# git from its release tarball into a cache folder and puts that folder's
# `bin` first on PATH for the test run. Nothing else changes.
#
# Which tests: the ones whose answer depends on git's own behaviour.
#   * `rewrite_preview::` (graft-ops): the rebase preview must equal what git
#     does when it runs, checked by running git in a copy. Until this script,
#     checked on the machine's git only.
#   * `cherry_pick::git_alone_stops_on_a_pick_of_an_ancestor`: what plain git
#     does with an ancestor pick, the case Tributary refuses.
#
# Versions: the floor (`GitVersion::MINIMUM`, graft-git/src/version.rs), 2.40.0
# (below it the rebase preview's conflict forecast says "not checked" by
# design, and the forecast tests cannot pass), and the newest release on
# kernel.org when this was written. Pass others as
# arguments. A failure on the floor raises the floor to the oldest passing
# version (TESTING.md says so); it is never answered with a skipped test.
#
# Where it goes:
#   built gits and full test logs: ${XDG_CACHE_HOME:-~/.cache}/graft-git-matrix/
#   summary lines:                 docs/release/git-matrix.txt (committed)
#
# Needs: curl, tar with xz, make, a C compiler. Network only for a version not
# yet built. No test calls this; a person or a milestone review runs it.
#
# Usage:
#   scripts/git-matrix.sh                 # 2.38.0, 2.40.0 and 2.56.0
#   scripts/git-matrix.sh 2.40.1 2.56.0   # any versions
set -eu
cd "$(dirname "$0")/.." || exit 1

cache="${XDG_CACHE_HOME:-$HOME/.cache}/graft-git-matrix"
out="docs/release/git-matrix.txt"
mirror="https://mirrors.edge.kernel.org/pub/software/scm/git"
filter='test(/^rewrite_preview::/) | test(=cherry_pick::git_alone_stops_on_a_pick_of_an_ancestor)'

if [ "$#" -gt 0 ]; then
    versions="$*"
else
    versions="2.38.0 2.40.0 2.56.0"
fi

mkdir -p "$cache" docs/release

# Build one git into $cache/<version>/, once.
build_git() {
    local version="$1" prefix="$cache/$1"
    if [ -x "$prefix/bin/git" ]; then
        return 0
    fi
    local src="$cache/src"
    mkdir -p "$src"
    if [ ! -d "$src/git-$version" ]; then
        curl -fsSL "$mirror/git-$version.tar.xz" -o "$src/git-$version.tar.xz"
        tar -xJf "$src/git-$version.tar.xz" -C "$src"
    fi
    # Only what the tests touch: no translations, no Tcl GUIs, no Perl or
    # Python helpers, no HTTP transport (the tests never reach a network).
    # Without CARGO_TARGET_DIR: newer gits build a Rust library with cargo
    # and link it from their own `target/`; an inherited target dir sends it
    # elsewhere and the link fails.
    env -u CARGO_TARGET_DIR make -C "$src/git-$version" -j4 prefix="$prefix" \
        NO_GETTEXT=1 NO_TCLTK=1 NO_PERL=1 NO_PYTHON=1 NO_CURL=1 NO_EXPAT=1 \
        NO_INSTALL_HARDLINKS=1 install >"$cache/build-$version.log" 2>&1
}

# Compile the test binary once, with the machine's git, so the run under a
# swapped PATH only runs tests.
cargo nextest run --workspace --no-run >"$cache/compile.log" 2>&1

{
    echo "# git matrix — scripts/git-matrix.sh, $(date -u +%Y-%m-%dT%H:%MZ)"
    echo "# host: $(uname -sm); filter: $filter"
} >"$out"

status=0
for version in $versions; do
    if ! build_git "$version"; then
        echo "git $version: BUILD FAILED (log: graft-git-matrix/build-$version.log)" >>"$out"
        status=1
        continue
    fi
    seen="$(PATH="$cache/$version/bin:$PATH" git --version)"
    log="$cache/test-$version.log"
    if PATH="$cache/$version/bin:$PATH" cargo nextest run --workspace \
        --no-fail-fast -E "$filter" >"$log" 2>&1; then
        verdict="ok"
    else
        verdict="FAILED"
        status=1
    fi
    summary="$(grep -E '^ *Summary' "$log" | tail -1 | sed 's/^ *//')"
    echo "git $version ($seen on PATH): $verdict — $summary" >>"$out"
    # Name each failing test, so the summary file alone says what broke.
    # `|| true`: grep exits 1 when nothing failed, which is the good case;
    # the verdict above already carries the run's real exit status.
    grep -E '^ *(FAIL|SIGABRT|TIMEOUT) ' "$log" | sed 's/^ */    /' | sort -u >>"$out" || true
done

cat "$out"
exit "$status"
