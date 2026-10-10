#!/usr/bin/env bash
#
# The release, made locally and published nowhere (m8-polish-and-ship/24,
# ADR-0057 §1): `dist plan`, then `dist build` for this machine's target,
# then the checks a person can make on what came out — the archive's
# contents, the formula's style, the installer's shellcheck.
#
# Never pushes, tags or uploads; dist's config has `publish-jobs = []` and
# `dist build` only writes under the target directory. `brew style` reads the
# formula file and installs nothing.
#
# Where it goes:
#   artefacts:     <target dir>/distrib/
#   summary lines: docs/release/plan.txt, docs/release/build.txt (committed)
#
# Needs `dist` 0.33 on PATH (or DIST=<path>); `brew` and `shellcheck` for
# the last two checks.
#
# Usage: scripts/release-local.sh
set -eu
cd "$(dirname "$0")/.." || exit 1

dist="${DIST:-dist}"
mkdir -p docs/release
logs="$(mktemp -d)"
trap 'rm -rf "$logs"' EXIT
today="$(date -u +%Y-%m-%d)"

"$dist" plan >"$logs/plan" 2>&1
{
    echo "# dist plan ($("$dist" --version)), scripts/release-local.sh, $today"
    # The tap warning repeats once per target; once is enough. It is the
    # point: publish-jobs stays empty until ADR-0057's search clears.
    # `|| true`: no warning (grep exits 1) is a fine plan too.
    grep -m1 -A1 'WARN' "$logs/plan" || true
    sed -n '/^announcing/,$p' "$logs/plan"
} >docs/release/plan.txt

host="$(rustc -vV | sed -n 's/^host: //p')"
"$dist" build --artifacts=all --target "$host" >"$logs/build" 2>&1
target_dir="$(cargo metadata --format-version 1 --no-deps |
    python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"
distrib="$target_dir/distrib"
archive="$distrib/graft-$host.tar.xz"
formula="$distrib/graft.rb"
installer="$distrib/graft-installer.sh"

{
    echo "# dist build --artifacts=all --target $host, scripts/release-local.sh, $today"
    echo "artefacts in <target>/distrib:"
    (cd "$distrib" && ls -1 | sed 's/^/    /')
    echo "tar tf graft-$host.tar.xz:"
    tar tf "$archive" | sed 's/^/    /'
    tar -xOf "$archive" "graft-$host/trib" >"$logs/trib"
    chmod +x "$logs/trib"
    echo "the archive's binary, --version: $("$logs/trib" --version)"
    # Homebrew refuses `--formula` on a file outside a tap, and making a tap
    # would change this machine's Homebrew. So the file goes in a temporary
    # `Formula/` folder and `brew style` lints it as a path, with Homebrew's
    # own RuboCop config and formula cops; nothing is tapped or installed.
    # (The first `brew style` on a machine installs Homebrew's style gems
    # into its own vendor folder.)
    mkdir -p "$logs/tap/Formula"
    cp "$formula" "$logs/tap/Formula/graft.rb"
    if brew style "$logs/tap/Formula/graft.rb" >"$logs/style" 2>&1; then
        echo "brew style graft.rb: ok ($(tail -1 "$logs/style"))"
    else
        echo "brew style graft.rb: FAILED"
        sed "s|$logs/tap/||" "$logs/style" | grep -v '^ *^' | sed 's/^/    /'
    fi
    if shellcheck -S warning "$installer" >"$logs/shellcheck" 2>&1; then
        echo "shellcheck -S warning graft-installer.sh: clean"
    else
        echo "shellcheck -S warning graft-installer.sh: FAILED"
        sed 's/^/    /' "$logs/shellcheck"
    fi
    echo "where the formula puts each file (its install block):"
    sed -n '/def install$/,/^  end$/p' "$formula" | sed 's/^/    /'
} >docs/release/build.txt

cat docs/release/plan.txt docs/release/build.txt
