#!/usr/bin/env bash
#
# The release, made locally and published nowhere (m8-polish-and-ship/24,
# 53; ADR-0057 §1): `dist plan`, then `dist build` for this machine's target,
# then the checks a person can make on what came out — the archive's
# contents, our Homebrew formula's style, the installer's shellcheck.
#
# Never pushes, tags or uploads; dist's config has `publish-jobs = []` and
# `dist build` only writes under the target directory. `brew style` reads the
# formula file and installs nothing; this script never runs `brew install`.
#
# The formula is ours (ADR-0057 amendment §11): scripts/formula.py fills
# packaging/homebrew/mergewright.rb.in from dist's manifest. A host build has
# one archive of four, so formula.py on its manifest stops with exit 2 naming
# a missing target; build.txt quotes that. `brew style` then lints a copy
# rendered from the plan's manifest, with the host archive's real sha256 and
# a stand-in (64 zeros) for each archive not built here. Never shipped.
#
# Where it goes:
#   artefacts:     <target dir>/distrib/
#   the formula:   .scratch/m8-polish-and-ship/mergewright.rb (not committed)
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
"$dist" plan --output-format=json >"$logs/plan.json" 2>/dev/null
{
    echo "# dist plan ($("$dist" --version)), scripts/release-local.sh, $today"
    # A warning repeats once per target; once is enough.
    # `|| true`: no warning (grep exits 1) is a fine plan too.
    grep -m1 -A1 'WARN' "$logs/plan" || true
    sed -n '/^announcing/,$p' "$logs/plan"
} >docs/release/plan.txt

host="$(rustc -vV | sed -n 's/^host: //p')"
"$dist" build --artifacts=all --target "$host" --output-format=json >"$logs/build.json" 2>"$logs/build"
target_dir="$(cargo metadata --format-version 1 --no-deps |
    python3 -I -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"
distrib="$target_dir/distrib"
package="$(python3 -I -c 'import json,sys; print(json.load(open(sys.argv[1]))["releases"][0]["app_name"])' "$logs/build.json")"
archive="$distrib/$package-$host.tar.xz"
installer="$distrib/$package-installer.sh"
formula=".scratch/m8-polish-and-ship/$package.rb"
mkdir -p "$(dirname "$formula")"

{
    echo "# dist build --artifacts=all --target $host, scripts/release-local.sh, $today"
    echo "artefacts in <target>/distrib:"
    (cd "$distrib" && ls -1 | sed 's/^/    /')
    echo "tar tf $package-$host.tar.xz:"
    tar tf "$archive" | sed 's/^/    /'
    command="$(tar tf "$archive" | sed -n "s|^$package-$host/\([^./][^./]*\)\$|\1|p" | head -1)"
    tar -xOf "$archive" "$package-$host/$command" >"$logs/$command"
    chmod +x "$logs/$command"
    echo "the archive's binary, --version: $("$logs/$command" --version)"

    # formula.py on the build's own manifest: one archive of four.
    if python3 -I scripts/formula.py "$logs/build.json" "$logs/host.rb" 2>"$logs/formula"; then
        echo "formula.py on the host build's manifest: exit 0"
    else
        echo "formula.py on the host build's manifest: exit $? — $(cat "$logs/formula")"
    fi
    # The plan's manifest lists all four; checksums from this build where
    # there is one, a stand-in of 64 zeros where not (for brew style only).
    python3 -I -c '
import json, sys
plan = json.load(open(sys.argv[1]))
build = json.load(open(sys.argv[2]))
stand_in = []
for name, entry in plan["artifacts"].items():
    if entry.get("kind") != "executable-zip":
        continue
    built = build["artifacts"].get(name, {}).get("checksums")
    if built:
        entry["checksums"] = built
    else:
        entry["checksums"] = {"sha256": "0" * 64}
        stand_in.append(name)
json.dump(plan, open(sys.argv[3], "w"))
print(", ".join(sorted(stand_in)))
' "$logs/plan.json" "$logs/build.json" "$logs/style-manifest.json" >"$logs/stand-in"
    python3 -I scripts/formula.py "$logs/style-manifest.json" "$formula"
    echo "formula.py on the plan's manifest (stand-in sha256 for: $(cat "$logs/stand-in")): exit 0, $formula"

    # Homebrew refuses `--formula` on a file outside a tap, and making a tap
    # would change this machine's Homebrew. So the file goes in a temporary
    # `Formula/` folder and `brew style` lints it as a path, with Homebrew's
    # own RuboCop config and formula cops; nothing is tapped or installed.
    mkdir -p "$logs/tap/Formula"
    cp "$formula" "$logs/tap/Formula/$package.rb"
    if brew style "$logs/tap/Formula/$package.rb" >"$logs/style" 2>&1; then
        echo "brew style $package.rb: ok ($(tail -1 "$logs/style"))"
    else
        echo "brew style $package.rb: FAILED"
        sed "s|$logs/tap/||" "$logs/style" | grep -v '^ *^' | sed 's/^/    /'
    fi
    if shellcheck -S warning "$installer" >"$logs/shellcheck" 2>&1; then
        echo "shellcheck -S warning $package-installer.sh: clean"
    else
        echo "shellcheck -S warning $package-installer.sh: FAILED"
        sed 's/^/    /' "$logs/shellcheck"
    fi
    echo "where the formula puts each file (its install block):"
    sed -n '/def install$/,/^  end$/p' "$formula" | sed 's/^/    /'
} >docs/release/build.txt

cat docs/release/plan.txt docs/release/build.txt
