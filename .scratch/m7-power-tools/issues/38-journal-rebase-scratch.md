# 38-journal-rebase-scratch — git's in-between commits from a finished rebase are not lost work, and a lost commit names only a branch that held it

Status: ready
Source: usability walk `docs/reviews/m7-usability.md` M5: after `R s s y` the journal draws `✗ lost 013a232 "# This
is a combination of 2 commits." · was feature-b's tip`; a detached-`HEAD` record that is not a checkout falls back to
the inferred branch (`crates/graft-ops/src/journal_list/rows.rs:208-222` `record_hint` → `Hint::On`) → `held_by`
(`:238-246`) → `HeldBy::Branch` → `was {name}'s tip` (`crates/graft/src/view/journal.rs:222-228`).
Build with: **Opus** — the M3 promise rests on `✗` rows: hiding a real one is the costly mistake, and a passing test
can't show the rule is the right one.
Scout: done (`scouts/38.md`, `9133428c`). No second scout.
Needs: none.
Blocked by: none. **Wave 1, concurrent with 36 and 37** (file sets disjoint, see "File set").

## File set (exactly these; nothing else without a report)
- graft-ops: `src/journal_list.rs` (`find_lost`/`candidates` :211-240, `build` :270-296, one `mod` line), new
  `src/journal_list/scratch.rs`, `src/journal_list/rows.rs`, `src/journal_list/steps.rs` (visibility of `is_rebase`
  :96-98 and the `returning to` parse :290-291 only, for reuse), `src/journal_list/tests.rs` (one `mod` line) + new
  `src/journal_list/tests/scratch.rs` (+ `tests/build.rs` only if an existing test there changes),
  `tests/it/journal_list_oracle.rs`, `tests/it/lost_work.rs` (only if a test there changes), `MAP.md` (`journal_list`
  section :343-355 only).
- graft: `tests/it/journal_screen.rs` (existing file: the capture test; **no** new file, so `tests/it/main.rs` — 36's —
  is not edited). `view/journal.rs`: **not** edited (words unchanged).
- `docs/shots/m7/journal-after-squash-187x44.txt`.
- **Never** (held by 36/37): graft-ops `apply/*`, `mistake_radar/*`, `preview*`, `rewrite/*`, `tests/it/main.rs`;
  graft `src/**`, other `tests/it/*`.

## Read first
- `scouts/38.md` (all).
- `22-journal-list.md` "Open points decided (oracle: git 2.54.0)" (:243-264): the moved-nothing drop, the rebase fold,
  the pair window, `refs/graft/**` never holds.
- graft-ops `MAP.md` `journal_list` (:343-355); `docs/design/02-keymap.md` §2.11 (`n`/`N`).

## What is verified, and what is not (rule 7)
- **Verified:** git writes a `HEAD` record `rebase (finish): returning to refs/heads/<b>` — documented at
  `journal_list/words.rs:77` ("`HEAD`'s"), parsed in both forms at `steps.rs:290-291`, used as a test record at
  `journal_list/tests/build.rs:92`, and seen by ticket 22's oracle run (22 :251-252, git 2.54.0).
- **Not verified:** that this record reaches the records `find_lost` sees. It moves nothing (old = new), and stage 2
  drops moved-nothing records (`steps.rs:14`; 22 :248-249). Records an entry claims may also leave the outside set
  (22 :261-263) — for a Graft `R`, the branch's finish record may be claimed. So the rule below reads the **raw**
  records (every `HEAD` and `refs/heads/*` record stage 2 read, before the drop and before any claim).
  **Stop condition:** if `find_lost`/`build` have no such raw slice in scope, don't add a reflog read: commit at the
  seam, report `blocked: question — raw records not in scope at <file:line>`, and say what the cheapest source is.
- The oracle test quotes git's actual `HEAD` and branch records for the squash and the abort in NOTES. If git's
  messages differ from those below, the test's quote wins and NOTES says where.

## Scope
In, both decided:
1. **Scratch commits.** Candidates stay **old values only** (`candidates` :226) plus entry befores; unchanged. A
   candidate is dropped (not lost) when **every** raw record naming it as old **or** new is a rebase **step** record
   inside a **finished** span of `HEAD`'s reflog:
   - span: opens at `rebase (start)` / `rebase -i (start)`; closes at `(finish)` (finished) or `(abort)`, or the next
     `(start)`, or the log's end (unfinished). Non-rebase records inside a span (a `commit`, `reset`, `checkout`
     during an `edit` stop) don't close it.
   - step record: `rebase (<step>): …` or `rebase -i (<step>): …`, `<step>` ∉ {start, finish, abort}, inside a span
     (reuse `is_rebase`, `steps.rs:96-98`).
   Why safe: a finished rebase's in-between commit is in the result (pick) or melded into it (squash/fixup/reword);
   the before is in the branch's reflog (the branch's `rebase (finish): … onto` record names it — not a step record,
   so it is never dropped by this rule). Kept as lost: anything also named by any other record, and every record of an
   aborted or unfinished span (could be his resolution work; never hidden). Shell and Graft rebases alike.
   `fold_rebases` (`steps.rs:114-134`) is **not** changed: it pairs with the branch's finish, a different question.
