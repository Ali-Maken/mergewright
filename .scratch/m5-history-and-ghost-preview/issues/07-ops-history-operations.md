# ops-history-operations: merge/rebase/cherry-pick/revert/reset/checkout/branch/tag as operations, the conflict span in the journal, resume, the time machine, and Ghost Preview planning

Status: done — pieces 1–9 built; **one acceptance box is deliberately unticked** (piece 2's throwaway-repo record, never made). Workspace gate red in `graft` only: `blocked: needs graft`, five sites in NOTES.
Blocked by: 02 (git-history-reads)
Build with: **Fable (signature)** — the journal shape of a conflicted operation and `PreviewPlan` are structural for M6 (resolver), M7 (chain rebase, ADR-0032) and the browser; "the ghost equals what git did" is not readable from a diff; the per-kind rules still need decisions the spec cannot fully pin.
Crates: graft-ops, graft-core
Needs: `git-history-reads` — `graft_git::history_reads::{merge_base, commits_between, forecast_merge, changed_paths}`, `graft_git::state::in_progress_detail`.

## Scout first (Sonnet `Explore`, ≤ 1 page) — even for Fable

- `crates/graft-ops/src/apply.rs`: the plan loop (`run_plan`), where the outcome line is written on non-zero exit, how `Applied.refresh` is chosen, what `recover_interrupted` closes.
- `crates/graft-ops/src/journal.rs`: `in_force` on a `Pending` entry; `Parsed` handling of a pending with no outcome.
- `crates/graft-ops/src/operation.rs`: how `check`'s shared prefix (bare → unmerged → in-progress) is written; `Refusal::explain`'s shape; `Invocation` construction helpers.
- Report: does anything today write an Outcome line for an exit status 1 with `MERGE_HEAD` present (it must not, after this group).

## Read first

- `docs/design-notes/m5-history-and-ghost-preview.md` — Promise; Interfaces (G5 block, whole); Hard choices 2–5; Proposed changes 2, 4; New UI "Ghost Preview" and "`u` after a push" (the words the plan must be able to supply).
- `docs/design/06-undo-journal-design.md` §6.3, §6.4 (whole), §6.6, §6.7. `docs/design/08-architecture.md` §8.2 lines 143–150.
- `docs/adr/0031-the-journal-answers-repo-state-at-a-time.md`, `docs/adr/0032-branch-dependency-and-chain-rebase.md` (a plausible caller), `docs/adr/0037` §3.
- `crates/graft-ops/MAP.md` whole; `crates/graft-core/MAP.md` §journal; `crates/graft-git/MAP.md` §history_reads (after G4), §state, §run.
- PROMPT.md §1.4 Fix… (line 159), §1.5, §8 no-resolver paragraph; `KNOWN_GAPS.md` rows 50, 229, 236, 478.

## Scope

In, in order (each piece a commit; stop seam after piece 4 with the rest in a handoff note if needed):

1. **`graft-core` `OperationKind`** += `Merge, Rebase, CherryPick, Revert, Reset, Checkout, BranchCreate, BranchDelete, TagCreate, TagDelete, BranchFromStash, RevertTo` (serde kebab names, match the existing convention).
2. **Operations** (`operation/history.rs`): params, `kind`, `describe`, `check`, `plan`, `touches` per the note's G5 block. Verify each `plan` in a throwaway repo first and record the results in NOTES: (a) `merge --no-edit` with `GIT_EDITOR=true` never opens an editor; (b) `rebase <onto>` on a conflict exits 1 leaving `rebase-merge/`; (c) `revert --no-edit -m 1 <merge>` on a merge; `-m` on a non-merge is refused by git (so `check` refuses `mainline` on a non-merge first); (d) `update-ref refs/heads/x <to> <expected>` refuses when moved; (e) `switch` on a dirty tree whose files differ → refuses with "would be overwritten" (classify into `WouldOverwrite`-style wording); (f) `stash branch` on a conflicting stash exits 1 and leaves the stash (Refusal or documented outcome); (g) `tag -a -F -` reads the message from stdin. `describe`: `merge feature/auth into main`, `rebase feature/auth onto main`, `cherry-pick a1b2c3d "subject"`, `revert a1b2c3d "subject"`, `reset main to 9f8e7d6 (hard)`, `move feature/x to 9f8e7d6`, `checkout feature/auth`, `checkout a1b2c3d (detached)`, `branch feature/y at a1b2c3d`, `delete branch feature/y`, `tag v1.2 at a1b2c3d`, `delete tag v1.2`, `branch feature/z from stash@{0}`.
   `check` rules beyond the shared prefix: every `Tip` must equal the fact (`Moved { name }`); rebase and checkout-with-switch on staged/unstaged **tracked** changes → `DirtyTree { count }` (Hard choice 4; untracked never counts); branch/tag names through `RefName` (`BadRefName`), existing → `RefExists`; delete needs `expected == fact`; `mainline` only on a merge; `Reset` Hard with dirty files is **allowed** (the preview says `LOSES`, the snapshot covers it — §6.3); `PushParams.source_commit` must be an ancestor of the branch tip (`merge-base --is-ancestor`, else `Refusal::NotOnBranch`).
   `Stage(Target::Paths)` on an unmerged path is now allowed **iff** the file has no `<<<<<<<` line (`MarkersRemain { path }` otherwise) — this is "mark resolved" for the interim; the `Unmerged` prefix rule exempts `Stage`/`Unstage` of exactly the unmerged paths.
3. **The conflict span** (`apply.rs`): a plan step whose exit is non-zero **and** `in_progress()` now reports the kind the operation started → the entry stays `Pending`; return `ApplyError::Conflicted { entry, kind, paths }` (`paths` from `status` unmerged). No outcome line. `Applied.refresh == Everything` for every history op. `recover_interrupted` skips a `Pending` entry whose kind matches a live `in_progress_detail` (the banner takes over) and reports it in `Recovered::live: Vec<EntryId>`. `journal::in_force` on `Pending`: not in force (confirm; write the test).
4. **`resume(ctx, Resume)`**: find the Pending entry (newest Pending whose kind matches the live state; none → `Refusal::NotInProgress`); `Continue` with unmerged paths → `Refused(Unmerged)` before git; run `<kind> --continue|--skip|--abort` with `GIT_EDITOR=true` (rebase after a `Skip` may stop again → `Conflicted` again, same entry); on success read after-state and write `Committed`; on `Abort` write `Aborted("aborted")` with the after-state read fresh (it should equal before; if not — an edit made mid-conflict — `in_force` treats it honestly and the backup holds the pre-op tree). Commands appended to the entry's `commands`.
5. **Undo** (`undo.rs`): the generic inverse already covers merge/rebase/pick/revert/reset/checkout (refs, HEAD, index, worktree from before-fields); add `BranchDelete`/`TagDelete` (recreate: `UpdateRef { expect: None, to: expected }`), `BranchFromStash` (`StashStore(pins[0])` + generic), and refuse `inverse` on a `Pending` entry (`NotInvertible::Pending` exists). `plan_revert_to`/`revert_to` per §6.6: the in-force entries newer than or equal to `target`, newest first, each inverted with its root; executed one `restore::execute` per plan (each its own Undo entry, so `U` redoes step by step); stop checked between plans (G2's rule inside each).
6. **`preview.rs`**: `plan(repo, git, op, facts, stop)`; `plan_revert_to(entries, target, now)`. Per kind:
   - Merge: base = `merge_base(into, source)`; `source` already reachable → description `already up to date`, no ghosts, `ref_moves` empty; base == into tip and `ff != Never` → fast-forward: no ghost, `RefMove { into → Real(source_tip) }`; else one ghost merge (parents `[Real(into), Real(source)]`, subject `Merge <source short name> into <into>`), `RefMove { into → Ghost(0) }`, `conflicts` from `forecast_merge(None, into, source)`; worktree `Updated { changed_paths(into, source) }`.
   - Rebase: range `commits_between(onto_tip, branch_tip, 50)`; one ghost per commit oldest first, parents chained `[Real(onto)]`, `[Ghost(0)]`…, `replays: Some(c)`; `RefMove { branch → Ghost(last) }`; conflicts = union over `forecast_merge(Some(c.parents[0]), onto_tip, c)` (Hard choice 5), `NotChecked` when `capped`; worktree `Updated`.
   - CherryPick/Revert: one ghost on `head` (`replays`), `RefMove { head → Ghost }`, forecast with base c^ (revert: base c, theirs c^ — the reverse patch); worktree `Updated`.
   - Reset Head: `RefMove { branch → Real(to) }`; worktree `Untouched` (Soft/Mixed) or `Lost { dirty tracked paths }` + `Updated` (Hard); `undo: Undoable` always. Reset Branch: one `RefMove`.
   - Checkout: `head: Some(..)`, worktree `Updated { changed_paths(HEAD, target) }`.
   - Branch/Tag create/delete: one `RefMove` (`from: None`/`to: Real`), no ghosts. BranchFromStash: `RefMove`, `head`, worktree `Updated { stash paths }`.
   - `commands` = `plan(&facts)` lines, always (KNOWN_GAPS row 229); `against` = every `Tip` in the params; `undo` = `Undoable` for all of these.
   - `plan_revert_to`: `state_at(target)` vs `now` → `ref_moves` for every differing ref, `head`, worktree `RestoredFromBackup`, description `revert to before "<target description>" (N steps)`.
   Ghost `oid` = SHA-1 over `"graft-ghost\0" + description + "\0" + n` (deterministic; tests assert two plans of the same op give equal oids).
7. **Push re-push**: `PushParams.source_commit: Option<Oid>` → refspec `<oid>:refs/heads/<remote_branch>`; `describe` `push 9f8e7d6 → origin/main (with lease)`.
8. Tests (`tests/it/history_ops.rs`, real git, corpus shapes + scratch commits), each proving by reading git back:
   - for **each** kind: `plan` runs through `apply` on a clean case, the journal entry has the expected `touches`/before/after, `undo` restores every ref and tree id, `redo` re-applies — `<kind>_applies_undoes_redoes`
   - **ghost equals git**: for merge (ff and non-ff), rebase (2 commits), cherry-pick, revert, reset: `preview::plan` first, then `apply`; assert every `RefMove` with `Real` target matches `for-each-ref`; every `Ghost` target's commit has the ghost's parents' real counterparts and subject (rebase: `replays` commits' patch-ids equal) — `ghost_matches_git_<kind>`
   - conflict span: merge with a conflict → `Conflicted`, entry `Pending`, `MERGE_HEAD` present; `resume(Continue)` → `Refused(Unmerged)`; write the file, `Stage` → allowed once markers are gone (`MarkersRemain` before); `resume(Continue)` → `Committed`, `undo` restores the pre-merge ref and tree — `conflict_span_is_one_entry_and_undoes_to_before_the_merge`; `resume(Abort)` → `Aborted("aborted")`, tree byte-identical to before — `abort_restores_before_state`; `recover_interrupted` with a live rebase leaves the entry Pending — `recovery_leaves_a_live_conflict_alone`
   - rebase stops twice (two conflicting commits): `Conflicted` → resolve → `Continue` → `Conflicted` again, same entry id — `rebase_conflicts_twice_same_entry`
   - `DirtyTree` refusal on rebase; `Moved` refusal after a tip moved since the `Tip` was read — `stale_preview_is_refused`
   - `forecast` marks the real conflict path `Likely` and a clean case `Clean`; capped range → `NotChecked` — `forecast_names_the_conflicting_path`
   - `revert_to` over three entries: `state_at(target)` equals the repository after, three Undo entries written, `U` ×3 returns — `time_machine_reverts_in_order`
   - `source_commit` push over `LocalRemote`: bare's `main` moves to the old tip with a lease — `repush_old_tip_with_lease`
9. MAP.md (graft-ops: operation, apply, undo, preview; graft-core journal); an `as-built.md` in `.scratch/m5-…/` naming every signature that differs from this spec (the app groups read it).

Out: drawing anything; the app's state machine; interactive rebase (M7); `merge --squash`, octopus merges; chained forecasts; multi-commit cherry-pick UI (the `Vec` is there).

## Signatures

The note's G5 block is the contract, verbatim. Additions fixed here:

```rust
pub struct Recovered { pub closed: Vec<EntryId>, pub unreadable: Vec<(EntryId, FactsError)>, pub live: Vec<EntryId> }
pub enum Refusal { …, Moved { name: String }, DirtyTree { count: usize }, BadRefName { name: String, rule: RefNameRule }, RefExists { name: String },
    MarkersRemain { path: PathBuf }, NotInProgress, NotOnBranch { commit: Oid, branch: String }, MainlineOnNonMerge }
pub enum PreviewError { Facts(FactsError), Read(RunError), Refused(Refusal), Stopped }   // explain()
```

## Acceptance

- [x] Every test in piece 8 green, and every `plan` string exercised against real git with (a)–(g) recorded — both halves, in two parts. **Piece 8**: 42 tests in `crates/graft-ops/tests/it/history_ops.rs` (35 the group wrote + 7 added for the three gaps), green in the workspace run below. **The plan strings**: `<kind>_applies_undoes_redoes` (13 kinds) runs each kind's `plan(&facts)` lines through `apply` against a real repository and reads the result back out of git, and `preview_commands_come_from_plan` proves the lines the box shows are those same lines for eleven kinds — so no plan string is asserted only against itself. **(a)–(g)**: recorded independently against git 2.54 in throwaway repositories by `/tmp/plan_behaviours.sh` and `/tmp/plan_behaviours2.sh`, output committed as `.scratch/m5-history-and-ghost-preview/plan-behaviours.txt` and `plan-behaviours-2.txt`; per-item reading under NOTES · **(a)–(g), recorded**. Two of the ticket's own premises did not survive that record — see there.
- [x] `preview::plan(...).commands == op.plan(&facts)` lines for every kind — `preview_commands_come_from_plan` (eleven kinds: merge, cherry-pick, revert, reset, checkout, branch create/delete, tag create/delete, rebase, branch-from-stash)
- [x] No outcome line is ever written for a step that left `in_progress` state — `conflict_span_is_one_entry_and_undoes_to_before_the_merge` (asserts the journal **file** holds exactly 1 line at the stop and 2 after the finish, and that `MERGE_HEAD` is present in between)
- [x] `journal::in_force` on `Pending` is false, with a test — `pending_is_not_in_force` (`crates/graft-ops/src/journal/tests.rs`: `!in_force`, `in_force_all == [true, false]`, and a `Pending` merge does not shadow `next_undo`)
- [x] MAP.md + `as-built.md` written — `crates/graft-ops/MAP.md` (operation, journal, apply, undo, new `## preview` section), `crates/graft-core/MAP.md` (`Oid::from_sha1`), `.scratch/m5-history-and-ghost-preview/as-built.md`; NOTES below

## Gates

`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` (also `--features heavy-tests`) · `cargo nextest run -p graft-ops -p graft-core` · `cargo test --doc -p graft-ops`; workspace once at the end (`OperationKind` is matched exhaustively in `graft` — add the minimal arms, name them in NOTES, or commit at the seam and report `blocked: needs graft`).

## Size

One context, the largest of the milestone. Stop seam after piece 4 (operations + conflict span + resume), pieces 5–8 in a handoff note if the context runs short.

## NOTES

**Model.** Pieces 1–2 were built by a Fable builder (`667ad8e`) which ran out
of credits mid-piece-3. Pieces 3–9 were finished on Opus by the conductor's
routing decision. The committed shapes from 1–2 were treated as given.

**Scout.** No scout. The ticket's scout was piece 1's, and I inherited the
worktree with pieces 1–2 committed and piece 3 half-written in
`apply.rs` — reading that code *was* the survey, and a scout would have
reported on a tree I already had open.

**Turns (ADR-0029), this session only.** Piece 3+4 ≈ 40 turns (`66342a0`);
pieces 5–7 ≈ 55 (`efbb96a`); piece 8 ≈ 20 (`ad203bb`); piece 9 ≈ 12. The
predecessor's counts are unknown. **This session compacted once**, between
pieces 7 and 8; nothing was left half-built across the boundary (piece 7 was
committed before it).

**(a)–(g), what the real-git tests actually cover.** Rule 1: this is what I
observed, not what I assume.

- (a) `merge --no-edit` + `GIT_EDITOR=true` never opens an editor — **held**:
  every merge test runs to completion with no tty (`run_plan` sets
  `GIT_EDITOR=true` on every invocation).
- (b) a rebase conflict exits 1 leaving `rebase-merge/` — **held**:
  `rebase_conflicts_twice_same_entry` reaches `Conflicted` twice and asserts
  `.git/rebase-merge` is gone only after the finish.
- (c) `revert --no-edit -m 1 <merge>`, and git refusing `-m` on a non-merge —
  **unverified**. No test reverts a merge commit. `check` refuses
  `MainlineOnNonMerge` first, so git is never asked the bad question, but I
  did not observe git's own behaviour.
- (d) `update-ref <name> <to> <expected>` refusing when moved — **unverified**
  for the move case. The delete path's compare-and-swap runs in
  `branch_delete_applies_undoes_redoes`, but nothing moves the ref underneath
  it first.
- (e) `switch` on a dirty tree — **partly**: `stale_preview_is_refused` proves
  the `DirtyTree` refusal fires for a rebase before git runs, and that an
  untracked file does **not** count. The checkout case is the same `check`
  code path but is not separately tested.
- (f) `stash branch` on a conflicting stash — **unverified**. The clean case
  round-trips in `branch_from_stash_applies_undoes_redoes`.
- (g) `tag -a -F -` reads the message from stdin — **held**:
  `tag_create_applies_undoes_redoes` asserts `cat-file -t refs/tags/v1.0`
  is `tag`, i.e. an annotated tag object, not a lightweight ref.

**Two spawn decisions the note did not settle.**

- *A ghost's author.* A replayed commit (rebase, cherry-pick) carries the
  original author, which `commits_between` already returns. A commit git
  would write fresh (merge, revert) gets `git var GIT_AUTHOR_IDENT`, parsed
  bytes-in-bytes-out so a Latin-1 name survives. A failure there propagates
  rather than defaulting: the real commit would fail the same way.
- *Reading a cherry-picked commit without a `graft-git` change.*
  `commits_between(head, commit, 1)` returns the picked commit itself — it is
  by definition not on `head` — so subject, author and parents come back in
  one spawn. A revert cannot use the trick (the commit *is* on `head`), which
  is why `RevertParams` grew `parent: Option<Oid>`.

**Four faults the real-git tests found, all fixed in `ad203bb`.** Details and
reasoning in `as-built.md`; in short: `SetHead` recorded `HEAD` as a touched
ref, so redoing a checkout moved the branch under it; `revert_to` minted one
`EntryId` for every step; `preview::head_tip` never read `HEAD`'s own entry;
the re-push ancestry check was skipped because `Push` named no refs in its
`FactsScope`. Each is a cause fix, not a symptom fix.

**Findings for the conductor.**

- `OperationKind::RevertTo` is defined in `graft-core` and **nothing
  constructs it**. The time machine journals one `Undo` entry per step, as
  the note asked, so the kind has no use. M6/M7 should claim it or it should
  go.
- `graft-ops` has no doctests, so `cargo test --doc -p graft-ops` is
  vacuously green (`0 passed`). Worth knowing before it is read as evidence.

**The seam — `blocked: needs graft`.** Five sites, all mechanical, listed in
`as-built.md` §7: `jobs/operate.rs:117` (`ApplyError::Conflicted` /
`ResumeFailed`), `jobs/undo.rs:95` (`UndoError::PartWay`), and three
`PushParams` literals needing `source_commit: None`
(`update/remote.rs:299,317`, `view/tests.rs:2998`). The workspace gate is red
there and nowhere else.


---

## NOTES — the seam and the leftovers (m5/07 follow-up session)

**What this session did.** Closed the `blocked: needs graft` seam, made the
(a)–(g) record that acceptance box 1 was waiting on, tested the three cases
the group left unverified, and settled `OperationKind::RevertTo`. Started by
merging `main` (at `9efa345`, 1904/1904) into the group's branch.

**Scout.** Yes — one `Explore` (sonnet) over `crates/graft/src`, for what the
app already holds about a live conflict before wiring two new variants into
it. It is what found `AppState::in_progress` (fed by the working-tree read,
read today only by the commit editor's refusal), that `Refresh::Everything`
re-reads it through `refresh_key`, and that `Operated`/`Undone` had no
variant that was neither a success nor a failure. Cheaper than reading the
crate.

**The merge was clean and the build was not** — as warned. `git merge`
reported no conflict; `cargo check --all-targets` then failed with exactly
the four seam errors and nothing else, so the semantic break that bit group
03/04 (`Status::Sticky` reshaped under a branch still using the tuple form)
did **not** recur here. Checked by compiling, not by reading the merge.

### The seam

- **`jobs/operate.rs`** — `Operated` gained `Conflicted { description, kind,
  paths, stalled }`. `ApplyError::Conflicted` and `ApplyError::ResumeFailed`
  both land there, neither in `Failed`. A conflicted merge is not something
  that went wrong: the entry is journaled `Pending`, git's state is on disk,
  and the user is *in* the operation — as-built.md §1's decision, carried
  through to the app. `ResumeFailed` shares the variant because it is the
  same state to the user, with `stalled` carrying the one fact that separates
  them (git's own line, where nothing is unmerged to explain the stop).
  Nothing is folded into a generic error arm.
- **`update/operate.rs`** — the `Conflicted` arm sets a **sticky** status
  (it has to outlive the refresh it triggers) worded by the new
  `conflicted_line`, and always refreshes `Everything`. That wide re-read is
  not a guess: it is the only scope that re-reads `AppState::in_progress` and
  the unmerged flags that put the files under `CONFLICTS`. The commit editor
  and its draft are **not** cleared — no commit was made, and throwing away
  typed words over someone else's conflict would be losing work.
- **Wording.** `conflicted_line` says what was being done, that the operation
  is still open, and what is in front of the user — and names no key,
  because `resume` has none yet (`docs/design/02-keymap.md` allocates it with
  the resolver). It never reads as an error. Three shapes: one path is named,
  several point at CONFLICTS, none says "no file left unmerged" rather than
  claiming a conflict that is not there (a rebase can stop on an `edit` step).
- **`jobs/undo.rs`** — `UndoError::PartWay` handled explicitly in both
  `refresh_after_failure` (steps that ran ⇒ `Everything`; none run ⇒ the
  inner error answers, so a rail that wrote nothing does not trigger a
  re-read) and `undone_from`. Its `done: Vec<Applied>` is deliberately not
  carried into `Undone`: the message already says "1 of 2 steps done, then:
  …", and `Everything` is wider than any of those `Applied`s would ask for,
  so nothing is left stale. Nothing reaches this arm today — `run_undo` only
  calls `undo`/`redo`; the time machine gets its own job and
  `Product::RevertedTo` in ticket 08 item 7, which is where "stopped after 2
  of 3 steps" is meant to be worded.
- **`PushParams.source_commit: None`** at all three sites
  (`update/remote.rs` ×2, `view/tests.rs`). `P` pushes the branch as it
  stands; re-pushing an *old* tip is ADR-0038 §4's separate offer with its
  own confirm, not built yet, so `None` is the honest value rather than a
  quiet reuse of whatever commit is selected. The existing lease machinery
  (`last_rejected_push` → `Lease { expected: tracking_tip }`) is untouched.

Seam tests (all in the workspace run): `a_conflicted_merge_is_shown_as_a_state_and_re_reads_everything`,
`a_conflict_in_one_file_names_that_file`,
`a_stop_with_nothing_unmerged_does_not_claim_a_conflict`,
`a_continue_git_refused_repeats_gits_own_words`,
`a_time_machine_that_stopped_part_way_re_reads_everything`,
`a_time_machine_that_stopped_before_its_first_step_asks_for_nothing`,
`a_part_way_revert_says_how_far_it_got`.

### (a)–(g), recorded

Run against git 2.54.0 (Apple Git-157). Full output in
`.scratch/m5-history-and-ghost-preview/plan-behaviours.txt` (a–g) and
`plan-behaviours-2.txt` (the (c), (e) and (f) follow-ups). Rule 1: every line
below is from that output.

- **(a) held.** `merge --no-edit` with a `GIT_EDITOR` that fails loudly:
  exit 0, editor never opened, 2-parent commit made.
- **(b) held.** `rebase <onto>` on a conflict: exit 1, `.git/rebase-merge`
  present, `.git/rebase-apply` absent, `UU shared.txt`.
- **(c) the ticket's premise is wrong.** `revert --no-edit -m 1 <merge>`
  works (exit 0, the merged side gone, mainline's side kept). But **git 2.54
  also accepts `-m 1` on a non-merge** — exit 0, an ordinary revert — where
  the ticket said git refuses it. Only `-m 2` on a one-parent commit fails
  ("does not have parent 2"). So `check`'s `MainlineOnNonMerge` is **Graft's
  own refusal, stricter than git's**, not a repeat of it. That is still the
  right rule (a mainline number on a non-merge is meaningless and the UI only
  offers the choice on a merge) but the reasoning in `history.rs`'s module
  doc is the one that holds, not the ticket's. A merge with **no** `-m` is
  refused by git (exit 128).
- **(d) held.** `update-ref <name> <to> <expected>` with a stale expected:
  exit 128, "is at … but expected …", ref unchanged. A zero-oid expected on
  an existing ref: exit 128, "reference already exists".
- **(e) held, and Graft is stricter.** `switch` with a dirty *tracked* file
  that differs: exit 1, "Your local changes … would be overwritten by
  checkout" — staged or unstaged alike. With a dirty file that is
  **identical** on both branches git carries the edit across (exit 0);
  Graft's `clean_tree` refuses `DirtyTree` there anyway, which is hard choice
  4 and deliberate. An **untracked** collision is a different git message
  ("The following untracked working tree files would be overwritten") and
  Graft does *not* pre-empt it — untracked never counts — so that one
  reaches git and fails there. Noted as a gap below.
- **(f) the ticket's premise is wrong.** `git stash branch` creates the
  branch at the **stash's own base commit** and applies there — verified
  directly: the new branch's tip equals `stash@{0}^`. A branch that moved
  under the stash therefore **cannot** make it conflict; the attempt exits 0
  with the stash dropped and nothing unmerged. The three ways it really
  fails: the branch name is taken (exit 1, stash kept, HEAD unmoved); a
  nonexistent stash ref (exit 1, no branch); and a **dirty working tree** —
  exit 1, but git has *already* created the branch and moved `HEAD` onto it,
  leaving the stash in the list. That last one is a genuine half-done state,
  and `check_branch_from_stash`'s `clean_tree` is what keeps it unreachable.
- **(g) held.** `tag -a -F - <name>` reads the message from stdin (the tag is
  a `tag` object, message byte-correct). An empty stdin message is accepted
  by git — `check_tag_create`'s `EmptySubject` is Graft's own guard.

### The three untested cases (job 2 item 2)

Seven tests added to `crates/graft-ops/tests/it/history_ops.rs`, each reading
its claim back out of git:

- **(c)** `revert_of_a_merge_with_mainline_one_removes_the_merged_side`
  (the fixture's tip is asserted to be a real 2-parent merge; after the
  revert `feature.txt` is gone, `main.txt` stays, `HEAD^` is the merge),
  `revert_of_a_merge_undoes_and_redoes` (full `round_trip`),
  `the_ghost_of_a_merge_revert_matches_git` (one ghost parented on the merge,
  `refs/heads/main` → `Ghost(0)`, `-m 1` in `plan.commands`, and after the
  run the ghost's subject and parent are the ones git wrote),
  `mainline_is_refused_on_a_non_merge_and_required_on_a_merge` (all three
  `check` rules, plus a throwaway repo proving git itself would have run the
  first one).
- **(d)** `a_forced_undo_still_refuses_when_the_ref_moved_under_it` — reset,
  then `main` moved behind Graft's back. Unforced: `UndoError::Drift`, ref
  untouched. Forced past the rail: the compare-and-swap itself refuses with
  `RestoreError::RefMoved { name: "refs/heads/main" }` (matched by variant,
  not by substring) and the ref stays where the other writer put it.
- **(f)** `branch_from_stash_starts_at_the_stashs_base_so_it_cannot_conflict`
  — a stash whose line `main` then rewrites; the new branch lands on the
  stash's base, nothing is unmerged, the stashed bytes are on disk.
  `branch_from_stash_is_refused_before_git_can_half_do_it` — `RefExists` for
  a taken name and `DirtyTree { count: 1 }` for a dirty tree, both before git
  is spawned, and neither refusal wrote a branch, moved `HEAD` or touched the
  stash.

**No new bugs found.** The group's own tests had already found the four it
reports; these three cases held. Two *ticket premises* were wrong, not the
code — (c) and (f) above. `history.rs`'s module doc already recorded both
correctly, so the previous builder had observed them; what was missing was
the record, not the verification.

### `OperationKind::RevertTo` (job 2 item 3)

**Unused, and on the design as it stands it stays unused.** §6.6 decided the
time machine journals one `Undo` entry per step so `U` walks a revert back
one operation at a time; a `RevertTo` entry would be exactly the summary that
decision rejected. `undo::revert_to` writes `Undo` lines, nothing constructs
this kind, and `apply.rs`'s `"revert to"` label for it is therefore
unreachable. Ticket 08's `PreviewSubject::RevertTo` is a **different type
with the same name** (it identifies what a preview is of) and is not a user
of this variant.

Not deleted: the M5 design note names it and removing it is the conductor's
call, not a builder's. The reasoning is now on the variant itself in
`crates/graft-core/src/journal.rs` and in `crates/graft-core/MAP.md`, so
nobody has to find `as-built.md` to learn it is dead. **Decision wanted:
delete it, or give it a use in M6/M7.**

### Gates, as printed

- `cargo fmt --check` — clean.
- `cargo clippy --all-targets --message-format=short -- -D warnings` — clean
  (three findings fixed on the way: identical match arms merged,
  `run_operation` split so the result mapping is its own `answer` fn, one
  redundant closure).
- `cargo nextest run` — `1978 tests run: 1978 passed, 29 skipped` in 41.4 s.
  Main was 1904; the difference is group 07's own suite plus the 7 added
  here.
- `cargo test --doc` — green, **and worth nothing**: 2 doctests in the whole
  workspace, 0 in every crate this ticket touched. Not evidence of anything.

All four workspace-wide, never `-p`.

### Gaps handed back

1. **`OperationKind::RevertTo`** — delete or claim (above).
2. **`Undone` cannot carry a part-way revert's `done` list.** Fine today,
   because nothing calls `revert_to`; ticket 08 item 7 needs it to say
   "stopped after 2 of 3 steps" and will need its own product type.
3. **An untracked-file collision on `checkout` is not pre-empted.** `check`'s
   `DirtyTree` counts tracked files only (hard choice 4), so a `switch` that
   would clobber an untracked file reaches git and fails with git's wording
   rather than Graft's. Recorded under (e) above; a `WouldOverwrite`-style
   classification was in piece 2's brief and does not exist.
4. **`Refusal::MainlineOnNonMerge` is misnamed** for one of the two cases it
   covers: a mainline number out of range on a commit that *is* a merge. Its
   `explain()` is honest ("-m names a parent this commit has not got; only a
   merge takes a mainline"), so this is a name, not a lie on screen.
   Untouched — renaming a public variant is outside this ticket.
5. **`resume` still has no key.** The app can now see a conflict and say what
   it is; finishing or aborting it is ticket 09's. `conflicted_line`
   deliberately names no key until then.
