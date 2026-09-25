# a-box-owns-the-bottom-row: while a box is open, the bottom row shows its keys, never a status from before it

Status: draft (reconciled with the scout, 2026-09-25)
From: `docs/reviews/m6-usability-final.md` N5 (:162-176). Folds in KNOWN_GAPS **M6/50** (the lease box's keys covered by
the rejection's sticky line; same cause), the stale-status part of **n3** (:229-235), and **Q4** (:269-270) as a pinning
test only.
Build with: **Opus**: a rule applied across every modal and a dozen answer sites, plus the name prompt's refusal moved
off the status slot (scout Q5). Structural inside graft, but one crate and checkable by the tables below; not Fable.
Scout: `.scratch/m6-conflicts/scouts/a-box-owns-the-bottom-row.md` (at `dbf4fcf2`).
Needs: **ticket 67 landed.** 67 closes a refused preview's box and puts the refusal on the status line
(`update/preview.rs:289,300`); this spec's Q5 guard reads those paths on 67's hash. Group B builds after 67.
Crates: **graft only.** The rung order stays in graft-ui (`graph_screen/footer.rs:61-109`) and is not changed: graft
hands graft-ui a `GraphView` whose status fields are already empty while a modal is open (Decided 1).
Group: **B** (the bottom row tells the truth), with `68-resume-words-say-what-happened`.

## Read first
- `docs/reviews/m6-usability-final.md` N5 (:162-176), n3 (:229-235), Q4 (:269-270).
- This ticket's scout (whole, ≤1 page).
- KNOWN_GAPS.md row M6/50 (:797).
- `.scratch/m6-conflicts/issues/50-confirm-box-takes-y.md` "Where `y` is shown"; capture
  `docs/shots/m6/fix-box-takes-y-120x40.txt` frame 2.
- `crates/graft-ui/MAP.md` :110-111 (`GraphView::problem`), :119 (`notice`), :130 (`stopped`), :135
  (`HintContext::ConfirmBox`).
- `crates/graft/MAP.md` :189 (`Status`), :275 (`undo_key` sets `Busy`), :285 (`reverted_to` → drift box), :670-677
  (`graph_view`, `footer_context`, `hint_context`, `graph_problem`).
- Ticket 67's spec and handoff NOTES (what its refused preview does, on its merged hash).
- `docs/research/ux-principles.md` §3.2 ("a mode that isn't shown"), H6.

## The cause (rule 7, lines at `dbf4fcf2`, from the scout)
1. **Draw order.** graft-ui draws `problem` → search → `notice` → `compare_footer` → reveal subject → hints, with
   `stopped` leading the hints (`graft-ui/src/graph_screen/footer.rs:61-109`). graft fills `problem`
   (`view/rows.rs:45` → `view/status.rs:82-99`), `stopped` (`rows.rs:88,156`) and `notice` (`view/frame.rs:175` →
   `view/frame/bars.rs:263-289`) whether or not a modal is open. The modal's context (`box_context`, `rows.rs:324-342`)
   only reaches the hints rung. The other bars (`bars.rs:33-50`) keep the same order in graft.
2. **A stale `Busy`.** `u` sets `Busy("undo · esc cancels")` (`undo.rs:257`); the drift answer opens the box without
   ending it (`undo.rs:348`). Same for the revert-to drift box (`Busy(description)` `:390`, box `:440`).
3. **A status written under an open modal (scout Q5).** An empty name sets `Sticky("a branch needs a name")` and returns
   with the name prompt open (`update/name_prompt.rs:234-241`). Under this spec's rule it would be hidden.

## Decided here
1. **An open modal owns the bottom row. graft blanks, graft-ui is unchanged.** Modals: credential prompt, name prompt,
   confirm box (all the `box_context` rungs), Ghost Preview, and the stash drop box over the stash overlay. While one is
   open, `graph_view` (and each bar at `bars.rs:33-50`) hands over `problem = modal_problem(state)`, `notice = None`,
   `stopped = None`, and no compare footer or reveal subject; the hints are the modal's context. The preview's context
   draws no hints (`graft-ui/.../hints/lists.rs:89`): the row is blank under it, not the graph's dead keys.
   Why graft and not graft-ui: the rungs are right for the graph; what is wrong is graft passing a status that does not
   belong to what is on screen. Moving the order into graft-ui would spread one rule over two crates.
   If the builder finds a rung that graft-ui decides from something graft cannot blank: **stop**, `blocked: needs
   graft-ui`, naming it.
2. **The status is kept, not cleared.** `esc` on the modal draws it again (the rejection line and its `p`/`P`). A
   modal's act replaces it as today.
3. **A modal's own refusal belongs to the modal (Q5).** A key that leaves its modal open never writes `AppState::status`.
   The name prompt's empty-name refusal moves to its own state (`refusal`), drawn as the bottom row's problem line
   while the prompt is open (same place and look as today), cleared by the next key typed, gone with the prompt on
   `esc`. `name_prompt.rs:234-241` stops writing a `Sticky`. Preview refusals: after 67 they close the preview first,
   so they are ordinary statuses; the guard test below holds that. Any other site found writing a status under an open
   modal: **stop** and report; do not add a refusal field on the spot.
4. **A job answer that opens a box ends its own `Busy`** (sets `Ready`) in the same update. A `Busy` of another job
   still running is not touched. The **credential prompt** is not an answer: its job is still running and waiting; its
   `Busy` stays in state, hidden while the prompt is open, drawn again after the answer.
5. **Out:** n3's "one place for a box's keys" (border vs bar across three modals): a look decision, left in KNOWN_GAPS.

## Scope
**In:** `modal_owns_bottom_row`, `modal_problem` and their readers; the name prompt's `refusal`; `Ready` at every
answer site that opens a box; the tables, pins and captures below.
**Out:** graft-ui; moving keys between border and bar; the stash overlay's own footer rungs beyond the drop box.

## Signatures
```rust
// graft, view/status.rs (beside `graph_problem`)
/// True while a modal stands over the screen: credential prompt, name prompt, confirm box, Ghost Preview, the stash
/// drop box. Every rung of the bottom row asks it before drawing a status. It hides; it never clears.
#[must_use] pub(crate) fn modal_owns_bottom_row(state: &AppState) -> bool;

/// The bottom row's problem line while a modal is open: only the modal's own refusal (today, the name prompt's).
#[must_use] pub(crate) fn modal_problem(state: &AppState) -> Option<graft_ui::graph_screen::ProblemLine>;

// graft, the type `AppState::name_prompt` holds: the empty-name refusal, set on `⏎` with an empty name, cleared by
// the next edit key. NOTES names the real type.
pub(crate) refusal: Option<String>,
```

## Answer sites to classify (scout Q3, lines at `dbf4fcf2`)
`undo.rs:348` (drift, own `Busy` :257), `undo.rs:440` (revert drift, own `Busy` :390), `undo.rs:39`, `stash.rs:482`,
`operate.rs:515,657`, `banner.rs:506,539`, `resolver/restart.rs:74`, `remote/after_push.rs:247`, `remote/push_box.rs:30`.
For each, NOTES says: key press (no own `Busy`) or job answer (its `Busy` text), with the line. Every job answer is a row
in `a_box_opened_by_an_answer_ends_its_own_busy`.

## The failing tests first
- `view` tests: `the_lease_box_owns_the_bottom_row_over_a_rejection`: rejection sticky standing, `P` opens the lease
  box: the drawn bottom row is `←/→ move  ⏎ choose  esc cancel`. Red today (the sticky line).
- `update::undo` tests: `the_drift_box_ends_the_undo_busy_line`: `u`, the drift answer: `confirm.is_some()` and the status
  is `Ready`. Red today (`Busy`).
- `update::name_prompt` tests: `an_empty_name_is_refused_inside_the_prompt`: `⏎` on an empty name: prompt open, status
  unchanged, the drawn bottom row reads `a branch needs a name`. Red today (status is `Sticky`).

## Touches (each entry, and how the builder checks it)
- **Every modal** (Decided 1): `a_modal_owns_the_bottom_row`: one row per modal, each with a sticky, a `Done` and a
  `Busy` standing; the row is that modal's context, or blank under the preview. **80×24 and 120×40.** Rows include:
  the credential prompt; the stash drop box over the stash overlay (the overlay's bar at `bars.rs:33-50` reads the same
  predicate); the banner's abort box with a stop live (no `stopped` wording); at **80×24 the tab form of the banner**
  (the scout did not read its footer) with the abort box open.
- **Q5 class, guarded:** `a_key_that_keeps_a_modal_open_leaves_the_status_alone`: per modal, every key its hint context
  names plus `⏎`/`y` on an invalid or empty input: where the modal is still open after the key, `state.status` equals
  its value before. Covers the preview after 67.
- **`esc` brings the status back:** `esc_from_the_box_brings_the_rejection_back` (the line and its `p`/`P` keys).
- **Busy ownership:** `a_box_opened_by_an_answer_ends_its_own_busy` (one row per answer site above);
  `a_busy_of_another_job_survives_a_box` (a fetch running, `P` opens the push box: the fetch's `Busy` unchanged in
  state, drawn again after `esc`); `a_credential_prompt_keeps_its_jobs_busy`.
- **Ticket 67's refused preview:** its tests green on this branch; its capture regenerated or byte-identical.
- **`hint_bar.rs` / `every_key_the_bar_draws_is_answered`:** green; the box contexts are unchanged, reached more often.
- **Captures:** `docs/shots/m6/fix-box-takes-y-{120x40,80x24}.txt` frame 2 now shows the lease box's keys (regenerated by
  `remote_by_key::capture_fix_box_takes_y`; the scout found no "differs from the spec" note, so none to drop).
  `docs/shots/m3-09-undo-drift-box-{160x48,80x24}.txt` hold `undo · esc cancels`: regenerated by their test (builder
  names it). Every other capture: `git diff --exit-code docs/shots` after the run, each moved file listed in NOTES.
