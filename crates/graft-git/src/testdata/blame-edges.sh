#!/bin/bash
# A tiny repo: a quoted path; two ignored commits: the first changes line 3
# (skipped) and adds line 4 (unblamable), the second changes line 4 again
# (ignored, then unblamable: both marks); an uncommitted working-tree line.
# Blamed with git 2.38.5 and the system git, porcelain and human format.
set -euo pipefail
OUT=$1
OLD=$TMPDIR/oldg238/inst/bin/git
NEW=$(command -v git)
R=$OUT/repo
rm -rf "$R"; mkdir -p "$R/src"; cd "$R"
export GIT_AUTHOR_NAME=Ana GIT_AUTHOR_EMAIL=ana@example.invalid GIT_COMMITTER_NAME=Ana GIT_COMMITTER_EMAIL=ana@example.invalid
export GIT_AUTHOR_DATE='1541721600 +0000' GIT_COMMITTER_DATE='1541721600 +0000'
"$NEW" init -q -b main
printf 'fn one() {}\nfn two() {\n    a();\n}\n' > "src/é.rs"
"$NEW" add -A; "$NEW" commit -q -m 'Add é file'
export GIT_AUTHOR_NAME=Tom GIT_AUTHOR_EMAIL=tom@example.invalid GIT_AUTHOR_DATE='1552089600 +0000' GIT_COMMITTER_DATE='1552089600 +0000'
printf 'fn one() {}\nfn two() {\n    a(1);\n    brand_new_line(1);\n}\n' > "src/é.rs"
"$NEW" commit -q -am 'Reformat'
"$NEW" rev-parse HEAD > .git-blame-ignore-revs
export GIT_AUTHOR_DATE='1552089700 +0000' GIT_COMMITTER_DATE='1552089700 +0000'
printf 'fn one() {}\nfn two() {\n    a(1);\n    brand_new_line(2);\n}\n' > "src/é.rs"
"$NEW" commit -q -am 'Reformat again'
"$NEW" rev-parse HEAD >> .git-blame-ignore-revs
printf 'fn one() {}\nfn two() {\n    a(1);\n    brand_new_line(2);\n}\nlet fresh = 1;\n' > "src/é.rs"
MARKS=(-c blame.markIgnoredLines=true -c blame.markUnblamableLines=true)
PINS=(-c blame.blankBoundary=false -c blame.coloring=none -c color.ui=never)
for g in OLD NEW; do
  bin=${!g}
  "$bin" "${MARKS[@]}" blame --porcelain -M -w --ignore-revs-file=.git-blame-ignore-revs -- "src/é.rs" > "$OUT/edges-porcelain-$g.txt"
  "$bin" "${MARKS[@]}" "${PINS[@]}" blame -l -s -M -w --ignore-revs-file=.git-blame-ignore-revs -- "src/é.rs" > "$OUT/edges-human-$g.txt"
  echo "== $g $($bin --version)"; cat "$OUT/edges-human-$g.txt"
  grep -n -E '^(ignored|unblamable|filename|[0-9a-f]{40} )' "$OUT/edges-porcelain-$g.txt" || true
done
