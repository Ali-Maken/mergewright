# preview-refuses-what-the-write-refuses: no `[ Confirm ]` for an operation `check` would refuse

Status: ready
From: `docs/reviews/m6-usability-final.md` N4 (:146-160), the bug half. The other half (pull or switch with a dirty tree)
is decided by pick 16: `docs/design-notes/m6-stash-put-back.md`, ADR-0045, group F `set-aside-around-rebase-and-switch`.
Build with: **Sonnet** — one line in ops, one wording function taking the op, one answer route in the app; every
behaviour is pinned by a table test, so correctness shows in the tests.
Scout: `.scratch/m6-conflicts/scouts/67.md` (done, main @ `49de926e`).
Needs: none.
Crates: graft-ops (`preview.rs` `plan`), graft (`update/preview.rs` answer path, `update/operate.rs` refusal words,
tests).
Group: **D** (the preview gate), alone. Runs before G, E and F (order in the design note).

## Amended 2026-09-25 (planner, after scout 67 and pick 16)
- **Pick 16 changes what this ticket is for.** Group F stops refusing a dirty tree for rebase and checkout: it sets the
  edits aside. So this ticket's dirty-tree cases are true at its own hash and flip in F. Kept: the gate (one `check`,
  preview and write), the one wording function, the refusal answer's route. Dropped: the 15-key walk (F replaces that
  journey with `p r r P y`, 5 keys) and the journey row below.
- **Scout finding 1** — a refused preview sets no `Status` today; the job's `Failed` answer opens `preview failed: …` with
  one `Close` (`update/preview.rs:718-747`). Decided: a `PreviewError::Refused` answer **closes the box and sets the
  status** (the route below). Any other `PreviewError` keeps the `preview failed:` box: that one is a failure; a
  refusal is not, and must not read as one (CLAUDE.md, status fields).
- **Scout finding 2** — the write-time words take their verb from the running write (`operate.rs:917-926`, `verb_of`
  `:945`). The shared function takes the `Operation`; the write-time caller passes the op it launched.
- **Scout finding 3** — plain pull has no preview and no clean-tree rule (`remote.rs:365`, `operation/remote.rs:200-214`).
  Out of this ticket; F's set-aside covers the rebase reached from `p r`, and a fast-forward pull stays git's to refuse
  (ADR-0045 Consequences).
- **Scout finding 4** — the Scout line now names the real file.

## Read first
- `.scratch/m6-conflicts/scouts/67.md` (the whole page).
- `docs/reviews/m6-usability-final.md` N4 (:146-160).
- `crates/graft-ops/MAP.md` :30 (`is_history`), :52 (`Refusal::DirtyTree`), :53 (`Operation::check`), :198-212
  (`preview`, `plan`, `PreviewError::Refused`).
- `crates/graft/MAP.md` §`update::preview` (:380-397), §`update::operate` (:218-229).
- `docs/design-notes/m6-stash-put-back.md` "Journeys" (why the walk moved to F).
- UX principles H5 ("block the impossible, don't warn after"): `docs/research/ux-principles.md`.

## The cause (rule 7, lines from scout 67 on `49de926e`)
- `preview::plan` dispatches per kind with no `op.check` call (`graft-ops/src/preview.rs:324-347`); `apply` runs `check`
  before writing (`operation/history.rs:432-437`, rebase `:460-464`, checkout `:491-501`). So the preview draws
  `[ Confirm ]` for a write `apply` will refuse.
- The refusal's words are set only after the confirm (`graft/src/update/operate.rs:917-926`).

## Decided here
- **`preview::plan` runs `op.check(facts)` first** and returns `PreviewError::Refused(refusal)`. `read_facts` always reads
  the status (`graft-ops/src/facts.rs:112`), so `check` has what it needs for every kind. `apply` keeps its own `check`.
- **A `Refused` answer opens no box.** The preview job's `Refused` answer clears the waiting box, sets the status from
  `refusal_status`, and gives focus back to where `.`/`r`/`m` was pressed. Why no box: the preview is modal, so the key
  the words name (`S`) would need `esc` first (UX §1.3); and a picture of a write that cannot run is not "the result
  before you confirm". This route also carries the kind-local refusals `plan` already returns (merge `NotFastForward`,
  `NotOnBranch`; scout :4): they move from the `preview failed:` box to the status line.
- **One wording**: `refusal_status(op, refusal)`, called by the preview answer and by `operated`'s refusal arm.

## Scope
**In:** the `check` call in `plan`; `refusal_status` and both callers; the `Refused` answer route; tests; one capture.
**Out:** set-aside (group F); plain pull's missing preview; the `p` "diverged" line; `merge-tree` forecasts;
`plan_revert_to` (separate, `jobs/preview.rs:58`, confirmed by the scout).

