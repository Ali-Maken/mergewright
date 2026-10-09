#!/usr/bin/env bash
#
# Runs the fast tier on Linux, in a throwaway Docker container
# (m8-polish-and-ship/24). The answer to "does it pass on Linux?" (ADR-0005)
# without pushing anything to a CI service.
#
# What runs: exactly TESTING.md's fast tier, `cargo nextest run` and
# `cargo test --doc`, workspace-wide, on a clone of the committed tree
# (uncommitted edits are not in it). A clone, not an export: some tests open
# the checkout itself as a repository. Linux arch is the host's: arm64 on
# Apple silicon, so this is `aarch64-unknown-linux-gnu`, not x86_64.
#
# As a plain user, not root: several tests make a file unreadable and expect
# git to fail, which root would not.
#
# The image is the official `rust` image (Debian bookworm, its own git). The
# container is started with --rm and leaves nothing behind; the image stays
# in Docker's cache until `docker image rm` removes it.
#
# Where it goes:
#   full log:      ${XDG_CACHE_HOME:-~/.cache}/graft-linux/fast-tier.log
#   summary lines: docs/release/linux.txt (committed)
#
# Usage: scripts/linux-fast-tier.sh [image]     # default rust:1-bookworm
set -eu
cd "$(dirname "$0")/.." || exit 1

# The newest stable at the time of writing, not the MSRV: the lock file and
# the fast tier are what is under test here, not the MSRV promise.
image="${1:-rust:1-bookworm}"
cache="${XDG_CACHE_HOME:-$HOME/.cache}/graft-linux"
log="$cache/fast-tier.log"
out="docs/release/linux.txt"
mkdir -p "$cache" docs/release

src="$(mktemp -d)"
trap 'rm -rf "$src"' EXIT
commit="$(git rev-parse --short HEAD)"
git clone -q --no-checkout . "$src/tree"
git -C "$src/tree" checkout -q --detach "$commit"

# Inside: a user `tester` owns a copy of the tree; nextest comes from its
# project's own prebuilt download (get.nexte.st), as its install docs say.
inner='
set -eu
apt-get -qq update >/dev/null && apt-get -qq install -y tmux >/dev/null
useradd -m tester
cp -a /src /work && chown -R tester /work
runuser -u tester -- bash -c "
set -eu
export CARGO_HOME=/home/tester/.cargo PATH=/home/tester/.cargo/bin:\$PATH
mkdir -p /home/tester/.cargo/bin
# A git identity, as every developer machine has: tests that commit through
# the app read the global one (a first run without it failed on
# \"Author identity unknown\"; m8-polish-and-ship/24 NOTES).
git config --global user.name Tester
git config --global user.email tester@example.invalid
case \$(uname -m) in aarch64) arch=linux-arm ;; *) arch=linux ;; esac
curl -fsSL https://get.nexte.st/latest/\$arch | tar -xzf - -C /home/tester/.cargo/bin
cd /work
echo \"== uname: \$(uname -srm)\"
echo \"== git: \$(git --version)\"
echo \"== rustc: \$(rustc --version)\"
status=0
cargo nextest run --workspace --no-fail-fast --status-level pass || status=1
cargo test --doc --workspace || status=1
exit \$status
"
'

set +e
docker run --rm -v "$src/tree:/src:ro" "$image" bash -c "$inner" >"$log" 2>&1
status=$?
set -e

{
    echo "# Linux fast tier — scripts/linux-fast-tier.sh, $(date -u +%Y-%m-%dT%H:%MZ), commit $commit, image $image"
    grep -E '^== ' "$log" | sed 's/^== //'
    echo "exit: $status"
    grep -E '^ *Summary' "$log" | sed 's/^ */nextest: /'
    # Doc tests: one `test result:` line per crate; the totals say enough.
    grep -E '^test result:' "$log" |
        awk '{p+=$4; f+=$6} END {print "doctests: " p " passed, " f " failed (" NR " crates)"}'
    echo "failing tests:"
    # `|| true`: grep exits 1 when nothing failed, the good case; the exit
    # line above carries the run's real status.
    grep -E '^ *(FAIL|SIGABRT|SIGSEGV|TIMEOUT) ' "$log" | sed 's/^ */    /' | sort -u || true
    # The paths unverified on Linux since M4 (thread teardown, HungRemote's
    # failed wake): every test in a module that drives a HungRemote, counted.
    # Modules found by `grep -rl HungRemote crates`.
    hung='(m4_walk::|m8_walk::clone|partial_fetch::|remote_by_key::|clone::|remote::)'
    echo "HungRemote/teardown modules $hung: $(grep -cE "^ *PASS .*$hung" "$log" || true) passed, $(grep -cE "^ *(FAIL|SIGABRT|SIGSEGV|TIMEOUT) .*$hung" "$log" || true) failed"
} >"$out"

cat "$out"
exit "$status"
