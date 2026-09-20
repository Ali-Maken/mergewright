#!/bin/zsh
# m5/07 acceptance box 1: the per-kind git behaviours (a)-(g) that the
# operations' `plan` and `check` rules are built on, recorded against a
# throwaway repository. Run with: zsh /tmp/plan_behaviours.sh
set -u
export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_SYSTEM=/dev/null
export GIT_AUTHOR_NAME=t GIT_AUTHOR_EMAIL=t@t GIT_COMMITTER_NAME=t GIT_COMMITTER_EMAIL=t@t

W=$(mktemp -d)
say() { print -r -- "$@" }
hr() { say ""; say "── $1 ──"; }

say "git: $(git --version)"
say "recorded: $(date -u +%Y-%m-%dT%H:%M:%SZ)"

# An editor that FAILS loudly if git ever opens one.
BOOM=$W/boom.sh
print -r -- '#!/bin/sh
echo "EDITOR OPENED on $1" >&2
exit 42' > $BOOM
chmod +x $BOOM

fresh() {                     # fresh <name>; echoes the path
  local d=$W/$1
  rm -rf $d; mkdir -p $d
  git -C $d init -q -b main
  print -r -- "$d"
}

# ── (a) merge --no-edit with GIT_EDITOR=true never opens an editor ──────
hr "(a) merge --no-edit, GIT_EDITOR=true"
R=$(fresh a)
echo base > $R/base.txt; git -C $R add -A; git -C $R commit -qm one
git -C $R branch feature
echo main > $R/main.txt; git -C $R add -A; git -C $R commit -qm two
git -C $R switch -q feature
echo feat > $R/feature.txt; git -C $R add -A; git -C $R commit -qm three
git -C $R switch -q main
# merge.log/edit defaults would open an editor for a non-ff merge without --no-edit.
GIT_EDITOR=$BOOM git -C $R merge --no-edit feature > $W/a.out 2> $W/a.err
say "exit=$? (0 expected; 42 would mean the editor opened)"
say "stderr: $(cat $W/a.err)"
say "merge commit subject: $(git -C $R log -1 --format=%s)"
say "parents: $(git -C $R rev-list --parents -n1 HEAD | wc -w | tr -d ' ') (3 words = 2 parents)"
# and the control: no --no-edit, same editor
R2=$(fresh a2)
echo base > $R2/base.txt; git -C $R2 add -A; git -C $R2 commit -qm one
git -C $R2 branch feature
echo main > $R2/main.txt; git -C $R2 add -A; git -C $R2 commit -qm two
git -C $R2 switch -q feature
echo feat > $R2/feature.txt; git -C $R2 add -A; git -C $R2 commit -qm three
git -C $R2 switch -q main
GIT_EDITOR=$BOOM git -C $R2 merge feature > /dev/null 2> $W/a2.err
say "control (no --no-edit) exit=$?  stderr: $(head -1 $W/a2.err)"

# ── (b) rebase on a conflict exits 1 and leaves rebase-merge/ ───────────
hr "(b) rebase <onto> on a conflict"
R=$(fresh b)
echo base > $R/shared.txt; git -C $R add -A; git -C $R commit -qm one
git -C $R branch feature
echo mains > $R/shared.txt; git -C $R add -A; git -C $R commit -qm two
git -C $R switch -q feature
echo feats > $R/shared.txt; git -C $R add -A; git -C $R commit -qm three
GIT_EDITOR=$BOOM git -C $R rebase main > $W/b.out 2> $W/b.err
say "exit=$?"
say "stderr head: $(head -3 $W/b.err | tr '\n' '|')"
say "rebase-merge dir exists: $([[ -d $R/.git/rebase-merge ]] && echo yes || echo no)"
say "rebase-apply dir exists: $([[ -d $R/.git/rebase-apply ]] && echo yes || echo no)"
say "unmerged paths: $(git -C $R diff --name-only --diff-filter=U)"
say "status --porcelain: $(git -C $R status --porcelain | tr '\n' '|')"

