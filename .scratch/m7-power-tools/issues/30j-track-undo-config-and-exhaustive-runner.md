# 30j: Track undo leaves branch config; runner catch-alls; five small fixes from review 16b..21b

Status: done
Build with: Opus. Undo of a config write (journal + restore) is the one hard part.
Needs: nothing new.
Blocked by: nothing. Concurrent: 03b (graft-ops `apply.rs` Pull path, `operation.rs` `sets_aside`/`touches`, pull
banner) — don't touch those. 21c waits for this ticket (shares `jobs/runner.rs`).
Scout: none. The review names every line (`docs/reviews/m7-16b-to-21b.md` Major 1, Major 2, Minors 1, 2, 3, 4, 8).

Files: `crates/graft-ops/src/operation/history.rs` (~:866), `crates/graft-ops/src/restore/steps.rs` (~:64), the journal
entry type if the config must be recorded there (name it in NOTES), graft-ops tests; `crates/graft/src/jobs/runner.rs`
(`not_a_job`, :1133, :1161; the `read_leaves` cancel path :1027-1041); `crates/graft/src/update/switch.rs` (:242-260);
`crates/graft/src/update/remote.rs` (:44, `RejectedPush.leaves`), `crates/graft/src/update/mod.rs` (`job_failed` ~:980,
the leaves arm only); `crates/graft-ui/src/detail_pane/body.rs` (:96-105, :240-249) and the `update::detail` file
where the arriving `FileDiff` lands (name it); `crates/graft-ui/src/graph_screen/hints/lists.rs` (:175, :177);
`docs/design-notes/m6-conflicts.md:526`; tests beside each; captures their tests re-take; MAP.md (each crate touched).

## Fixes (from the review; each is decided)

1. **Major 1 — Track undo leaves `branch.<b>.remote/merge`.** Probe (git 2.54): after undo, a later `b sara/x`
   silently tracks `origin/sara/x`. Journal the config the Track switch writes; undo removes it (only what it wrote;
   config that existed before stays). Gap `M7/10b-track`.
2. **Major 2 — `not_a_job` catch-alls** (`runner.rs:1133`, `:1161`): list every `Command` variant; no `other` arm, so
   a new `Command` fails to compile instead of panicking on the event loop.
3. **Minor 1 — switching create lands `on <nb> ✓ · u goes back`**: one arm in `landed_words`
   (`Operation::BranchCreate(p) if p.checkout`). Gap `M7/10b-q` (answered yes, note `:702-707`).
4. **Minor 2 — a failed/cancelled leaves read strands `Reading`**: keep the `JobId` beside the key; on
   `JobFailed`/`JobCancelled` for it, store `Failed(<words>)`. Then `P` no longer says `still reading` forever and the
   prelude's `settled` answers.
5. **Minor 3 — 30i's facts, cause fix**: merge `SubmoduleRows`' facts into the arriving `FileDiff` in `update::detail`
   keyed by path; remove the draw-time copy in `body.rs`. Gap `M7/30i-facts`.
6. **Minor 4 — `tab` → `⇥`** in the history bar (`lists.rs:175,177`). Conductor ruling: `⇥`, as every other bar
   (H4 consistency). Re-take the history captures.
7. **Minor 8 — journey fence**: `m6-conflicts.md:526` count → 2 (keep the dated amendment comment).

## Acceptance

- [x] Red first: `undo_of_a_track_switch_removes_the_upstream_it_wrote` (graft-ops `tests/it/switch_rule.rs`, real
      git 2.54.0 Apple Git-157): red (left `["origin"]`), green after. Also checks main's config unchanged, redo
      re-adds it, a later branch of the same name has no upstream.
- [x] Red first: `a_pre_existing_upstream_for_the_same_name_survives_undo` (same file) — red, then green.
- [x] `not_a_job` exhaustive — `runner.rs` dispatch lists every `Command`; `grep -nE "^\s*(other|_) =>"
      runner.rs` finds only `:328` (dispatch_detail) and `:361` (dispatch_history), none in dispatch. Build proves it.
- [x] Red first: `a_switching_create_lands_on_the_new_branch` (`update/switch/tests.rs`); walk
      `m7_walk::switch` new-branch-with-edits asserts `on nb` / `u goes back`.
- [x] Red first: `a_failed_leaves_read_is_shown_not_still_reading`, `a_cancelled_leaves_read_is_shown_not_still_reading`
      (`update/remote/tests/lease/leaves.rs`).
- [x] Red first: `submodule_facts_arrive_with_the_diff` (`update/detail/inside_facts.rs`); `grep cursor_submodule`
      in graft-ui finds nothing.
