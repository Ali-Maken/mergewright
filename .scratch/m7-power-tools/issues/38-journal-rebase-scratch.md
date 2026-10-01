# 38-journal-rebase-scratch — git's in-between commits from a finished rebase are not lost work, and a lost commit names only a branch that held it

Status: done
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
- [x] `a_finished_squash_leaves_no_lost_row` (red first, real git, `tests/it/journal_list_oracle.rs`): `rebase -i`
      squashing 3 → 1, finished; no `ListRow::Lost` for any in-between commit. Red observed: `✗ # This is a
      combination of 2 commits. · main · under 0`. Green: `journal_list_oracle::a_finished_squash_leaves_no_lost_row`.
      git's records quoted in NOTES.
- [x] `an_aborted_rebase_keeps_its_lost_rows` (`journal_list_oracle`): conflict on D, `rebase --abort` → the squash
      (`HEAD@{1}`) and the pick's copy (`HEAD@{2}`) are both lost-row tips.
- [x] `a_reset_during_an_edit_stop_stays_lost` (`journal_list_oracle`): `edit` stop, commit E, `reset --hard HEAD~1`,
      `--continue` to finish → E is a lost row.
- [x] `rebase_scratch_table` (`journal_list/tests/scratch.rs`): both forms (`rebase -i (…)` / `rebase (…)`),
      aborted, unfinished, restarted, `edit` stop with commit + `reset` inside, no `(start)` in the log; plus
      `rebase_scratch_table_another_log_names_it` (a branch record naming an interim keeps it).
- [x] `a_lost_commit_names_no_branch_whose_reflog_lacks_it` (red first, observed failing before the `rows.rs` change):
      a rebase step record on detached `HEAD`, inferred `feature-b` existing, its log lacking the commit →
      `HeldBy::Head`. `a_branch_reflog_record_still_names_its_branch` green. Also
      `an_inferred_branch_whose_log_has_it_names_it`, `a_gone_inferred_branch_still_names_itself` (both
      `tests/scratch.rs`).
- [x] `a_head_row_still_recovers` (`tests/scratch.rs`): `recover_for` on the row turned `Head` →
      `OfferBranch { at, name: "restored", why: "no branch held it, only HEAD" }`, the `Head` path at
      `recover.rs:238-240`. Stop **not** hit: `Head` rows do recover (`⏎` → `keys.rs` `enter` → `RecoverFor` →
      offer). The view draws no `⏎` hint on them (`view/journal.rs:303`) — see NOTES.
- [x] Existing journal_list tests green, none changed (`cargo nextest run -p graft-ops`: 984/984 before the gates;
      workspace run below).
- [x] **Capture** `docs/shots/m7/journal-after-squash-187x44.txt` — `journal_screen::capture_journal_after_squash`
      (child `squash_capture_git_clock_pinned`, git's clock pinned to J's). Asserts: newest row selected is `●`
      rewrite; no `# This is a combination` anywhere; every `✗ lost` row is the pre-rewrite tip; `n` from the top
      lands on the pre-rewrite tip's row or stays. Red observed with the `find_lost` filter disabled: the frame gained
      `└ ✗ lost ce8254c "# This is a combination of 2 commits." · was HEAD`. Backup-ref assertion changed — see
      NOTES. Colour unverified.
- [x] graft-ops MAP.md `journal_list`: `find_lost` line (scratch rule, raw records) + a lost-placement line
      (`branch_log_names`).
- [x] Gates: `cargo fmt --check` ok, `cargo clippy --all-targets -- -D warnings` ok, `cargo test --doc` ok,
      `cargo nextest run` 5023/5024 — the one fail `graft-ops::it set_aside::a_stop_before_each_step_loses_nothing`
      (9.5 s, under load from concurrent builders); passes alone and 3× in `set_aside` reruns (68/68). Not touched by
      this ticket (no `journal_list` use in `set_aside.rs`). Failure message not captured — unverified flake.

## NOTES (builder, 2026-10-01)
- Scout used: `scouts/38.md`. No compaction.
- **Stop 1 (raw records) not hit:** `find_lost` and `build` both take `outside: &[OutsideRecord]`, `read_outside`'s
  full read before `steps` drops or entries claim anything (`journal_list.rs` `find_lost`, `build`).
- **git 2.54.0's records (oracle `eprintln`):**
  - Finished squash (`rebase -i HEAD~3`, `2,3s/^pick/squash/`), `HEAD` newest first: `rebase (finish): returning to
    refs/heads/main`, `rebase (squash): B`, `rebase (squash): # This is a combination of 2 commits.`, `rebase
    (start): checkout HEAD~3`. No `pick` record: B fast-forwards. `main`: `rebase (finish): refs/heads/main onto
    <sha>`. Modern git writes `rebase (…)`, not `rebase -i (…)`, for `-i` too.
  - Abort: `rebase (abort): returning to refs/heads/main`, `rebase (squash): B`, `rebase (pick): B`, `rebase (start):
    checkout side`. The squash's subject is "B", not "# This is a combination": it is its group's last squash.
  - Edit stop: `rebase (finish): …`, `rebase: fast-forward`, `reset: moving to HEAD~1`, `commit: E`, `rebase:
    fast-forward`, `rebase (start): checkout HEAD~2`. **git writes `rebase: fast-forward` with no parentheses** for a
    fast-forwarded pick. `is_rebase` doesn't match it, so it is no step record (and doesn't close a span). Safe: a
    fast-forward's old is the original parent, never an interim. Knock-on not in scope: `fold_rebases` also misses
    these records (a fast-forwarding shell rebase's `HEAD` run breaks there) — a question for J's `$` rows, not filed.
- **Pre-rewrite tip's row (capture), unchanged by this ticket:** `└ ✗ lost 90ebf9e "b3" · was feature-b's tip · no
  branch holds it · +2 under it` under the `●` rewrite row; `n` stops on it.
- **Spec deviation — backup ref:** the ticket asked that `refs/graft/backup/**` *resolves to* the pre-rewrite tip.
  It doesn't: a backup ref is a snapshot commit (`{index/, worktree/}` tree) whose parents are the entry's pins
  (`graft-ops/src/snapshot.rs:19-21, 223`). The test asserts instead that a backup ref **contains** it
  (`for-each-ref --contains <tip> refs/graft/backup/`), which held.
- **Signature deviations (private, say so):** `rebase_scratch` and `branch_log_names` take
  `impl IntoIterator<Item = &ReflogRecord>` (callers hold `&[OutsideRecord]`; no copy) and are `pub(super)` like the
  module's other helpers. `branch_log_names` takes the full refname as `&str`, not `&RefName`: the records' refnames
  are strings, and building a `RefName` adds a fallible conversion with nothing to check.
- Entries' befores are not consulted by the scratch rule (spec: "every raw record"). Every Graft op writes reflog
  records too, so an entry's before is some non-step record's old; checked by reasoning, not by a test.
- `Head` rows have no `⏎` hint in the bar (`view/journal.rs:303`) though `⏎` offers a branch on them — more rows
  now take that path. Visibility gap, not in this file set; for the conductor.
- Touches: `✗` count/`n` — fewer stops after a finished rebase (capture: one `✗`, was two); J tests green.
  `recover_for` — `recover` tests green + `a_head_row_still_recovers`. M7 lost-work scenario —
  `m7_walk::journal::the_three_lost_work_journeys` green. Scan cost — one pass over records already in memory, no
  read, no spawn. `fold_rebases` unchanged (`a_shell_rebase_folds_into_one_row` green). Captures: only the new one;
  existing J captures unchanged (their tests green). Keymap, undo journal: untouched.

