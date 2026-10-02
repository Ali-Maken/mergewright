# `q` while a write that can't stop runs: quit waits for it; `q quit` on the banner's bar

Status: done
Build with: Opus (conductor, after scout `scouts/13b-quit.md`)
Blocked by: 13
Needs: ticket 13's review fix (Major 1: `banner_key` sends `q` to `quit::asked`), merged. No other ticket's signature.
Brownfield (ADR-0035): **scout first**. The conductor sets `Build with` after it.

**The user has not seen this.** Box words, the waiting status line, Graft quitting by itself when the write ends,
and `q quit` on the banner's bar are all new on screen. Keymap rows are marked Proposed. The captures below are what
he judges.

## Read first
- `docs/design/02-keymap.md`: §2.2 `q` row and the "`q` while a write runs" paragraph; §2.6a `q` row; §2.1 bullet
  "The hint bar is a ranked selection"; §2.3c
- `docs/reviews/m8-13-landing.md` Major 2 (launch sites with lines), Minor 3 (shutdown grace)
- `.scratch/m8-polish-and-ship/issues/13-daily-landing.md`: Rulings 4, "Review fixes"
- `docs/research/ux-principles.md` §3.1
- `crates/graft/MAP.md`: `update::quit`, `ConfirmThen` (`Quit`), `PendingConfirm::yes`, `Stoppable`,
  `cancel_operating` / `stop_write`; `crates/graft-ui/MAP.md`: `HintContext::Banner`, `graph_screen::hints`,
  `ConfirmYes`

## Scope
In:
1. graft `update/quit.rs`: `running_box` words the box by the running write's `Stoppable` (keymap §2.2, exact words).
   `confirmed` for a `Stoppable::No` write sets the wait and the status line. It does not call `stop_write` and does
   not emit `CancelJob`.
2. graft: every answer path of a `Stoppable::No` write calls `quit::write_ended` **after** the answer is applied and
   journaled. If the write is followed by a put-back (edits set aside), call it after the put-back's answer, not the
   write's. Done → quit. Stopped, failed, incomplete or refused → drop the wait; landing and words are unchanged.
3. graft: `Esc` while waiting drops the wait. Order: held-failure dismiss first (§2.2 `Esc`), then the wait, then
   `stop_write`'s line. `q` while waiting: nothing changes (no second box).
4. graft: the waiting status words survive the write's progress updates (rebase `step i of n`, busy lines).
5. graft-ui `banner_list` (`HintContext::Banner`): `q quit` becomes the last hint on the banner bar, the first one
   dropped when the row is narrow.
6. Re-shoot every capture the banner bar change makes stale. Re-assemble the contact sheets they sit in. Update the
   captions and TESTING.md rows that quote the bar.
7. MAP.md rows: graft `update::quit`, `ConfirmThen`, `AppState` field; graft-ui hints, if `banner_list` has a row.

Out:
- the stopped-operation box (`stopped_box`; ruling 2b pending with the user) and its wording (review Minor 4)
- `Ctrl+C` behaviour (see Question)
- whether the resolver (§2.6, modal) answers `q`. Not asked here.

## Signatures
These are crate-private. The scout may correct a name to match the code; MAP.md records the final shape.
```rust
// graft, update/quit.rs — what [ Quit ] / y does; replaces `ConfirmThen::Quit { stops: Option<JobId> }`
pub(crate) enum QuitThen {
    Now,             // nothing running (today's `stops: None`)
    Stop(JobId),     // Stoppable::Yes — stop it, then quit (ticket 13, unchanged)
    WaitFor(JobId),  // Stoppable::No — quit once it ends done
}
// ConfirmThen::Quit { then: QuitThen }

// graft, AppState
pub(crate) quit_when_done: Option<JobId>,

// graft, update/quit.rs — called by each Stoppable::No answer path, after apply + journal
pub(crate) fn write_ended(state: &mut AppState, job: JobId, done: bool);
// Esc chain: true when it dropped a wait (so the Esc goes no further)
pub(crate) fn drop_wait(state: &mut AppState) -> bool;
```
`done` is true only for a complete answer (`Operated::Done`, "up to date" included). An Incomplete answer is not done.

## Touches (check each one, and say how in the report)
- `update/quit.rs` `running_box`, `confirmed`; `ConfirmThen::takes_y` and `PendingConfirm::yes` stay `ConfirmYes::Quit`.
  No new `ConfirmYes`, so the graft-ui `COVERAGE_SLOTS` count does not change.
- Answer paths of a `Stoppable::No` launch. The review lists the launch sites: `operate.rs:116-131`,
  `banner/resume.rs:46`, `preview/answer.rs:271`, `undo.rs:507`. Each one's answer must reach `write_ended`. A test
  per path, or one choke point shown by the scout.
