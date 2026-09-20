# ops-history-operations: merge/rebase/cherry-pick/revert/reset/checkout/branch/tag as operations, the conflict span in the journal, resume, the time machine, and Ghost Preview planning

Status: ready
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

- [ ] Every test in piece 8 green, named at tick time; every `plan` string verified in a throwaway repo, results in NOTES (a)–(g)
- [ ] `preview::plan(...).commands == op.plan(&facts)` lines for every kind — `preview_commands_come_from_plan`
- [ ] No outcome line is ever written for a step that left `in_progress` state — `conflict_span_is_one_entry_and_undoes_to_before_the_merge`
- [ ] `journal::in_force` on `Pending` is false, with a test — `pending_is_not_in_force`
- [ ] MAP.md + `as-built.md` written; NOTES says scout used or why not, turn count at each piece boundary (ADR-0029)

## Gates

`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` (also `--features heavy-tests`) · `cargo nextest run -p graft-ops -p graft-core` · `cargo test --doc -p graft-ops`; workspace once at the end (`OperationKind` is matched exhaustively in `graft` — add the minimal arms, name them in NOTES, or commit at the seam and report `blocked: needs graft`).

## Size

One context, the largest of the milestone. Stop seam after piece 4 (operations + conflict span + resume), pieces 5–8 in a handoff note if the context runs short.
