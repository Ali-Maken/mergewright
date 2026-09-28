# rewrite-editor — `R` opens the rewrite editor on the cursor's row; the result and the message show before `y`

Status: ready (respecced by planner 2026-09-27 after scout `scouts/08.md`; scope unchanged)
Build with: Opus. A new modal overlay on the `Place`/`finish` precedents, every seam fixed below; walks and captures
judge it. Not Fable: the one shared type (`RewriteEditorView`) is consumed only inside this ticket.
Scout: done (`scouts/08.md`, at b90a49e4). No second scout; re-check its named lines against main before editing.
Needs: `graft_ops::rewrite::{plan, squash_groups, default_message, validate, TodoAction, TodoLine, RewriteParams,
RewriteRequest::Open, RewriteError}` (graft-ops MAP :263-284); `update::preview::{preview_refusal, PreviewRequest,
PreviewSubject, Planned, PreviewFailure}` and `Command::PlanPreview` (graft MAP :442-460, :875);
`operate::launch`; the `$EDITOR` handover `Command::EditCommitMessage` (MAP :165, :213).
Blocked by (all **merged**; the conductor checks `git log` before dispatch):
- **09b** — `jobs/rewrite.rs` neighbours, `update/preview.rs` (its Q4 may change the preview rows this reads), the
  preview captures.
- **07b** — `view/rows.rs`, `update/events.rs`, `jobs/runner.rs`, `update/commit_editor.rs`, `update/operate.rs`.
- **30j** — `jobs/runner.rs` (`not_a_job` made exhaustive: the new `Command` must be listed there), `update/mod.rs`,
  graft-ui `graph_screen/hints/lists.rs`.
- Not beside **21c** (`jobs/runner.rs`). Not beside **10 / 10c** if unmerged (`GraphOverlay`, `update/state.rs`,
  `view/frame.rs`, `view/rows.rs::overlay_context`, `HintContext`).
Crates: graft, graft-ui. graft-ops **read-only**: a missing read or refusal word → stop, `blocked: needs <signature>`.
Source: note "Pass 2a" R1 (:436-533); journeys 1, 3; fence :815-840; keymap §2.3 `R` (:203), §2.3b (:245), §2.7
(:378-393, already applied, no doc edit); field notes :69.

## Read first
- `scouts/08.md` (all).
- Note `docs/design-notes/m7-power-tools.md` :436-533 (R1 whole), :815-840, :912-930.
- `docs/design/02-keymap.md` §2.1 (modal panes, the bar rule), §2.3, §2.3b, §2.6b, §2.7, §2.10.
- `docs/shots/m7-proto-1-rewrite.txt` (rev 1, what he saw).
- `crates/graft/MAP.md` :163-171 (`commit_editor`, `external_editor`, `graph_overlay`, `GraphOverlay`), :207-208
  (`text_field_open`, `modal_owns_keys`), :428-429 (`finish_key`/`finish_mouse`: the modal-chain shape copied here),
  :442-460 (`update::preview`), :625 (`Place`: an overlay that draws the whole frame, bar on the last row), :875
  (`jobs::preview`), :914-920 (`jobs::rewrite`).
- `crates/graft-ops/MAP.md` :263-284. `crates/graft-ui/MAP.md`: `HintContext`, the hint lists, the finish summary's
  hit test (precedent for `rewrite_row_at`).

## Rulings (scout 08)
1. **`GraphOverlay` stays a fieldless `Copy` enum derived from state** (`state.rs:653`, `graph_overlay()`
   `:614-645`, the one enumeration). Add a fieldless `GraphOverlay::Rewrite`, derived from
   `state.rewrite.editor.is_some()`, placed **after `Resolver`, before `CommitEditor`**. The editor lives in
   `AppState::rewrite: RewriteState`. Test: `the_overlay_is_rewrite_while_the_editor_is_up`.
2. **The open read is its own command**, not `PlanRewrite`. `Command::PlanRewrite` is 09's struct variant
   (`command.rs:194`), and its answer `RewritePlanned` (`events.rs:324`) has no editor shape. A new variant would force
   an arm into 09b's `amend_into::planned`. So:
   - `Command::OpenRewrite { id, from }` → `Task::OpenRewrite` → `Product::RewriteOpened`.
   - `jobs/rewrite.rs:152` (`Open` → `Err("R is not a put-in")`) stays as it is.
