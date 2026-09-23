# fix-landing-after-a-stop: after a stop, a launch into a live operation, an `Esc`, or a `c`, the next key is on screen and one press away

Status: done (2026-09-23; filed 2026-09-23, P1–P8 approved by the user, P9 by the conductor — see docs/field-notes.md)
Blocked by: none
Was: draft · **revised 2026-09-23 (Amendment 2)**: item 5 now covers rebase, cherry-pick and revert only. A merge's `c` opens the finish summary, and `fix-finish-summary-screen` asserts where its commit lands.
**Revised after scout (2026-09-23, `.scratch/m6-conflicts/scout-12.md`):**
- (a) **A stop is an `AppState` field, not a `Status` variant.** `Status` is one slot (`update/state.rs:648`), and during a live operation other things take it: a refused `c`, a failed resume's `Sticky`, `Busy`, `Done`, `Copied`. As a variant, the stop line would be wiped by any of them, and it could not come back after `Esc` dismissed a real problem. `Status::Stopped` is dropped.
- (b) The reason is **worded at draw time** from the live working-tree read, so the step and the count stay true as files get resolved.
- (c) A new `land_on_head` puts the cursor on `HEAD`'s row, spent by the refs answer (no such helper exists; it rides `History::seek`).
- (d) Footer precedence is spelled out below: `stopped_row` replaces only the plain hints row.
- (e) `hand_the_keyboard_back` no longer clears a `Sticky` on `Tab`/`BackTab`. Only `Esc` dismisses a `Sticky` (the `Status` contract), so `Tab` now keeps a failed resume's words on screen. `Esc` is unchanged.

Build with: **Opus**: the change spans `graft` and `graft-ui`, adds a state field whose lifetime crosses three answers (stop, refs, operation end), and changes the footer's precedence. Tests and captures can judge it; no signature is left open. Scouts used: `scout-stuck-after-merge.md`, `scout-12.md`.
Needs: none.
Crates: graft, graft-ui (`graph_screen::hints`, `graph_screen::footer`, `GraphView`).

