# a-box-owns-the-bottom-row: while a box is open, the bottom row shows its keys, never a status from before it

Status: done (2026-09-25; resumed after the conductor's decision, Decided 6; see NOTES at the end)
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
6. **Branch-from-stash `⏎` refused by its preview (conductor, 2026-09-25: option 1).** The refusal (a write running,
   git mid-operation; `preview.rs:303/314` at the stop, reached via `name_prompt.rs:321-322`) goes to the name
   prompt's own `refusal` field, the one Decided 3 creates. The typed name is kept, the prompt stays open, and
   `AppState::status` is not written. Why: it is the prompt's own `⏎` and the prompt stays open, so it is Decided 3's
   principle; option 2 would drop the typed name, which review m5-10-30-31 kept on purpose. Also decided: the
   graft-ui `search` rung is blanked too; `reveal` blanks only the footer subject, not the hovered row's ref reveal.

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
- [x] `the_lease_box_owns_the_bottom_row_over_a_rejection` (`view::tests::bottom_row`), 120x40 and 80x24. Red observed
      with the predicate forced off (`left: "push rejected: origin/main has moved · p pull first · P force with
      lease"`), green with it.
- [x] `the_drift_box_ends_the_undo_busy_line` (`update::undo::box_answers`), red then green — red observed
      (`left: Busy("undo · esc cancels")`), green after `undone`'s `Drift` arm sets `Ready`.
- [x] `an_empty_name_is_refused_inside_the_prompt` (`update::name_prompt::tests`), red observed (`left: None, right:
      Some("a branch needs a name")`), green after Decided 3. The drawn row: `the_name_prompts_refusal_is_its_bottom_row`
      (`view::tests::bottom_row`, both sizes: `a branch needs a name`; red observed with the predicate forced off).
- [x] Decided 6: `a_fetch_in_flight_refuses_inside_the_stash_branch_prompt` (`update::name_prompt::tests`): a real `f`
      launch, then `S`-overlay `b`, a name, `⏎`: prompt open, name kept, `refusal` = `wait for … fetch … to finish`,
      status unchanged, nothing planned. Red observed (`refused inside the prompt`), green after.
- [x] `a_modal_owns_the_bottom_row` (`view::tests::bottom_row`): lease box, credential prompt, name prompt, stash drop
      box over the overlay, the banner's abort box with a stop live, the Ghost Preview (row blank), and at 80x24 the
      abort box in the tab layout — each × sticky / `Done` / `Busy` × 120x40 and 80x24. The row equals the box
      context's hints and never holds the status words.
- [x] `a_key_that_keeps_a_modal_open_leaves_the_status_alone` (`view::tests::bottom_row`): every modal above × the
      three statuses × `y ← → ⇥ ⇤ ⏎ ⌫ ↑ ↓ j k n x`; wherever the same modal is still open after the key, the status is
      unchanged (or, for a `Done`, acknowledged to `Ready` by `keys::acknowledge`, the standing any-key rule).
- [x] `esc_from_the_box_brings_the_rejection_back` (`view::tests::bottom_row`), both sizes: the line and its `p`/`P`.
- [x] `a_box_opened_by_an_answer_ends_its_own_busy` (`update::undo::box_answers`: `u`, `U` over `undone`, the time
      machine over `reverted_to`); `a_credential_prompt_keeps_its_jobs_busy` (same file: `Busy` unchanged when the
      prompt opens and after a key typed into it).
- [x] `a_busy_of_another_job_survives_a_box` (`view::tests::bottom_row`), both sizes, **built with a fetch and the name
      prompt**, not `P`: `P` refuses while a write runs (`crates/graft/src/update/remote/push_box.rs:22-24`,
      `operation_in_flight`), but `b` opens the name prompt over a running `f`. The fetch's `Busy` is unchanged in
      state, the row is the prompt's keys, and after `esc` the row is the fetch line again, the fetch still running.
- [x] **Q4 pin** `undo_over_drift_then_undo_again` (`tests/it/undo_over_drift.rs`, real repo): checkout `fix` through
      the menu, edit `fix.txt`, `u y` over the drift box, then `u`. Observed and pinned: see NOTES.
- [x] Captures `docs/shots/m6/fix-box-owns-the-bar-{120x40,80x24}.txt` (`box_owns_the_bar::capture_fix_box_owns_the_bar`,
      bottom rows asserted): frame 1 lease box `←/→ move  ⏎ choose  esc cancel`; frame 2 after `esc` the rejection with
      `p pull first · P force with lease`; frame 3 drift box `y undo  ←/→ move  ⏎ choose  esc cancel`; frame 4 the
      name prompt after an empty `⏎`, `a branch needs a name`. Judged by eye: not by me (rule 6).
- [x] `fix-box-takes-y-{120x40,80x24}` and `fix-box-keeps-on-y-120x40` regenerated
      (`remote_by_key::capture_fix_box_takes_y`, now asserting the lease frame's bottom row is the box's keys; header
      wording changed with it). `m3-09-undo-drift-box-*` regenerated in the first session (below).
- [x] MAP.md (`crates/graft/MAP.md`): `modal_owns_bottom_row`, `modal_problem`, `bottom_row_problem`,
      `NamePrompt::refusal`, `preview_refusal`, `graph_view`'s blanking, `Status::Sticky`'s note, `live_refusal`,
      `name_refusal`/`name_prompt_key`; `undone`/`reverted_to` from the first session. Docs, no test.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
Medium: two predicates read at two or three places, one refusal field, ~11 sites classified, three tables, four
capture frames. One context with 68. If the context runs short, 68 is the half to hand off.

## HANDOFF (builder, 2026-09-25, first session) — kept for history; resolved by Decided 6
- Scout used: `.scratch/m6-conflicts/scouts/a-box-owns-the-bottom-row.md`, with the amended spec, on a base with 67
  and 71 merged.
- **Why stopped (Decided 3, rule 7).** A second site writes a status under an open modal, which Decided 3 says to stop
  on rather than add a refusal on the spot. `⏎` in the name prompt for **branch from stash** (`S`, `b`, a name, `⏎`):
  `name_prompt.rs` `create` closes the prompt and calls `branch_from_stash`; that calls `preview::open_preview`, which
  refuses by **writing `state.status`** (`crates/graft/src/update/preview.rs:303`, `wait for <write> to finish`, while a
  write runs; `:314`, `live_refusal`, while git has an operation in progress), and then
  `crates/graft/src/update/name_prompt.rs:321-322` **reopens the prompt** (`state.stash.shown = true;
  state.name_prompt = Some(reopen);`). Reachable: `open_name_prompt` does not refuse while a write runs, so a fetch in
  flight is enough. Under Decided 1 that refusal would be hidden while the reopened prompt stands, and drawn only after
  `esc` — a regression the ticket's guard test exists to catch.
- **Options for the conductor (one decision):**
  1. Route it into the name prompt's own `refusal` (the field Decided 3 already creates): it is the prompt's own `⏎`,
     and the prompt stays open, so it is Decided 3's principle — but the spec forbade deciding that here.
  2. Leave the prompt closed on that refusal (drop the reopen at `name_prompt.rs:321-322`), making it an ordinary
     status as 67 did for previews — at the cost of the typed name (review m5-10-30-31 m6 kept it on purpose).
  3. Another shape.
- **What landed (Decided 4, independent of the stop):** `undo::undone`'s and `undo::reverted_to`'s `Drift` arms set
  `Status::Ready` as they open the box. Answer sites classified (lines at this branch's base, `6e1d42bc`):
  - job answers, own `Busy` ended here: `undo.rs:348` (`undone` Drift; `Busy("undo · esc cancels")` from `launch`,
    `undo.rs:257`), `undo.rs:440` (`reverted_to` Drift; `Busy(description)` from `revert_to_forced`, `undo.rs:390`,
    or the preview's `PreviewSubject::RevertTo`, `preview.rs:710`).
  - key presses, no own `Busy`: `undo.rs:39` (`u` after a push), `undo.rs:68` → `remote/after_push.rs:247`
    (`repush_chosen`, a choice in the undo-after-push box), `stash.rs:482` (`d` on a stash), `operate.rs:529`
    (`request_discard_at_diff_cursor`), `operate.rs:671` (`request_discard`), `banner.rs:506,539` (`a`/`s` boxes),
    `resolver/restart.rs:74` (`d` in the resolver), `remote/push_box.rs:30` (`P`).
  - the credential prompt (`credential.rs:291`) is a job's question, not an answer: its `Busy` stays (pinned).
- **Exact next step:** the conductor decides the option above; then build Decided 1 (`view::status::
  modal_owns_bottom_row`/`modal_problem`, read in `view::rows::graph_view` and `opening_view`, `view::frame` where
  `screen.notice` is set, and `view::frame::bars::draw_status_or_hints` — `draw_box_bars` already routes every
  `box_context` modal's bar through that function, so the confirm/name/credential boxes need only it; the preview is
  not in `box_context` and needs `graph_view`'s blanking) and Decided 3's `NamePrompt::refusal`.
- **Gotchas found:** the graft-ui footer rung `search` (`graft-ui/src/graph_screen/footer.rs`, before `notice`) is not
  in Decided 1's list; blank it too, or say why not. `reveal` also drives the hovered row's ref reveal, not only the
  footer subject.
- **Touches (what landed):** `undo.rs` two arms; `m3-09-undo-drift-box-{80x24,160x48}` and the m3 contact sheet
  (rebuilt, 129 620 B); m3 captions rows 17-18; TESTING.md:1336; MAP.md `undone`/`reverted_to`. Every other capture:
  `git status docs/shots` after the full `GRAFT_UPDATE_SHOTS`-free run lists only those. Journal/undo entries, keymap,
  graft-ui: none.
- No compaction.

## NOTES (builder, 2026-09-25, second session)
- **Compacted mid-ticket.** This session's context was compacted once while resuming after the conductor's decision.
  Nothing remains open from the ticket; everything below was done after the compaction and checked by the gates.
- Scout used: `.scratch/m6-conflicts/scouts/a-box-owns-the-bottom-row.md`.
- **Decided 1, where it lives.** `view::status::modal_owns_bottom_row` (credential ∨ name prompt ∨ confirm ∨ preview),
  `modal_problem` (the name prompt's `refusal`, no keys), `bottom_row_problem` (one or the other). Readers:
  `view::rows::graph_view`/`opening_view` (`problem`; `search`, `compare_footer`, `stopped` through `modal_blanked`),
  `view::frame::bars::footer_notice` (`None` while a modal is open, which covers `frame.rs`'s `screen.notice` and every
  `draw_status_or_hints` bar), and `draw_status_or_hints` (`bottom_row_problem`). graft-ui unchanged.
- **`search`:** blanked in graft (`GraphView::search = None`), no graft-ui change needed.
- **`reveal`:** not changed, and it needs no change. Under a box `draw_box_bars` writes the whole bar over the footer, so
  the footer subject never shows; under the preview `rows_for_frame` already returns `reveal = None`. The hovered row's
  ref reveal is untouched outside the preview, as before. The two could not be separated in graft by the `GraphView`
  field alone (one `reveal: Option<usize>` drives both), and nothing required it.
- **Resolver has no `a`** (68's premise, rule 7): the line in 68's Touches is reworded, `c` only; conductor's call.
- **Q4 answer (observed, pinned):** after `u y` over the drift box the checkout is undone (`HEAD` on `main`), the edit
  left the working tree with it (`fix.txt` is not on `main`), and the line reads `undo: checkout fix ✓` — nothing on
  screen says where the edit went. The edit's blob is in the undo entry's own backup ref
  (`worktree/fix.txt`, asserted). A second `u` does **not** bring it back: `nothing to undo`, `fix.txt` still absent.
  Not changed here (read only, per Touches). A question for the conductor, not a defect filed: whether `U` or the
  journal restores it was not tried.
- **The guard's one allowance.** `a_key_that_keeps_a_modal_open_leaves_the_status_alone` accepts a `Done` becoming
  `Ready`: `update::keys::acknowledge` (m6-conflicts/42) ends an acknowledgement on any key through `key_event`,
  whatever the key does. The credential prompt takes its keys before that and leaves it. Neither writes a refusal.
- **Not blanked, out of scope:** the stash overlay's own notice line (`view/stash.rs` `notice`, drawn one row above the
  overlay's footer by `graft-ui/src/stash_screen.rs:377-384`) still shows a status while the drop box is open. The bottom
  row itself is the drop box's keys (tested). Scope says "the stash overlay's own footer rungs beyond the drop box" are out.
- **KNOWN_GAPS M6/50** (the lease box's keys covered by the rejection's sticky line) is closed by this ticket:
  `the_lease_box_owns_the_bottom_row_over_a_rejection` and the regenerated `fix-box-takes-y-*` frame 2. KNOWN_GAPS itself
  not edited (builder rule); the conductor moves the row.
- **Captures moved** (`git status docs/shots` after the full run): `m6/fix-box-takes-y-{120x40,80x24}`,
  `m6/fix-box-keeps-on-y-120x40` (bottom row: the box's keys), **`m4-credential-prompt-in-app`** and
  **`m5-ui-credential-over-graph-{80x24,160x48}`** (bottom row under the credential prompt: `⏎ send  esc cancel`, was
  `push main → origin/main · esc cancels` — Decided 1/4, the credential's `Busy` hidden while the prompt is open; their
  headers said the line behind the prompt shows the push, reworded). New: `m6/fix-box-owns-the-bar-{120x40,80x24}`.
  Contact sheets rebuilt: m6 (now 4 parts: 179 678 / 177 038 / 163 404 / 31 722 B), m4 (114 794 B), m5
  (172 347 / 112 029 B); m3 unchanged this session. m6 captions: two rows added after 68's, rows for
  `fix-box-keeps-on-y` and `fix-box-takes-y-120x40` reworded. Every part ≤ 200 000 B.
- **Captures' dates:** frames 1, 2, 4 of `fix-box-owns-the-bar-*` are drawn with the wall clock (`captures::frame`), as
  `capture_fix_box_takes_y` is; the ages (`6y`, `7y`) could roll over some day and fail the capture. Same exposure as
  the existing takes-y capture, not new in kind.
- **Colour:** unverified (rule 5).
- **Touches:** `update/name_prompt.rs` (refusal, `refuse`, `create`); `update/preview.rs` (`preview_refusal` split out,
  `open_preview` unchanged in behaviour — its tests green); `update/undo.rs` (`live_refusal_words`); `view/status.rs`,
  `view/rows.rs`, `view/frame/bars.rs` (readers); `tests/it/m6_stash_put_back.rs` (four helpers made `pub(crate)`);
  `tests/it/remote_by_key.rs` (assertion and headers); `view/tests.rs` and `tests/it/two_boxes_over_content.rs`
  (credential capture headers); MAP.md. Hint bar: `hint_bar.rs` and `every_key_the_bar_draws_is_answered` green (box
  contexts unchanged). Ticket 67's tests green. Journal/undo: none changed. Keymap: none. graft-ui: none.
