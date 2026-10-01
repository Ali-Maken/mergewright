# 38-journal-rebase-scratch — git's in-between commits from a finished rebase are not lost work, and a lost commit names only a branch that held it

Status: ready
Source: usability walk `docs/reviews/m7-usability.md` M5: after `R s s y` the journal draws `✗ lost 013a232 "# This
is a combination of 2 commits." · was feature-b's tip`; a detached-`HEAD` record that is not a checkout falls back to
the inferred branch (`crates/graft-ops/src/journal_list/rows.rs:207-221`) → `HeldBy::Branch` (`:238-243`) → `was
{name}'s tip` (`crates/graft/src/view/journal.rs:222-223`).
Build with: **blank — scout first.** Suggested after the scout: Opus (the M3 promise rests on `✗` rows: hiding a real
one is the costly mistake, and a passing test can't show the rule is the right one).
Scout: **yes, one, ≤1 page**: how `find_lost` picks candidates and what each `OutsideRecord` carries (message, which
reflog); how `build` already folds a shell rebase's records (its span detection: start/finish markers, both message
forms); `rows.rs:207-243`; what `HeldBy::Head` draws in `view/journal.rs` (exact words).
Needs: none.
Blocked by: none
Crates / files (wave 1): graft-ops `journal_list/` only (+ its tests). graft: **one new** integration test file for the
capture, nothing else (its `mod` line in `tests/it/main.rs` is the only shared line; the conductor merges it).

## Scope
In, both decided:
1. **Scratch commits.** A candidate is **not** lost when every record naming it (as old or new) is a rebase **step**
   record — `rebase (<step>): …` or the older `rebase -i (<step>): …`, `<step>` ∉ {start, finish, abort} — inside a
   rebase span in `HEAD`'s reflog that reached `rebase (finish)`. Why safe: a finished rebase's in-between commit is
   either in the result (pick) or melded into it (squash/fixup/reword); the before is in the branch's reflog and, for a
   Graft rewrite, its backup ref. Kept as lost: anything also named by any other record (a `reset`, `commit (amend)`,
   `checkout` during an `edit` stop), and every record of an **aborted** or unfinished rebase (could be his resolution
   work; never hidden). Applies to shell and Graft rebases alike (both run git's `rebase -i`).
2. **Who held it.** `HeldBy::Branch { name }` only when that branch's own reflog has a record with the commit as old
   or new. Otherwise `HeldBy::Head` (today's words for it, unchanged).
Out: changing `HeldBy`'s variants or the J1 words; the lost scan's caps; `⏎` recover rules.

## Signatures
```rust
// graft_ops::journal_list (rows.rs or a sibling) — pure over the records stage 2 read.
pub(crate) fn rebase_scratch(records: &[OutsideRecord]) -> HashSet<Oid>; // commits named only by finished-rebase step records
pub(crate) fn held_by(commit: Oid, inferred: Option<&RefName>, records: &[OutsideRecord]) -> HeldBy; // Branch only if its reflog names `commit`
```
`find_lost`, `build`: signatures unchanged (MAP :351-352); `find_lost` drops `rebase_scratch` from its record-sourced
candidates before any read.

## Acceptance (each box names its test at tick time; red first where marked)
- [ ] `a_finished_squash_leaves_no_lost_row` (red first, real git via fixture): `rebase -i` squashing 3 → 1, finished;
      no `ListRow::Lost` for any in-between commit. Quote git's reflog messages in NOTES (the oracle).
- [ ] `an_aborted_rebase_keeps_its_lost_rows`: same todo with a conflict, `rebase --abort` → in-between commits still
      lost rows.
- [ ] `a_reset_during_an_edit_stop_stays_lost`: an `edit` stop, a commit, `reset --hard HEAD~1`, continue to finish →
      the reset-away commit is a lost row.
- [ ] `the_older_rebase_i_messages_count_too`: hand-written records `rebase -i (squash): …` / `rebase -i (finish): …`.
- [ ] `a_lost_commit_names_no_branch_whose_reflog_lacks_it` (red first): the walk's world → `HeldBy::Head`, not
      `Branch { feature-b }`. And `a_branch_reflog_record_still_names_its_branch` (today's `reset --keep` case unchanged).
- [ ] Existing journal_list tests green, each changed one named with why.
- [ ] **Capture** `docs/shots/m7/journal-after-squash-187x44.txt` (the new graft test) — **right:** after `R s s y` the
      journal's newest rows are the `●` rewrite row and nothing marked `✗`; `n` (next lost, §2.11) does not stop on a
      `# This is a combination` row. Colour unverified.
- [ ] graft-ops MAP.md (`journal_list`: the scratch rule, `held_by`).
- [ ] Gates.

## Touches (check each; report how)
- **The `✗` count and `n`/`N` (§2.11):** fewer stops after a rebase. Check: J's tests green; list counts that moved.
- **`recover_for` / `OfferBranch`:** a row now `Head` offers a branch by its existing `Head` rule. Check: recover
  tests green.
- **The M7 acceptance lost-work scenario** (`J ⏎ y` after a shell `reset --keep`): unchanged. Check: its test green.
- **M3 promise:** nothing reachable only through a hidden row: the rewrite's backup ref (`refs/graft/backup/**`) holds
  the before. Check in the squash test: the backup ref resolves to the pre-rewrite tip.
- **Captures, keymap, undo journal file:** none besides the new capture.

## Gates
Workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`,
`cargo test --doc`. Worktree only, own `CARGO_TARGET_DIR`. Never read `PROGRESS.md` or `KNOWN_GAPS.md`.

## Size
~35k. One context.