# ── (c) revert --no-edit -m 1 on a merge; -m on a non-merge ────────────
hr "(c) revert -m on a merge, and -m on a non-merge"
R=$(fresh c)
echo base > $R/base.txt; git -C $R add -A; git -C $R commit -qm one
git -C $R branch feature
echo main > $R/main.txt; git -C $R add -A; git -C $R commit -qm two
git -C $R switch -q feature
echo feat > $R/feature.txt; git -C $R add -A; git -C $R commit -qm three
git -C $R switch -q main
GIT_EDITOR=$BOOM git -C $R merge --no-edit feature -q
M=$(git -C $R rev-parse HEAD)
say "merge commit: $M  parents: $(git -C $R rev-list --parents -n1 $M | cut -d' ' -f2-)"
say "feature.txt present before revert: $([[ -f $R/feature.txt ]] && echo yes || echo no)"
GIT_EDITOR=$BOOM git -C $R revert --no-edit -m 1 $M > $W/c.out 2> $W/c.err
say "revert -m 1 exit=$?"
say "stdout: $(head -2 $W/c.out | tr '\n' '|')"
say "stderr: $(head -2 $W/c.err | tr '\n' '|')"
say "revert commit subject: $(git -C $R log -1 --format=%s)"
say "feature.txt present after revert: $([[ -f $R/feature.txt ]] && echo yes || echo no)"
say "main.txt still present: $([[ -f $R/main.txt ]] && echo yes || echo no)"
# -m 2 keeps the other side
R2=$(fresh c2); cp -R $R/. $R2/ 2>/dev/null
# -m on a NON-merge commit
N=$(git -C $R rev-parse HEAD~1)   # the merge's predecessor on main... use a plain commit
N=$(git -C $R rev-parse main~2)
say "non-merge commit: $N ($(git -C $R log -1 --format=%s $N))"
GIT_EDITOR=$BOOM git -C $R revert --no-edit -m 1 $N > $W/c3.out 2> $W/c3.err
say "revert -m 1 on NON-merge exit=$?"
say "stderr: $(head -2 $W/c3.err | tr '\n' '|')"
# and: revert a merge with NO -m
GIT_EDITOR=$BOOM git -C $R revert --no-edit $M > $W/c4.out 2> $W/c4.err
say "revert (no -m) on a MERGE exit=$?"
say "stderr: $(head -2 $W/c4.err | tr '\n' '|')"

# ── (d) update-ref with an expected old value ──────────────────────────
hr "(d) update-ref refs/heads/x <to> <expected> when the ref moved"
R=$(fresh d)
echo a > $R/a.txt; git -C $R add -A; git -C $R commit -qm one
C1=$(git -C $R rev-parse HEAD)
echo b > $R/b.txt; git -C $R add -A; git -C $R commit -qm two
C2=$(git -C $R rev-parse HEAD)
git -C $R branch x $C1
say "x is at C1=$C1"
# someone moves it under us
git -C $R update-ref refs/heads/x $C2
say "x moved to C2=$C2"
git -C $R update-ref refs/heads/x $C1 $C2 && say "CAS with correct expected: exit=0 (x now $(git -C $R rev-parse x))"
git -C $R update-ref refs/heads/x $C2 $C2 > /dev/null 2> $W/d.err
say "CAS with STALE expected exit=$?"
say "stderr: $(head -2 $W/d.err | tr '\n' '|')"
say "x is still: $(git -C $R rev-parse x) (unchanged = refused)"
# creation guard: expected = zero oid on an existing ref
Z=$(git -C $R hash-object -t tree /dev/null > /dev/null; printf '%040d' 0)
git -C $R update-ref refs/heads/x $C2 $Z > /dev/null 2> $W/d2.err
say "CAS with zero-oid expected on an existing ref exit=$?  stderr: $(head -1 $W/d2.err)"

