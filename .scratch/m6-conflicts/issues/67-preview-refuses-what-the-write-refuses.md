# preview-refuses-what-the-write-refuses: no `[ Confirm ]` for an operation `check` would refuse

Status: draft
From: `docs/reviews/m6-usability-final.md` N4 (:146-160), the bug half. The other half (pull or switch with a dirty tree:
stash around it, or refuse) is a feature decision; see `.scratch/m6-conflicts/usability-final-plan.md`.
Build with: (conductor, after the scout)
Scout: `.scratch/m6-conflicts/scouts/preview-refuses-what-the-write-refuses.md` — brief below.
Needs: none.
Crates: graft-ops (`preview.rs` `plan`), graft (`update/preview.rs` open path, `update/operate.rs` refusal words, tests,
walks).
Group: **D** (the preview gate), alone.

## Read first
- `docs/reviews/m6-usability-final.md` N4 (:146-160) and the task table row "Push rejected → recover, dirty tree" (:69).
- `crates/graft-ops/MAP.md` :30 (`is_history`), :52 (`Refusal::DirtyTree`), :53 (`Operation::check`), :198-212
  (`preview`, `plan`, `PreviewError::Refused`).
- `crates/graft/MAP.md` :283 (`live_refusal`, used by `preview::open_preview`).
- `docs/design-notes/m5-history-and-ghost-preview.md` hard choice 4 (:165, "refused, never `--autostash`"): unchanged here.
- UX principles H5 ("block the impossible, don't warn after"): `docs/research/ux-principles.md`.

## The cause (rule 7, lines at `1b1995cc`; the scout re-reads them on main)
- The clean-tree rule lives in `Operation::check` (`graft-ops/src/operation/history.rs:432-436`), which `apply` runs
  before writing. `preview::plan` does not run it, so the preview draws `[ Confirm ]` for a rebase `apply` will refuse.
- The refusal's words are set only after the confirm (`graft/src/update/operate.rs:916-926`). Observed: the preview read
  `working tree: 1 file updated · no conflicts expected · undo: u takes this back` with `[ Confirm ]`; `r` then gave
  `rebase needs a clean tree · S stash 1 changed file`.
- Two gates for one act, one of them late. The fix makes `plan` ask the same `check` first.

## Decided here
- **`preview::plan` runs `op.check(facts)` before anything else** and returns `PreviewError::Refused(refusal)`. One gate:
  whatever `apply` would refuse, the preview refuses first. `apply` keeps its own `check` (the tree can change between).
- **No preview opens on a refusal.** The status line shows the same words the write-time refusal shows, through one
  wording function, and focus stays where it was. Why not a preview with the refusal in place of `[ Confirm ]`: the
  preview is modal, so `S` (the key the words name) would need `esc` first, one more key for nothing (UX §1.3); and a
  picture of a write that cannot run is not "the result before you confirm".