- Set-aside put-back chains (ADR-0045): quit after the last answer.
- The `Esc` chain (`escape_key`, `stop_write`, the held failure) and the commit editor's `Esc` through `stop_write`.
- Status priority: the waiting line must not be overwritten by a busy or progress line, and must not outlive the wait.
- `?` help `IN PROGRESS` / `HERE` for the banner (§2.12): lists `q` if it lists the banner's keys.
- graft-ui: `hints` banner context tests, `the_bare_keys_are_a_tail_not_a_scatter`, the bar-fragment test.
  graft: `every_bound_action_is_hinted_live_or_is_named_here`.
- Captures that draw the banner bar. Grep `docs/shots` for `M mergetool  e edit  ⇥ graph` and `a abort`: m6
  `m6-06-relaunch-sticky-80x24.txt`, `m6-contact-sheet-2.txt` among them. Also their captions (`*-captions.tsv`) and
  TESTING.md rows.
- `m8-quit-while-running-80x24.txt` (fetch box): unchanged words. Confirm it still matches.
- Journal and undo: no new entry. The quit waits for the write's entry.

## Acceptance
Each tick names its test inline.
- [x] `q` during a `Stoppable::No` write (a merge) → box `Quit?`, lines exactly `<desc> is still running · quit
      waits for it to finish` and `if it stops at a conflict or fails, you stay to see it`, buttons
      `[ Cancel ] [ Quit ]`, cursor on Cancel, bar leads `y quit`.
      — `update::quit::tests::q_during_a_merge_says_quit_waits_for_it`
- [x] `y` → box closed, app running, no `CancelJob` emitted, status `<desc> · quitting when it finishes · esc stays`.
      — `update::quit::tests::y_over_a_merge_waits_and_cancels_nothing`; real pool: `it::quit_waits::quit_mid_merge_waits_for_git_then_quits`
- [x] Then the write answers done → app quits; the write's journal entry exists (read the journal after the quit).
      — `update::quit::tests::the_merge_ending_done_quits`; journal read from disk, entry `Committed`:
      `it::quit_waits::quit_mid_merge_waits_for_git_then_quits`
- [x] Answers stopped at a conflict → no quit; resolver open on the first conflicted file; wait gone (a later done
      write does not quit). — `update::quit::tests::the_merge_stopping_at_a_conflict_stays_in_the_resolver`
- [x] Answers failed → no quit; the failure's own words on the status row.
      — `update::quit::tests::the_merge_failing_stays_with_its_words`
- [x] Answers incomplete → no quit; its warning kept. — `update::quit::tests::the_merge_ending_incomplete_stays_with_its_warning`
- [x] `Esc` while waiting → wait dropped, write runs on, then answers done → no quit.
      — `update::quit::tests::esc_while_waiting_drops_the_wait`, `esc_dismisses_a_held_failure_before_the_wait`,
      `esc_on_the_banner_drops_the_wait`
- [x] A progress update while waiting keeps the waiting words. — `update::quit::tests::progress_while_waiting_keeps_the_waiting_words`
- [x] `q` during a `Stoppable::Yes` write is unchanged: ticket 13's `quit::tests` still pass, names listed.
      — `q_with_nothing_running_quits`, `q_during_a_fetch_opens_the_box`, `quit_from_the_box_cancels_the_fetch_and_quits`,
      `y_quits_from_the_box_and_cancels_the_fetch`, `cancel_or_esc_keeps_the_app_and_the_fetch`,
      `quit_after_the_write_ended_cancels_nothing`, `ctrl_c_during_a_fetch_quits_without_a_box`,
      `q_on_the_banner_while_a_write_runs_names_the_write` (all `update::quit::tests`, passing)
- [x] `q` on the focused banner with a `c` continue running → this box (extends
      `q_on_the_banner_while_a_write_runs_names_the_write`). — `update::quit::tests::q_on_the_banner_while_its_continue_runs_waits_for_it`,
      then `a_continue_ending_done_quits`, `a_continue_stopping_again_stays`; the revert path:
      `a_revert_ending_done_quits`, `a_revert_failing_part_way_stays`
- [x] Banner bar: `q quit` is the last hint at 187 columns. At 80 columns it is dropped before any other banner hint.
      graft-ui hints test. — `graft-ui graph_screen::hints::tests::q_quit_is_the_banner_bars_last_hint_and_first_dropped`
      (every width 20–187, four banner shapes), `q_quit_gives_way_to_the_stop_reason`; the dispatcher anchor
      `update::banner::tests::every_hint_the_focused_banner_draws_is_answered_by_banner_key` now presses `q`
