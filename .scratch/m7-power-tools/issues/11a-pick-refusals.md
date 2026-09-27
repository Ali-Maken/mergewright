# 11a pick-refusals — 11's graft-ops part: `check_pick` refuses what it can't take; set-aside for a pick

Status: partial — blocked: needs `crates/graft-ops/src/preview/kinds.rs` and `crates/graft-ops/src/apply.rs` (see NOTES)
Build with: Opus. Two refusals plus an ADR-0045 extension (a new set-aside kind: undo must hold).
Needs: none.
Blocked by: none. Wave: see `.scratch/m7-power-tools/parallel-plan.md` (after 10b and 05: all three hold
graft-ops `operation.rs` / `operation/history.rs`).
Split by: planner, 2026-09-27, from `11-cherry-pick-key.md` (ADR-0051 §1). 11 keeps the `o` route, the `.` slot-2
label and dim, the app tests, walks and captures.
Crates: graft-ops only.
Files: crates/graft-ops/src/{operation.rs, operation/history.rs, operation/history/tests.rs}; crates/graft-ops/src/
preview/put_back.rs (only if the set-aside forecast needs a CherryPick arm); crates/graft-ops/tests/it/
cherry_pick.rs (new, seam); MAP.md (graft-ops). A graft exhaustive match on `Refusal` that breaks, or a graft test that
asserted today's pick-over-edits behaviour → `blocked: needs <graft file>`, stop.
Scout: 11's scout (ADR-0035), brief items 1 and 4. Item 1 answers KNOWN_GAPS :806's question with its line.

## Scope
11 "Decided": the two `Refusal`s with their words; "Unsaved edits the pick touches" (set aside and put back, one entry)
**only if** the conductor confirms "Where I'd differ" 4 is approved — else nothing is set aside and the refusal words
11 names stand. 11 "Signatures", graft-ops block.

## Acceptance (tick 11's graft-ops box in 11's file, each test named)
- [ ] The five graft-ops tests of 11 "Acceptance" (the set-aside one only if approved). **Not ticked:** all five exist
      and pass, but the three refusal tests prove `check_commit`, not that the preview/apply refuse (blocked wiring).
      11's box stays unticked.
- [x] All set-aside tests green — `test(/pick|set_aside|put_back/)` 164/164, incl.
      `picking_over_touched_unsaved_edits_sets_them_aside`, `a_pick_put_back_that_clashes_stops_as_a_clash`.
- [x] NOTES: KNOWN_GAPS :806 answered with its line (below). ADR-0045 amendment rows owed (the conductor). MAP.md
      (graft-ops) updated.
- [x] Gates workspace-wide: fmt, clippy `-D warnings`, nextest 4457/4457, `cargo test --doc`.

## Size
~35k.

## NOTES (builder, 2026-09-27)

**Started from** main @ 6fb4b713 (10b merged). Lines confirmed: `check_pick` `operation/history.rs:567-572`,
`sets_aside` `operation.rs:1261`, pick preview `preview/kinds.rs:295`, `put_back::forecast` `preview/put_back.rs:53`.
Scout: `.scratch/m7-power-tools/scouts/11.md` used (items 1 and 4); its line numbers were pre-10b, re-read above.

**Why blocked.** `check` reads no object (`history.rs` `RevertParams::parents` doc: "`check` reads no object"), so
`check_pick` cannot see "HEAD or an ancestor" or "a merge". The codebase's pattern for a refusal needing a read is the
re-push: `preview/kinds.rs:637` refuses in the preview, `apply.rs:438` `guard_repush` asks again before the `Pending`
line. Adding fields to `PickParams` would break graft's struct literals (`update/drag.rs:286`,
`update/context_menu/verbs.rs:123`); a `RepoFacts` field breaks five graft-ops test files off the list. So the decision
is built and tested here, pure, and the two call sites are the seam:

- `PickParams::check_commit(commit, on_head, parents) -> Result<(), Refusal>` (`operation/history.rs:135`), public:
  the app can ask it from its graph for slot 2's dim words (11), the same words.
