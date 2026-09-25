# set-aside-around-rebase-and-switch: a rebase or checkout on a dirty tree sets the edits aside and puts them back, one entry

Status: partial — built, one graft test red pending a decision (see NOTES, "Blocked")
Blocked by: 72, 72b
From: pick 16 (`.scratch/m6-conflicts/user-picks-2026-09-24.md` :25); review N4, second half (`docs/reviews/m6-usability-final.md`
:158-160).
Build with: **Fable** (conductor, 2026-09-25, after the scout) → `builder-fable`: a git write whose inverse must be
proven; a wrong step order loses edits and no diff shows it (design note "Genuinely risky")
Scout: `.scratch/m6-conflicts/scouts/set-aside-around-rebase-and-switch.md` — brief below.
Needs: 72 (`PutBack`, `PutBackWhy::SetAside`, `PendingLine::put_back`, `ApplyError::Clashed`, `Clash`, `live_clash`,
`DropPutBack`); 72b merged (its `Clashed` arm and `refresh_clash` receive a set-aside clash with no change here);
67 (`plan` runs `check` first).
Crates: graft-ops (operation check, apply sequence, resume, recover, inverse, preview plan); graft-git only if the scout
finds no `stash create` / `stash store` / `stash apply <commit>` wrapper; graft **tests only**: 67's app test for `r` on a
dirty tree, which this ticket's `check` change flips (below). No other graft file.
Group: **F1**, the write. 74 (app and words) follows it.

## Amended 2026-09-25 (planner, after scout 72)
- E split into 72 (ops) and 72b (app). The ops shapes this ticket uses are 72's, unchanged. A set-aside clash is
  answered `Clashed { why: SetAside }` from inside Rebase/Checkout's own apply (step 6), not from 72's stash failure
  branch; the entry it writes (`index_before` merged, `index_after` unmerged, `put_back` set) is what 72b's
  `refresh_clash` → `live_clash` finds. The app lands it through 72b with stash words until 74 rewords them.

## Read first
- `docs/adr/0045-stash-put-back-is-recorded-and-one-entry.md` — all of it. §4–§7 are this ticket's spec.
- `docs/design-notes/m6-stash-put-back.md` "Hard-to-reverse choices", "Interfaces".
- `docs/research/commit-stash.md` addendum A1.2, A1.5, A1.6.
- `.scratch/m6-conflicts/scouts/67.md` :9-15 (where `DirtyTree` is refused; pull has no clean-tree rule).
- `crates/graft-ops/MAP.md` :30 (`is_history`), :47-57 (stash kinds), :52-53 (`DirtyTree`, `check`), :86-94 (journal),
  :103-116 (inverse, `InversePlan` pins, `RepoStateNow`), :161-179 (apply, resume, `recover_interrupted`), :198-212
  (`preview::plan`).
- `crates/graft-git/MAP.md` §stash.
- 72's MAP.md entries for `PutBack`, `Clashed`, `live_clash`, the stash failure branch (the index read to copy).

## Decided (ADR-0045 — not open here)
- **Which:** `Rebase` and `Checkout` with tracked changes. Untracked files stay; the backup covers them (ADR-0009).
  Branch create-with-switch (`history.rs:509`) and branch-from-stash (`:557`) keep `DirtyTree`. Plain `Pull` has no
  clean-tree rule and no preview (scout 67 :11): unchanged; a fast-forward over overlapping edits stays git's refusal.
- **Order**, one journal entry: 1 `stash create` → S · 2 snapshot + Pending with `put_back = SetAside(S)`, `pins ∋ S`,
  `touches ∋ refs/stash` · 3 `stash store -m "graft: your edits, set aside for <description>" S` · 4 `reset --hard` ·
  5 the op's own plan · 6 `stash apply <S>` without `--index`, then read the index whatever the exit code: clean → drop
  S by id, Committed; unmerged → keep S, Committed, `ApplyError::Clashed { why: SetAside }`; other failure → keep S,
  `ApplyError::Git` whose words name `stash@{n}`.
- **Stop at 5** (rebase conflict): entry stays Pending, S in the list. `resume` runs 6 after `--continue` finishes, after
  a final `--skip`, and after `--abort` (onto the original tip). **Refused at 5** (git refused): 6 onto the unchanged
  tip, then the refusal.
