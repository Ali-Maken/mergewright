# 30j: Track undo leaves branch config; runner catch-alls; five small fixes from review 16b..21b

Status: ready
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

- [ ] Red first: `undo_of_a_track_switch_removes_the_upstream_it_wrote` (graft-ops, real git): after undo,
      `git config branch.<b>.remote` is unset; a pre-existing config on another branch unchanged. Record git version.
- [ ] Red first: a pre-existing upstream for the same branch name survives undo (name the test).
- [ ] `not_a_job` exhaustive: no `other`/`_` arm (grep in NOTES); build proves it.
- [ ] Red first: `a_switching_create_lands_on_the_new_branch` — status `on nb ✓ · u goes back`.
- [ ] Red first: `a_failed_leaves_read_is_shown_not_still_reading` and the cancelled twin.
- [ ] Red first: `submodule_facts_arrive_with_the_diff` (update::detail); `body.rs` has no cursor copy.
- [ ] History bar says `⇥`; captures re-taken by their tests; ≤200 KB; colour unverified.
- [ ] Journey walk parses 2 for `menu-checkout` in m6-conflicts (name the walk test).
- [ ] MAP.md per crate. Gates workspace-wide.

## Touches (check each, say how)

Every other undo of a checkout (config untouched). Every `Command` added since 21 (list). Other bars' `⇥`. Every
capture with the history bar or a submodule diff. KNOWN_GAPS rows closed by this (list for the conductor; don't edit).

## NOTES (builder)