- **Next step (exact):**
  1. `preview/kinds.rs` `cherry_pick`, in the loop: `commits_between(repo, head, commit, 1)` already answers both
     facts — empty ⇒ `on_head`; else `summary.parents.len()`. Replace the `"is already here"` early return
     (:312) with `params.check_commit(*commit, true, 0).map_err(PreviewError::Refused)?`, and after `summary`
     call `params.check_commit(*commit, false, summary.parents.len()).map_err(PreviewError::Refused)?`.
  2. `apply.rs`: `guard_pick(ctx, op, facts)` beside `guard_repush` (called after it, before `guard_in_the_way`):
     for each commit, `crate::preview::is_ancestor(repo, commit, params.head.oid)` and its parent count (one
     `rev-list --parents -n1`, or `commits_between` as the preview does), then `check_commit`. Refused ⇒
     `ApplyError::Refused`, nothing written.
  3. Tests in `tests/it/cherry_pick.rs`: preview and apply of HEAD, an ancestor, a merge each refused with the
     words; the journal untouched on refusal.
- **Gotcha:** until step 2 lands, `apply` of a pick of `HEAD`/an ancestor over dirty edits now sets them aside first
  (new with this ticket) and then runs git; what git does on that pick (empty-commit stop or error) is **unverified** —
  the sandbox refused a throwaway git run outside the worktree. The `.` menu slot 2 reaches that today (never dimmed).

**KNOWN_GAPS :806 answered, from code at 6fb4b713:** `check_pick` refused only `NoCommits` (`history.rs:568`) and a
moved/changed HEAD (`history.rs:571` → `head_is`). A pick of HEAD or an ancestor: the preview opened with no ghosts and
`UndoNote::NotUndoable("<sha> is already here")` (`preview/kinds.rs:312`); nothing blocked `y`. A merge: no check at
all; `plan_pick` (`history.rs:774`) never passes `-m`, so git's own refusal would surface (git's text not confirmed).

**Built (all green):**
- `Refusal::AlreadyOnBranch { commit, branch: String }` and `Refusal::PickMerge { commit }` with the note's words
  (`operation.rs`). `branch` is a short-name `String`, not `RefName` as 11 "Signatures" says: it matches the sibling
  `NotOnBranch`, and a detached `HEAD` has no ref name (words read `already on HEAD`).
- Set-aside for `CherryPick` ("Where I'd differ" 4, approved per the conductor): `sets_aside` (`operation.rs`),
  `touches` adds `refs/stash` (without it `u` failed: "never recorded refs_before[refs/stash]" — the undo test caught
  it), put-back forecast `Landing::Picked` (`preview/put_back.rs`: each commit replayed from its own parent by
  `merge_tree_step`, then `commit-tree`; a pick that may stop ⇒ `NotChecked`). `forecast` split (`landing_of`) for
  clippy's line cap.
- **Deviation:** the pick wraps any changed tracked file (as rebase does), not only "the paths the pick touches":
  `sets_aside` reads no commit. An edit to a file the pick leaves alone is **not tested for the pick**; the wrap is
  the shared one rebase's round-trips (`set_aside.rs`) cover.

**Tests (named):**
- `picking_head_is_refused_already_on_branch`, `picking_an_ancestor_is_refused_already_on_branch`,
  `picking_a_merge_is_refused` (`operation/history/tests.rs`) — at `check_commit`, red first (did not compile).
  They prove the words and the decision, **not** that the preview never opens (that is the blocked wiring).
- `picking_over_touched_unsaved_edits_sets_them_aside` (`tests/it/cherry_pick.rs`): preview `set_aside` + forecast
  `Clean`; apply → one `Committed` entry, both changes in `a.txt`, status and stash list as before; `u` exact to before,
  `U` exact to after. Red first (undo failed on `refs/stash`).
- `a_pick_put_back_that_clashes_stops_as_a_clash`: forecast `Likely{a.txt}`; `Clashed{SetAside}` on `a.txt`, `S`
  kept, entry `Committed`, unmerged; `u` exact to before.
- `a_pick_that_sets_aside_touches_the_stash`, `sets_aside_table` (pick moved into the wrapping kinds).
- Set-aside filter `test(/pick|set_aside|put_back/)`: 164/164. Workspace: nextest 4457/4457, doctests ok, fmt, clippy
  `-D warnings` clean.

**Touches:** graft's `Refusal` matches — none exhaustive (workspace builds, clippy clean); graft tests asserting
pick-over-edits — none broke (full nextest); set-aside/put-back tests all green (filter above); journal/undo — proven
by the two `u` tests; M6 cherry-pick stop/banner — unchanged code paths, m6 tests green in the full run.
**ADR-0045 amendment rows owed (conductor):** header "for rebase and checkout" → add cherry-pick; §4 kinds list.
MAP.md (graft-ops) updated: `SetAside`/`sets_aside`, `PickParams::check_commit`, `touches`, `PreviewPlan.put_back`.
**Cause vs symptom:** cause — the refusals live in one decision fn every caller shares; set-aside by the existing wrap.