- **Undo:** generic inverse + `StashDrop(S)` only when this entry's `refs/stash` moved to S. **Redo:** after-state +
  `StashStore(S)` only when the undo's `refs/stash` moved. Pending: not undoable (`a` is the way back).
- **Recover** (killed mid-way): `recover_interrupted` closes it as today; the interrupted entry keeps `put_back`, so the
  app can name `stash@{n}` (74).

## Scout brief (≤1 page, code not intent)
1. `graft-git` stash wrappers: `create`, `store -m`, `apply <commit>`, `drop` by index; which exist, their signatures.
2. `apply`'s shape for `Rebase` and `Checkout` (`operation/history.rs:460-501`, `apply.rs:193-262`): where `check` runs,
   where the snapshot and `begin` happen, where the op's own commands run. Is there one place to wrap steps 1–4 before
   and 6 after? How `stopped_on_conflict` (`apply.rs:361-377`) sees a rebase that stops with S aside.
3. `resume` (Continue / Skip / Abort): where a rebase finishing, being skipped to the end, or aborted returns; does it
   read the Pending line (so `put_back` is reachable after a relaunch)?
4. `recover_interrupted` and `Journal::mark_interrupted`: do they carry the Pending line's fields onto the closed entry?
5. `inverse` for a Committed entry whose `touches` includes `refs/stash`: what it emits today (push/pop precedent,
   m6-conflicts/07 F2); where the "refs/stash moved" test lives.
