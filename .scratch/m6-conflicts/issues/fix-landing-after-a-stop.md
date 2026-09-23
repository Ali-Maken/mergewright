# fix-landing-after-a-stop: after a stop, a launch into a live operation, an `Esc`, or a `c`, the next key is on screen and one press away

Status: draft · **revised 2026-09-23 (Amendment 2)**: item 5 now covers rebase, cherry-pick and revert only. A merge's `c` opens the finish summary, and `fix-finish-summary-screen` asserts where its commit lands.
Build with: blank — brownfield (ADR-0035). `scout-stuck-after-merge.md` covers the start paths; a second scout owes the footer's draw path (`view::status`, `view::rows::footer_context`, `hints_for(Banner)`). Expected **Opus**: three state transitions and the footer's precedence, judged by tests and captures.
Needs: none.
Crates: graft, graft-ui (`graph_screen::hints` only).

## Read first
- `docs/design-notes/m6-conflicts.md`:
  - Amendment 1: Need; Journeys J1, J3, J7 and "Landing per event"; Interfaces (the `fix-landing-after-a-stop` block); Hard choice 1; New UI (bottom rows).
  - Amendment 2: "Where each event lands him".
- `.scratch/m6-conflicts/scout-stuck-after-merge.md` (whole; one page; the file:lines this ticket fixes).
- `crates/graft/MAP.md` §`update::banner` (`take_focus`, `banner_key`, `resumed`), §`update::relaunch`, §`update::operate` (`conflicted_line`, `operated`), §update `Status`; `crates/graft-ui/MAP.md` §`graph_screen::hints` (`HintContext`, `every_context`, `marked_form`, `pane_focused_form`, `fit_status_line`).
- `docs/design/02-keymap.md` §2.1 (the hint-bar paragraph), §2.6a.
- `docs/research/ux-principles.md` §3.1, §5 items 1 and 6.

## Scope
In:
1. **A stop is a state.**
   - `Status::Stopped { reason }` replaces the `Sticky` the stop sets today (`operate::operated`'s `Conflicted`, `banner::resumed`'s `Conflicted` again, and the launch takeover).
   - It is drawn on the bottom row as `reason · ` followed by the current bar, through `stopped_row`: the bar is fitted first, by its own ladder; the reason takes what is left, cut with `…`, and is dropped under 12 cells.
   - It never draws `esc dismiss` and never replaces a key.
   - It is cleared by `undo::operation_ended`, not by `Esc` and not by a refresh.
   - A real problem (`Failed`/`Sticky`) still wins the row, as today.
2. **Reason words** (`stop_reason`):
   - merge: `merge stopped · 1 file conflicted`;
   - rebase: `rebase stopped at step 3 of 7 · 2 files conflicted`;
   - rebase stopping again: `rebase stopped again at step 4 of 7 · …`;
   - nothing unmerged: `rebase stopped at step 3 of 7`;
   - `PreviousRun`: `merge still in progress from a previous run · …`;
   - `OutsideGraft`: `merge in progress · begun outside Graft · …`.
3. **Launch keyed on git, not the journal.**
   - `land_on_live_operation` replaces `launch_into_live`. It runs on the first working-tree read whose `in_progress` is `Some` and `banner_focusable`, **whether or not** `live_entries` holds anything (this fixes `relaunch.rs:43`).
   - Origin: a live entry → `PreviousRun`; none → `OutsideGraft`.
   - It lands the same way today's relaunch does: banner focused, changes pane on `CONFLICTS`' first row.
   - Not while a modal or text field is open. Then only the Stopped line shows, and focus waits for the modal to close.
4. **A way back.**
   - While an operation is live, the banner is unfocused and `one_tab_to_banner` is true, the graph footer's context is `HintContext::OperationLive(row)`: the row's list with `⇥ banner` (Live) first.
   - `Esc` on the banner is unchanged: it hands the keyboard back.
