# 56r — shot-drift-test runs git in a sandbox

Status: partial (acceptance box 2 unverified)
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
- [x] `bash scripts/check-scripts.sh` exits 0 on the branch: output `check-sandbox: ok` / `shot-drift-test: ok` / `check-scripts: ok`
- [ ] UNVERIFIED: the harness would not run `GIT_CONFIG_GLOBAL=<scratch>/sign.cfg python3 scripts/shot-drift-test.py` (it blocks setting GIT_CONFIG_GLOBAL in an isolated worktree). The code sets GIT_CONFIG_GLOBAL=os.devnull and GIT_CONFIG_NOSYSTEM=1 in the child env, overriding the caller's value. Needs a run outside the worktree. Original box: `python3 scripts/shot-drift-test.py` passes with `GIT_CONFIG_GLOBAL` pointed at a config that sets `commit.gpgsign=true` (name the command)

## Added by the conductor (review 55-56 minors, textual only)
Records keep their wording; these are live-doc lines.
- [x] (`grep -n 'pre-M8 captures' TESTING.md`) Minor 1: one line above the M4 per-breakpoint table near `TESTING.md:1791`: "pre-M8 captures read the name of their time (ADR-0057 §12)". Do not edit the capture.
- [x] (`grep -n "any generation's" TESTING.md`; re-sweep in NOTES) Minor 2: `TESTING.md:1384` backup prefix → any generation's `<dir> <ulid>` (current `mergewright`). Re-sweep TESTING.md with `grep -niE '\b(graft|tributary|trib)\b'`, skipping code paths, crate names and dated rows; list what you changed and what you left.
- [x] (`grep -n 'm8-proto-3a/3c' TESTING.md`) Minor 3: `TESTING.md:3295-3296` → "M8 captures carry the app's name, except the two throwaway prototypes `m8-proto-3a/3c` (records)".
- [x] (`git diff CHANGELOG.md`) Minor 7: `CHANGELOG.md:6-7` → "Entries after the newest name the app as it was called then."
Ticks for these name the grep/diff that shows them.

## NOTES (builder, 2026-10-12)
- Cause fix: git calls now take `env=sandbox_env(config_home=<tmp>/.config, GIT_CONFIG_GLOBAL=os.devnull, GIT_CONFIG_NOSYSTEM="1")`, the sanctioned pattern check-sandbox accepts. The script runs `git add docs`, so the temp config folder stays out of the commit. check-sandbox untouched.
- Minor 2 re-sweep: changed 1384 only. Left 1009 and 1273 (fixture URL `graft.git`), 1342 (quoted capture text), 1903/1905 (code paths), crate names and dated rows.
- No scout: a single 50-line script.