- [x] History bar says `⇥`: `hints/tests.rs`, `hint_bar.rs`, `history_place.rs`, `place/tests/hints.rs` (8 red first);
      captures re-taken by `history_place::capture_*` (4 files, 1 line each, all < 10 KB). Colour unverified.
- [x] `menu_checkout_is_fenced_at_two` (`tests/it/m5_walk/journeys.rs`): red (`Ok(3)`), green after the note edit.
- [x] MAP.md: graft-core, graft-ops, graft-ui, graft. Gates workspace-wide (see NOTES).

## Touches (check each, say how)

Every other undo of a checkout (config untouched). Every `Command` added since 21 (list). Other bars' `⇥`. Every
capture with the history bar or a submodule diff. KNOWN_GAPS rows closed by this (list for the conductor; don't edit).

## NOTES (builder)

- **Compacted: yes**, once, after all seven fixes were built; only the gates, ticket and commit remained. Nothing left.
- Scout: none — the review names every line.
- **Journal type changed:** `graft_core::JournalEntry` gains `config_before` / `config_after`
  (`BTreeMap<String, Vec<String>>`, serde default + skipped when empty, so old journals read). Recorded for every
  touched `refs/heads/<b>` (`branch.<b>.*`, via new `graft-ops/src/branch_config.rs`). Undo/redo emit
  `Step::SetBranchConfig` where before ≠ after; restore rollback reverses it. Cost: +1 `git config` spawn at begin and
  finish of ops touching a branch ref; none when no branch ref is touched.
- Cause vs symptom: all seven are cause fixes. Floor of the history bar ladder 30 → 28 cells (⇥ is narrower) — a
  knock-on of the ruling, comment updated.
- Known limits (not fixed, for KNOWN_GAPS): an *interrupted* Track (`mark_interrupted`) does not read
  `config_after`, so its undo won't remove the upstream; config restore is not compare-and-swap (a config edit made
  outside Graft between op and undo is overwritten). `runner.rs` still has catch-alls at `:328` and `:361`
  (dispatch_detail/history sub-dispatch, out of scope). Pre-existing size caps: `update/detail/ops.rs` 705,
  `update/remote.rs` 531, `runner.rs` ~1200 lines.
- Out-of-list files Major 1 needs (each line): `graft-core/src/journal.rs:313-324` — the two config fields; the
  journal is where undo reads "before", so config must be recorded there. `graft-ops/src/apply/record.rs:89,132-133,
  145-157` — `begin` reads `config_before` for the touched branch refs (helper `config_before`); `:259-262` —
  `read_after` reads `config_after`, a failed read becomes `AfterState::BranchConfig`, not swallowed.
  `graft-ops/src/apply/fold.rs:157-158` — the struct literal of the folded entry must carry the two fields (compile).
  `fold.rs:63`'s stale doc (07a note) not touched — not in that line.
- Files outside the listed set: graft-ops `facts.rs`, `apply/record.rs`, `apply/error.rs`, `apply/fold.rs`,
  `journal/lines.rs`, `undo.rs`, `undo/replaced.rs`, `restore.rs`, `restore/rollback.rs`, `lib.rs`; graft
  `update/keys/bound.rs`, `update/mod.rs` (job_cancelled too), `prelude.rs`, `push_box.rs`; graft-ui
  `hints/context.rs`; JournalEntry literals in ~19 test/source sites; `TESTING.md:2624,2631` (`tab file` → `⇥ file`).
  None in 03b's files (`apply.rs`, `operation.rs`). **03b merge risk:** any new `JournalEntry` literal there needs the
  two config fields.
- Touches: other checkout undos — plain Checkout touches no branch ref, commits record equal before/after → no step;
  graft-ops+core suite 908 pass. Commands since 21 (`TraceHistory`, `ReadWholeFile`, `PlanRewrite`, `ResolveRowTags`,
  `ReadLeaves`) all armed — build proves. Other bars' `⇥`: `lists.rs:73,302,356,365`, `hints.rs:483` already `⇥`.
  Captures: only the 4 history-bar captures drifted in a full `GRAFT_UPDATE_SHOTS=1` run (raster png regenerated
  identical). Still saying `tab file`: `docs/design/01-view-mockups.md`, `docs/design-notes/m7-power-tools.md`,
  prototype shots — design/historical, left.
- Gates (2026-09-28, workspace-wide): fmt ok, clippy ok, nextest 4531/4531 passed (4 skipped), doctests ok.
  A first run stalled on the watchdog with no output; re-run clean.
- Gap rows closed (conductor to move): `M7/10b-track`, `M7/10b-q`, `M7/30i-facts`; the leaves `Reading` strand
  (Minor 2).
