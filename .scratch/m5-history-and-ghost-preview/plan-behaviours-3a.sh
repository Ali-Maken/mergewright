#!/bin/bash
# m5/07 fix round: git-config honesty checks, run outside the repo in /tmp.
set -u
W=$(mktemp -d /tmp/m5fixXXXXXX); cd "$W" || exit 1
echo "git: $(git --version)"
echo "workdir: $W"

mk() { # ff-able: main behind feat
  rm -rf "$1"; git init -q -b main "$1"; cd "$1" || exit 1
  git config user.email t@t; git config user.name t
  echo base > f.txt; git add .; git commit -qm one
  git branch feat
  git switch -q feat; echo more >> f.txt; git add .; git commit -qm two
  git switch -q main
  cd ..
}
div() { # diverged: non-ff
  rm -rf "$1"; git init -q -b main "$1"; cd "$1" || exit 1
  git config user.email t@t; git config user.name t
  echo base > f.txt; git add .; git commit -qm one
  git branch feat
  echo m > m.txt; git add .; git commit -qm "on main"
  git switch -q feat; echo f > ff.txt; git add .; git commit -qm "on feat"
  git switch -q main
  cd ..
}
parents() { git rev-list --parents -n1 HEAD | wc -w | tr -d ' '; }

echo
echo "### 1. merge.ff=false + explicit --ff, ff-able (the M1 fix). want: exit 0, 2 words = fast-forward"
mk a; cd a || exit 1; git -c merge.ff=false merge --no-edit --ff -m x feat >/dev/null 2>&1; echo "exit=$?"
echo "rev-list --parents words: $(parents)  (2 = ff, 3 = merge commit)"; cd ..

echo
echo "### 2. merge.ff=false + NO flag (today's Default) — reviewer's M1"
mk b; cd b || exit 1; git -c merge.ff=false merge --no-edit -m x feat >/dev/null 2>&1; echo "exit=$?"
echo "rev-list --parents words: $(parents)"; cd ..

echo
echo "### 3. merge.ff=only + explicit --no-ff, diverged (FastForward::Never)"
div c; cd c || exit 1; git -c merge.ff=only merge --no-edit --no-ff -m x feat >/dev/null 2>&1; echo "exit=$?"
echo "rev-list --parents words: $(parents)"; cd ..

echo
echo "### 4. merge.ff=false + --ff-only, ff-able (FastForward::Only, and Pull's own plan)"
mk d; cd d || exit 1; git -c merge.ff=false merge --ff-only --no-edit feat >/dev/null 2>&1; echo "exit=$?"
echo "rev-list --parents words: $(parents)"; cd ..

echo
echo "### 5. pull.rebase=true over 'merge --ff-only' (Pull's plan runs fetch + merge, never 'git pull')"
mk e; cd e || exit 1; git -c pull.rebase=true merge --ff-only --no-edit feat >/dev/null 2>&1; echo "exit=$?"
echo "rev-list --parents words: $(parents)"; cd ..

echo
echo "### 5b. pull.ff=only / pull.rebase over a real 'git pull' for contrast — not Graft's path"
echo "(skipped: Graft never runs 'git pull'; see plan_pull)"

echo
echo "### 6. merge.ff=only + no flag, diverged (Default) — reviewer's second M1 finding"
div f; cd f || exit 1; git -c merge.ff=only merge --no-edit -m x feat >/dev/null 2>&1; echo "exit=$?"; cd ..

echo
echo "### 7. rebase.updateRefs=true, without and with --no-update-refs"
for flag in none --no-update-refs; do
  rm -rf g; git init -q -b main g; cd g || exit 1
  git config user.email t@t; git config user.name t
  echo a > a.txt; git add .; git commit -qm base
  git branch f1x
  echo m > m.txt; git add .; git commit -qm "on main"
  git switch -q f1x; echo x > x.txt; git add .; git commit -qm c1
  git branch f2x
  echo y > y.txt; git add .; git commit -qm c2
  git branch -f f2x
  git switch -q f2x
  BEFORE=$(git rev-parse f1x)
  if [ "$flag" = none ]; then
    git -c rebase.updateRefs=true rebase main >/dev/null 2>&1; echo "flag=none exit=$?"
  else
    git -c rebase.updateRefs=true rebase --no-update-refs main >/dev/null 2>&1; echo "flag=$flag exit=$?"
  fi
  AFTER=$(git rev-parse f1x)
  if [ "$BEFORE" = "$AFTER" ]; then echo "  f1x UNCHANGED"; else echo "  f1x MOVED $BEFORE -> $AFTER"; fi
  cd ..
done

echo
echo "### 8. --no-update-refs accepted by this git?"
rm -rf h; git init -q -b main h; cd h || exit 1
git config user.email t@t; git config user.name t
echo a > a.txt; git add .; git commit -qm base
git rebase --no-update-refs HEAD >/dev/null 2>&1; echo "rebase --no-update-refs HEAD exit=$?"
cd ..

echo
echo "### 9. --no-autostash on a dirty tree (rebase) — Graft refuses DirtyTree first; what does git do?"
rm -rf i; git init -q -b main i; cd i || exit 1
git config user.email t@t; git config user.name t
echo a > a.txt; git add .; git commit -qm base
git branch other; echo m >> a.txt; git add .; git commit -qm two
git switch -q other; echo dirty >> a.txt
git -c rebase.autostash=true rebase --no-autostash main >/dev/null 2>&1; echo "exit=$? (non-zero = refused, which is what we want)"
cd ..
echo
echo "done"