- [x] Captures (colour **unverified**): `docs/shots/m8-quit-waits-80x24.txt` (the box over a running merge, then
      the waiting status row); `docs/shots/m8-banner-bar-q-187x50.txt` and `-80x24.txt` (focused banner after a
      conflict stop). A capture drawn from a synthetic running state, as `capture_quit_while_a_fetch_runs` is, says so.
      — `it::quit_waits::capture_quit_waits` (a real merge, real pool, its answer not pumped between the keys — the
      same method as `capture_quit_while_a_fetch_runs`; not synthetic state), `it::conflict_interim::capture_banner_bar_with_q`.
      TestBackend grids, not pty.
- [x] Stale captures re-shot and contact sheets re-assembled; list them. — test-checked (`GRAFT_UPDATE_SHOTS=1`, then a
      clean run green): `m5-ui-banner-conflict-120x40`, `m6/fix-landing-b-launch-{120x40,187x50}`,
      `m6/fix-tab-words-{120x40,80x24}`. pty walk (`.scratch/m6-conflicts/pty-walk-m6.py`, real binary):
      `m6/m6-06-relaunch-sticky-{80x24,120x40}` (SIGKILL confirmed by the script), `m6/m6-18-launch-shell-begun-{80x24,120x40,160x48,187x50}`;
      `-200x50` re-run and settled (unchanged: `q quit` gives way there). Sheets re-assembled: `m5-contact-sheet-{1,2}`,
      `m6/m6-contact-sheet-{1,2,4}`. Captions: 9 rows in `m6-contact-sheet-captions.tsv`, 1 in `m5-…-captions.tsv`;
      TESTING.md scenario 49. `m8-quit-while-running-80x24.txt` unchanged (its test passes).
- [x] **Ruling (user, 2026-10-02):** git stopped and nothing running → `q` quits, no box; relaunch offers continue.
      — `update::quit::tests::q_with_a_stopped_rebase_quits_without_a_box`, `q_on_the_banner_after_a_conflict_stop_quits_with_no_box`,
      `q_on_the_banner_after_a_relaunch_quits_with_no_box`, `update::undo::tests::q_on_the_banner_after_an_undo_puts_a_merge_back_quits_with_no_box`;
      quit then relaunch on a real repository: `it::conflict_interim::q_at_a_stop_quits_and_the_relaunch_offers_continue`
- [x] **Conductor decision:** `Ctrl+C` over a `Stoppable::No` write boxes like `q`, never signals it (red before the
      fix: both tests below failed with the old `Ctrl+C` path restored) — `update::quit::tests::ctrl_c_during_a_merge_boxes_like_q_and_signals_nothing`,
      `ctrl_c_over_the_waiting_box_does_not_quit`; real pool: `it::quit_waits::ctrl_c_mid_merge_boxes_and_never_cancels`.
      Shutdown never signals it and waits: `jobs::tests::shutdown_waits_for_an_unstoppable_job_and_never_cancels_it`,
      `an_unstoppable_job_ignores_a_cancel_by_id` (both red with `Live::unstoppable` made a no-op),
      `it::quit_waits::shutdown_mid_merge_lets_it_land` (red with `cancel_all` cancelling everything: HEAD had 2 parents).
      No child outlives shutdown: `jobs::tests::no_git_child_outlives_shutdown` (a real git child's pid checked gone
      with `kill -0`), `the_shutdown_grace_covers_a_git_childs_stop`.

## Question (out of scope, no line verified: rule 7)
`Ctrl+C` during a `Stoppable::No` write. The review says `shutdown` cancels every job (`event_loop/startup.rs:205`,
read, not observed). Does that signal a git child halfway through a rebase? The builder files what the scout finds,
with lines. The conductor decides.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
One context, ~40k tokens. Crates: graft and graft-ui (a two-file touch in graft-ui).

## Conductor decision on the Question (after scout `scouts/13b-quit.md`, read not run)
Scout: `Ctrl+C` is a key in raw mode; `quit.rs:28` quits with no box → `cancel_all` (`runner.rs:478`) → `run.rs:495` SIGTERM, SIGKILL after 2 s (`exec.rs:235`); shutdown waits 200 ms (`startup.rs:192`), so the worker is abandoned. A `Stoppable::No` write (rebase, merge) can be SIGTERMed halfway. That breaks M3 ("you cannot lose work").
**In scope now:**
- `Ctrl+C` during a `Stoppable::No` write behaves like `q` (the same box, the same wait). It never signals that write.
- Shutdown never signals a `Stoppable::No` write. It waits for the write to end.
- For stoppable jobs, shutdown waits for the stop to land (SIGTERM → exit or SIGKILL) and never abandons a child. Name the bound.
- Acceptance: a test that a `Stoppable::No` git child is never sent a signal on `q`/`y` or `Ctrl+C` (red today for `Ctrl+C`). A test that no child outlives shutdown. Before the fix, run in a temp repo what a SIGTERMed rebase leaves on disk and in the journal, and record it in NOTES (evidence for the gap row, not a fix).

## NOTES (builder, 2026-10-02)

