#!/bin/sh
# Builds three throwaway repos for trying the M6 stash put-back by hand.
#   pop/          a stash that clashes when popped:            S p
#   switch/       uncommitted edits, switch goes clean:         . ⏎ y   (menu on the `feature` row)
#   switch-clash/ uncommitted edits that clash on switch back:  . ⏎ y
# Location: $GRAFT_TRY (default ~/graft-try-stash). Delete with: rm -rf ~/graft-try-stash
set -eu
T="${GRAFT_TRY:-$HOME/graft-try-stash}"
rm -rf "$T"; mkdir -p "$T"; cd "$T"
g() { git -c user.name=try -c user.email=try@example.invalid "$@"; }

# pop: stash an edit to notes.txt, then commit a different edit to the same line.
g init -q -b main pop; cd pop
echo "colour = blue" > notes.txt; g add notes.txt; g commit -qm "base"
echo "colour = green" > notes.txt; g stash push -q -m "my green edit"
echo "colour = red" > notes.txt; g commit -qam "main: red"
cd ..

# switch: feature changes b.txt; you have an uncommitted edit to a.txt. No clash.
g init -q -b main switch; cd switch
echo a > a.txt; echo b > b.txt; g add .; g commit -qm "base"
g checkout -qb feature; echo "b on feature" > b.txt; g commit -qam "feature: b"
g checkout -q main; echo "a, edited, not committed" > a.txt
cd ..

# switch-clash: feature changes notes.txt; you have an uncommitted edit to the same line.
g init -q -b main switch-clash; cd switch-clash
echo "colour = blue" > notes.txt; g add notes.txt; g commit -qm "base"
g checkout -qb feature; echo "colour = green" > notes.txt; g commit -qam "feature: green"
g checkout -q main; echo "colour = red (not committed)" > notes.txt
cd ..

echo "ready: $T/pop  $T/switch  $T/switch-clash"