- **Journey** (the review's dirty-tree row, walked from a rejected push, tree dirty): `p r` (refused, words name `S`) ·
  `S s <msg> ⏎ esc` · `p r r` · `P y` · `S p esc` = **15** (was 16: `p r r` then the refusal). The rest of the gap to
  JetBrains' 3–4 is the stash-around decision, not this ticket.

## Scout brief (≤1 page, code not intent)
1. `graft-ops/src/preview.rs` `plan`: does it read full `RepoFacts` (the working-tree status included) for every
   previewable kind, or does `FactsScope` leave `status` empty for some? `check` needs it for `DirtyTree`.
2. `operation/history.rs:420-440`: which kinds `check` refuses `DirtyTree` for (rebase, checkout, others?), and every
   other refusal `check` can return for a previewable kind. Mark which could be true when a preview opens today.
3. How the app shows a `PreviewError::Refused` today (e.g. an `--ff-only` merge of diverged branches): which function,
   which `Status` variant, where focus lands. And how `operate.rs:916-926` words `DirtyTree`. Are these one function?
4. Every caller of `preview::plan` (the app's preview job, `plan_revert_to` is separate — confirm), and every test that
   previews on a dirty tree or mid-operation and expects a picture.
5. Where `S` works from after the refusal: the graph, the changes pane, the Tab layout at 80 columns.

## Scope
**In:** the `check` call in `plan`; one wording function for a refusal (preview or write), called from both; tests, walk,
capture.
**Out:** stash-around (autostash); the `p` "diverged · m merge it  r rebase onto it" line (it still offers `r`; the
refusal comes one key later, before any preview); `merge-tree` forecasts; the time machine's `plan_revert_to`.

## Signatures
```rust
// graft-ops, preview.rs — signature unchanged; the first statement is new:
pub fn plan(repo: &RepoHandle, git: &GitVersion, op: &Operation, facts: &RepoFacts, stop: &dyn Stop)
    -> Result<PreviewPlan, PreviewError>;
//     op.check(facts).map_err(PreviewError::Refused)?;      // as `apply` does, before any read
// (the parameter types are as they are today; the scout names them in NOTES if they differ)

// graft, update/operate.rs (or wherever scout Q3 finds the write-time words) — the one wording of a refusal of an
// operation the user asked for, preview or write. The two callers are the preview's `Refused` answer and `operated`'s.
pub(crate) fn refusal_status(op: &graft_ops::Operation, refusal: &graft_ops::Refusal) -> Status;
```

## The failing tests first
- graft-ops `preview` tests: `a_rebase_preview_refuses_a_dirty_tree` — one tracked file changed, `plan(Rebase ..)` →
  `Err(Refused(DirtyTree { count: 1 }))`. Red today (a picture). Same for `a_checkout_preview_refuses_a_dirty_tree`.
- graft `update` tests: `r_on_a_dirty_tree_opens_no_preview_and_says_why` — the diverged line up, tree dirty, `r`: no
  preview, status `rebase needs a clean tree · S stash 1 changed file`. Red today (the preview opens).

## Touches (each entry, and how the builder checks it)
- **Every previewable kind** now passes `check` first. Check: `every_preview_refuses_exactly_what_check_refuses` — a
  table over each kind scout Q2 lists, on a clean tree and on a dirty one: `plan` is `Err(Refused(r))` iff
  `check` is `Err(r)`.
- **Tests that preview in a state `check` refuses** (scout Q4): each named in NOTES with the refusal it now gets; any
  that was pinning a picture on purpose → **stop** and report (`blocked: <test> previews what check refuses`).
- **The two wordings** become one: `grep -n "needs a clean tree" crates/graft/src` → one site.
- **`live_refusal` in `open_preview`** (MAP :283): keeps its place ahead of the job; the new refusal comes from the job.
  Check: an in-progress preview still shows `finish or abort the <kind> first` (existing test green).
- **Checkout from the `.` menu and `⏎` on a sidebar branch** on a dirty tree: refused before the preview. Check: a row
  in the table above.
- **Captures** of a preview drawn on a dirty tree: `grep -rln "working tree: .* updated" docs/shots`; each fixture's
  tree checked; regenerated or unchanged.
- **Journal/undo:** none (nothing is written on a refusal). **Keymap:** none.
- **MAP.md:** graft-ops `preview::plan` ("runs `check` first"); graft `refusal_status`.

## Acceptance (name the verifying test at tick time)
- [ ] The three tests above, red then green.
- [ ] `every_preview_refuses_exactly_what_check_refuses`.
- [ ] Control: `a_merge_preview_on_a_dirty_tree_draws_when_check_allows_it` (only if scout Q2 finds merge not refused;
      else NOTES says merge is in the refused set and this row is the table's).
- [ ] `the_preview_refusal_and_the_write_refusal_read_the_same` (same `Status` from both callers for `DirtyTree`).
- [ ] Walk, real repo (`tests/it`): `a_rejected_push_with_a_dirty_tree_is_fifteen_keys` — the journey above, the key
      count asserted ≤ 15, `git log origin/main` has his commit on top of the teammate's at the end, and the stash list
      is empty.
- [ ] Capture (rule 6; colour **unverified**, rule 5): `docs/shots/m6/fix-rebase-refused-before-preview-120x40.txt`.
  - **What right looks like:** after `p r` with one changed file: no preview box on screen; the bottom row reads
    `rebase needs a clean tree · S stash 1 changed file`; the graph is where it was.
- [ ] MAP.md updated (doc, no test).

## For the conductor
Journey row to add to `docs/design-notes/m6-usability-fixes.md` Journeys: "Rejected push → recover, dirty tree: 16 as
walked → **15**; stash-around (open, see the plan) → ~6".

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
Medium: one line in ops, one wording function, a kind table, one walk, one capture. One context.