# ── (e) switch onto a branch whose files differ, dirty tree ────────────
hr "(e) switch with a dirty tracked file that differs"
R=$(fresh e)
echo base > $R/shared.txt; git -C $R add -A; git -C $R commit -qm one
git -C $R branch feature
echo mains > $R/shared.txt; git -C $R add -A; git -C $R commit -qm two
git -C $R switch -q feature
echo feats > $R/shared.txt; git -C $R add -A; git -C $R commit -qm three
git -C $R switch -q main
echo "dirty edit" >> $R/shared.txt
git -C $R switch feature > $W/e.out 2> $W/e.err
say "switch (dirty, file differs) exit=$?"
say "stderr: $(cat $W/e.err | tr '\n' '|')"
say "still on: $(git -C $R rev-parse --abbrev-ref HEAD)"
# dirty file that does NOT differ between the branches
git -C $R checkout -q -- shared.txt
echo other > $R/other.txt; git -C $R add -A; git -C $R commit -qm four
git -C $R switch -q feature; git -C $R switch -q main
echo "dirty" >> $R/other.txt
git -C $R switch feature > /dev/null 2> $W/e2.err
say "switch (dirty file NOT in the diff) exit=$?  now on: $(git -C $R rev-parse --abbrev-ref HEAD)"
say "stderr: $(head -2 $W/e2.err | tr '\n' '|')"
say "the dirty edit survived: $(tail -1 $R/other.txt)"
# untracked file that would be overwritten
git -C $R switch -q main 2>/dev/null

# ── (f) stash branch on a stash that conflicts ─────────────────────────
hr "(f) stash branch on a conflicting stash"
R=$(fresh f)
echo base > $R/shared.txt; git -C $R add -A; git -C $R commit -qm one
echo "work in progress" > $R/shared.txt
git -C $R stash push -q -m wip
say "stash list after push: $(git -C $R stash list)"
STASH_BASE=$(git -C $R rev-parse 'stash@{0}^')
# move main on so the stash's base commit is no longer the tip, and make the
# same line conflict
echo "someone elses line" > $R/shared.txt; git -C $R add -A; git -C $R commit -qm two
say "stash base: $STASH_BASE  main now: $(git -C $R rev-parse HEAD)"
git -C $R stash branch wipbranch 'stash@{0}' > $W/f.out 2> $W/f.err
say "stash branch exit=$?"
say "stdout: $(cat $W/f.out | tr '\n' '|')"
say "stderr: $(cat $W/f.err | tr '\n' '|')"
say "branch now: $(git -C $R rev-parse --abbrev-ref HEAD)"
say "stash list after: '$(git -C $R stash list)'  (empty = the stash was dropped)"
say "unmerged paths: '$(git -C $R diff --name-only --diff-filter=U)'"
say "status: $(git -C $R status --porcelain | tr '\n' '|')"
say "shared.txt has markers: $(grep -c '<<<<<<<' $R/shared.txt)"

# ── (g) tag -a -F - reads the message from stdin ───────────────────────
hr "(g) tag -a -F - from stdin"
R=$(fresh g)
echo a > $R/a.txt; git -C $R add -A; git -C $R commit -qm one
printf 'a tag message\nsecond line\n' | GIT_EDITOR=$BOOM git -C $R tag -a -F - v1.2 > $W/g.out 2> $W/g.err
say "exit=$?  stderr: $(cat $W/g.err)"
say "tag type: $(git -C $R cat-file -t v1.2)"
say "tag message: $(git -C $R tag -l --format='%(contents)' v1.2 | tr '\n' '|')"
# an empty message from stdin
printf '' | GIT_EDITOR=$BOOM git -C $R tag -a -F - v1.3 > /dev/null 2> $W/g2.err
say "empty stdin message exit=$?  stderr: $(head -1 $W/g2.err)"
say "v1.3 type: $(git -C $R cat-file -t v1.3 2>&1)"
# lightweight, for contrast
git -C $R tag v1.4 && say "lightweight v1.4 type: $(git -C $R cat-file -t v1.4)"

say ""
say "── done ──"
rm -rf $W
