# 56r — shot-drift-test runs git in a sandbox

Status: ready
Build with: Sonnet
Blocked by: none
From: docs/reviews/m8-mergewright-55-56.md, Major 1

## What
`scripts/shot-drift-test.py:29,33,34` runs `git` with the caller's environment and the user's global git config.
`scripts/check-sandbox.py` rejects that, so `bash scripts/check-scripts.sh` exits 1 on main. A global signing
config could also make the test hang on a prompt.

Fix the cause: run git the way the other script tests that already pass check-sandbox do (find one and copy its
pattern — isolated env, no global/system config). Do not weaken check-sandbox.

## Touches
`scripts/shot-drift-test.py` only. Check: `bash scripts/check-scripts.sh` lists every script test; confirm none
other fails. No Rust, no captures.

## Acceptance
- [ ] `bash scripts/check-scripts.sh` exits 0 on the branch (name the command output line)
- [ ] `python3 scripts/shot-drift-test.py` passes with `GIT_CONFIG_GLOBAL` pointed at a config that sets `commit.gpgsign=true` (name the command)