5. **After a rebase, cherry-pick or revert `c` lands** (`resumed` → `Done`):
   - the graph is focused;
   - the history cursor is on `HEAD`'s row (the new commit);
   - `Status::Done` shows, as today.

   Only while `fix-finish-summary-screen` has not landed, a merge's `c` still goes through this same path.
6. `HintContext::OperationLive` joins `every_context`, `marked_form`, `pane_focused_form` and `coverage_slot` (the exhaustive matches force it). `every_key_the_bar_draws_is_answered` covers it.

Out:
- opening the resolver on a stop (P5, `fix-stop-opens-the-resolver`, which updates this ticket's first test to press `Esc` first);
- the finish summary;
- `c` on a merge begun in the shell (P4);
- any new key;
- the preview.

## Signatures
```rust
// graft/src/update/mod.rs
pub enum Status { /* existing */ Stopped { reason: String } }
// graft/src/update/relaunch.rs
pub enum LiveOrigin { ThisRun, PreviousRun, OutsideGraft }
pub(super) fn land_on_live_operation(state: &mut AppState, commands: &mut Vec<Command>, origin: LiveOrigin, next_id: /* launch_into_live's own type, unchanged */) -> bool;
// graft/src/update/banner.rs
pub fn stop_reason(state: &AppState, origin: LiveOrigin, again: bool) -> String;
pub fn one_tab_to_banner(state: &AppState) -> bool;
// graft-ui/src/graph_screen/hints.rs
pub enum HintContext { /* existing */ OperationLive(Box<HintContext>) }
pub fn stopped_row(reason: &str, hints: &[Hint], columns: u16) -> String;
```

## Acceptance (name the verifying test when you tick)
- [ ] `update::banner::tests::a_stop_keeps_the_banner_keys_on_the_bottom_row`: a `Conflicted` answer, with the frame drawn at 80x24, 120x40 and 200x50. The bottom row contains `⏎ resolve` and `a abort`, and not `esc dismiss`.
- [ ] `update::banner::tests::esc_on_the_banner_leaves_a_way_back_on_screen`: after `Esc` the bottom row contains `⇥ banner`; one `Tab` → `banner_focus`; `⏎` → the resolver opens on `first_conflicted`.
- [ ] `update::relaunch::tests::a_merge_begun_in_the_shell_lands_on_the_banner`: the first working-tree read has `in_progress: Some(Merge)` and the journal is empty. Result: `banner_focus`, the pane on `CONFLICTS`, and a `Status::Stopped` reason containing `begun outside Graft`. One `⏎` opens the resolver.
- [ ] `update::relaunch::tests::a_previous_runs_merge_still_lands_the_same_way` (scenario 26's shape keeps passing).
- [ ] `update::banner::tests::continue_lands_on_the_new_commit`, on a **rebase** whose last step was resolved: `c` → `Done`, the cursor row's oid = `HEAD`, the graph focused, and `Stopped` gone.
- [ ] graft-ui `hints::tests::operation_live_leads_with_the_way_back` and `every_context_reaches_every_shape`, green with the new variant.
- [ ] Captures under `docs/shots/m6/`: `fix-landing-a-stop-{120x40,80x24}.txt`, `fix-landing-after-esc-120x40.txt`, `fix-landing-b-launch-120x40.txt`, `fix-landing-after-continue-rebase-120x40.txt`. **What right looks like:**
  - Stop: the last row starts `merge stopped · 1 file conflicted ·` and lists `⏎ resolve  c continue  a abort`.
  - After `Esc`: the last row lists `⇥ banner` first.
  - Launch: the last row starts `merge in progress · begun outside Graft`, and the pane shows `CONFLICTS` with the cursor on its first file.
  - After continue: the cursor row is the rebased branch's tip (its subject is the fixture's last replayed commit), and the last row ends `✓`.
  - Focus by reverse video is **colour, unverified** (rule 5).
  - The user judges by eye.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, workspace-wide.

## Size
Medium: about 5 functions over `banner.rs`, `relaunch.rs`, `operate.rs`, `view/status.rs` and `view/rows.rs`, plus one enum variant in `hints.rs`. One context.