3. **Modal.** §2.7 says the editor claims every key but `Ctrl+C`:
   - `modal_owns_keys()` is true while `rewrite.editor` is `Some`.
   - `text_field_open()` is true while its `r` field is open.
   - Consumers: `modal_owns_keys` at `mod.rs:980`, `relaunch.rs:75,129`, `undo.rs:221`, `clash.rs:174`,
     `place/trail.rs:166`, `banner/stop.rs:398,430`; `text_field_open` at `keys.rs:193,213`, `clash.rs:185`,
     `drag.rs:151`, `remote/in_flight.rs:21`. They now see the editor; see Touches.
   - `rewrite_key` joins `keys::key_event`'s **modal chain after the boxes**, as `finish_key` does. Not
     `pane_and_overlay_key` (that chain is for Journal/Stash, which leave `/` live).
4. **The AFTER plan and "the last edit wins"** (scout Q3 could not confirm a supersede path; none is assumed).
   - The editor owns its plan job. Every edit that changes the params does three things:
     - if the current `after` is `Pending(old)`, push `Command::CancelJob(old)` (as `close_preview` does for
       `planning()`);
     - mint a new id and push `Command::PlanPreview` with
       `PreviewSubject::Operation { op: Box::new(Operation::Rewrite(params)), opened_by: "R" }`, `resident` = the
       set `open_preview` snapshots;
     - set `after = Pending(new)`.
   - The answer is routed **by id, before `preview_produced`**:
     - `rewrite_editor::owns(state, id)` true → `after_planned`;
     - an answer for any other old editor id is dropped;
     - `state.preview` is never set by the editor.
   - Two things to verify, not assume (rule 7). Name each line in NOTES **before** writing the routing:
     - what `JobRunner::cancel` / `runner.rs:1171` do for an id whose job already finished (the race `close_preview`
       already has);
     - that `preview_produced` drops an id it doesn't hold with `state.preview == None`.
     - If either misbehaves → stop, `blocked: needs <file>`, with the line.
   - Tests: `an_edit_replans_and_the_last_wins`, `a_superseded_after_answer_is_dropped`.
5. **Todo validity before planning.** When `validate(&todo)` is `Err`, nothing is planned:
   - the AFTER side and `y` both give the problem's words;
   - the row gets `✗ nothing below to meld into`, or the notice reads `every commit dropped · the branch would be main
     again`.