- **Contact sheets m3 and m6:** the builder lists the moved captures; the conductor rebuilds both.
- **KNOWN_GAPS M6/50:** closed by this; NOTES says so with the test.
- **Journal/undo:** none changed. Q4 read, not changed. **Keymap:** none. **graft-ui:** none (Decided 1).
- **MAP.md:** graft `view::status::modal_owns_bottom_row`, `modal_problem`; the name prompt's `refusal`; `graph_view`'s
  blanking; `Status::Sticky`'s list of uses loses the empty-name refusal.

## Acceptance (name the verifying test at tick time)
- [ ] `the_lease_box_owns_the_bottom_row_over_a_rejection`, red then green.
- [ ] `the_drift_box_ends_the_undo_busy_line`, red then green.
- [ ] `an_empty_name_is_refused_inside_the_prompt`, red then green.
- [ ] `a_modal_owns_the_bottom_row` (every modal × sticky/`Done`/`Busy` × 80×24, 120×40; stash drop box, credential,
      80-column tab banner rows included).
- [ ] `a_key_that_keeps_a_modal_open_leaves_the_status_alone`.
- [ ] `esc_from_the_box_brings_the_rejection_back`.
- [ ] `a_box_opened_by_an_answer_ends_its_own_busy`; `a_busy_of_another_job_survives_a_box`;
      `a_credential_prompt_keeps_its_jobs_busy`.
