#!/usr/bin/env bash
# m8/22: shellcheck over every shell script under scripts/ (lib/ included,
# sourced files followed with -x). Warnings and errors fail; info/style notes
# do not. Exit 0 = clean, 2 = shellcheck missing.
set -eu
cd "$(dirname "$0")/.." || exit 1
command -v shellcheck >/dev/null || { echo "check-scripts: shellcheck not installed" >&2; exit 2; }
find scripts -name '*.sh' -print0 | xargs -0 shellcheck -x -P scripts -S warning
python3 scripts/check-sandbox.py
python3 scripts/check-sandbox-test.py
echo "check-scripts: ok"
