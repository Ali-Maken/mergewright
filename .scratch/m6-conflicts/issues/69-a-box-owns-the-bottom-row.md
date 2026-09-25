# a-box-owns-the-bottom-row: while a box is open, the bottom row shows its keys, never a status from before it

Status: draft
From: `docs/reviews/m6-usability-final.md` N5 (:162-176). Folds in KNOWN_GAPS **M6/50** (the lease box's keys covered by
the rejection's sticky line; same cause), the stale-status part of **n3** (:229-235: the preview's bar shows the
previous status), and **Q4** (:269-270) as a pinning test only.
Build with: (conductor, after the scout)
Scout: `.scratch/m6-conflicts/scouts/a-box-owns-the-bottom-row.md` — brief below.
Needs: none. Same group as `resume-words-say-what-happened`; either order, rebase on the first.
Crates: graft (`view/frame/bars.rs`, `view/status.rs`, `view/rows.rs`, `update/undo.rs`, the other box-opening answer
handlers, tests, walks). graft-ui only if scout Q1 finds the problem/notice rows chosen there.
Group: **B** (the bottom row tells the truth).

## Read first
- `docs/reviews/m6-usability-final.md` N5 (:162-176), n3 (:229-235), Q4 (:269-270).
- KNOWN_GAPS.md row M6/50 (:797).
- `.scratch/m6-conflicts/issues/50-confirm-box-takes-y.md` "Where `y` is shown" and its NOTES' last paragraph (the
  question this closes); capture `docs/shots/m6/fix-box-takes-y-120x40.txt` frame 2.
- `crates/graft-ui/MAP.md` :110-111 (`GraphView::problem` "replaces the key hints while it lasts"), :119 (`notice`),
  :130 (`stopped`), :135 (`HintContext::ConfirmBox`).
- `crates/graft/MAP.md` :275 (`undo_key` sets `Busy`), :285 (`reverted_to` → drift box), :670-677 (`graph_view`,
  `footer_context`, `hint_context`, `graph_problem`).
- `docs/research/ux-principles.md` §3.2 ("a mode that isn't shown"), H6.

## The cause (rule 7, lines at `1b1995cc`; the scout re-reads them on main)
1. **Draw order.** The bottom row takes a `Busy`/`Done` notice first (`graft/src/view/frame/bars.rs:263-289`) and a
   `Sticky` as the problem line (`view/status.rs:82-99`). A box's keys (`HintContext::ConfirmBox`,
   `graft-ui/src/graph_screen/hints/lists.rs:57-69`) draw only when neither is set. So an open box loses its row to
   whatever was said before it opened. Observed twice: the lease box under the rejection line; the drift box under
   `undo · esc cancels`.
2. **A stale `Busy`.** `u` sets `Busy("undo · esc cancels")` (`update/undo.rs:257`); the drift answer opens the box without
   ending it (`:344-349`). While the box asks, the state says an undo is running. That is a false state, not only a
   draw problem: after `esc` the line would come back.

## Decided here
- **An open modal owns the graph screen's bottom row**: a confirm box, a Ghost Preview, the name prompt. While one is
  open, no notice, sticky or stop wording is drawn there; the row is the modal's own hint context, as it is today
  when no status is set. Where scout Q4 finds a modal with no bar context of its own (keys on its border only), the
  row is left blank under it rather than showing the graph's dead keys.
- **The status is kept, not cleared.** Closing the modal with `esc` draws it again (the rejection line and its `p`/`P`
  come back). A modal's act replaces it as today.
- **A job answer that opens a box ends its own `Busy`** (sets `Ready`) in the same update. A `Busy` of another job still
  running is not touched.
- **Out:** n3's "one place for a box's keys" (border vs bar: the preview and name prompt draw keys on their border, the
  confirm box on the bar). That is a look decision across three modals, left in KNOWN_GAPS.

## Scout brief (≤1 page, code not intent)
1. The bottom row's rungs, in order, as drawn: where `problem`, `notice`, `stopped`, search, compare footer and hints are
   chosen (`graph_view`, `view::frame::bars`, `graph_screen::hints`). Which crate decides the order.
