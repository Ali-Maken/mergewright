# Review: M5 group 07 — history operations, conflict span, resume, preview plan, re-push

Reviewed at `7180f07` (merge parent of `5cae018`), diffed from `3b9d855` and
restricted to `crates/graft-ops`, `crates/graft-core`, and the seam commit
`16fbc50..7180f07` in `crates/graft`. Read: CLAUDE.md, the ticket, `as-built.md`,
the design note's G5 block and hard choices 2–5, ADR-0038, DESIGN §6.4/§6.6/§6.7,
`graft-ops/MAP.md`, `graft-core/MAP.md` §journal, the `graft/MAP.md` seam diff.

**No cargo command was run** (a builder holds a worktree; conductor's instruction).
Gates are the conductor's at the merge: `fmt ok · clippy ok · 1978/1978, 29 skipped`.
Doctests: 0 in every crate touched — not evidence of anything. Every tick below was
verified by reading the named test's source. Where only a run can settle a claim, the
exact command is named.

Real git (2.54.0, Apple Git-157) was run **outside the repo** in `/tmp` to check four
behavioural claims; results under "Verified against git".

Counts: **3 Critical · 11 Major · 15 Minor.**

---

## Verdict on the two structural shapes

**1. One `Pending` entry per conflicted operation, closed by `resume`.** The shape is
right and the executor half is sound (`stopped_on_conflict`, `still_pending`,
`close_resume`, `Recovered.live`, `hold_commands` all do what as-built says). **It does
not survive a resolver that lands mid-operation as built** — see C1. The as-built's M6
contract ("mark a file resolved with an ordinary `Stage`, then `resume`… there is no
second entry to reconcile… `u` after the finish takes the whole span back — verified
by `conflict_span_is_one_entry_and_undoes_to_before_the_merge`") is false: an ordinary
`Stage` *is* a second entry, and the named test deliberately stages with raw `git add`
to avoid exactly that (its own comment says so). ADR-0031 ("what was the repo at T")
holds while an entry is open: the `Pending` line carries the before-state, and
`state_at_skips_a_pending_entry_that_recorded_no_after_state` pins the rule.

**2. `PreviewPlan`.** The mechanism is genuinely structural: `plan.commands` is
overwritten from `op.plan(&facts)` for every kind (`preview::plan`, one place),
`RefMove.to: Option<Target>` makes deletes expressible, and `plan_revert_to` is drawn
from the very `InversePlan`s `revert_to` runs. Nothing lets the *commands* drift. What
lets the *picture* drift is git's own config, which neither `plan` nor `preview` reads:
`merge.ff=false` turns the previewed fast-forward into a merge commit (M1, verified),
`rebase.updateRefs=true` moves branches the preview and the journal never see (M2,
verified). Plus one wrong diff pair in the revert planner (M3).

**Model.** The journal shape wanted the signature model: the hole in C1 is a
journal-semantics judgement ("what is an entry while another is open") that no diff
shows and that the ticket routed to Fable for precisely this reason. `PreviewPlan` was
fine on Opus — its three faults are ordinary omissions with local fixes.

---

## Critical

### C1. The conflict span breaks under a journaled "mark resolved" — `u` is dead after a merge resolved in-app

- `crates/graft-ops/src/apply.rs:374-392` (the `Conflicted` return), `crates/graft-ops/src/undo.rs:238`, `crates/graft-ops/src/journal.rs` (`in_force_flags`, `next_undo`), `.scratch/m5-history-and-ghost-preview/as-built.md` §1.
- **What.** `Stage(Target::Paths)` of an unmerged path is now allowed by `check` and
  goes through `apply` like any operation: a new entry, positioned *after* the `Pending`
  merge entry in the file. Its before-snapshot runs `write-tree` on an unmerged index →
  `SnapshotError::Unmerged` → `index_before: None` (`snapshot.rs:144-149`,
  `apply.rs:1017`). After `resume(Continue)` writes the merge's outcome line, the
  journal is `[Merge(Committed), Stage(Committed)]` in position order. `next_undo`
  picks the Stage (newest in-force operation). `inverse` of it hits
  `entry.index_before.ok_or(NotInvertible::NoBackup)` (`undo.rs:238`) → `u` refuses
  "no backup". Undo is linear, so the merge entry is unreachable; `plan_revert_to`
  across the span refuses the same way (`not_invertible → "an operation in the range
  has no backup"`).
- **Why it matters.** This is the M6 resolver's path and the M3 promise ("you cannot
  lose work"): a user who resolves a conflict inside Graft and finishes the merge has
  no `u`. The as-built states the opposite as verified.
- **Fix (cause).** A step taken inside a live `Pending` entry of Graft's is part of
  that entry, not an entry. Either (a) when `facts.in_progress` matches a live Pending
  entry, `apply` of `Stage`/`Unstage` runs the plan, appends its command line via
  `hold_commands`, and writes no journal line; or (b) journal it with
  `part_of: Option<EntryId>` and make `in_force_flags`, `next_undo`, `plan_revert_to`
  and the browser skip such entries. (a) keeps the journal file format; (b) keeps the
  Command Log per line. Decide in M6's design note; do not leave as-is.
- **Settle by running.** Add to `conflict_span_is_one_entry_and_undoes_to_before_the_merge`:
  `try_run(&repo, &Operation::Stage(Target::Paths(vec!["shared.txt".into()])), NOW+3)`
  in place of `repo.git(&["add", …])`, keep the final `undo_to` — then
  `cargo nextest run -p graft-ops --test it conflict_span`. Expected today: panic at
  `undo_to` with "no backup".

### C2. `plan_revert_to` inverts in-force `Undo`/`Redo` entries — the time machine re-applies what the user undid

- `crates/graft-ops/src/undo.rs:383-400` (`plan_revert_to`), contrast `journal.rs:256`
  (`next_undo` filters `is_operation`).
- **What.** The loop skips `!flags[index] || !undoable(entry)`; `undoable` excludes
  only Fetch/Push/no-op-Pull. An in-force `Undo` entry (`took_effect && !reversed`,
  `in_force_flags`) is therefore inverted — and inverting an Undo entry re-applies its
  root. Journal `[pick A, branch B, undo(B)]`, revert to A: plans = `redo B`, (B
  skipped, not in force), `inverse A`. Step 1's CAS on `refs/heads/second` expects
  `None` — true — so the branch is re-created; step 2 compares A's `refs_after` (main
  only) with now — unchanged — so A is undone. Result: state-before-A **plus** a branch
  the user had explicitly undone, and a `Redo` journal line nobody asked for. No rail
  catches it. The preview draws the same plans, so the box would show a "revert to
  before" that *creates* a branch.
- **Fix.** `if !flags[index] || !is_operation(entry) || !undoable(entry) { continue; }`
  (make `is_operation` `pub(crate)`), and a test with an undo inside the range asserting
  `state_of == start`. `time_machine_reverts_in_order` has no undo in its range and
  cannot see this.

### C3. False tick (small): tick 1 credits a per-kind redo that one kind never runs

- Ticket "Acceptance" box 1 → `<kind>_applies_undoes_redoes` "(13 kinds)";
  `crates/graft-ops/tests/it/history_ops.rs:321-364`.
- **What.** Piece 8 requires, per kind, "`undo` restores every ref and tree id, `redo`
  re-applies". There are 11 kinds and 11 such tests (not 13).
  `branch_from_stash_applies_undoes_redoes` does apply → undo only — no `round_trip`,
  no `run_redo`. Redo of `BranchFromStash` (StashDrop of the pin + `UpdateRef` re-create
  + `SetHead` + tree restore) is the one inverse in this group with a stash step *and*
  a HEAD move, and it is unverified while its name and the tick say otherwise.
- **Fix.** `round_trip(&repo, &op)` in that test (it already has `before`), or rename
  and file the redo leg as unverified in KNOWN_GAPS. Correct "13" to "11".

---

## Major

### M1. `merge.ff` config silently changes what a `FastForward::Default` merge does — ghost ≠ git

- `crates/graft-ops/src/operation/history.rs` `plan_merge` (no flag for `Default`);
  `crates/graft-ops/src/preview.rs` `merge()` (`base == Some(into) && ff != Never` →
  fast-forward picture).
- **Verified:** `git -c merge.ff=false merge --no-edit -m … feat` on an ff-able branch
  wrote a 2-parent merge commit. The preview drew `RefMove main → Real(feat)`, no ghost,
  `conflicts: Clean` — the user sees a fast-forward and gets a merge commit.
  `merge.ff=only` makes git refuse (exit 128, entry Aborted) where the preview drew a
  ghost. Both common team settings.
- **Fix.** Emit `--ff` explicitly for `Default` (git accepts it; the preview's rule is
  then git's rule), or read `merge.ff` into `RepoFacts` and let `preview::merge`
  follow it. The first is one line and keeps `check` pure.

### M2. `rebase.updateRefs=true` moves refs the journal never recorded — `u` leaves them moved

- `history.rs` `plan_rebase` (bare `rebase <onto>`), `touches` = `[branch]` only.
- **Verified:** with `rebase.updateRefs=true`, `git rebase main` on `f2` moved `f1`
  (`316cf4e → 711bf49`). `f1` is in neither `refs_before`/`refs_after` nor the
  preview's `ref_moves`; the §6.7 rail compares only recorded refs, so `u` restores
  `f2` and leaves `f1` rewritten, silently. The user works on dependent branches
  (memory) — this is the setting such users turn on.
- **Fix.** `--no-update-refs` (git ≥ 2.38; gate on `GitVersion`, older git has no
  such config) and `--no-autostash` in `plan_rebase`. M7's chain rebase (ADR-0032)
  then opts in deliberately with every moved branch in `touches.refs` and `ref_moves`.

### M3. `preview::revert` diffs the wrong pair — a revert of `HEAD` previews an empty worktree effect

- `preview.rs` `revert()`: `plan.worktree = updated(repo, params.commit, head)`.
- The paths a revert rewrites are the commit's own change, `changed_paths(parent, commit)`;
  `changed_paths(commit, head)` is the history *after* the commit, and for `commit == head`
  it is empty. `ghost_matches_git_revert` checks parents and subject, not `worktree`.
- **Fix.** `match params.parent { Some(p) => updated(repo, p, params.commit)?, None => Untouched-with-a-reason }`
  and an assertion in `ghost_matches_git_revert` that `main.txt` is in the paths.

### M4. `UndoError::PartWay` invents its total: "N of N+1 steps done"

- `crates/graft-ops/src/undo/execute.rs:38`:
  `#[error("{} of {} steps done, then: {error}", done.len(), done.len() + 1)]`.
- A 3-step revert stopping after step 1 says "1 of 2 steps done". The app test
  `a_part_way_revert_says_how_far_it_got` (`graft/src/jobs/undo.rs`) pins the wrong
  string. Nothing reaches it today (ticket 08 item 7 will).
- **Fix.** Carry `total: usize` in `PartWay` (known: `plans.len()`) and print it.

### M5. ADR-0038 §4's ancestry guard lives only in the preview; `apply` will force-push any commit under a lease

- `crates/graft-ops/src/operation/remote.rs` `check_push` (refuses `RepushNeedsLease`
  only), `preview.rs` `push()` (`NotOnBranch` via `merge-base --is-ancestor`).
- `check` is pure and cannot spawn, so a `Push { source_commit: Some(x), lease: Some(_) }`
  that reaches `apply` without going through `preview::plan` runs
  `push --force-with-lease=… <x>:refs/heads/main` for a commit that is not on the
  branch. "A caller that confirms still goes through `check`" is true and insufficient:
  the seam rule says `ops` decides, and the deciding read is in the wrong function.
- **Fix.** `apply` can spawn: run `is_ancestor` there for a push with `source_commit`
  (before the `Pending` line), or make `PushParams` carry `Tip`-style proof the executor
  re-checks. Not exploitable from the app today (`source_commit: None` at all three
  sites), which is why this is Major, not Critical.

### M6. `Recovered.live` is dropped by the app — a relaunch into a live conflict says nothing from the journal

- `crates/graft/src/jobs/undo.rs:258-281` `read_journal` builds `JournalRead` from
  `recovered.unreadable` and `entries` only; `recovered.live` is never read anywhere in
  `crates/graft`.
- The conductor asked whether any path drops a user into a conflicted repo with
  nothing on screen. This one: `kill -9` mid-rebase, relaunch. `recover_interrupted`
  correctly leaves the entry `Pending`, then the app throws away the id. The graph pane
  shows nothing (`AppState::in_progress` is read only by the commit editor's refusal;
  scout report). Belongs with the banner wiring (group 09), but this group built the
  field and left it unconsumed at the seam.
- **Fix.** `JournalRead.live: Vec<EntryId>` → `AppState`, so the banner group has it;
  until then a sticky status line from `read_journal` is one line.

### M7. Live-entry matching is by kind alone — Graft can adopt a shell's merge as its own entry

- `apply.rs` `live_entry` / `resume_target` / `recover_interrupted`.
- The same class as the four bugs the group found (a ref recorded, compared or re-read
  loosely). A Graft merge entry left `Pending` by a crash *before* git ran + a `git merge`
  started in the shell → recovery keeps the stale entry as "live", `resume(Continue)`
  finishes the shell's merge and closes Graft's entry `Committed` with a before-state
  from a different act. `in_progress_detail` (`head_name`, `onto`) exists and the ticket
  named it; `MERGE_HEAD`/`ORIG_HEAD` are one read away.
- **Fix.** In `live_entry`, require `entry.head_before` to match `rebase-merge/head-name`
  (rebase) or the current symbolic `HEAD` (merge/pick/revert), and `refs_before[branch]`
  to equal `ORIG_HEAD`/`rebase-merge/orig-head`. Refuse `NotInProgress` otherwise.

### M8. A refusal line names a dead key: `MarkersRemain` says "M opens the mergetool"

- `crates/graft-ops/src/operation.rs` `Refusal::explain`, `MarkersRemain` arm;
  `docs/design/02-keymap.md:170` allocates `M` (CONFLICTS context) but nothing binds it
  at this hash (`grep Char('M') crates/graft/src/update/keys.rs` → none).
- Reachable today: stage a marker-bearing file from the changes pane → `Failed` status
  line naming `e` and `M`. Group 03's review made "dead keys hinted" Critical; this is
  the same fault on a refusal line rather than the hint bar. `ApplyError::Conflicted`'s
  Display ("c continues, a aborts") and `CannotSkip` name unbound keys too, though the
  app currently routes `Conflicted` to `conflicted_line`, which names none.
- **Fix.** Drop the key names from `explain` until group 09 binds them, or bind them
  in the same change (the keymap rule).

### M9. A relaunch loses the plan's command lines for the whole entry, permanently

- `crates/graft-ops/src/journal/file.rs` `hold_commands`; `apply.rs` `resume`
  (`entry.commands.clone()` is empty after a relaunch).
- The outcome line `resume` writes then records only `merge --continue`; the Command
  Log for that entry never shows the `merge` that started it. As-built calls this
  "honest rather than reconstructed" — but the plan is *known before the run* and could
  be written on the `Pending` line; nothing needs reconstructing.
- **Fix.** Add `commands` to `PendingLine` (format change, backward-compatible: absent
  = empty). M6's call; file it as a gap if deferred.

### M10. `check_checkout` pins nothing — the preview's worktree list can be stale at confirm

- `history.rs` `check_checkout`: `facts.head != params.head_before` compares
  `HeadRef::Branch(name)` by *name* (no oid), and `CheckoutTarget::Branch(String)`
  carries no tip. Hard choice 3 says every history params is pinned; checkout is the
  one that is not, so a fetch between preview and confirm changes what lands on disk
  without `Moved`. As designed in the note, but the note's rule is violated by the
  note's own shape.
- **Fix.** `CheckoutTarget::Branch(Tip)` and `head_before: Tip`-like oid; `tip_holds`
  covers both.

### M11. File size and placement (PROMPT 6.2)

- `apply.rs` is 1,244 lines; `preview.rs` 1,054; `operation.rs` 905. `resume`,
  `resume_target`, `still_pending`, `close_resume`, `live_entry`, `starts` are one
  cohesive 230-line unit that belongs in `apply/resume.rs`; the eleven per-kind
  planners in `preview.rs` belong in `preview/kinds.rs`, leaving the types, `plan`,
  `fold`, `ghost_oid` and the time machine. `starts()` in `apply.rs` duplicates
  `Operation::starts_in_progress` as a second kind→state table.

---

## Minor

1. **`close_resume` trusts exit 0.** On `Ok(_)` it writes `Committed` without checking
   `state::in_progress == None`. A `rebase --continue` that exits 0 and stops again
   cannot happen for a non-interactive rebase today, but M7's interactive one can; one
   read guards it.
2. **`ghost_oid` input is not what the docs say.** MAP/rustdoc: "SHA-1 over the
   description". Code: merge/revert hash the *subject*, rebase/pick hash each
   `commit.subject`. Fine per plan (ids are per-plan), but the doc is wrong — fix the doc.
3. **Rebase worktree effect over-reports.** `updated(branch.oid, onto)` includes the
   branch's own files, which do not change on disk. `changed_paths(merge_base, onto)`
   is the honest bound.
4. **Dead `Unstage` exemption.** `check` exempts `Unstage(Paths)` of unmerged paths from
   `Unmerged`, but `State::Staged.admits` never admits an unmerged entry, so the plan is
   empty. Harmless now; if ever admitted, `restore --staged` would wipe the conflict
   stages. Remove the exemption or document why it stays.
5. **Handwritten SHA-1 (60 lines) in `preview.rs`.** Tested against standard vectors,
   but it is bit-twiddling in the ops crate where one dependency (`sha1`) would do.
   Plain over clever.
6. **Two "preview" concepts.** `Operation::preview()` (m4, remote command lines) and
   `preview::plan` (m5) — `preview::plan` answers `NotPreviewable(Push)` for the very
   operation `Operation::preview` exists for. Rename one, or route one through the other.
7. **`Refusal::MainlineOnNonMerge` misnamed** for the out-of-range case (acknowledged
   in NOTES; `explain` is honest).
8. **`OperationKind::RevertTo` is dead** (gap filed; decision wanted). If deleted, drop
   the `"revert to"` arm in `kind_name` and the design note's mention.
9. **Detached-HEAD merges still record `HEAD` as a touched ref** (`commit::head_ref_name`
   → `"HEAD"` when detached). Safe only because HEAD is not symbolic then; the group's
   own rule says HEAD is never a touched ref. Worth a comment at `head_ref_name`, or a
   `Detached` branch in `touches` that records nothing.
10. **`identity_now` fails the whole preview** when `user.name` is unset, where the box
    could still show refs and the forecast. Documented as deliberate; noting the cost.
11. **`stopped_on_conflict` maps `rebase-apply` to `Rebase`** as well; a `git am` in
    progress would match a Graft rebase entry's kind. Covered by M7's fix.
12. **Tick 1 arithmetic.** "42 tests (35 + 7)" is right; "13 kinds" is 11 (see C3).
13. **`Conflicted`/`ResumeFailed` `explain` strings are unsanitised** (git stderr,
    journal description). Consistent with the existing `ApplyError` contract and
    documented in MAP; the app sanitises at `view/status.rs::problem`. Fine — recorded
    so the M6 banner group knows the contract.
14. **`Recovered.live` is `Vec<EntryId>` documented "At most one".** Make it
    `Option<EntryId>` or drop the doc claim.
15. **`a_part_way_revert_says_how_far_it_got` asserts the wrong count** (see M4) — the
    test should change with the fix, not be preserved.

---

## Checks the conductor asked for

- **`Operated::Conflicted` carried as a state.** Yes, consistently at the seam:
  `jobs/operate.rs::answer` maps both `ApplyError::Conflicted` and `ResumeFailed` there,
  `update/operate.rs` sets a sticky line via `conflicted_line`, refreshes `Everything`,
  leaves the commit draft. Four app tests pin wording and refresh. The one path that
  still says nothing is the relaunch (M6).
- **Ref record/compare/re-read class.** Found one more (M7, kind-only matching) and one
  design-level (M10, checkout unpinned). `facts.rs` now resolves `HEAD` through its
  branch; `tip_holds`/`head_is`/`check_delete` compare oids; `touches_checkout` and
  `undo::touches_of(SetHead)` record no `HEAD` ref — the four fixes hold.
- **ADR-0038 §4.** `--force-with-lease` only: `check_push` refuses `RepushNeedsLease`,
  `plan_push` has no bare `--force` path (`repush_old_tip_with_lease` asserts both).
  Explicit confirm + wording: `describe` = "force-push 9f8e7d6 → origin/main (with
  lease)", `UndoNote::NotUndoable("this rewrites origin/main for everyone who has
  already pulled it; u undoes your own branch, never a remote")`. The box itself is not
  built; all three app `PushParams` sites pass `source_commit: None`. Ancestry guard
  placement: M5.
- **`forecast_merge` wording.** `ConflictForecast::Likely` throughout; `fold` makes
  `NotChecked` win; rebase past `FORECAST_CAP` → `NotChecked("more than 50 commits")`
  (`a_capped_range_is_not_checked_rather_than_called_clean`). No "will" anywhere.
- **Git work on the UI thread.** None added: `preview::plan` and `resume` are `ops`
  functions; the app calls `apply` from jobs. `PlanPreview`/`Resume` commands are
  ticket 08/09's.
- **`unwrap`/`expect` outside tests.** None in the diffed `graft-ops` sources
  (`grep` clean). No `let _ =`, no `Err(_) => {}`.
- **Architecture.** `graft-ops` depends on `git` + `core` only (unchanged);
  ARCHITECTURE.md's `ops` row already says "Ghost Preview planning". `Oid::from_sha1`
  in `core` is pure. Seam rule held: the app builds `Operation`s and never a
  `GitInvocation`.

## Verified against git (outside the repo, `/tmp`, git 2.54.0)

| Claim | Result |
|---|---|
| (c) `revert --no-edit -m 1 <non-merge>` is accepted | **Held.** exit 0, `Revert "second"` written. `MainlineOnNonMerge` is Graft's own rule, as the builder says. |
| (f) `stash branch` builds at the stash's base, so it cannot conflict | **Held.** New branch tip == stash's first parent, 0 stashes left, exit 0. |
| `merge.ff=false` changes a `Default` merge | **Confirmed** — 2-parent commit where the preview draws a fast-forward (M1). |
| `rebase.updateRefs=true` moves other branches | **Confirmed** — `f1` moved (M2). |

## Tick spot-check

Five acceptance boxes; all five checked against their named tests' source.

| Tick | Test(s) | Holds? |
|---|---|---|
| 1. piece 8 green + (a)–(g) recorded | 42 tests present; `plan-behaviours*.txt` present; (c)/(f) independently confirmed | **Partly** — `branch_from_stash_applies_undoes_redoes` has no redo leg; "13 kinds" is 11 (C3) |
| 2. `preview.commands == plan` lines | `preview_commands_come_from_plan` (11 kinds) + `the_box_says_it_rewrites_the_remote` (push) | Holds |
| 3. no outcome line while in-progress state is live | `conflict_span_…` asserts 1 file line at the stop, 2 after; `rebase_conflicts_twice_same_entry` asserts 1 entry across two stops | Holds |
| 4. `in_force` on `Pending` is false | `pending_is_not_in_force` | Holds |
| 5. MAP + as-built written | files present, signatures match code (checked `RefMove.to`, `plan_revert_to`, `Recovered.live`, `PushParams.source_commit`) | Holds for MAP; **as-built §1's M6 claim is false** (C1) |

4 of 5 hold as written; 1 holds with a false sub-claim (C3). Separately, the as-built —
the document the conductor named as most worth reviewing — asserts an M6 contract the
named test does not exercise and the code contradicts (C1).

## Laptop-safety

`history_ops.rs` builds ~40 small repos; `a_capped_range_is_not_checked…` makes 52
commits; `repush_old_tip_with_lease` uses `LocalRemote` (file transport). All fast-tier;
no sleeps, no timeouts, no network. Conductor's fast run: 41.4 s for 1978 tests. Could
not measure the 5 slowest, peak memory or `target/` size without running cargo.
