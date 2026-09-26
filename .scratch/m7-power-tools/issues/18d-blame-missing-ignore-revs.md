# 18d blame-missing-ignore-revs — blame fails when a configured ignore-revs file is missing

Status: ready
Blocked by: none.
Crates: graft-git only (`blame.rs`, its tests).
Build with: Sonnet (one flag, one test).
Scout: none.
Split by: conductor, 2026-09-26, from 06c's report. No new decision.

## Why
`crates/graft-git/src/blame.rs:246` passes `--ignore-revs-file=`, and the comment says an empty name clears every list. 06c probed git 2.54: it does **not** clear a configured `blame.ignoreRevsFile` that is missing, and git fails with `fatal: could not open object name list`. Absorb switched to `--no-ignore-revs-file` (06c, `crates/graft-ops/src/absorb/read.rs`). Blame should do the same.

## Acceptance
- [ ] A test with `blame.ignoreRevsFile` set to a missing file, blaming with ignore off: red first (git's fatal), then green.
- [ ] The `(None, true)` arm: a configured file that is missing still fails. State whether it is git's error, and test it or name the existing test.
- [ ] The comment at :245 says what git actually does.
- [ ] Gates workspace-wide.

## Touches
Every `blame(` caller (grep); 20e's blame place reads its errors. Name how each was checked.