- [ ] **Q4 pin** (a question, not a fix; the scout did not read it): `undo_over_drift_then_undo_again`: real repo, edit
      a file after a checkout, `u y` over the drift box, then `u`. Assert what happens; record in NOTES whether the edit
      comes back and what the line says. If it does **not** come back, report it as a finding with its line; do not fix.
- [ ] Captures (rule 6; colour **unverified**, rule 5): `docs/shots/m6/fix-box-owns-the-bar-120x40.txt` and `-80x24`.
  - Frame 1: after a rejected push, `P`: the lease box; the bottom row reads `←/→ move  ⏎ choose  esc cancel` (no `y`:
    ADR-0044 §3's lease exception). No `push rejected` text anywhere on that row.
  - Frame 2: `esc`: the rejection line is back on the bottom row, `p pull first · P force with lease`.
  - Frame 3: the drift box after `u`: the bottom row reads `y undo  ←/→ move  ⏎ choose  esc cancel`; no
    `undo · esc cancels` anywhere.
  - Frame 4: the name prompt, `⏎` on an empty name: the bottom row reads `a branch needs a name`; the prompt still open.
- [ ] `fix-box-takes-y-*` regenerated; frame 2 matches frame 1 above. `m3-09-undo-drift-box-*` regenerated; no
      `undo · esc cancels` under the box.
- [ ] MAP.md updated (doc, no test).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
Medium: two predicates read at two or three places, one refusal field, ~11 sites classified, three tables, four
capture frames. One context with 68. If the context runs short, 68 is the half to hand off.