**Context compacted once, mid-ticket.** Nothing remains undone from scope; the list below is what a reviewer should know.

How it works (cause, not symptom): `remote::launched` returns `Command::Unstoppable(id)` for a `Stoppable::No` write and
every launch site queues it ahead of the job command. The pool's `Live` keeps an unstoppable set; `cancel`/`cancel_all`
skip it; `JobRunner::shutdown` cancels the rest, then waits for unstoppable jobs **with no bound**, then up to
`SHUTDOWN_GRACE` for the rest. `SHUTDOWN_GRACE` = `graft_git::run::STOP_WITHIN` (`POLL` + `TERM_GRACE`, 2.05 s) + 500 ms
= 2.55 s, up from 200 ms; paid only when a git child is slow to take SIGTERM. Before, a worker still in the 2 s
SIGTERM→SIGKILL wait was abandoned with its child alive.

**What a SIGTERMed rebase leaves** (observed, git 2.54, temp repo, `git rebase main` of 40 commits with a `post-commit`
hook sleeping 0.1 s, SIGTERM at 1.5 s): exit 143; detached HEAD at the 10th replayed commit; the branch ref unchanged;
`.git/rebase-merge/` left (`done` 10 lines, `git-rebase-todo` 30); `AUTO_MERGE`, `ORIG_HEAD` present; no `.lock` files;
`git status`: "interactive rebase in progress". **Journal: read, not observed** — `graft-ops/src/apply/engine.rs:118-125`
gives a `PlanRun` with a rebase still in progress the failure `Stopped`, no cleanup; `restore/outcome.rs:134` comments on
a SIGTERMed step. Recoverable by `--continue`/`--abort`, but the user would land on a stop he never caused.

Signature deviation: `AppState::quit_when_done` is `Option<QuitWait { job, line }>`, not `Option<JobId>` — the
waiting line is kept with the wait so `shown_status` can draw it over progress updates (scope 4). `QuitThen`/`QuitWait`
are `pub` inside the private `quit` module (`private_interfaces` lint), so crate-private in effect.

`q quit` gives way: `stopped_row` (graft-ui `hints/fitting.rs`) drops a trailing `q quit` whenever that lets a fuller
stop wording fit whole. Without it, adding `q quit` cost the conflict count at the 108 cells the changes pane leaves at
187 columns (`stopped_row_keeps_the_count_whole_at_108_cells` went red). This is one named key (`QUIT_YIELDS`), a
deliberate rule, not a general one — flagged for the reviewer.

Knock-ons and open points:
- **Unbounded wait at shutdown** if a `Stoppable::No` write hangs (a hook waiting on input). Shutdown runs before the
  terminal is restored, so Graft would look frozen. No bound by decision (cutting it off is the defect); a gap row with
  a "still waiting for <op>" line is the natural follow-up. Unverified in practice.
- A `Stoppable::No` job still **queued** when shutdown starts is waited for and runs (nothing cancelled it). A stalled
  reader ahead of it on a full pool would delay it. Read, not observed.
- `Ctrl+C` over `Stoppable::No` now boxes: keymap §2.2/§2.6a and the "never boxed" wording are the planner's to update
  (conductor's instruction; DESIGN files untouched).
- At 80 columns the quit box wraps its first line after `quit waits for it` (`m8-quit-waits-80x24.txt`): for the user's
  eye.
- `?` help (§2.12): no `IN PROGRESS`/`HERE` list exists in the code (grep), so nothing to add `q` to.
- `m5-contact-sheet-2` also picked up an unrelated, already-committed capture change (`history.top is 24 → 30`,
  `48 → 54`): the sheet was stale against its source; re-assembly fixed it.
- No m8 contact sheet or captions file exists yet; the three new captures wait for the acceptance ticket's sheet.
- Ticket 13's three banner-quit test names changed (`…reaches_the_quit_box` → `…quits_with_no_box`); ticket 13's
  ticks updated to the new names.

Touches: ticket 03's files (`event_loop/{rest,running,mod}.rs`, `update/{mod,selection}.rs`, `update/detail/*`) not
edited (git status). Ticket 04's `event_loop/graphics.rs` not edited. Edited outside `quit.rs`: `update/state.rs`
(field + `shown_status`), `update/tests.rs` (one initialiser line), `place/write.rs` (the exhaustive arm), the four
launch sites, the three answer paths, `remote/in_flight.rs` and `banner/focus_keys.rs` (`Esc`), `jobs/{pool,runner,mod}.rs`,
`graft-git` `run.rs`/`run/exec.rs` (`STOP_WITHIN`, `TERM_GRACE` to `pub(super)`). Journal/undo: no new entry kind; the
quit waits for the write's own entry (`it::quit_waits` reads it `Committed`).

Scout: `scouts/13b-quit.md` (used).
