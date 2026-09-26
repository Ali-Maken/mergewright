# 18d blame-missing-ignore-revs — blame fails when a configured ignore-revs file is missing

Status: done
Blocked by: none.
Crates: graft-git only (`blame.rs`, its tests).
Build with: Sonnet (one flag, one test).
Scout: none.
Split by: conductor, 2026-09-26, from 06c's report. No new decision.

## Why
`crates/graft-git/src/blame.rs:246` passes `--ignore-revs-file=`, and the comment says an empty name clears every list. 06c probed git 2.54: it does **not** clear a configured `blame.ignoreRevsFile` that is missing, and git fails with `fatal: could not open object name list`. Absorb switched to `--no-ignore-revs-file` (06c, `crates/graft-ops/src/absorb/read.rs`). Blame should do the same.

## Acceptance
- [x] (`blame::ignore_revs_false_skips_a_configured_file_that_is_missing`; seen red: `fatal: could not open object name list: no-such-file`, then green) A test with `blame.ignoreRevsFile` set to a missing file, blaming with ignore off: red first (git's fatal), then green.
- [x] (`blame::ignore_revs_true_with_a_missing_configured_file_is_gits_error`: yes, git's own fatal, surfaced through `BlameError::Run`) The `(None, true)` arm: a configured file that is missing still fails. State whether it is git's error, and test it or name the existing test.
- [x] (`src/blame.rs` `(_, false)` arm comment; also `BlameOptions::ignore_revs` doc and MAP.md `blame` line) The comment at :245 says what git actually does.
- [x] Gates workspace-wide: fmt, clippy, nextest 3935/3935, doctests pass.

## Touches
Every `blame(` caller (grep); 20e's blame place reads its errors. Name how each was checked.

## NOTES
- Fix is the cause: `--ignore-revs-file=` replaced by `--no-ignore-revs-file`. No compaction. No scout (ticket said none; one arm).
- Touches: grep `BlameOptions` / `blame(` across crates: only graft-git (examples/blame_longfile.rs, examples/trace_perf.rs, tests/it/blame.rs); trace/log.rs mentions blame only in text. No app-crate caller. Error text for the `true` path unchanged, so 20e's error reading is unaffected (not separately exercised).