2. Every modal that stands over the graph screen and how the code knows it is open (`AppState::confirm`, the preview
   state, the name prompt, the stash drop box over the stash overlay, the 80-column tab banner's footer).
3. Every answer handler that sets `confirm = Some(..)` (or opens a preview) while a `Busy` it set is standing:
   `undo.rs:344-349`, `reverted_to`'s drift arm, others. For each: the `Busy` text and whether another job can own it.
4. What the bar draws under the preview and under the name prompt when **no** status is set: their `HintContext`, or
   none.
5. Is any status written **while** a modal is open by the modal's own keys, with the modal staying open (a refusal on
   `y`, a failed re-check on a preview's confirm)? List each. If any exists, the rule needs a rung for it: **stop** and
   report before building.
6. Q4, read only: after `u y` over the drift box, what the journal holds and what `u` again would reverse.

## Scope
**In:** the one predicate and its readers; ending the answering job's `Busy` where a box opens; the tests and captures
below; the Q4 pin.
**Out:** moving keys between border and bar (n3's rest); the stash overlay's own footer (it is its own screen), unless
scout Q2 finds a box over it losing its keys the same way, which then joins this spec's table.

## Signatures
```rust
// graft, view/status.rs (beside `graph_problem`)
/// True while a modal stands over the graph screen: a confirm box, a Ghost Preview, the name prompt.
/// The one question every rung of the bottom row asks before drawing a status there (`graph_view` for `problem`
/// and `stopped`, `view::frame::bars` for the notice). It hides, it does not clear.
#[must_use] pub(crate) fn modal_owns_bottom_row(state: &AppState) -> bool;
```
No new update-side function: each handler from scout Q3 sets `Status::Ready` where it sets `confirm`, and the
table test below holds them to it.

## The failing tests first
- `view` tests: `the_lease_box_owns_the_bottom_row_over_a_rejection` — rejection sticky standing, `P` opens the lease
  box: the drawn bottom row is `←/→ move  ⏎ choose  esc cancel`. Red today (the sticky line).
- `update::undo` tests: `the_drift_box_ends_the_undo_busy_line` — `u`, the drift answer: `confirm.is_some()` and the
  status is `Ready`. Red today (`Busy`).

## Touches (each entry, and how the builder checks it)
- **Every modal** (scout Q2): `a_modal_owns_the_bottom_row` — a table, one row per modal, each with a sticky, a
  `Done` and a `Busy` standing: the row is that modal's context (or blank per Decided). **80×24 and 120×40.**
- **`esc` brings the status back:** `esc_from_the_box_brings_the_rejection_back` (the line and its `p`/`P` keys).
- **Every box-opening answer** (scout Q3): `a_box_opened_by_an_answer_ends_its_own_busy`, one row each; plus
  `a_busy_of_another_job_survives_a_box` (a fetch running, `P` opens the push box, the fetch's `Busy` is unchanged in
  state and drawn again after `esc`).
- **The stop wording** (`GraphView::stopped`) under the banner's abort box: not drawn while the box is open. Check: a row
  in the modal table.
- **`hint_bar.rs` / `every_key_the_bar_draws_is_answered`:** green; the box contexts are unchanged, only reached more often.
- **Captures:** `fix-box-takes-y-120x40.txt` / `-80x24` frame 2 now shows the lease box's keys; its header's
  "differs from the spec" note goes. Regenerated by `remote_by_key::capture_fix_box_takes_y`. Also
  `grep -rln "undo · esc cancels" docs/shots` and any capture with a box over a sticky; each regenerated or unchanged.
- **Contact sheet m6:** the builder lists the moved captures; the conductor rebuilds.
- **KNOWN_GAPS M6/50:** closed by this; NOTES says so with the test.
- **Journal/undo:** none changed. Q4 is read, not changed. **Keymap:** none.
- **MAP.md:** graft `view::status::modal_owns_bottom_row`; the notice/problem rung order where it is documented.

## Acceptance (name the verifying test at tick time)
- [ ] `the_lease_box_owns_the_bottom_row_over_a_rejection`, red then green.
- [ ] `the_drift_box_ends_the_undo_busy_line`, red then green.
- [ ] `a_modal_owns_the_bottom_row` (every modal × sticky/Done/Busy × 80×24, 120×40).
- [ ] `esc_from_the_box_brings_the_rejection_back`.
- [ ] `a_box_opened_by_an_answer_ends_its_own_busy`; `a_busy_of_another_job_survives_a_box`.
- [ ] **Q4 pin** (a question, not a fix): `undo_over_drift_then_undo_again` — real repo: edit a file after a checkout,
      `u y` over the drift box, then `u`. Assert what happens and record it in NOTES: does the edit come back, and what
      does the status line say. If the edit does **not** come back, do not fix it here: report it as a finding with
      its line.
- [ ] Captures (rule 6; colour **unverified**, rule 5): `docs/shots/m6/fix-box-owns-the-bar-120x40.txt` and `-80x24`.
  - Frame 1: after a rejected push, `P`: the lease box, and the bottom row reads `←/→ move  ⏎ choose  esc cancel` (no
    `y`: ADR-0044 §3's lease exception). No `push rejected` text anywhere on that row.
  - Frame 2: `esc`: the rejection line is back on the bottom row, `p pull first · P force with lease`.
  - Frame 3: the drift box after `u`: the bottom row reads `y undo  ←/→ move  ⏎ choose  esc cancel`; no
    `undo · esc cancels` anywhere.
- [ ] `fix-box-takes-y-*` regenerated; frame 2 matches frame 1 above.
- [ ] MAP.md updated (doc, no test).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
Medium: one predicate read at two or three rungs, a handful of handlers, a modal table, three captures. One context.
If scout Q5 finds a status written under an open modal, stop: the rule needs a decision first.