6. The `Stop` trait: how existing tests inject a stop before a given step (list one test to copy).
7. `preview::plan` for Rebase and Checkout: where `PreviewPlan` is built.
8. Every test asserting `DirtyTree` for rebase or checkout (67's, as merged, and older ones: grep), in which crate.
9. What `is_history` is computed from (a kind list, or the refusal set its MAP line describes).

## Scope
**In:** `sets_aside`; `check` no longer refusing `DirtyTree` for Rebase/Checkout; the six-step sequence; resume and
recover carrying it; inverse and redo; `PreviewPlan::set_aside`; the tests below; the flipped tests' expectations.
**Out:** app words, landing, preview row, walks, captures (74); `--index` on put-back (rejected, ADR); pull (above).

## Signatures
```rust
// graft-ops
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SetAside { pub files: usize, pub staged: usize }   // tracked files changed; of those, staged
impl Operation {
    /// ADR-0045 §4: Some for Rebase and Checkout when tracked files are changed, else None. Pure over the facts.
    pub fn sets_aside(&self, facts: &RepoFacts) -> Option<SetAside>;
}
pub struct PreviewPlan { /* … */ pub set_aside: Option<SetAside> }
// `ApplyError::Clashed { entry, clash: Clash { why: PutBackWhy::SetAside, .. } }` — from 72, unchanged.
// graft-git (only if missing; scout Q1): the wrapper names the scout finds, or
pub fn create(repo: &RepoHandle, message: &str, stop: &dyn Stop) -> Result<Option<Oid>, RunError>; // None: nothing to set aside
pub fn store(repo: &RepoHandle, commit: Oid, message: &str, stop: &dyn Stop) -> Result<(), RunError>;
pub fn apply_commit(repo: &RepoHandle, commit: Oid, stop: &dyn Stop) -> Result<RunOutcome, RunError>; // no --index
```

## The failing tests first (real repos unless marked pure)
- `sets_aside_table` (pure): Rebase/Checkout × clean / tracked-only / staged+unstaged / untracked-only → counts or None;
  Merge, Pick, Revert, Reset → None.
- **Round trip** `set_aside_round_trips` — a table of trees: staged + unstaged + untracked; a staged new file; a
  deletion; a rename. For Rebase and for Checkout: apply → undo leaves index tree, working-tree tree and stash list equal
  to before; redo equal to after. (ADR §7.)
- **Stop injection** `a_stop_before_each_step_loses_nothing` — a stop before steps 1–6 in turn: the edits are in the
  tree or in the stash list, never neither, asserted by reading both. (ADR §7, §5 table.)
- `a_clean_put_back_drops_the_set_aside_stash` — stash list as before; entry Committed.
- `a_clashing_put_back_is_clashed_and_keeps_the_stash` — `Clashed { why: SetAside }`, entry Committed, index unmerged,
  S in the list; `live_clash` over the journal it wrote finds it.
- `a_put_back_that_fails_without_conflict_keeps_the_stash` — an untracked file in the way: `Git`, S listed, words name
  it. (ADR §7: the index decides, not the exit code.)
- `a_rebase_that_stops_keeps_the_edits_aside_until_it_ends` — Pending, S listed; `resume(Continue)` → put back; the
  same with `Abort` (edits back on the original tip) and a final `Skip`.
- `a_refused_rebase_puts_the_edits_back_on_the_same_tip`.
- `relaunch_after_a_kill_keeps_put_back_on_the_entry` — `recover_interrupted`; the closed entry has `put_back`.
- `undo_after_a_clashed_put_back_drops_the_duplicate_stash`; `redo_after_that_undo_stores_it_again`.
- **Flipped from 67** (renamed in place, never disabled; names as merged, from 67's NOTES):
  - graft-ops `a_rebase_preview_refuses_a_dirty_tree` → `a_rebase_preview_on_a_dirty_tree_says_what_is_set_aside`
    (`set_aside: Some(SetAside { files: 1, staged: 0 })`, no refusal); the same for checkout.
  - graft-ops `every_preview_refuses_exactly_what_check_refuses`: unchanged, green with the new `check`.
  - graft: 67's `r`-on-a-dirty-tree test → `r_on_a_dirty_tree_opens_the_preview` (the box opens; no refusal status).
    74 asserts the row's words; this ticket asserts only that the box opens.

## Touches (each entry, and how the builder checks it)
- **`Operation::check`** — `DirtyTree` gone for Rebase/Checkout only. Check: `grep -n "DirtyTree" crates/graft-ops/src`,
  each remaining site named in NOTES; 67's table green.
- **Tests asserting a dirty rebase/checkout refusal** (scout Q8): each flipped or named in NOTES. One outside this
  crate set → **stop**, `blocked: needs <crate>`, naming the test.
- **`is_history`** (MAP :30 defines it by the refusal set): semantics unchanged. Check: scout Q9; its MAP line reworded.
- **Journal**: `put_back` written for SetAside; `pins ∋ S` keeps S past gc. Check: round-trip test reads the line.
- **Undo / redo / drift** (m6-conflicts/07 F2, `InversePlan`): Check: the undo tests above plus the existing undo suite.
- **`resume` / `recover_interrupted`**: Check: the stop and relaunch tests above; existing resume suite green.
- **72's `live_clash`**: a SetAside clash satisfies its rule. Check: the clash test above.
- **72b's app routing**: a `Clashed { SetAside }` lands through `land_on_clash` with stash words until 74. Check:
  workspace gates green at this hash; no graft source file in the diff.
- **Radar (ticket 66)**: S lives in `refs/stash` → the allowlist already skips it. Check: 66's
  `duplicates_skip_the_stash` green.
- **67's capture** `docs/shots/m6/fix-rebase-refused-before-preview-120x40.txt`: now untrue. 74 replaces it; NOTES flags it.
- **Keymap / hint bars**: none here. **Captures**: none written here.
- **MAP.md**: graft-ops `sets_aside`, `SetAside`, the sequence, `check`, resume/recover/inverse lines, `PreviewPlan`.

## Acceptance (name the verifying test at tick time)
- [x] Every test above, green: `operation::history::tests::sets_aside_table`; `tests/it/set_aside.rs`:
      `set_aside_round_trips`, `a_stop_before_each_step_loses_nothing`, `a_clean_put_back_drops_the_set_aside_stash`,
      `a_clashing_put_back_is_clashed_and_keeps_the_stash`, `a_put_back_that_fails_without_conflict_keeps_the_stash`,
      `a_rebase_that_stops_keeps_the_edits_aside_until_it_ends`, `a_refused_rebase_puts_the_edits_back_on_the_same_tip`,
      `relaunch_after_a_kill_keeps_put_back_on_the_entry`, `undo_after_a_clashed_put_back_drops_the_duplicate_stash`,
      `redo_after_that_undo_stores_it_again`; flipped `history_ops::a_rebase_preview_on_a_dirty_tree_says_what_is_set_aside`,
      `…a_checkout_preview_on_a_dirty_tree_says_what_is_set_aside`, graft `r_on_a_dirty_tree_opens_the_preview`;
      `every_preview_refuses_exactly_what_check_refuses` unchanged and green. **Red, honestly:** the first red run was a
      compile failure (no `SetAside`/`sets_aside`), not a behavioural one. Behavioural red was shown afterwards by
      mutation (NOTES).
- [x] NOTES: the order as built, file:line per step (NOTES, "Order as built").
- [x] NOTES: `--index`-free put-back verified by `set_aside_round_trips` (staged/unstaged split, staged new file,
      deletion, rename, all back byte for byte and in the index); A1.5's `(?)` not relied on.
- [x] MAP.md updated (doc, no test).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
Large (~75k): one sequence, its three exits, resume/recover/inverse, a test matrix. No UI.

## NOTES (builder, 2026-09-25)

**Compacted once, mid-ticket** (after the tests were green, before the gates). Nothing is left of the build. What
remains is the blocked decision below.

**Scout used:** `.scratch/m6-conflicts/scouts/set-aside-around-rebase-and-switch.md`. There are no stash wrappers in
graft-git, so git args are built in graft-ops (`apply/set_aside.rs`). graft-git is untouched.

### Blocked: needs decision
`graft::it remote_by_key::capture_fix_rebase_refused_before_preview` (`crates/graft/tests/it/remote_by_key.rs:1270`)
is **red** at this hash. It asserts 67's refusal (`:1294` no preview box; `:1295` the `Sticky` clean-tree status) and
checks the capture `docs/shots/m6/fix-rebase-refused-before-preview-120x40.txt`. This ticket's `check` change makes
both untrue. It can't be flipped without writing a capture (not allowed here). Deleting it touches the captions tsv,
the contact sheet and TESTING.md, which is 74's job (74 :99-100). Pick one:
(a) pull 74's deletion of the test and capture into 73;
(b) merge 73 red and let 74 land it;
(c) regenerate the capture here.
The ticket's scope line doesn't cover it: its Touches (:128) assumed the capture only goes stale, and didn't see the
test asserting the refusal.

### Flipped outside the ticket's one named graft test (deviation, reversible)
- m5 walk `rebase-dirty-tree-refused-offers-stash` → `rebase-dirty-tree-opens-the-preview`
  (`crates/graft/tests/it/m5_walk/merge_rebase.rs:306`, `every_scenario.rs:45`). It now asserts the rebase box opens
  with `Confirm`, no clean-tree status, and `Esc` moves nothing. TESTING.md row 12 is reworded to match. It still
  waits on `Wait::PreviewAnswered`, so a regression to the refusal fails on the box check, not on a timeout. It also
  keeps 67's variant in use (clippy dead-code otherwise). The ticket said "no other graft file"; this test was red
  under the new `check`, and it is graft tests only.
- `set_aside: None` added to `PreviewPlan` literals in `crates/graft/src/update/preview/slice/tests.rs`,
  `update/preview/tests.rs` (`a_plan`, `an_empty_plan`) and `view/tests.rs`. These are compile-forced, test modules
  only.

### Order as built (ADR-0045 §4)
1. `stash create <msg>`: `apply.rs:222` → `apply/set_aside.rs:56` (`create`, git args `:68`). Empty output → `None`,
   no set-aside. A failure → `ApplyError::Snapshot`, or `Stopped` on a stop; nothing is journaled.
2. `begin`: `apply.rs:231` (`put_back: SetAside(S)`; pins ∋ S; `touches` ∋ `refs/stash` via
   `operation.rs` `stash_ref_if_set_aside`), `apply.rs:237`.
3. `stash store -m <msg> S`: `apply.rs:240` → `set_aside.rs:94` (`put_aside`, `:103`).
4. `reset --hard`: `set_aside.rs:108`. A failure at 3 or 4 → `abandon` (`set_aside.rs:122`): entry `Aborted`, and the
   words name `stash@{n}` when it is listed.
5. the operation's plan: `apply.rs:252-254`. A rebase that stops → `Conflicted`, entry stays `Pending`, S stays
   listed.
6. `stash apply S` (no `--index`), then read the status whatever the exit code: `apply.rs:290-296` →
   `set_aside.rs:195` (`close`) → `:370` (`put_back`, apply `:374`, status read after it).
   - Nothing unmerged → `drop_by_id` `:415` (`stash drop stash@{n}`, `n` looked up by commit, `:421`, `:429`) →
     `Committed`.
   - Unmerged → S kept, `Committed`, `Clashed { SetAside }`.
   - Otherwise → S kept, `Aborted`, `Git` whose words name `stash@{n}`.
   Step 6 never stops (`&()`).
   After a stopped rebase, `resume` runs the same `close`: `apply/resume.rs:370-376`.
   - Continue/Skip → `Committed`.
   - Abort → `Aborted("aborted")`, with the edits back on the original tip.

**Undo/redo** (`undo.rs:343` `stash_step`; the SetAside branch is at `:362`):
- Undo → `StashDrop(S)` only when the root's `refs/stash` ended at S.
- Redo → `StashStore(S)` only when that undo's own `refs/stash` moved (`stash_ref_moved`, `:440`).

**Recover:** `recover_interrupted` keeps `put_back`, because the entry is cloned.

### Verified, and how
- **The inverse is proven on a real repo:**
  - `undo_after_a_clashed_put_back_drops_the_duplicate_stash`: whole state and stash list equal to before.
  - `redo_after_that_undo_stores_it_again`.
  - Mutations, each reverted:
    - Disabling the undo SetAside branch and resume's put-back failed `undo_after_…` and `a_rebase_that_stops_…`.
    - Disabling only the redo store failed `redo_after_…`.
- **Stop injection** (`a_stop_before_each_step_loses_nothing`): each stop is keyed to the previous step's file
  effect. A temporary probe confirmed each stop lands on its step, for both kinds, and the probe was removed:
  - 1–2: `Stopped`, nothing journaled.
  - 3: S not stored.
  - 4: S listed, no reset.
  - 5: reset done, op killed, S listed.
  - 6: the full sequence.
- **Round trip:** `--index`-free put-back restores the staged/unstaged split, a staged new file, a deletion and a
  rename (`set_aside_round_trips`, index read back).
- **Not verified:** the app-side words and landing of a `Clashed { SetAside }` (74).

### Remaining `DirtyTree` sites
`operation/history.rs:514` (new branch with checkout) and `:562` (branch from stash) still refuse. The synthetic
`DirtyTree` app tests are left as they are, since they build the refusal directly and are still valid:
- `update/preview/tests.rs` ~1165/1176 (`the_preview_refusal_and_the_write_refusal_read_the_same`)
- `update/operate.rs:2109`
- graft-ops `every_history_refusal_explains_itself_in_one_line`

### Deviations to know
- Stash message: `graft: your edits, set aside for <description>`. `stash create` makes the subject `On <branch>:
  <msg>`, so a redo's `StashStore` re-lists S under that subject, with slightly different reflog words. This only
  happens on the clash → undo → redo path.
- The op fails at 5 and the put-back also clashes → the entry is `Aborted` with git's reason (so the failure isn't
  lost), and the answer is `Clashed`.
- A put-back that is kept after a stop (not an `Exited` error): the stash words are in the entry's reason only, not
  in `explain()`.
- 67's capture `docs/shots/m6/fix-rebase-refused-before-preview-120x40.txt` is now untrue (see Blocked).

**Touches:**
- `check`: grep above.
- journal `put_back`/pins: round-trip test.
- undo/redo: tests above plus the existing undo suite, green.
- resume/recover: stop and relaunch tests plus the existing suites.
- `live_clash`: clash test.
- 72b routing: no graft source file changed.
- Radar: `duplicates_skip_the_stash` green.
- MAP.md: updated.
- m5 walk + TESTING.md row 12: flipped.
- Capture test: blocked.

**Gates** (workspace-wide, this hash):
- fmt ok.
- clippy ok.
- `cargo test --doc` ok.
- nextest 3398/3399: the one failure is the blocked capture test.
