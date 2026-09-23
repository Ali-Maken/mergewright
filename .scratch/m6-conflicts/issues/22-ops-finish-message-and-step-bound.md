# ops-finish-message-and-step-bound: the user's `#` lines reach the commit; a multi-step operation's conflict set is this step's files only

Status: done (2026-09-23)
Blocked by: none (09 merged)
Was: draft (2026-09-23, from `docs/reviews/m6-amend-09.md` Q1 = Minor 1 and Q2 = Minor 3; ruled by the conductor and the planner the same day)
Blocked by: none (09 is merged). **Blocks** `13-resolver-file-list.md` and `19-finish-summary-screen.md`.
Build with: **Opus**. It changes how the backup chain is encoded (a git-data path: backup commits) and the commit-message cleanup mode. Tests against real git judge both. Neither is a cross-crate interface, so this is not a signature ticket.
Needs: none.
Crates: graft-ops, graft-fixtures (one new `Shape`).

## Read first
- `docs/reviews/m6-amend-09.md`: "Git-data safety of `ContinueWith`", and Minor 1, 3 and 7.
- `docs/design-notes/m6-conflicts.md`: the Interfaces lines for `operation_conflicts`, `finish_summary` and `Resume` (around lines 528-543), and Hard choice 3.
- `crates/graft-ops/MAP.md`: `ConflictSet`, `conflict/merge_msg.rs` (`without_comments`), `chain_backup`, `recorded_index_state`, `backup_chain`, `apply::stage_source`, `Resume`.
- `crates/graft-fixtures/MAP.md`: `Shape::StoppedRebase`, the model for the new shape.

## Scope
In:
1. **Q1: keep the user's text verbatim.**
   - `Resume::ContinueWith` commits with `git commit --cleanup=whitespace -F -`, no longer `--cleanup=strip` (`apply/resume.rs:183`).
   - git's comment lines are removed once, when the message is **read**: `finish_summary`'s `message` already passes through `merge_msg::without_comments`. Keep that, and pin it with a test. Any `#` line in what Graft then sends is the user's own, and git keeps it.
   - Minor 7: tighten the assert to the exact command, `assert_eq!(…last(), Some("git commit --cleanup=whitespace -F -"))`.
   - Correct `Resume::ContinueWith`'s doc comment to match.
2. **Q2: a multi-step operation's set covers the current step only.**
   - Each backup-chain link records why it was written:
     - `LinkKind::Step` for a resolver write (`apply/fold.rs:108`);
     - `LinkKind::Resume` for every link `apply/resume.rs` chains (the review cites the one before each `--continue`, `resume.rs:174`).
   - The kind is written into the backup commit's message as its own trailer line: `Graft-Link: step` or `Graft-Link: resume`.
   - A link without that line (written before this change) reads as `Step`.
   - The chain part of `operation_conflicts` (`conflict/set.rs:153`) walks newest first and **stops at the first `Resume` link**, leaving it out. Only links written since the current step stopped count.
   - A merge has no `Resume` link before its finish, so its set is unchanged.
   - `recorded` keeps its meaning, now over the shorter walk.
   - `apply::stage_source`, the other reader MAP.md names for the chain, is **unchanged**. Look at whether it needs the same bound too. If it does, stop and report it: that is a separate question.
3. **Fixture.** Add `Shape::TwoStopRebase`: a rebase of 2 commits where step 1 conflicts on `a.txt` and step 2 on `b.txt`, left stopped at step 1.

Out:
- The empty-message refusal (review Minor 2): it is safe as it stands, and ticket 19 already refuses an empty subject in the app.
- `by_git` mapping tests (Minor 6).
- A `Stop` parameter on either read (Minor 5): the app calls both from a job.

## Signatures
```rust
// graft-ops/src/snapshot/chain.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkKind { Step, Resume }
// chain_backup: keeps its existing parameters and return type, plus a trailing `link: LinkKind`.
// backup_chain: every item it yields gains its LinkKind, as the pair (existing item, LinkKind).
// graft-ops/src/apply/resume.rs: no signature change; ContinueWith's invocation changes as above.
// graft-fixtures: Shape::TwoStopRebase
```