2. **Who held it.** The bug is the `Hint::On` (inferred) path. New rule in `rows::held_by` (name kept, :238): an
   inferred branch that **exists** becomes `HeldBy::Branch` only if its own raw reflog names the commit as old or new;
   otherwise `HeldBy::Head` (today's words, `· was HEAD`, `view/journal.rs:228`). A **gone** inferred branch: today's
   rule unchanged (its log went with it; oracle `a_deleted_branch_once_checked_out_is_found_from_head` stays green). A
   `Hint::Branch` from the branch's own record: unchanged.
Out: changing `HeldBy`'s variants or the J1 words; the lost scan's caps; `⏎` recover rules; `fold_rebases`.

## Signatures
```rust
// graft_ops::journal_list::scratch (new file) — pure over the raw records.
pub(crate) fn rebase_scratch(records: &[ReflogRecord]) -> HashSet<Oid>; // commits named only by finished-span step records
// graft_ops::journal_list::rows — name chosen to avoid the existing private `held_by(hint, branch_exists)` (:238).
pub(crate) fn branch_log_names(branch: &RefName, commit: Oid, records: &[ReflogRecord]) -> bool;
// `held_by` keeps its name; it gains the raw records (or a precomputed answer) to call `branch_log_names` in its
// inferred-branch arm. Its exact parameter list is the builder's; private.
```
`find_lost`, `build`: public signatures unchanged (MAP :351-352); `find_lost` drops `rebase_scratch`'s set from its
candidates before any read.

## Acceptance (each box names its test at tick time; red first where marked)
- [ ] `a_finished_squash_leaves_no_lost_row` (red first, real git, `tests/it/journal_list_oracle.rs`): `rebase -i`
      squashing 3 → 1, finished; no `ListRow::Lost` for any in-between commit. NOTES quotes git's records (oracle).
- [ ] `an_aborted_rebase_keeps_its_lost_rows`: same todo with a conflict, `rebase --abort` → in-between commits still
      lost rows.
- [ ] `a_reset_during_an_edit_stop_stays_lost`: an `edit` stop, a commit, `reset --hard HEAD~1`, continue to finish →
      the reset-away commit is a lost row.
- [ ] `rebase_scratch_table` (`journal_list/tests/scratch.rs`): hand-written records, both message forms
      (`rebase -i (squash): …` / `rebase -i (finish): returning to …`), unfinished span, span with a `reset` inside.
- [ ] `a_lost_commit_names_no_branch_whose_reflog_lacks_it` (red first): a detached-`HEAD` record, not a checkout,
      inferred branch `feature-b` existing, its log lacking the commit → `HeldBy::Head`.
      `a_branch_reflog_record_still_names_its_branch` (today's `reset --keep` case unchanged).
- [ ] `a_head_row_still_recovers`: `⏎` on a row this change turned `Head` opens the same recover offer a `Head` row
      gets today (`recover.rs:88-89,238`). If `Head` rows recover nothing today (`view/journal.rs:301-303` draws no
      `⏎` for them), **stop**: part 2 would strip a recover path — report it as a question, ship part 1 alone.
- [ ] Existing journal_list tests green, each changed one named with why.
- [ ] **Capture** `docs/shots/m7/journal-after-squash-187x44.txt` (new test in `graft/tests/it/journal_screen.rs`) —
      **right:** after `R s s y` the journal's newest row is the `●` rewrite row; no `✗` row for the
      `# This is a combination` commit or any squashed pick's copy; `n` (next lost, §2.11) does not stop on them. The
      pre-rewrite tip's row: whatever today's build draws, unchanged, named in NOTES. The test also asserts the
      rewrite's backup ref (`refs/graft/backup/**`) resolves to the pre-rewrite tip. Colour unverified.
- [ ] graft-ops MAP.md (`journal_list`: the scratch rule, `branch_log_names`, the raw-records input).
- [ ] Gates.

## Touches (check each; report how)
- **The `✗` count and `n`/`N` (§2.11):** fewer stops after a rebase. Check: J's tests green; list counts that moved.
- **`recover_for` / `OfferBranch`:** see `a_head_row_still_recovers`. Check: recover tests green.
- **The M7 acceptance lost-work scenario** (`J ⏎ y` after a shell `reset --keep`): unchanged; its test green.
- **Lost scan cost** (30p): `rebase_scratch` is one pass over records already read; no spawn added. Say so.
- **`fold_rebases`** and the rebase row's words: unchanged; `a_shell_rebase_folds_into_one_row` green.
- **Captures, keymap, undo journal file:** none besides the new capture.

## Gates
Workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`,
`cargo test --doc`. Worktree only, own `CARGO_TARGET_DIR`. Never read `PROGRESS.md` or `KNOWN_GAPS.md`.

## Size
~40k. One context.
