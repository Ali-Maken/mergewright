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

## Review fixes (planner, 2026-10-04 — review `docs/reviews/m8-13b-quit.md`, Majors 1–4)

Build with: **Opus** — a cause-level change to the quit and shutdown order. A wrong order breaks his terminal or cuts
git half-way, and neither shows in a diff. Not Fable: the shape is decided below.
Needs: none. Crate set: **graft only** (graft-git's `STOP_WITHIN` and `Stop` are used, not changed).
Brownfield: `scouts/13b-quit.md` covers `quit.rs`, `jobs/*`, `finish/keys.rs`. Not covered: the exit order in
`event_loop/startup.rs` (shutdown `:314` before restore `:397`, per the review) and `event_loop/signals.rs`. The
builder reads those two from MAP first, then the lines the review names. The conductor may send a scout instead.

**Q1 settled 2026-10-04: recommended default** (user: "whatever you think is best"; field-notes). Was pending Q1 (below). Built on its recommended default. If the user picks the alternative, only M1 items 3–5 and
their boxes change (noted). Items 1–2, 6–10 and M2 stand either way.

### Q1 for the user (one question; the conductor asks it in his words, no ticket numbers)
"If a merge or rebase hangs (say a git hook stuck on the network) and you quit anyway, should the app **stop git where
it is**, leaving the merge or rebase paused so the next launch offers continue or abort? Or should it **never touch
git** and leave it running after the app has gone?"
- **Recommended: stop it.** Offered only after 10 s of waiting, behind a box that names the cost. Why: the paused
  state is recoverable, and it was measured (NOTES above: `.git/rebase-merge/` kept, no `.lock` files, `--continue`
  and `--abort` both work). Leaving git running gives no way back into the app if the hook never ends. Git may also
  die anyway at its next output line once the app's end of its pipes closes. That last point is **unverified**:
  graft-git MAP `run` says only that stdout and stderr are drained on the app's threads.
- Alternative (never stop it): no stop box; item 3 becomes a line saying how to leave anyway (`q` then closes the
  app and leaves git running). Item 8's bound expiry exits without signalling. The exit line reads
  `<bin>: git is still running <op> in the background`.

### M1: a way out of a hung write, and a signal path that always gives the terminal back
Read first (in addition): review Majors 1–2; keymap §2.2 "`q` while a write runs" (after the doc edits below);
graft MAP `event_loop::signals`, `Exit`, `Quit`, `jobs` (`JobRunner::shutdown`, `unstoppable`, `SHUTDOWN_GRACE`),
`rest::Rest::deadline`, `Wakes::next_wake_by`, `Wakes::now`, `TerminalSession::restore`; graft-git MAP `run`
(`STOP_WITHIN`, `Stop`).

In the app:
1. For the first 10 s the wait stays as built (`STOP_OFFER_AFTER`). In that time, `q` or `Ctrl+C` while waiting does
   nothing: the status row already says it.
2. At 10 s the status row reads exactly `<op> · still running after 10 s · q stop it now · esc stays`. The text is
   fixed, with no ticking count, so nothing wakes after this. **The loop owns the clock**: while a wait stands, it
   stamps `since` the first turn it sees the wait (`Wakes::now`), adds `since + STOP_OFFER_AFTER` to the deadlines
   it already waits on (`Pacer`, `Rest`, through `Wakes::next_wake_by`), and at that instant calls
   `quit::waited_long`. `update` stays pure: no thread, no poll.
3. After that, `q` or `Ctrl+C` opens the stop box. Title `Quit now?`. Lines `<op> is still running · a git hook may
   be stuck` and `quit now stops it half-way · the next launch shows where it stopped`. Buttons `[ Cancel ] [ Quit ]`,
   cursor on Cancel, `y` quits, and the bar leads `y quit` (`ConfirmYes::Quit`, so `COVERAGE_SLOTS` does not change).
   Cancel or `Esc` closes the box, and the wait stands with its long line. A further `Esc` drops the wait (the
   existing chain).
4. `y` → `Command::ForceStop(job)`, then `Quit::StoppedHalfWay`. `JobRunner::force_stop` lifts the job's
   unstoppable mark and cancels it, using the existing `run` stop: SIGTERM, SIGKILL after 2 s, reaped, all within
   `SHUTDOWN_GRACE`. **Nothing else lifts the mark.**
5. After the terminal is restored, stderr gets one line:
   `<bin>: stopped <op> half-way after <n> s · <bin> here shows where it stopped`. `<bin>` is the binary's own name
   (the man page's). `<n>` is whole seconds since `since`.
6. Where he lands on relaunch is what recovery shows today: the banner with `c continue · a abort` for a merge, rebase,
   cherry-pick or revert left in progress, and the edits set aside through `JournalRead::interrupted_edits`. The
   builder **observes** both on a real repo. If the relaunch does not offer the set-aside edits, that is lost work
   (M3): **stop and report it with lines**. Do not patch it here.

Signal path (SIGTERM, SIGHUP, a SIGINT; `Exit::Signal`):
7. On **every** exit, the terminal is restored **before** any wait that can last longer than `SHUTDOWN_GRACE`.
   Anything that needs the terminal before the restore (Kitty frees, per `pump`) stays before it; the job wait moves
   after it. The NOTES name the lines that moved.
8. If a `Stoppable::No` write is running when a signal arrives:
   - Restore the terminal.
   - Print to stderr: `<bin>: waiting for <op> to finish (up to 60 s) · send the signal again to stop it half-way`.
     After a hang-up nobody can read stderr, so a write error there is ignored, with a comment saying why that is safe.
   - Wait up to `EXIT_WRITE_WAIT` = 60 s.
   - The write ends → re-raise the signal, as today.
   - The bound is reached, or a second watched signal arrives → `force_stop`, wait `SHUTDOWN_GRACE`, print item 5's
     line, re-raise.

   Why 60 s: a merge or rebase with ordinary hooks lands well inside it, and a hung hook does not leave an invisible
   process for good. Why the second signal: that is how a person asks for "now", as with a second `Ctrl+C`.
9. `JobRunner::shutdown` takes the bound and the force flag, so **no path waits without a bound**. That includes an
   unstoppable job still **queued** behind stalled workers (review Major 1, bullet 3).
10. Observe this, don't fix it: does a terminal hang-up's SIGHUP reach the git child directly, because it shares the
    app's process group?
    - Method: the real binary in a pty, a merge with a `pre-merge-commit` hook running `sleep 600`, then close the
      pty.
    - Record whether git or the hook got the SIGHUP, and the spawn line in `graft-git` `run/exec.rs` that sets the
      process group, or doesn't.
    - Put the finding in NOTES with lines. Changing how git is spawned is **out of scope**: a background process group
      that reads the tty stops on SIGTTIN, which would be a new hang.

Signatures (the `quit` module's internals may be renamed; MAP records the final shape):
```rust
// graft, update/quit.rs
pub(crate) const STOP_OFFER_AFTER: Duration = Duration::from_secs(10);
pub struct QuitWait { /* existing: job, line */ pub since: Option<Instant>, pub long: bool }
pub(crate) fn stamp_wait(state: &mut AppState, now: Instant);   // the loop, first turn it sees the wait
pub(crate) fn waited_long(state: &mut AppState);                // the loop, at since + STOP_OFFER_AFTER
pub enum QuitThen { Now, Stop(JobId), WaitFor(JobId), StopNow(JobId) } // StopNow: the stop box's [ Quit ]

// graft, update (Command)
Command::ForceStop(JobId)

// graft, update::Quit
pub enum Quit { Requested, HandedBack(KeyCode), Fatal(String), StoppedHalfWay { what: String, after: Duration } }

// graft, jobs
pub const EXIT_WRITE_WAIT: Duration = Duration::from_secs(60);
impl JobRunner {
    /// The only path that signals a Stoppable::No job: the stop box's `y`, or the signal path's bound / second signal.
    pub fn force_stop(&self, id: JobId);
    pub fn shutdown(self, grace: Duration, writes: Duration, force: &dyn graft_git::run::Stop) -> Shutdown;
}
// Shutdown says which unstoppable jobs it force-stopped, so the exit line can name them.
```

Touches: `update/quit.rs`; `update/state.rs` (`shown_status`); `event_loop/{running,startup,signals}.rs`; `jobs/{pool,runner,mod}.rs`; `update` `Command` (each exhaustive match); `Quit` (each match: `Quit::should_quit`, the exit report); the `?` help (it lists the long-wait line's `q` if it lists status keys); captures that draw the waiting row (`m8-quit-waits-80x24.txt`: its first 10 s are unchanged, so it should still match — confirm it); `TESTING.md` quit rows; journal/undo (no new entry kind; a forced stop leaves the write's entry `Pending`, which recovery reads as live or interrupted).

- [x] Before 10 s, `q` and `Ctrl+C` while waiting change nothing: no box, same line. Virtual timeline.
      — `update::quit::tests::stuck::before_ten_seconds_q_and_ctrl_c_change_nothing`; on the loop's virtual
      timeline, the `q` at 2 s in `event_loop::tests::quit_offer::the_loop_offers_the_stop_at_ten_seconds_and_y_quits_with_the_seconds_waited`.
- [x] At 10 s, on a virtual timeline (own `Clocked` wakes, not `settle.rs` `Timed`: it records deadlines and draw
      times), the status row is exactly the long line — `update::quit::tests::stuck::at_ten_seconds_the_row_offers_the_stop_and_nothing_is_due_after`,
      `event_loop::tests::quit_offer::the_offer_line_is_drawn_from_the_clock_alone`. The loop waits toward
      `since + 10 s` and draws a frame there with no key, and no deadline after it waits toward a later offer —
      `the_loop_offers_the_stop_at_ten_seconds_and_y_quits_with_the_seconds_waited`, `running::quit_clock::tests::*`.
      **Not asserted:** that no *other* deadline wakes the loop earlier (the test checks the offer deadline is
      present, not that it is the only one).
- [x] After that, `q` and `Ctrl+C` each open the stop box with the exact words above. Cancel or `Esc` keeps the wait
      and its long line. The next `Esc` drops it. — `stuck::after_ten_seconds_q_and_ctrl_c_open_the_stop_box`,
      `stuck::cancel_or_esc_keeps_the_long_wait_and_the_next_esc_drops_it`, `stuck::ctrl_c_over_the_stop_box_does_not_quit`.
- [x] `y` → `ForceStop(id)` emitted exactly once, quit `StoppedHalfWay`. No other path emits `ForceStop`, and
      `cancel` and `cancel_all` still skip a marked job (the existing `an_unstoppable_job_ignores_a_cancel_by_id`
      still holds). — `stuck::y_on_the_stop_box_force_stops_once_and_quits_stopped_half_way`,
      `stuck::no_other_path_emits_force_stop`, `jobs::tests::an_unstoppable_job_ignores_a_cancel_by_id`,
      `jobs::tests::force_stop_lifts_the_mark_and_stops_the_write`.
- [ ] **NOT MET — the merge half (stop and ask, see NOTES "Review fix, 2026-10-04").** Real pool, real repo, a merge with a `pre-merge-commit` hook `sleep 600`, `STOP_OFFER_AFTER` injected short for
      the test: `q y`, wait, `q y`. Then the git child is gone (`kill -0`), the app quits within `SHUTDOWN_GRACE`,
      `.git/MERGE_HEAD` is present, there is no `*.lock` under `.git`, and the journal entry is `Pending`. Relaunch →
      the banner offers `c continue · a abort`.
      What holds: git gone, pool clean within `SHUTDOWN_GRACE`, no `*.lock`, one merge entry —
      `it::quit_waits::stuck::stop_now_mid_merge_stops_git_and_leaves_no_lock`. What does not: no `MERGE_HEAD`,
      entry `Aborted`, no relaunch banner (NOTES).
- [x] The same walk over a rebase with edits set aside: the relaunch offers the edits back. —
      `it::quit_waits::stuck::stop_now_mid_rebase_with_edits_set_aside` (rebase dir kept, no lock, entry `Pending`
      with `put_back`, banner `rebase still in progress from a previous run · edits aside · c continue s skip a abort`).
- [x] Exit line exactly item 5's words, on stderr, **after** the restore sequence in the terminal transcript. —
      `it::quit_waits::signal::stop_now_after_the_real_offer_restores_then_says_what_it_stopped` (real binary, real
      10 s, exit 0). The words are true for the rebase, **false for the merge** (NOTES).
- [x] Signal path: a SIGTERM with a hung unstoppable write leaves the transcript in this order: restore, then the
      waiting line. A second SIGTERM → git child gone, item 5's line, the signal re-raised (exit status 15). —
      `it::quit_waits::signal::a_signal_restores_then_waits_and_a_second_stops_git`. **Bound expiry** is covered at
      the pool only (`jobs::tests::shutdown_stops_a_write_still_running_at_its_bound_and_names_it`): `EXIT_WRITE_WAIT`
      is a const, so the binary cannot be given a short bound; not walked through the binary.
- [x] No unbounded wait is left. — `jobs::tests::shutdown_waits_up_to_the_bound_and_never_cancels_before_it` (the
      rewrite), `a_forced_shutdown_stops_the_write_without_waiting_out_the_bound`,
      `a_queued_write_behind_a_stuck_worker_is_bounded_and_never_runs`; each joins on a thread with `recv_timeout`.
- [x] Item 10's SIGHUP observation is in NOTES, with the spawn line (`graft-git/src/run.rs:413`). — observation,
      scratch script, no test.
- [x] Capture `docs/shots/m8-quit-stop-now-80x24.txt`: the long status row, then the stop box over it. Colour
      **unverified**. Injected offer stated in its caption. — `it::quit_waits::capture_quit_stop_now`. Looks-right is
      the user's.
- [x] Journey count: `quit-while-write 2` and `quit-stuck-write 4`, walked and counted against the note's fence —
      `it::quit_waits::stuck::quit_while_write_is_two_keys`, `stuck::stop_now_mid_rebase_with_edits_set_aside` and
      `stuck::stop_now_mid_merge_stops_git_and_leaves_no_lock` (via `within_the_note`).
- [x] (review: `crates/graft/MAP.md`, read by hand) MAP rows: `JobRunner::shutdown`, `force_stop`, `EXIT_WRITE_WAIT`, `SHUTDOWN_GRACE` (still the stoppable grace),
      `Quit`, `QuitWait`, `update::quit`. Also `confirm_key` (review Major 3): "`Ctrl+C` passes through (`false`, so
      the app quits)" becomes "passes through to `quit::asked`: quits, or over a `Stoppable::No` write boxes like
      `q`".

### M2: `Esc` on the finish summary drops the wait (a bug, cause-level)
- [x] `Esc` drops the wait over the finish summary; with no wait it stays swallowed; `q` unchanged. **Deviation:**
      done in `update/keys.rs` `key_event`, right after the quit box and before every modal pane, not in `finish_key`
      — the resolver and rewrite editor also claimed `Esc` first (their tests were red too). `finish/keys.rs` is back
      to `return true`. — `update::quit::tests::modals::esc_drops_the_wait_over_the_finish_summary`.
- [x] One test per modal: `esc_drops_the_wait_over_{the_finish_summary,the_resolver,the_rewrite_editor}` in
      `update::quit::tests::modals`. All three were red before the fix (observed).
- [x] The same three modals, after 10 s: `Ctrl+C` opens the stop box. —
      `modals::after_the_offer_ctrl_c_opens_the_stop_box_over_each_modal`.

### Doc edits (apply verbatim; the planner had no edit tool on this pass)
Review Major 3 (keymap) and Major 4 (design note). The builder applies these. They are design text, not the
builder's wording.
- `docs/design/02-keymap.md:199`: replace `` `Ctrl+C` quits at once, never boxed. `` with
  `` `Ctrl+C` quits at once with no box, except over a write that can't stop, where it boxes exactly like `q` and never signals that write (**Proposed**, M8/13b conductor decision 2026-10-02; not yet seen by the user). ``
- `docs/design/02-keymap.md:233`: replace the bullet `` - `Ctrl+C` is not changed by this. `` with three bullets:
  - `` - `Ctrl+C` over a write that can't stop does what `q` does: the same box, the same wait. Over the quit box itself it does not quit (**Proposed**, M8/13b, conductor 2026-10-02). ``
  - `` - **A stuck write** (**Proposed**, M8/13b review fix; stopping git is pending the user's answer): after 10 s of waiting the status row reads `<op> · still running after 10 s · q stop it now · esc stays`. `q` or `Ctrl+C` then opens `Quit now?`, with the lines `<op> is still running · a git hook may be stuck` and `quit now stops it half-way · the next launch shows where it stopped`, and `[ Cancel ] [ Quit ]`; `y` quits. Before 10 s, `q` and `Ctrl+C` while waiting do nothing. Why 10 s: a merge or rebase with ordinary hooks ends inside it, and an earlier offer invites stopping a write that was about to land. The shell then reads `<bin>: stopped <op> half-way after <n> s · <bin> here shows where it stopped`. ``
  - `` - **Quit from outside** (SIGTERM, SIGHUP; **Proposed**, M8/13b): the terminal is given back first. A write that can't stop gets up to 60 s to land, then it is stopped half-way; the same signal again stops it at once. The shell gets one line saying what was stopped. ``
- `docs/design/02-keymap.md:289`: replace `` `Ctrl+C` passes through (quits, §2.1) `` with
  `` `Ctrl+C` passes through to §2.2's quit (over a write that can't stop it boxes like `q`; over the quit box itself it does not quit — **Proposed**, M8/13b) ``
- `docs/design-notes/m8-polish-and-ship.md`, Prior art table, a new row after `:71`:
  `` | **Quit while a write runs; a write that hangs** | **no study**: nothing in `docs/research/` covers how lazygit, VS Code Git, Sublime Merge, JetBrains or gitui quit mid-merge or mid-rebase (grep `quit`, 2026-10-04; review m8-13b Major 4) | wait for it, then quit; after 10 s `q` offers to stop it half-way; a signal gives the terminal back first, then waits ≤ 60 s (keymap §2.2) | **I**. Nothing fits, because no source was read. The shape argues from ux-principles §3.1 (where he lands), heuristic 4 (one box, one `y`) and the measured half-rebase (13b NOTES). **A researcher pass is owed**: `docs/research/quit-during-operation.md` (lazygit, VS Code Git, Sublime Merge, JetBrains, gitui) | ``
- Same note, Journeys table, after row 23:
  `` | 24 | Quit while a merge or rebase runs | `q` `y`, then nothing: it quits when the write lands | 2+1 | no study | ``
  `` | 25 | Quit while a git hook hangs | `q` `y`, wait 10 s, `q` `y` | 4+2 | no study | ``
- Same note, "Where he lands" table, after the `q`-while-a-write-runs row:
  `` | Stuck write, `q` `y` after 10 s | shell, with `<bin>: stopped <op> half-way after <n> s · <bin> here shows where it stopped`; next launch: banner `c continue · a abort` (merge, rebase, cherry-pick, revert), set-aside edits offered (13b review fix) | ``
  `` | SIGTERM / SIGHUP while a write runs | terminal given back at once; the write lands within 60 s → exits; else it is stopped half-way, with the same line; a second signal stops it at once | ``
- Same note, `journey-counts` fence, after `undo-see-result`:
  ```
  quit-while-write              2     # +1 · no study · q y (quits when the write lands)
  quit-stuck-write              4     # +2 · no study · q y … 10 s … q y
  ```

**For the conductor, not the builder:**
- Correct the wording of gap row `M8/13b-unbounded-wait` now. It says "a hung git leaves the app open until Esc", but
  `Esc` does not get him out.
- Add a gap row for the keymap's Proposed `Ctrl+C` change until he sees it.
- Dispatch the researcher pass named in the Prior art row.
- If `accept-m8` is already built, it owes the two new fence ids a walk.

Gates: as above, workspace-wide. Size: one context, ~55k tokens; graft only.

## NOTES — Review fix (builder, 2026-10-04..07) — status **partial**

**Compacted twice** mid-ticket. Scout used: `.scratch/m8-polish-and-ship/scouts/13b-review-fix.md` (written by me).
Gates: fmt, clippy `--all-targets -D warnings`, nextest 5552/5552, `test --doc` — workspace-wide, all green.

**Stop and ask — the merge premise is false (Q1's default).** After `q y`, the offer, `q y` with a `pre-merge-commit`
hook hung, observed (`it::quit_waits::stuck::stop_now_mid_merge_stops_git_and_leaves_no_lock` prints it): no
`MERGE_HEAD`; `main-one.txt`/`main-two.txt` staged (`A`); HEAD still on `feature`; entry
`Aborted { reason: "git was stopped" }` (graft-ops `src/apply.rs:459` `outcome_status`); no relaunch banner. So
"the merge is left paused, continue/abort offered" does not hold, and the exit line's "here shows where it stopped"
is false for a merge. `took_effect` (`graft-ops/src/journal.rs:165`) may still let `u` undo an Aborted entry with
changes — **not verified**. Fixing it means graft-ops (recording a stopped merge as pending / restoring), outside this
crate set. The rebase half holds (box ticked).

**Hook outlives git's stop.** `ForceStop` SIGTERMs git's pid only; the hook's `sleep 600` survives (tests reap it by
pid). Same root as item 10.

**Item 10 (SIGHUP), observation only.** graft, git and hook share one pgid: git is spawned with no process group,
`graft-git/src/run.rs:413` `Command::new("git")`. macOS, master pty closed: HUP or EIO reaches graft, git stays alive;
no waiting line seen — suspected the input-EIO path (not "signalled") waiting silently up to 60 s (unverified). With
stderr on the dead pty, graft exited 101 — suspected panic from ratatui-core-0.1.2 `src/terminal.rs:483` `eprintln!`
in `Terminal::drop` (unverified) — and git and the hook then died. Scratch script, no test.

**Lines that moved (item 7):** pool shutdown and watcher close left `run_repo` (old `startup.rs` ~:333/:338) for
`wind_down`, after `session.restore()` (`event_loop/exit_wait.rs` `Pool::stop`).

**Deviations from the signatures:** clock lives in the loop (`event_loop/running/quit_clock.rs` `QuitClock`) because
`update_cannot_reach_the_world` forbids `std::time` in update — so no `QuitWait::since`, no `stamp_wait`;
`offer_pending` instead. `Quit::StoppedHalfWay { what }` (the loop adds `after`). `Command::Unstoppable(JobId,
String)`. `Shutdown` is a struct `{ stalled, forced }`. `quit` is `pub mod`, `waited_long` `pub` (the it harness plays
the clock). `quit_when_done` is `Option<Box<QuitWait>>`: the `long` field pushed `AppState` past clippy's
`large_stack_arrays` in three existing tests; boxing keeps its size. M2's `Esc` fix sits in `keys::key_event`, not
`finish_key` (all three modals claimed `Esc`). `<bin>` = `CARGO_PKG_NAME`.

**Touches:** `update/{quit,keys,state,mod}.rs`, `finish/keys.rs` (reverted to `return true`); `event_loop/{running,
turn,startup,signals,exit_wait}.rs`; `jobs/{pool,runner,mod}.rs`; `main.rs` (exit line) and examples (exhaustive
`Exit`/`Command` matches — compiler-checked); capture `m8-quit-waits-80x24.txt` still matches (`capture_quit_waits`
green), new `m8-quit-stop-now-80x24.txt`; keymap doc :199/:233/:289 applied verbatim; design note Prior art row now
cites `docs/research/quit-while-git-runs.md` (not "no study"), Journeys 24/25, two "Where he lands" rows (stuck-write
row carries the merge caveat), fence `quit-while-write 2`, `quit-stuck-write 4`; `MAP.md` rows; `TESTING.md` has no
quit-wait rows (grep `quit`) — none added; journal: no new entry kind; `?` help draws no status keys (unchanged).