6. **`y` runs the planned op.** `y` launches `Operation::Rewrite(p)` through `operate::launch`. `p` is `Planned::runs`
   if `Some`, else the params that plan was asked for (hard choice 3: the op run is the op pictured). `y` is inert
   while `after` is `Pending`, `Failed`, `Refused`, or the todo is invalid.
   - The editor moves to `rewrite.launched`.
   - `operate::operated` calls `rewrite_editor::write_answered` first:
     - a refusal restores the editor, words on its notice row (R1 "Refused at apply: stays in the editor");
     - any other answer drops it (Done, Conflicted, stop → 07's world, Failed).
   - A moved branch: the AFTER plan's `check` refuses first. Words: `operate::refusal_status`'s, one wording (H4). If
     they don't read `feature-a moved since this opened · esc, then R again`, say so in NOTES as a question. Don't
     add a second wording.
7. **Opening.**
   - `R` → `rewrite.opening = Some(id)` and `Status::Waiting("reading <branch>…")`. No overlay yet.
   - A second `R` starts nothing. `Esc` cancels the read (`CancelJob`) and clears the line.
   - While a write runs or git is part-way through → `preview_refusal(state)`'s words (one wording, H4).
   - Answers:
     - `Refused(r)` → the status line with `r`'s words, no overlay;
     - `Editor(o)` → the overlay, cursor on the `from` row (HEAD's from the uncommitted row);
     - `Err(words)` → `Status::Failed`, no overlay (rule 3).
   - `JobFailed` / `JobCancelled` for any id `owns` answers are handled in `job_failed`'s chain (`update/mod.rs`
     ~:980, next to 30j's leaves arm).
8. **Messages** are kept per commit (`Oid`), because moves change todo indices. `params()` rebuilds `messages`
   (keyed by todo index) on every call:
   - each squash group → his edit of the kept row, else `default_message(group, whole_of, &todo)`;
   - each `Reword` row → his edit.
   - `r` on a row:
     - a pick row, or the kept row of a group → opens the field, prefilled with the subject line;
     - on a pick row, `⏎` keeps the text and sets `Reword`;
     - on the kept row, `⏎` edits the squash message and the action stays;
     - a melded row → notice `this row melds into "<kept>" · r there changes the message`.
   - `Ctrl+E` hands the whole message to the existing handover under the editor's own id. The answer is routed by
     `owns` before `commit_editor`'s handler; `AppState::external_editor` stays the commit editor's.
9. **`⏎`** toggles the right side (below 120: the card) between AFTER and the row's changes: its file list with
   `+a −b`, read by `Command::ListFiles` under an editor id (routed by `owns`). No inline diff here.
10. **`R rewrite` on the graph bar** needs "a commit row of his branch". The app has no membership fact at key time,
    so, from what is in state:
    - drawn on a commit row when HEAD is on a branch, no operation is live, and `RadarContext::on_default_chain(row)`
      is false (MAP :870);
    - when HEAD *is* the default branch, or the context isn't `Ready`, drawn on every commit row;
    - elsewhere `R` refuses in `plan`'s words (`R rewrites your branch · <sha> isn't on it`).
11. **Parser: no change.** `parse_counts` already reads every fence and errs on a duplicate (`journeys.rs:51-70`), and
    `m7_walk.rs:19-26` reads this note. The walks call `noted_keys` as 09's do.

## Decided (note R1; not open)
- Rows newest first (todo reversed), then the `─ <base>` row: not editable; `p s f d e r` and a move onto it give
  `the base stays · it isn't yours to rewrite`. Base label: the upstream's (else the default branch's) short name when
  `onto` is the merge-base, else `onto`'s short sha.
- Keys (§2.7):
  - `p s f d e`: set the action and move one older; `p` on a marked row un-marks it.
  - `r`: the field (ruling 8).
  - `Alt+↑/↓`: move the row; it shows `↕ moved`.
  - `⏎` (ruling 9). `j/k`.
  - `Tab`: <120 only, swaps rows ↔ AFTER (bar `tab after`); ≥120 it is swallowed.
  - `y` (ruling 6). `Esc`: close, nothing written; in the field it drops the field's edit only.
  - Mouse: a click selects; a drag moves.
- Autosquash rows arrive pre-marked and moved by `plan` (graft-ops MAP :271). Any row not `Pick` at open carries
  `marked by its name`.
- Message block: under the rows (≥180 and <120), under AFTER (120–179). `┊` gutter, label
  `MESSAGE of the squashed commit · r on "<kept subject>" changes it`. At 24 rows: 4 lines + `…`.
- Glyphs and words: exactly R1's distinctions table (`↓ ✎ ⏸ ✗`, bold action word when not pick, `│`, `○ new` /
  `● new`, `built on yours · follows`, `marked by its name`, `3 in one`, `fixup melded in`).
- AFTER rows are the plan's ghosts grouped per branch of `params.line`, drawn top-down, base `unchanged`. Below them:
  `built on yours: …`, `on origin: <b> · P will need force after` (`published`), `no conflicts expected` or
  `! may stop here · <paths>` / `not checked past here` (`StopsAt`). The header reads `checking…` while `Pending`.
- Hint bar (`HintContext`, the Place precedent):
  `y rewrite  ⏎ changes  p pick  r reword  e edit  s squash  f fixup  d drop  alt+↑↓ move  esc cancel`
  (+ `tab after` <120). `y rewrite` shows while planning or ready, never on a refused, failed or invalid plan
  (MAP :460's rule). Field open: `⏎ keep  ctrl+e editor  esc drop the edit`.
- Lands: today's `Done` landing through `operate`. The walk asserts the cursor on the new HEAD only if `land_on_head`
  covers `Operation::Rewrite` (name the line); if not, NOTES, and the conductor files it.

## Signatures
```rust
// graft — update/rewrite_editor.rs (new; children allowed: keys, after, words)
#[derive(Default)]
pub(crate) struct RewriteState {
    pub(crate) opening: Option<JobId>,               // the open read in flight
    pub(crate) editor: Option<Box<RewriteEditor>>,   // Some ⇔ GraphOverlay::Rewrite
    pub(crate) launched: Option<Box<RewriteEditor>>, // after `y`, until the write answers (ruling 6)
}
pub(crate) struct RewriteEditor { /* private: opened, todo, moved, edits by Oid, cursor, field, after, right, … */ }
impl RewriteEditor {
    /// What `y` runs and the AFTER plan is asked for; `messages` rebuilt per ruling 8.
    pub(crate) fn params(&self) -> Result<RewriteParams, TodoProblem>;
}
// `next_id`, `KeyPress`, the mouse event: exactly the types `finish_key` / `finish_mouse` take (MAP :428-429).
pub(crate) fn open_rewrite(state: &mut AppState, commands: &mut Vec<Command>, from: Oid, next_id: /* finish_key's */);
pub(crate) fn opened(state: &mut AppState, commands: &mut Vec<Command>, id: JobId,
                     answer: Result<RewriteOpened, String>, next_id: /* … */);
pub(crate) fn rewrite_key(state: &mut AppState, commands: &mut Vec<Command>, key: KeyPress, next_id: /* … */) -> bool;
pub(crate) fn rewrite_mouse(state: &mut AppState, commands: &mut Vec<Command>, mouse: /* finish_mouse's */,
                            next_id: /* … */) -> bool;
/// Is `id` the open read, the AFTER plan, the `⏎` file list or the `$EDITOR` handover of the live editor?
pub(crate) fn owns(state: &AppState, id: JobId) -> bool;
pub(crate) fn after_planned(state: &mut AppState, id: JobId, answer: Result<Arc<Planned>, PreviewFailure>);
pub(crate) fn files_listed(state: &mut AppState, id: JobId, answer: /* ListFiles' product payload */);
pub(crate) fn message_returned(state: &mut AppState, commands: &mut Vec<Command>, id: JobId,
                               answer: /* the handover answer payload */, next_id: /* … */);
pub(crate) fn job_ended(state: &mut AppState, id: JobId, why: /* job_failed's words */);
pub(crate) fn write_answered(state: &mut AppState, answer: &Operated);  // first line of operate::operated

// graft — update/state.rs
// AppState::rewrite: RewriteState;  GraphOverlay::Rewrite (fieldless; after Resolver, before CommitEditor)

// graft — jobs/rewrite_open.rs (new)
pub(crate) enum RewriteOpened { Editor(Box<OpenedRewrite>), Refused(graft_ops::operation::Refusal) }
pub(crate) struct OpenedRewrite {
    pub params: RewriteParams,        // plan(Open { from }) as returned
    pub whole: HashMap<Oid, String>,  // every row's whole message (default_message's message_of);
                                      // read as the detail pane reads a commit message: name it in NOTES
    pub base: BaseRow,
}
pub(crate) struct BaseRow { pub oid: Oid, pub subject: String, pub label: String }
pub(crate) fn open_rewrite(repos: /* run_preview's repos type */, from: Oid, default_branch: Option<&RefName>,
                           stop: /* run_preview's cancel type */) -> Result<RewriteOpened, String>;
// RewriteError::Refused → Refused; Facts → Err(explain()); Stopped → as jobs/rewrite.rs maps it (name the line).
// Command::OpenRewrite { id: JobId, from: Oid } → Task::OpenRewrite { from } → Product::RewriteOpened(Result<RewriteOpened, String>)
// (declared beside PlanRewrite / RewritePlanned: command.rs:194, events.rs:324)

// graft — view/rewrite_editor.rs (new): builds the view; view/frame/overlays.rs: draw_rewrite

// graft-ui — src/rewrite_editor.rs (new; no graft-ops types)
pub struct RewriteEditorView {
    pub title: String,                  // `REWRITE feature-a · 5 commits since it left main · newest first · nothing written until y`
    pub rows: Vec<EditorRow>,           // newest first; base last
    pub cursor: usize,
    pub field: Option<FieldView>,
    pub message: Option<MessageBlock>,
    pub right: RightSide,
    pub narrow_shows_right: bool,       // <120: Tab's side
    pub notice: Option<String>,         // a refusal in words, on the row above the bar
    pub ascii: bool,
}
pub enum RowMark { Pick, Reword, Edit, Squash, Fixup, Drop, Base }
pub struct EditorRow { pub mark: RowMark, pub short: String, pub subject: String, pub note: Option<String>, pub moved: bool }
pub struct FieldView { pub row: usize, pub text: String, pub caret: usize }
pub struct MessageBlock { pub label: String, pub lines: Vec<String> }
pub enum RightSide { After(AfterView), Changes { short: String, files: ChangesView } }
pub enum AfterView { Checking, Failed(String), Refused(String), Ready { rows: Vec<AfterRow>, facts: Vec<String> } }
pub struct AfterRow { pub branch: Option<String>, pub mark: AfterMark, pub subject: String, pub note: Option<String> }
pub enum AfterMark { NewTip, New, Unchanged { short: String } }
pub enum ChangesView { Reading, Failed(String), Files(Vec<(String, u32, u32)>) }
pub fn draw_rewrite_editor(frame: &mut Frame, area: Rect, view: &RewriteEditorView);
pub fn rewrite_row_at(area: Rect, view: &RewriteEditorView, column: u16, row: u16) -> Option<usize>;
// HintContext gains RewriteEditor { narrow: bool, can_start: bool } and RewriteMessage (hints/context.rs:10)
```

## Files (ADR-0051: outside this list → stop, `blocked: needs <file>`)
- graft src:
  - New: `update/rewrite_editor.rs` (+ children, `rewrite_editor/tests.rs`), `jobs/rewrite_open.rs` (+ tests),
    `view/rewrite_editor.rs`.
  - `update/state.rs`: the field, the variant, `graph_overlay`, `text_field_open`, `modal_owns_keys`.
  - `update/keys.rs`: `R` beside the graph-row keys (`graph_has_focus`, no overlay); `rewrite_key` in the modal
    chain; `Esc` while opening. Also `keys/predicates.rs` if `R`'s predicate lives there.
  - Mouse: the file routing `finish_mouse` (name it).
  - The job: `update/events.rs`, `update/events/command.rs`, `update/mod.rs` (`Product` arms, routing by `owns`,
    `job_failed` arm, `mod` line), `jobs/mod.rs`, `jobs/kinds.rs`, `jobs/runner.rs` (dispatch + `not_a_job` list),
    `event_loop/label.rs`.
  - Every other forced arm, literal-only: grep, list in NOTES.
  - `update/operate.rs`: one call (ruling 6).
  - `GraphOverlay` match sites: `view/frame.rs:131-155`, `view/frame/boxes.rs:290`, `update/undo.rs:1669`.
    `view/frame/overlays.rs`.
  - `view/rows.rs`: `overlay_context`, the graph bar's `R rewrite`.
- graft tests: `update/tests.rs` (sweep `:3696`; the hint-bar tests `:3880`, `:5376`, `:5462` run, extended only
  in their lists), `update/search/tests.rs` (the slash sweep), `tests/it/m7_walk/rewrite_editor.rs` (new; the
  conductor adds its `mod` line at the seam).
- graft-ui: `src/rewrite_editor.rs` (new) + tests, `src/lib.rs` (mod/use), `graph_screen/hints/context.rs`,
  `graph_screen/hints/lists.rs`.
- Docs: MAP.md (graft, graft-ui), TESTING.md, `docs/shots/m7/`, the captions TSV, the contact sheet.

## Acceptance (each tick names its test; red quoted in NOTES)
- [x] **App** `update::rewrite_editor::tests` (08-a: every name below except Mouse and the first two Graph-bar tests, which move to the 08-b box after this one; each named test exists and passes, `cargo nextest run -E 'test(rewrite_editor)'`):
  - Opening:
    - `capital_r_opens_on_the_cursor_row` (**red first**: today `R` is unbound, `keys/predicates.rs`; quote it);
    - `capital_r_on_the_uncommitted_row_opens_on_head`;
    - `a_second_capital_r_while_reading_starts_nothing`;
    - `esc_while_reading_cancels_the_open`;
    - `capital_r_during_an_operation_is_refused_in_words`;
    - `a_refused_open_shows_the_words_and_no_overlay`;
    - `a_failed_open_read_says_so`.
  - Rows:
    - `s_marks_and_moves_one_older`;
    - `p_on_a_marked_row_unmarks_it`;
    - `autosquash_rows_open_marked_by_their_name`;
    - `a_bottom_squash_is_refused_in_words`;
    - `every_commit_dropped_is_refused_in_words`;
    - `the_base_row_refuses_actions`;
    - `alt_up_moves_the_row_and_marks_it`.
  - The field and messages:
    - `r_opens_a_field_that_owns_every_key` (`text_field_open`; `u`, `/`, `q`, `y` typed);
    - `esc_in_the_field_drops_only_the_edit`;
    - `enter_in_the_field_rewords_and_replans`;
    - `the_squash_message_is_default_message_until_edited`;
    - `ctrl_e_hands_the_message_over_and_back`;
    - `a_commit_editor_handover_answer_is_not_the_editors`.
  - Right side: `enter_shows_the_rows_changes`, `tab_swaps_sides_below_120_only`.
  - `esc_writes_nothing`.
  - Planning and `y`:
    - `y_before_the_plan_lands_does_nothing`;
    - `y_launches_the_planned_rewrite`;
    - `an_edit_replans_and_the_last_wins`;
    - `a_superseded_after_answer_is_dropped`;
    - `a_failed_after_plan_says_so_and_y_refuses`;
    - `a_refused_write_restores_the_editor_with_the_words`;
    - `a_moved_branch_refuses_y_in_words`.
  - Modality: `the_overlay_is_rewrite_while_the_editor_is_up`; `u_while_the_editor_is_up_does_nothing`.
  - Graph bar (08-a's one): `capital_r_on_another_branch_is_refused_in_words`.
  - Also added: `r_on_a_melded_row_says_whose_message_it_is`, `ctrl_c_still_quits_under_the_editor`,
    `view_facts_are_the_screen_in_order`, `view_facts_show_the_plan_that_landed`.
- [ ] **08-b** App: Mouse `a_click_selects_the_row`, `a_drag_moves_the_row` (need `rewrite_row_at`); Graph bar
  `the_bar_offers_capital_r_on_his_commit_rows`, `the_bar_hides_capital_r_on_default_branch_rows` (need `HintContext`).
- [x] Sweeps extended, green: `update/tests.rs:3696` (the field) —
  `every_text_field_the_app_has_is_named_by_text_field_open` (rows: not a field; `r`: a field);
  `search::tests::slash_belongs_to_every_modal_that_is_up` (the editor in `boxes()`, its `r` field via `rewrite_field()`
  in `every_modal`, so `n_and_capital_n_never_step_a_search_behind_a_modal` covers both too). Also
  `view::frame::tests::standing_on` stands the editor (forced arm).
- [x] **Job** `jobs::rewrite_open::tests` (real git): `open_reads_every_rows_whole_message`,
  `open_refuses_forked_dependents_in_words` (both pass).
- [ ] **UI** `graft_ui::rewrite_editor` snapshots at 187×50, 140×40, 100×30. Each asserts the non-colour carriers:
  `↓ ✎ ⏸ ✗`, `│` (187 only), `○ new`, the `┊` gutter, `built on yours · follows`, `↕ moved`.
  `rewrite_row_at_matches_the_drawn_rows`.
- [ ] **Walks** `tests/it/m7_walk/rewrite_editor.rs` (fixture: R1's mock, feature-a 5 since main incl. a `fixup!`,
  feature-b built on it):
  - `squash_last_three` = `R s s y` (start: cursor on HEAD), `len(keys) <= noted_keys("squash-last-three")`.
    After it:
    - one commit where there were three;
    - its message = the `default_message` the editor showed (git's whitespace cleanup);
    - feature-b rebased onto it;
    - `u` restores all refs.
  - `swap_adjacent` = `R Alt+↑ y` (start: cursor on HEAD's parent, placed uncounted),
    `len(keys) <= noted_keys("swap-adjacent")`. The two commits' order is swapped, and `u` restores.
  - `rewrite_look_and_leave_writes_nothing` = `R s esc`: refs, index, files and journal unchanged.
  - `journey_counts_match_the_note` green (unchanged).
- [ ] **Captures** (rule 6; colour **unverified**), written by the walk's capture tests through `check_shot`:
  - `docs/shots/m7/rewrite-editor-187x50.txt`. **Right:**
    - the title starts `REWRITE feature-a · 5 commits since it left main · newest first`;
    - two `squash ↓` rows over `pick 5d4c3b2`;
    - the `│` rule and `AFTER y · not written yet`;
    - `3 in one`;
    - `MESSAGE of the squashed commit` with `wip: tests` first, then `Handle expired tokens`, then `wip`;
    - the bar starts `y rewrite`.
  - `docs/shots/m7/rewrite-editor-140x40.txt`. **Right:** AFTER below the rows, labelled; the message below it.
  - `docs/shots/m7/rewrite-editor-100x30.txt`. **Right:** rows only; `tab after` on the bar.
  - `docs/shots/m7/rewrite-refused-fork-120x40.txt`. **Right:** status `feature-b and feature-c are both built on
    yours · Graft moves one line`; no overlay.
- [ ] Hint-bar tests green; `R rewrite` on the graph bar for his commit rows (ruling 10).
- [ ] MAP.md (graft, graft-ui). TESTING.md section "The rewrite editor": the four capture rows, the journey rows, what
  the walk does not prove (colour). Captions + contact sheet.

## Touches (check each, report how)
- **`modal_owns_keys` / `text_field_open` consumers** (ruling 3 lists). While the editor is up, `/`, `u`, relaunch,
  clash, trail and the banner's stop keys stand down. Check: `u_while_the_editor_is_up_does_nothing`, the two sweeps,
  and each consumer's existing tests green.
- **`GraphOverlay` users:**
  - match sites: every site compiles with an arm;
  - `is_some()` readers (`banner.rs:118`, `undo.rs:222`, `place/trail.rs:168`) hide the banner, refuse undo and keep
    the trail while it is up;
  - check: their tests, plus the Journal/Stash/Map/Place overlay tests, green.
- **The preview job and `preview_produced`:** shared with every preview. Routing by `owns` is added before it, and
  `update/preview.rs` is not edited. Check: preview tests green, preview captures byte-identical (`check_shot`).
- **`CancelJob`, `JobFailed` / `JobCancelled` routing:** the other owners (preview, map, detail, 30j's leaves) are
  unchanged. Check: their tests.
- **The `$EDITOR` handover:** reused, not changed. Check: the commit-editor tests green;
  `a_commit_editor_handover_answer_is_not_the_editors`.
- **`operate::operated`:** one call added. Check: 09's `an_r_editor_rewrite_does_not_take_amend_into_words`, 07b's
  stop tests, 10b's refused-arm tests green.
- **Graph keys `R`, `Alt+↑/↓`, `Tab`:** only `R` is new on the graph; `Alt`/`Tab` live inside the modal only. Check:
  a grep for `'R'` and `Modifiers::ALT` in `update/` in NOTES; the hint-bar tests.
- **Graph bar captures:** `R rewrite` changes every capture showing a commit row's bar. Place it by the hint lists'
  existing order rule. Run `check_shot` over all shots, list each changed file and why, re-take with
  `GRAFT_UPDATE_SHOTS=1`, then a second pass without it.
- **Journal/undo:** through the engine, one entry. Check: the `u` steps in both walks.
- **The `.` menu, `a`, absorb:** unchanged. Check: context-menu and amend-into tests green.
- **Keymap:** §2.3/§2.7 already applied; no doc edit.

## Rules / Gates
As rewrite-engine. Gates workspace-wide. Commit at each seam.

## Size
Large (~120k), over one context → **build as two, in sequence**:
- **08-a:** state, job, keys, routing, operate call, unit + job tests, the sweeps.
- **08-b:** graft-ui draw + snapshots, `view/`, `HintContext`, graph bar, walks, captures, docs.

08-b needs 08-a's `RewriteEditor` accessors: 08-a exposes a `pub(crate) fn view_facts(&self)`, whose shape it names
in NOTES.

## NOTES (08-a)
- **Status:** 08-a done. **Compacted once mid-ticket** (context ran out after the source, before the tests).
- **Red first:** `capital_r_opens_on_the_cursor_row` with the `R` binding off (`keys.rs` write_key arm disabled):
  `assertion left == right failed; left: None; right: Some(Oid(…03))` at `rewrite_editor/tests.rs:319`. Binding
  restored, green.
- **Ruling 4, verified before routing:**
  - `JobRunner::cancel` (`jobs/runner.rs:454`) → `Live::cancel` (`jobs/pool.rs:43`): cancels only a registered token;
    an id already finished is a no-op and its answer is still on its way (runner.rs doc :440-442). The editor marks every
    cancelled id `stale` and drops its answer or cancellation (`rewrite_editor.rs` `owns`/`stale`).
  - `preview_produced` returns when `state.preview` is `None` (`update/preview.rs` ~:626-629) and drops a mismatched
    job (~:630-633). The editor's ids never reach it: `owns` routes first (`update/mod.rs`, the `Produced` arm after
    `Started`).
- **Deviations (say so, not silent):**
  - `y` launches with `operate::launch_unstoppable` (as `preview::confirm_preview` does for every history op), not
    `operate::launch`: `Esc` must not kill a `rebase -i` part-way. Words `HISTORY_CANNOT_STOP`, `opened_by` `R`.
  - `write_answered` sits right after `operated`'s `if state.operating != Some(id)` guard, not literally first, so a
    stale answer can't take the launched editor.
  - Whole messages: `graft_git::history::read_commit(handle, oid).message`, the detail pane's read.
- **Questions (rule 7, one wording kept):**
  - Moved after `y`/in the AFTER plan: `Refusal::Moved.explain()` = `<b> moved since the preview was drawn — press
    the key again` (graft-ops `operation.rs:918`); `history_refusal` says `<b> moved since the preview · R again`
    (`operate.rs:1264`). Neither reads `feature-a moved since this opened · esc, then R again`. Not changed.
  - `TodoProblem::EverythingDropped.explain()` = `every commit dropped · the branch would be its base again`
    (graft-ops `rewrite.rs:270`), not `…main again`. Used as is.
  - The bar hides `R` on default-chain rows (ruling 10) but R1 says `R` deeper opens more rows; `R` currently sends the
    open for any row and `plan` decides.
- **Forced arms (literal only):** `event_loop/label.rs` (Task/Product labels), `update/place/write.rs`
  (`OpenRewrite` is a read), `update/mod.rs` (Started/no-op lists, `rewrite_answer`, `job_cancelled`/`job_failed`),
  `jobs/runner.rs` (dispatch), `view/frame.rs` + `view/frame/overlays.rs` (`draw_rewrite` placeholder: clears the
  area; 08-b replaces it), `view/rows.rs::overlay_context` (`Rewrite` → no hints until 08-b), `update/tests.rs`
  (`every_field_spelled_out`), `view/frame/tests.rs::standing_on`. `view/frame/boxes.rs:290` and `update/undo.rs:1669`
  compile unchanged (no exhaustive arm needed). `jobs/kinds.rs`: no edit needed.
- **`view_facts` shape (for 08-b):** `RewriteEditor::view_facts(&self) -> ViewFacts<'_>`:
  `ViewFacts { branch: &str, since: usize, rows: Vec<RowFacts> (newest first), base: &BaseRow { oid, subject, label },
  cursor: usize (rows.len() = base), field: Option<(screen_row, &str, caret)>, messages: Vec<MessageFacts { kept,
  kept_subject, squashed, text }>, after: AfterFacts { Checking | Failed(&str) | Refused(&str) | Invalid(String) |
  Ready(&Planned) }, changes: Option<(Oid, &Loading<Arc<FileList>>)>, narrow_shows_right, notice: Option<&str>,
  handing_over }`; `RowFacts { action, commit, subject, moved, by_name, keeps, problem: Option<String> }`. The types
  carry `cfg_attr(not(test), expect(dead_code))` until 08-b reads them: drop those attributes then.
- **Touches:** grep `Char('R')` in `update/` (non-test): only `keys/predicates.rs:115`. `modifiers.alt` readers: Alt+↑/↓
  handled only in `rewrite_editor/keys.rs`, which claims every key while up, before the other modals. Consumers of
  `modal_owns_keys`/`text_field_open`, `GraphOverlay` users, preview/`CancelJob`/`JobFailed` routing, the `$EDITOR`
  handover, `operate::operated`: full `cargo nextest run` green (4726/4726), incl. commit-editor, preview, amend-into,
  stop and search sweeps. `update/preview.rs` not edited. Captures not re-taken: 08-a draws nothing new on any
  existing screen (graph bar unchanged; 08-b adds `R rewrite` and re-takes).
- **Left for 08-b:** graft-ui draw + snapshots, `view/rewrite_editor.rs`, replace `draw_rewrite`'s placeholder,
  `HintContext` + graph bar `R rewrite` (+ `bound.rs` row), mouse hit test (`rewrite_row_at`) in `rewrite_mouse`, the
  two mouse and two bar tests, walks, captures, TESTING.md, graft-ui MAP.md, captions, contact sheet.
