#!/bin/zsh
# m5/07 acceptance box 1, second pass: (e) and (f) again, with setups that
# actually isolate what was being asked, plus the (c) mainline question.
set -u
export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_SYSTEM=/dev/null
export GIT_AUTHOR_NAME=t GIT_AUTHOR_EMAIL=t@t GIT_COMMITTER_NAME=t GIT_COMMITTER_EMAIL=t@t
W=$(mktemp -d)
say() { print -r -- "$@" }
hr() { say ""; say "── $1 ──"; }
say "git: $(git --version)"
fresh() { local d=$W/$1; rm -rf $d; mkdir -p $d; git -C $d init -q -b main; print -r -- "$d" }

hr "(e2) switch with a dirty tracked file that is IDENTICAL on both branches"
R=$(fresh e2)
echo same > $R/carried.txt; echo base > $R/shared.txt
git -C $R add -A; git -C $R commit -qm one
git -C $R branch feature
echo mains > $R/shared.txt; git -C $R add -A; git -C $R commit -qm "two on main"
git -C $R switch -q feature
echo feats > $R/shared.txt; git -C $R add -A; git -C $R commit -qm "one on feature"
git -C $R switch -q main
echo "uncommitted edit" >> $R/carried.txt
say "carried.txt differs between main and feature: $(git -C $R diff --name-only main feature | grep -c carried)"
git -C $R switch feature > $W/e2.out 2> $W/e2.err
say "switch exit=$?  now on: $(git -C $R rev-parse --abbrev-ref HEAD)"
say "stderr: $(cat $W/e2.err | tr '\n' '|')"
say "the dirty edit came along: $(tail -1 $R/carried.txt)"
say "status: $(git -C $R status --porcelain | tr '\n' '|')"

hr "(e3) switch with a STAGED change to a file that differs between branches"
R=$(fresh e3)
echo base > $R/shared.txt; git -C $R add -A; git -C $R commit -qm one
git -C $R branch feature
echo mains > $R/shared.txt; git -C $R add -A; git -C $R commit -qm two
git -C $R switch -q feature
echo feats > $R/shared.txt; git -C $R add -A; git -C $R commit -qm three
git -C $R switch -q main
echo staged > $R/shared.txt; git -C $R add shared.txt
git -C $R switch feature > /dev/null 2> $W/e3.err
say "switch (staged, file differs) exit=$?  now on: $(git -C $R rev-parse --abbrev-ref HEAD)"
say "stderr: $(cat $W/e3.err | tr '\n' '|')"

hr "(e4) switch with an UNTRACKED file that the target branch has"
R=$(fresh e4)
echo base > $R/base.txt; git -C $R add -A; git -C $R commit -qm one
git -C $R branch feature
git -C $R switch -q feature
echo "on feature" > $R/newfile.txt; git -C $R add -A; git -C $R commit -qm two
git -C $R switch -q main
echo "untracked local" > $R/newfile.txt
git -C $R switch feature > /dev/null 2> $W/e4.err
say "switch (untracked collision) exit=$?  now on: $(git -C $R rev-parse --abbrev-ref HEAD)"
say "stderr: $(cat $W/e4.err | tr '\n' '|')"

hr "(f2) what does 'git stash branch' actually check out?"
R=$(fresh f2)
echo base > $R/shared.txt; git -C $R add -A; git -C $R commit -qm one
B=$(git -C $R rev-parse HEAD)
echo wip > $R/shared.txt; git -C $R stash push -q -m wip
echo "someone else" > $R/shared.txt; git -C $R add -A; git -C $R commit -qm two
git -C $R stash branch wipbranch 'stash@{0}' > /dev/null 2>&1
say "branch wipbranch tip == the stash's base commit: $([[ $(git -C $R rev-parse wipbranch) == $B ]] && echo yes || echo no)"
say "  (that is why a moved branch can never make 'stash branch' conflict)"

hr "(f3) stash branch when the branch name is already taken"
R=$(fresh f3)
echo base > $R/shared.txt; git -C $R add -A; git -C $R commit -qm one
echo wip > $R/shared.txt; git -C $R stash push -q -m wip
git -C $R branch taken
git -C $R stash branch taken 'stash@{0}' > $W/f3.out 2> $W/f3.err
say "exit=$?"
say "stderr: $(cat $W/f3.err | tr '\n' '|')"
say "stash list after: '$(git -C $R stash list)'  (kept = the stash survived)"
say "HEAD still: $(git -C $R rev-parse --abbrev-ref HEAD)"

hr "(f4) stash branch from a DIRTY working tree"
R=$(fresh f4)
echo base > $R/a.txt; echo other > $R/b.txt; git -C $R add -A; git -C $R commit -qm one
echo wip > $R/a.txt; git -C $R stash push -q -m wip
echo "new dirty" > $R/a.txt
git -C $R stash branch fromdirty 'stash@{0}' > $W/f4.out 2> $W/f4.err
say "exit=$?"
say "stdout: $(cat $W/f4.out | tr '\n' '|')"
say "stderr: $(cat $W/f4.err | tr '\n' '|')"
say "stash list after: '$(git -C $R stash list)'"
say "HEAD: $(git -C $R rev-parse --abbrev-ref HEAD)"
say "a.txt: $(cat $R/a.txt)"

hr "(f5) stash branch where the stash's INDEX and worktree differ"
R=$(fresh f5)
echo base > $R/a.txt; git -C $R add -A; git -C $R commit -qm one
echo staged > $R/a.txt; git -C $R add a.txt
echo worktree > $R/a.txt
git -C $R stash push -q -m "index and worktree differ"
git -C $R stash branch split 'stash@{0}' > $W/f5.out 2> $W/f5.err
say "exit=$?"
say "stdout: $(cat $W/f5.out | tr '\n' '|')"
say "stderr: $(cat $W/f5.err | tr '\n' '|')"
say "stash list after: '$(git -C $R stash list)'"
say "index restored too: $(git -C $R diff --cached --name-only | tr '\n' '|')"

hr "(f6) stash branch onto a nonexistent stash ref"
R=$(fresh f6)
echo base > $R/a.txt; git -C $R add -A; git -C $R commit -qm one
git -C $R stash branch nope 'stash@{7}' > $W/f6.out 2> $W/f6.err
say "exit=$?  stderr: $(head -2 $W/f6.err | tr '\n' '|')"
say "branch created anyway: $(git -C $R branch --list nope)"

hr "(c2) is -m on a non-merge really accepted, and what does it do?"
R=$(fresh c2)
echo a > $R/a.txt; git -C $R add -A; git -C $R commit -qm one
echo b >> $R/a.txt; git -C $R add -A; git -C $R commit -qm two
N=$(git -C $R rev-parse HEAD)
git -C $R revert --no-edit -m 1 $N > $W/c2.out 2> $W/c2.err
say "revert -m 1 on a non-merge exit=$?"
say "stdout: $(head -1 $W/c2.out)"
say "a.txt after: $(cat $R/a.txt | tr '\n' '|')  (back to 'a' = an ordinary revert)"
git -C $R revert --no-edit -m 2 $N > /dev/null 2> $W/c2b.err
say "revert -m 2 on a non-merge exit=$?  stderr: $(head -1 $W/c2b.err)"
say ""
say "── done ──"
rm -rf $W