## Signatures
```rust
// graft-ops, preview.rs — signature unchanged; the first statement is new:
pub fn plan(repo: &RepoHandle, git: &GitVersion, op: &Operation, facts: &RepoFacts, stop: &dyn Stop)
    -> Result<PreviewPlan, PreviewError>;
//     op.check(facts).map_err(PreviewError::Refused)?;      // as `apply` does, before any read
// (parameter types as they are on main; NOTES names them if they differ)

// graft, update/operate.rs — the one wording of a refusal of an operation he asked for, preview or write.
// Replaces `history_refusal`'s use of `state.remote.running` for the verb.
pub(crate) fn refusal_status(op: &graft_ops::Operation, refusal: &graft_ops::Refusal) -> Status;
```

## The failing tests first
- graft-ops `preview` tests: `a_rebase_preview_refuses_a_dirty_tree`, `a_checkout_preview_refuses_a_dirty_tree` — one
  tracked file changed → `Err(Refused(DirtyTree { count: 1 }))`. Red today. **F flips both** (they become set-aside
  tests); say so in their doc comments.
- graft `update` tests: `r_on_a_dirty_tree_opens_no_preview_and_says_why` — diverged line up, tree dirty, `r`, then the
  job's `Refused` answer: `preview` is `None`, status `rebase needs a clean tree · S stash 1 changed file`, focus on the
  graph. Red today (a box). **F flips it.**
- graft `update` tests: `a_refused_merge_preview_says_why_on_the_status_line` — `NotFastForward` answer: no box, status
  set by `refusal_status`. Red today (`preview failed:` box).

## Touches (each entry, and how the builder checks it)
- **Every previewable kind** now passes `check` first. Check: `every_preview_refuses_exactly_what_check_refuses` — a
  table over every kind `plan` matches, clean tree and dirty tree: `plan` is `Err(Refused(r))` iff `check` is `Err(r)`.
- **Tests that preview on a dirty tree** — *pin: the scout could not enumerate them.* The builder runs the suite after
  the `plan` line alone, lists every test that turned red in NOTES with the refusal it now gets. Any that pinned a
  picture on purpose → **stop**, `blocked: <test> previews what check refuses`.
- **`preview failed:` box** — still drawn for non-refusal errors. Check: an existing failure test stays green; `grep -rn
  "preview failed" crates/graft/src` → each remaining site is a non-`Refused` arm.
- **The two wordings** become one: `grep -n "needs a clean tree" crates/graft/src` → one site.
- **`live_refusal` pre-job gate** (`update/preview.rs:283-302`) keeps its place. Check: an in-progress preview still
  shows `finish or abort the <kind> first` (existing test green).
- **Checkout from the `.` menu and `⏎` on a sidebar branch** on a dirty tree: refused before the box. Check: rows in the
  table above.
- **Captures** — *pin: fixture tree states not checked by the scout.* For each of `docs/shots/m5-01-preview-rebase-*`,
  `m5-02-preview-merge-*`, `m5-ui-preview-{box,rebase-*,scrolled-*}.txt` and any `NotFastForward`/`preview failed`
  capture (`grep -rln "preview failed" docs/shots`): read the writing test's fixture, say in NOTES whether its tree is
  dirty, then `git diff --exit-code` on the file or regenerate it through its test. `m5-contact-sheet-captions.tsv`
  follows.
- **TESTING.md** rows describing a `preview failed:` box for a refusal: `grep -n "preview failed" TESTING.md`; listed in
  NOTES for the acceptance ticket.
- **Journal/undo:** none (nothing is written on a refusal). **Keymap:** none. **Hint bars:** none (no new box).
- **MAP.md:** graft-ops `preview::plan` ("runs `check` first"); graft `refusal_status`, the `Refused` route.

## Acceptance (name the verifying test at tick time)
- [ ] The four tests above, red then green.
- [ ] `every_preview_refuses_exactly_what_check_refuses`.
- [ ] Control: `a_merge_preview_on_a_dirty_tree_still_draws` (merge is not refused dirty, scout :11).
- [ ] `the_preview_refusal_and_the_write_refusal_read_the_same` (same `Status` from both callers for `DirtyTree`).
- [ ] Capture (rule 6; colour **unverified**, rule 5): `docs/shots/m6/fix-rebase-refused-before-preview-120x40.txt`,
      written by a test. **F replaces it** (the dirty-tree refusal no longer exists after F).
  - **What right looks like:** after `p r` with one changed file: no preview box; the bottom row reads
    `rebase needs a clean tree · S stash 1 changed file`; the graph is where it was.
- [ ] NOTES: the pinned lists (tests turned red, capture fixtures' tree states).
- [ ] MAP.md updated (doc, no test).

## For the conductor
Journey row: none from this ticket. The dirty-tree journeys are group F's (`m6-stash-put-back.md` Journeys).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
Medium–small: one line in ops, one wording function, one answer route, a kind table, one capture. One context.