## Read first
- `.scratch/m6-conflicts/scout-stuck-after-merge.md` and `.scratch/m6-conflicts/scout-12.md` (whole; one page each; every file:line this ticket touches).
- `docs/design-notes/m6-conflicts.md`:
  - Amendment 1: Need; Journeys J1, J3, J7 and "Landing per event"; Interfaces (the `fix-landing-after-a-stop` block, **superseded by this ticket's Signatures**); Hard choice 1; New UI (bottom rows).
  - Amendment 2: "Where each event lands him".
- `crates/graft/MAP.md` §`update::banner` (`take_focus`, `banner_key`, `resumed`), §`update::relaunch`, §`update::operate` (`conflicted_line`, `operated`), §update `Status`, `AppState::modal_owns_keys`, `AppState::refs`, §history `History::seek`, `DetailState::kept_cursor` (the set-then-spent precedent `land_on_head` copies).
- `crates/graft-ui/MAP.md` §`graph_screen::hints` (`HintContext`, `every_context`, `marked_form`, `pane_focused_form`, `fit_status_line`), `GraphView::problem`/`search`/`notice`/`compare_footer`/`reveal`; `crates/graft-git/MAP.md` `InProgress`, `InProgressDetail`.
- `docs/design/02-keymap.md` §2.1 (the hint-bar paragraph), §2.6a.
- `docs/research/ux-principles.md` §3.1, §5 items 1 and 6.

## Scope
In:
1. **A stop is a state.**
   - `AppState::stopped: Option<Stop>` is set:
     - by `operate::operated`'s `Conflicted` (`ThisRun`, `again: false`);
     - by `banner::resumed`'s `Conflicted` again (`ThisRun`, `again: true`);
     - by `land_on_live_operation`.
   - At those three sites, the `Sticky` they set today is gone. `Status` becomes `Ready` there, which also clears the launch's `Busy`, as the `Sticky` did.
   - It is cleared **only** by `undo::operation_ended`. `Esc`, a refresh and a `JobFinished` leave it.
   - It never draws `esc dismiss` and never replaces a key.
   - A failed resume still sets its `Sticky` (`a abort`), as today, and `stopped` stays set under it.
2. **Reason words**: `stop_reason(stop, live)`, called by the view each frame.
   - `live` is the working-tree read's `in_progress` detail and its unmerged count, **only when** that detail's kind equals `stop.kind`. Otherwise `None`: the stop's own `kind` and `paths_at_stop` are used, and the step is left out.
   - The count part is left out when it is 0.
   - `ThisRun`:
     - merge: `merge stopped · 1 file conflicted`;
     - rebase: `rebase stopped at step 3 of 7 · 2 files conflicted`;
     - stopping again: `rebase stopped again at step 4 of 7 · …`;
     - nothing unmerged: `rebase stopped at step 3 of 7`;
     - cherry-pick and revert: `InProgress::name`, with the merge's shape.
   - `PreviousRun`: `merge still in progress from a previous run · 1 file conflicted`. A rebase adds `· step 3 of 7` before the count.
   - `OutsideGraft`: `merge in progress · begun outside Graft · 1 file conflicted` (rebase as above).
   - Plural `files` from 2 up.
3. **Footer precedence.** The graph screen's bottom row draws the first of these that applies (`footer.rs:61-100`; only rung 6 changes):
   1. `problem` (`Failed`/`Sticky`): the whole row, `fit_status_line`, as today.
   2. search line.
   3. notice.
   4. compare footer.
   5. reveal.
   6. hints:
      - `view.stopped` is `Some` → `stopped_row(reason, &hints, columns)`. The bar is fitted first, by its own ladder, at `columns`. If `columns − bar − 3 ≥ 12`, the row is `<reason, cut with …> · <bar>`. Otherwise it is the bar alone.
      - `None` → as today.

   A consequence, and it is accepted: beside the banner's fixed list the reason shows. Beside a laddered bar (after `Esc`) the ladder usually fills the row and the reason drops. The header banner (`GraphView::banner_op`) still names the operation.
   `view::rows::graph_view` fills `GraphView::stopped` from `stop_reason`. The status screen and the stash overlay are unchanged.
4. **Launch keyed on git, not the journal.**
   - `land_on_live_operation` replaces `launch_into_live`. It runs on the first working-tree read whose `in_progress` is `Some` and `banner_focusable`, **whether or not** `live_entries` holds anything (this fixes `relaunch.rs:43`). It is called from both current call sites (`mod.rs:546-568`, `journal.rs:104,130-132`).
   - Origin: a live entry → `PreviousRun`; none → `OutsideGraft`. A journal read that lands later turns `OutsideGraft` into `PreviousRun` if it names a live entry.
   - It lands the way today's relaunch does: `banner_focus`, then `land_on_the_work`, which puts the changes pane on `CONFLICTS`' first row.
   - While `modal_owns_keys()` is true, it sets `stopped` only and returns `false`. The focus landing stays owed (`AppState::live_landing_owed`) and runs on the first working-tree read after the modal closes. The debt is cleared when the landing runs or `operation_ended` fires.
5. **A way back.**
   - While an operation is live, the banner is unfocused and `one_tab_to_banner` is true, the graph footer's context is `HintContext::OperationLive(row)`: the row's list with `⇥ banner` (Live) first.
   - `one_tab_to_banner`: `in_progress` is `Some`, `banner_focusable`, the banner is not focused, and `modal_owns_keys()` is false.
   - `Esc` on the banner is unchanged: it hands the keyboard back.
6. **`hand_the_keyboard_back`**: delete the `Sticky → Ready` reset for `Tab`/`BackTab` (`banner.rs:278-280`), and keep today's behaviour for `Esc`. Update the comment there: the banner's line is no longer a `Sticky`.
7. **After a rebase, cherry-pick or revert, `c` lands** (`resumed` → `Done`):
   - the graph is focused;
   - `Status::Done` shows, as today;
   - `AppState::land_on_head = true`.
   - The refs answer from the refresh `resumed` issued spends `land_on_head` through `land_on_head(state, …)`, which calls `History::seek(refs.head())` and clears the flag. If the refs answer already drops stale answers by `JobId`, the flag rides that guard. If it does not, record the refresh's refs job id beside the flag and spend it only on that id. Say which it was in NOTES.
   - Until `fix-finish-summary-screen` lands, a merge's `c` goes through this same path.
8. `HintContext::OperationLive` joins `every_context`, `marked_form`, `pane_focused_form` and `coverage_slot` (the exhaustive matches force it). `every_key_the_bar_draws_is_answered` covers it.

Out:
- opening the resolver on a stop (P5, `fix-stop-opens-the-resolver`, which updates this ticket's first test to press `Esc` first);
- the finish summary;
- `c` on a merge begun in the shell (P4);
- any new key;
- the preview;
- the status screen's and stash overlay's problem line.

## Signatures
```rust
// graft/src/update/state.rs  (Status lives here, not in update/mod.rs; Status is unchanged)
pub struct Stop { pub origin: LiveOrigin, pub again: bool, pub kind: graft_git::state::InProgress, pub paths_at_stop: usize }
pub struct AppState { /* existing */ pub stopped: Option<Stop>, pub land_on_head: bool, pub live_landing_owed: bool }

// graft/src/update/relaunch.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LiveOrigin { ThisRun, PreviousRun, OutsideGraft }
pub(super) fn land_on_live_operation(state: &mut AppState, commands: &mut Vec<Command>, origin: LiveOrigin, next_id: &mut impl FnMut() -> JobId) -> bool;

// graft/src/update/banner.rs
pub fn stop_reason(stop: &Stop, live: Option<(&graft_git::state::InProgressDetail, usize)>) -> String;
pub fn one_tab_to_banner(state: &AppState) -> bool;
/// Spends `land_on_head`: the cursor to `HEAD`'s row via `History::seek`. Takes exactly what
/// `History::seek` needs to issue a `Find`; add `commands`/`next_id` only if it does.
pub(super) fn land_on_head(state: &mut AppState, commands: &mut Vec<Command>);

// graft-ui/src/graph_screen (GraphView)
pub struct GraphView { /* existing */ pub stopped: Option<SanitizedText> }
// graft-ui/src/graph_screen/hints.rs
pub enum HintContext { /* existing */ OperationLive(Box<HintContext>) }
pub fn stopped_row(reason: &SanitizedText, hints: &[Hint], columns: u16) -> String;
```

## Acceptance (name the verifying test when you tick)
- [x] `update::banner::tests::a_stop_keeps_the_banner_keys_on_the_bottom_row`: a `Conflicted` answer, the frame drawn at 80x24, 120x40 and 200x50. The bottom row contains `⏎ resolve` and `a abort`, and not `esc dismiss`. At 120x40 it starts `merge stopped · 1 file conflicted ·`.
- [x] `update::banner::tests::the_stop_line_outlives_a_dismissed_problem`: a stop, then a failed resume (`Sticky`), then `Esc`. The bottom row starts with the stop's reason again. **As built:** `Esc` dismisses the problem *and* leaves the banner (scope item 6, "keep today's behaviour for `Esc`"), and beside the laddered bar at 120 the reason drops (item 3's accepted consequence). So the test asserts `Ready`, `stopped` still `Some`, then one `Tab` back → the row starts `merge stopped · 1 file conflicted`. See NOTES, deviation 4.
- [x] `update::banner::tests::tab_keeps_a_failed_resumes_words`: `Sticky` from a failed resume, then `Tab` on the banner → `status` still `Sticky`.
- [x] `update::banner::tests::esc_on_the_banner_leaves_a_way_back_on_screen`: after `Esc` the bottom row contains `⇥ banner`; one `Tab` → `banner_focus`; `⏎` → the resolver opens on `first_conflicted`.
- [x] `update::banner::tests::stop_reason_words`: every row of scope item 2, including the fallback when the working-tree read's kind doesn't match (no step), a count of 0 (no count part), and `1 file` / `2 files`.
- [x] `update::banner::tests::the_count_follows_the_read`: a stop with 2 conflicted, then a working-tree read with 1 unmerged → the reason reads `1 file conflicted`.
- [x] `update::relaunch::tests::a_merge_begun_in_the_shell_lands_on_the_banner`: the first working-tree read has `in_progress: Some(Merge)` and the journal is empty. Result: `banner_focus`, the pane on `CONFLICTS`, and a reason containing `begun outside Graft`. One `⏎` opens the resolver.
- [x] `update::relaunch::tests::a_previous_runs_merge_still_lands_the_same_way` (scenario 26's shape keeps passing).
- [x] `update::relaunch::tests::a_modal_defers_the_landing`: a modal is open at the first read → `stopped` is set, the banner is not focused. The modal closes, the next working-tree read comes in → `banner_focus`.
- [x] `update::banner::tests::continue_lands_on_the_new_commit`, on a **rebase** whose last step was resolved: `c` → `Done`, then the refs answer. The cursor row's oid = the new `HEAD`, the graph is focused, and `stopped` is `None`.
- [x] `update::banner::tests::a_stale_refs_answer_does_not_spend_the_landing`: a refs answer from before the `c` arrives after `Done` → `land_on_head` is still true.
- [x] graft-ui `hints::tests::operation_live_leads_with_the_way_back`, `hints::tests::stopped_row_drops_the_reason_under_twelve_cells`, `hints::tests::stopped_row_keeps_the_bar_whole`, and `every_context_reaches_every_shape`, green with the new variant.
- [x] graft-ui `footer::tests::a_problem_still_wins_over_a_stop` and `footer::tests::a_search_line_still_wins_over_a_stop`.
- [x] Captures under `docs/shots/m6/`: `fix-landing-a-stop-{120x40,80x24}.txt`, `fix-landing-after-esc-120x40.txt`, `fix-landing-b-launch-120x40.txt`, `fix-landing-after-continue-rebase-120x40.txt`, each also at 187x50 (and the stop at 80x24). Verified by `it::conflict_interim::landing_shots::{capture_a_merge_that_stops, capture_after_esc_on_the_banner, capture_a_launch_into_a_shell_merge, capture_after_continue_on_a_rebase}` — each asserts the row text below, then `check_shot`. The stop fixture has **2** conflicted files, so it reads `2 files conflicted`. **What right looks like:**
  - Stop: the last row starts `merge stopped · 1 file conflicted ·` and lists `⏎ resolve  c continue  a abort`. At 80x24 the keys are whole, and the reason is cut with `…` or absent.
  - After `Esc`: the last row lists `⇥ banner` first.
  - Launch: the last row starts `merge in progress · begun outside Graft`, and the pane shows `CONFLICTS` with the cursor on its first file.
  - After continue: the cursor row is the rebased branch's tip (its subject is the fixture's last replayed commit), and the last row ends `✓`.
  - Focus by reverse video is **colour, unverified** (rule 5).
  - The user judges by eye.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, workspace-wide.

## Size
Medium to large: about 7 functions over `state.rs`, `banner.rs`, `relaunch.rs`, `operate.rs`, `undo.rs`, `view/rows.rs` and the refs-answer site. Plus one enum variant and `stopped_row` in `hints.rs`, one rung in `footer.rs`, and one `GraphView` field. One context. If it runs past ~80k tokens, stop after item 4 and hand off. Items 5–8 split cleanly.

## NOTES (builder, 2026-09-23)

**I compacted mid-ticket.** Nothing of the ticket is left undone; what remains is listed under "Gaps" and "Not verified" below.

Scouts used: `.scratch/m6-conflicts/scout-12.md` and `scout-stuck-after-merge.md`.

Fix type: **cause**. The stop is now a state (`AppState::stopped`), not a `Status` value, so nothing that takes the status slot can wipe it. The launch keys on git's `in_progress`, not on a journal entry.

**The refs guard (item 7).** The refs answer was **not** guarded by `JobId`: `Command::ReadRefs` carried no id, and `took_fact` accepted any refs answer. Changes:
- `ReadRefs(JobId)`: the id is chosen by `update` (`init`, `refresh_key`), and `jobs::runner::read_refs` queues under it.
- `AppState::head_landing { refs, walk }` records the refresh's refs id **and** its walk id.
- `land_on_head` runs only once both have come in: the refs answer with that id, and the walk has started. `History::seek` needs a window from the new walk; seeking in the old walk would put the cursor on a row that is about to be replaced.
- A refs **failure** under that id drops the landing. The failure is on screen, and the stale refs are not used.

### Deviations (the conductor should rule on each)
1. **The stalled `Sticky` is kept.** When git gives a stalled reason (`Operated::Conflicted { stalled: Some }`), `operate` and `resumed` still set a `Sticky` with git's words (`stalled_line` / `stopped again: …`). The stop is set underneath. Dropping it would swallow git's own message (rule 3). With no stalled reason, the status is `Ready`, as the ticket says.
2. **A second clearing path for `stopped`.** It is also cleared by a working-tree read with `in_progress: None` (`update::mod::working_tree_read`), not only by `undo::operation_ended`. An operation finished or aborted in another terminal never runs `operation_ended`, and without this clear the stop line would word an operation git no longer has.
3. **`one_tab_to_banner` has a fifth condition:** the pane is hidden, or it is on its diff. `take_focus` sends `Tab` from the graph to the pane when the pane is open. Without this condition, `⇥ banner` would name a key that does something else (`update::tests::every_swept_state_is_the_context_it_claims` caught it).
4. **`Esc` on a failed resume.** Scope item 6 says `Esc` keeps today's behaviour: it dismisses the `Sticky` and leaves the banner in one press. The acceptance line for `the_stop_line_outlives_a_dismissed_problem` expects the reason on the row right after `Esc`. That is impossible at 120 columns once the banner is left, because the ladder fills the row (item 3's accepted consequence). I kept `Esc` as it is today, and the test presses one `Tab` back before it reads the row.
   - I briefly tried "`Esc` only dismisses", and reverted it. It cost the refused-`c` journey an extra `Esc`, and `continue_refuses_until_resolved…` failed on it.
   - The comment in `hand_the_keyboard_back` records this.
5. **`OperationLive` does not wrap `ChangesPane`.** The changes pane is one `Tab` from the banner only from its diff. With the diff focused, the pane's own bar draws `space stage` and `d discard`, which do nothing there (`every_key_the_bar_draws_is_answered` flagged 34 dead keys). That dead-key issue predates this ticket (the ChangesPane bar is drawn with Diff focus) and was **not fixed**. It is filed below.
6. **Crate set.** Besides `graph_screen::hints`, I edited `graph_screen::footer`, `graph_screen::mod` (`GraphView::stopped`), `graph_screen::tests` and `tests/support/screen.rs`. The ticket's scope names them. The conductor's message named only `hints`.

### Other changes the ticket forced
- The m5 walk expected the old `Sticky` text or a manual `Tab` to reach the banner at launch. These now read `graft::update::banner::stop_line` (a new `pub fn`) or wait for `BannerFocused`: `conflicts.rs`, `resolved_operations.rs` (`kill_9_*`, `resolver_is_modal…`, `tiny_frame…`), `regressions.rs` (the `hints_at_80` bar now opens `⇥ banner  ⏎ changes  c commit`), and `gestures.rs` (the resolver walker leaves the banner with `Esc`).
- `conflict_interim.rs`: the relaunch tests assert `Status::Ready` plus the stop row (`stop_line_reads`).
- The m5 banner captures `docs/shots/m5-ui-banner-conflict-{80x24,120x40,160x48}.txt` were rewritten, one row each: the bottom row now leads with `merge stopped · …`.

### Gaps (for KNOWN_GAPS, marked apart)
- **Broken, pre-existing:** the `ChangesPane` bar drawn with the diff focused offers `space stage` / `d discard`, which do nothing there. Found by the sweep once a Diff-focus fixture existed.
- **Missing:** there is no way-back hint while the changes pane's file list has focus. `⇧⇥` would reach the banner from there, but nothing says so.
- **Missing:** a merge begun in the shell *mid-session* lands only on the first working-tree read, or on an owed landing. A later read that newly finds `in_progress` records nothing. Only launch is covered, as the ticket scoped it.
- **Unverified:** colour. The focused banner's reverse video and the cursor row on the pane are not in a character grid (rule 5).
- **Capture artefact:** in `fix-landing-after-continue-rebase-*`, the replayed commits' age reads `futu…`. Git stamps the commit time now, and the capture clock is fixed at 2021. Their ids are zeroed (`without_invented_id`). The committer is set to the fixture identity so no real name reaches the file.

### Not strictly test-first
The graft-ui work (`stopped_row`, `OperationLive`, the footer rung) was written with its tests in the same pass, not red first.
