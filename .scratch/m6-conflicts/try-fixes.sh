#!/bin/sh
# Builds two throwaway repos under ~/graft-try for trying fixes 47–49 by hand.
#   push/   one commit ahead of a remote that has moved on: the push gets rejected.
#   merge/  a merge stopped on a conflict, begun in the shell.
# Delete with: rm -rf ~/graft-try
set -eu
T="$HOME/graft-try"
rm -rf "$T"; mkdir -p "$T"; cd "$T"
g() { git -c user.name=try -c user.email=try@example.invalid "$@"; }

# push: a bare remote, our clone, and a teammate who pushes first.
g init -q --bare -b main remote.git
g clone -q remote.git push 2>/dev/null
cd push; echo one > a.txt; g add a.txt; g commit -qm "first"; g push -q origin main; cd ..
g clone -q remote.git mate; cd mate; echo mate > b.txt; g add b.txt; g commit -qm "teammate's commit"; g push -q origin main; cd ..
rm -rf mate
cd push; echo mine > c.txt; g add c.txt; g commit -qm "my commit"; g fetch -q; cd ..

# merge: both branches change the same line, merged in the shell.
g init -q -b main merge; cd merge
echo "colour = blue" > notes.txt; g add notes.txt; g commit -qm "base"
g checkout -qb feature; echo "colour = green" > notes.txt; g commit -qam "feature: green"
g checkout -q main; echo "colour = red" > notes.txt; g commit -qam "main: red"
g merge feature >/dev/null 2>&1 || true
cd ..

echo "ready: $T/push  $T/merge"
