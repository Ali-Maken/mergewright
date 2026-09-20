#!/bin/bash
# What git leaves behind when each kind of operation stops on a conflict.
set -u
W=$(mktemp -d /tmp/m5stateXXXXXX); cd "$W" || exit 1
echo "git: $(git --version)"

setup() {
  rm -rf "$1"; git init -q -b main "$1"; cd "$1" || exit 1
  git config user.email t@t; git config user.name t
  echo base > s.txt; git add .; git commit -qm one
  git branch feat
  echo mainline > s.txt; git add .; git commit -qm "on main"
  git switch -q feat; echo feature > s.txt; git add .; git commit -qm "on feat"
  git switch -q main
}

report() {
  echo "  HEAD symbolic: $(git symbolic-ref -q HEAD || echo '(detached)')"
  echo "  ORIG_HEAD: $(git rev-parse -q --verify ORIG_HEAD || echo '(none)')"
  echo "  branch main: $(git rev-parse -q --verify refs/heads/main || echo '(none)')"
  for f in rebase-merge/head-name rebase-merge/orig-head rebase-merge/onto; do
    if [ -f ".git/$f" ]; then echo "  $f: $(cat ".git/$f")"; else echo "  $f: (absent)"; fi
  done
  echo "  MERGE_HEAD: $([ -f .git/MERGE_HEAD ] && cat .git/MERGE_HEAD || echo '(absent)')"
  echo "  CHERRY_PICK_HEAD: $([ -f .git/CHERRY_PICK_HEAD ] && cat .git/CHERRY_PICK_HEAD || echo '(absent)')"
  echo "  REVERT_HEAD: $([ -f .git/REVERT_HEAD ] && cat .git/REVERT_HEAD || echo '(absent)')"
}

echo
echo "### merge conflict"
setup a; PRE=$(git rev-parse HEAD); git merge --no-edit --no-ff -m x feat >/dev/null 2>&1
echo "  pre-merge HEAD was: $PRE"; report; cd ..

echo
echo "### rebase conflict"
setup b; git switch -q feat; PRE=$(git rev-parse HEAD); git rebase main >/dev/null 2>&1
echo "  pre-rebase feat was: $PRE"; report; cd ..

echo
echo "### cherry-pick conflict"
setup c; PRE=$(git rev-parse HEAD); git cherry-pick --no-edit feat >/dev/null 2>&1
echo "  pre-pick HEAD was: $PRE"; report; cd ..

echo
echo "### revert conflict (revert a commit whose change was later rewritten)"
rm -rf d; git init -q -b main d; cd d || exit 1
git config user.email t@t; git config user.name t
echo one > s.txt; git add .; git commit -qm one
echo two > s.txt; git add .; git commit -qm two
TARGET=$(git rev-parse HEAD)
echo three > s.txt; git add .; git commit -qm three
PRE=$(git rev-parse HEAD)
git revert --no-edit "$TARGET" >/dev/null 2>&1
echo "  pre-revert HEAD was: $PRE"; report; cd ..

echo
echo "### does a plain 'git merge' set ORIG_HEAD even when it succeeds?"
setup e; git merge --no-edit --no-ff -m x feat >/dev/null 2>&1 || true
echo "  (conflicted; see above)"; cd ..
echo done
