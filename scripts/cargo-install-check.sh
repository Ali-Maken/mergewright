#!/usr/bin/env bash
#
# README's "from a checkout" install, checked the way a newcomer meets it
# (m8-polish-and-ship/24): a fresh clone of the committed tree, an empty
# CARGO_HOME (no registry cache, no installed tools), then
# `cargo install --locked --path crates/mergewright` (package `mergewright`) and the
# installed binary, `mwr`, run.
# Everything lives in a temporary folder, removed on exit; nothing touches
# the real ~/.cargo.
#
# Summary lines: docs/release/cargo-install.txt (committed).
#
# Usage: scripts/cargo-install-check.sh
set -eu
cd "$(dirname "$0")/.." || exit 1

out="docs/release/cargo-install.txt"
mkdir -p docs/release
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
commit="$(git rev-parse --short HEAD)"
git clone -q --no-checkout . "$tmp/clone"
git -C "$tmp/clone" checkout -q --detach "$commit"

start="$(date +%s)"
# CARGO_TARGET_DIR unset: cargo install then builds in its own temporary
# folder, as it does for anyone who has not set one.
if (cd "$tmp/clone" && env -u CARGO_TARGET_DIR CARGO_HOME="$tmp/cargo-home" \
    cargo install --locked --path crates/mergewright >"$tmp/install.log" 2>&1); then
    verdict="ok"
else
    verdict="FAILED"
fi
took=$(($(date +%s) - start))

{
    echo "# cargo install --locked --path crates/mergewright — scripts/cargo-install-check.sh, $(date -u +%Y-%m-%d), commit $commit"
    echo "fresh clone, empty CARGO_HOME, $(cargo --version)"
    echo "install: $verdict in ${took}s"
    grep -E '^ *(Installed|Installing|error)' "$tmp/install.log" | sed "s|/private$tmp/||g; s|$tmp/||g; s/^ */    /"
    if [ "$verdict" = ok ]; then
        echo "installed: $(cd "$tmp/cargo-home/bin" && ls -1 | tr '\n' ' ')"
        echo "mwr --version: $("$tmp/cargo-home/bin/mwr" --version)"
    else
        tail -20 "$tmp/install.log" | sed "s|$tmp/||g; s/^/    /"
    fi
} >"$out"
cat "$out"
[ "$verdict" = ok ]
