#!/bin/bash
# codename-rename 01: graft -> mergewright (mechanical pass). Run from the repo root.
set -euo pipefail
for d in crates/graft-*; do git mv "$d" "${d/graft-/mergewright-}"; done
git mv crates/graft crates/mergewright

# Path patterns: crate names, crate paths, lib path. Never on-disk names.
PATHS='s/\bgraft([-_])(core|git|graph|ops|term|ui|fixtures|capabilities)\b/mergewright$1$2/g;
 s{crates/graft\b}{crates/mergewright}g;
 s/(?<![\w.\/-])graft::/mergewright::/g;'
# Name patterns: the app's name in prose and identifiers. Not in migration files.
NAMES='s/(?<!Generation::)\bGraft\b(?!-Link)/Mergewright/g;
 s/(?<=[a-z])Graft\b/Mergewright/g;
 s/\bGraft(?=[A-Z])/Mergewright/g;
 s/\bgraft_(?=[a-z])/mergewright_/g;'

text_files() { git ls-files -- "$@" | grep -E '\.(rs|toml|md|sh|py|yml|yaml|1|txt|in|html)$' || true; }

MIGRATION='crates/mergewright-core/src/on_disk|crates/mergewright-ops/src/rename|crates/mergewright-ops/tests/it/rename\.rs'

text_files crates Cargo.toml clippy.toml fuzz scripts .github README.md ARCHITECTURE.md TESTING.md DESIGN.md \
  docs/design docs/man docs/release docs/agents docs/capabilities docs/generated .claude CLAUDE.md \
  | xargs perl -pi -e "$PATHS"
text_files crates fuzz README.md ARCHITECTURE.md TESTING.md DESIGN.md docs/design docs/man \
  | grep -v -E "$MIGRATION" | xargs perl -pi -e "$NAMES"
