# ops-finish-message-and-step-bound: the user's `#` lines reach the commit; a multi-step operation's conflict set is this step's files only

Status: ready (filed 2026-09-23 after review m6-amend-09)
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
- [ ] `tests/it/finish.rs::a_hash_line_the_user_typed_survives`: on the fixture's merge, `ContinueWith { message: b"Merge feature\n\n#123 fixes login\n" }` → `git log -1 --format=%B` contains the line `#123 fixes login`.
- [ ] `tests/it/finish.rs::gits_comment_lines_never_reach_the_summary`: the fixture's `MERGE_MSG` carries git's `# Conflicts:` block → no line of `finish_summary().message` starts with the comment prefix. Run it with `core.commentChar` unset, and again set to `;`.
- [ ] `tests/it/finish.rs::an_unedited_message_lands_as_shown`: `Resume::Continue` → `%B` equals the `finish_summary().message` read just before, modulo git's trailing-newline normalisation (say which). **If this fails, stop and report.** It means `⏎` on an unedited summary commits text the user never saw, and that is ticket 19's call, not this one's.
- [ ] The Minor 7 assert, exact.
- [ ] `tests/it/conflict_set.rs::a_rebase_lists_only_the_current_steps_files`: `TwoStopRebase`; resolve and stage `a.txt`; `Resume::Continue` stops at step 2 → `operation_conflicts().paths == [b.txt]`. **Write this first and run it against the unchanged code.** NOTES say whether it failed there. That settles review Minor 3, which the reviewer left unverified.
- [ ] `…::a_rebase_step_keeps_its_resolved_files`: at step 2, resolve and stage `b.txt` → the set is still `[b.txt]`, `recorded`.
- [ ] `…::a_merge_set_is_unchanged`: 09's merge ticks stay green; cite their exact test names.
- [ ] `…::a_link_without_a_kind_reads_as_step`.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, workspace-wide.

## Size
Small–medium: one flag and one assert in `resume.rs`; one enum, a message trailer and a walk bound in `chain.rs`/`set.rs`; one fixture; about 8 tests. One context.