## Acceptance (name the verifying test when you tick; truth from git)
- [x] `tests/it/finish.rs::a_hash_line_the_user_typed_survives`: on the fixture's merge, `ContinueWith { message: b"Merge feature\n\n#123 fixes login\n" }` → `git log -1 --format=%B` contains the line `#123 fixes login`.
- [x] `tests/it/finish.rs::gits_comment_lines_never_reach_the_summary` (unset and `;` in one test; each asserts git's block is in `MERGE_MSG` first, so it is not vacuous): the fixture's `MERGE_MSG` carries git's `# Conflicts:` block → no line of `finish_summary().message` starts with the comment prefix. Run it with `core.commentChar` unset, and again set to `;`.
- [x] `tests/it/finish.rs::an_unedited_message_lands_as_shown` — passed **byte for byte** against `cat-file commit HEAD`'s body, no normalisation needed: both end in exactly one `\n`: `Resume::Continue` → `%B` equals the `finish_summary().message` read just before, modulo git's trailing-newline normalisation (say which). **If this fails, stop and report.** It means `⏎` on an unedited summary commits text the user never saw, and that is ticket 19's call, not this one's.
- [x] The Minor 7 assert, exact: `tests/it/finish.rs::an_edited_message_is_the_commit_message` (`assert_eq!(…last(), Some("git commit --cleanup=whitespace -F -"))`; its body check now compares with `git stripspace` without `--strip-comments` and asserts `# a comment line` landed).
- [x] `tests/it/conflict_set.rs::a_rebase_lists_only_the_current_steps_files`: `TwoStopRebase`; resolve and stage `a.txt`; `Resume::Continue` stops at step 2 → `operation_conflicts().paths == [b.txt]`. **Write this first and run it against the unchanged code.** NOTES say whether it failed there. That settles review Minor 3, which the reviewer left unverified.
- [x] `tests/it/conflict_set.rs::a_rebase_step_keeps_its_resolved_files` (also with `MERGE_MSG` removed, so the chain alone is judged): at step 2, resolve and stage `b.txt` → the set is still `[b.txt]`, `recorded`.
- [x] `tests/it/conflict_set.rs::a_merge_set_is_unchanged` (new), plus 09's unchanged and green: `conflict_set.rs::a_merge_lists_resolved_files_too`, `a_graft_step_is_remembered_without_git_s_list`, `a_relaunch_lists_the_same_files`, `each_kind_says_what_git_records`.
- [x] `tests/it/conflict_set.rs::a_link_without_a_kind_reads_as_step` (also: the writer puts `Graft-Link: step` last; the same link relabelled `resume` bounds the walk). Shape: `graft-fixtures` `shapes::conflicts::tests::two_stop_rebase_stops_at_step_one_on_a_alone`.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, workspace-wide.

## Size
Small–medium: one flag and one assert in `resume.rs`; one enum, a message trailer and a walk bound in `chain.rs`/`set.rs`; one fixture; about 8 tests. One context.

## NOTES (builder, 2026-09-23)
- **No new scout**, on the conductor's word: read `scout-09.md`, 09's NOTES and `docs/reviews/m6-amend-09.md` instead. No compaction.
- **Minor 3 was real.** `a_rebase_lists_only_the_current_steps_files` on the unchanged code: `left: ["b.txt", "a.txt"]`, `right: ["b.txt"]` — step 1's chain link leaked `a.txt` into step 2's set. Fixed at the cause (the walk bound), not by filtering paths.
- **Trailer**: the kind is the backup commit message's last line; the walk reads it with one `cat-file commit` per link, which also gives the first parent (replaces the `rev-list --parents` spawn, so no extra spawn per link). An unknown value (`Graft-Link: foo`) is `SnapshotError::UnknownLink`, not read as `Step`: a later Graft's kind guessed wrong would mis-bound the set (rule 3).
- **Fixture**: `Shape::TwoStopRebase` is in a new `Shape::STEPPED`, **not** `STOPPED` — `graft-git/src/conflict/tests.rs:685` iterates `STOPPED` expecting the marker files, and `graft-git` is outside this crate set. graft-ops tests fetch the shape's `main`/`feature` into a fresh repo so the rebase is Graft's own (entry + chain); the corpus's stopped repo is shell-begun and has no chain.
- **Question (not changed, per scope): `apply::stage_source` walks the whole chain.** At step 2 of a rebase, `d` on a path step 1 conflicted on and step 2 did not: `check_restart` admits a path status does not call unmerged (MAP.md `RestartParams`), and `stage_source` would find step 1's stages on the chain and restore a finished step's conflict into this one. Not tested; the app only offers `d` on set paths, which no longer list it. Whether `stage_source` takes the same bound is the conductor's call.
- **Edge, not a defect:** a `resume` that git refuses (hook) and leaves at the same step still wrote a `Resume` link, so the chain part of the set is empty afterwards; git's `Conflicts:` list (written for every kind that has a `resume`) still names the files. Not tested.

## Review fix M1 (builder, 2026-09-23)
- **No scout**: ticket 22's own code, written today; read the review's M1 and `restart.rs`/`set.rs`/`chain.rs` directly. No compaction.
- **Red first.** `tests/it/conflict_set.rs::a_restart_never_reaches_a_finished_step` (the reviewer's probe: `at_step_two()`, `RestartConflict { a.txt }`, expect `Refusal::NoStagesKept`, `a.txt` unchanged, no `UU a.txt`) on unchanged code: `expected NoStagesKept, got Ok(Applied { .. commands: ["git update-index -z --index-info", "git --literal-pathspecs checkout --merge -- a.txt"] ..})` — the restore ran. Passes after the fix.
- **Guard**: `a_restart_reaches_the_current_steps_file` — `b.txt` staged at step 2 still restarts from step 2's own link (markers back). Passed before and after.
- **Fix, at the cause:** one bound in one place. New `snapshot::current_step_chain` (`snapshot/chain.rs`): the chain walk cut at the first `LinkKind::Resume` link, left out. `apply::restart::chain_states` and `conflict::set::chained_conflicts` both read through it, so the list and the restore cannot disagree again. `backup_chain` stays whole (tests read kinds through it).
- **Other chain-walk callers:** none left unbounded. `backup_chain` has no other caller in `crates/` outside graft-ops tests; nothing outside graft-ops walks the chain. `journal_states` (no live operation: stash pop) is not a chain walk and has one step per entry — unchanged.
- **Not checked:** the review's open question — whether a resolver left open across a resume can still sit on a finished step's path in the app (graft crate, outside this set). With this fix, `d` there now refuses `NoStagesKept` instead of restoring.
