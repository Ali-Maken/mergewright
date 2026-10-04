# Review: M8 ticket 13b, quit waits for a write

Diff reviewed: `c2f6a708^1..c2f6a708` ("merge: m8 13b quit waits for a write"). Code read at main HEAD `cc542e1f`. The 88
commits after the merge do not touch `update/quit.rs`, `jobs/pool.rs` or `jobs/runner.rs`. I checked this with
`git diff --stat c2f6a708..HEAD`.
Ticket: `.scratch/m8-polish-and-ship/issues/13b-quit-waits-for-a-write.md`. Scout: `scouts/13b-quit.md`. Keymap §2.2
(`:199`, `:214-233`) and §2.6a (`:370`).

**0 Critical · 4 Major · 7 Minor.**

## Gates (own detached worktree at `cc542e1f`, own `CARGO_TARGET_DIR`, one command at a time)

| Gate | Result |
|---|---|
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass |
| `cargo clippy --all-targets --features heavy-tests -- -D warnings` | pass |
| `cargo nextest run --workspace --no-fail-fast` | **5424/5424 pass**, 10 skipped, 2 slow |
| `cargo test --doc --workspace` | pass |
| Heavy tier, release perf run, fuzz, bench | **skipped on the conductor's instruction** ("No perf measurements") |

- Fast suite: 1174 s (nextest), 1324 s wall including the build. A builder was running cargo on the same machine at the
  same time (1124 s system CPU), so these times are not a baseline. Peak RSS was 1.13 GB. `target/` was 5.6 GB.
- Slowest tests: nextest flagged 2 as slow but did not print their names (`status-level = "fail"`). While the run was
  going, `ps` showed the graft-ui hints sweeps running for 30–50 s each (`the_bar_never_draws_a_fragment_of_itself`,
  `the_bare_keys_are_a_tail_not_a_scatter`, `the_three_most_relevant_never_fall_below_their_floor_rung`). I did not
  record the rest of the top 5.
- Every capture under `docs/shots/` is checked on every fast run, so this green run also shows the re-shot captures
  are not stale.

## Spot-checked ticks: 10 checked, 10 held

I read each named test against the claim in its tick.

1. Box words: `quit::tests::q_during_a_merge_says_quit_waits_for_it` asserts the title, both lines exactly, the
   `Cancel`/`Quit` labels, cursor 0 and `ConfirmYes::Quit`. Held.
2. `y` waits: `y_over_a_merge_waits_and_cancels_nothing` checks there is no `CancelJob`, the box is closed and the
   merge still runs, and that `shown_status` shows the waiting line. Held.
3. Done quits, and the journal is on disk: `it::quit_waits::quit_mid_merge_waits_for_git_then_quits` reads
   `undo-journal.jsonl` after `app.close()` and finds one `Merge` entry, `Committed`, plus a 2-parent HEAD. Held.
4. Conflict stays: `the_merge_stopping_at_a_conflict_stays_in_the_resolver` asserts there is no quit, the wait is
   gone, and `OpenConflict(a.txt)` was emitted. A later write does not quit. Held, with one gap: it checks the open
   command, not an open resolver.
5. `Esc` drops the wait on the graph and on the banner: `esc_while_waiting_drops_the_wait`,
   `esc_dismisses_a_held_failure_before_the_wait` and `esc_on_the_banner_drops_the_wait`. Held for the screens the
   tick names. Major 2 covers a screen it does not name.
6. Progress keeps the words: `progress_while_waiting_keeps_the_waiting_words`. Held, but weakly; see Minor 5.
7. Banner bar: `graft-ui hints::tests::q_quit_is_the_banner_bars_last_hint_and_first_dropped` sweeps widths 20–187
   over 4 shapes, and `q_quit_gives_way_to_the_stop_reason` covers the reason. Held.
8. User's ruling: `undo::tests::q_on_the_banner_after_an_undo_puts_a_merge_back_quits_with_no_box`, plus
   `it::conflict_interim::q_at_a_stop_quits_and_the_relaunch_offers_continue` (a real repo, quit, then relaunch,
   with `c continue`/`a abort` on the stop line). Held.
9. Conductor's `Ctrl+C` decision: `ctrl_c_during_a_merge_boxes_like_q_and_signals_nothing`,
   `ctrl_c_over_the_waiting_box_does_not_quit`, `it::quit_waits::ctrl_c_mid_merge_boxes_and_never_cancels`,
   `jobs::tests::{an_unstoppable_job_ignores_a_cancel_by_id, shutdown_waits_for_an_unstoppable_job_and_never_cancels_it,
   no_git_child_outlives_shutdown}`. Held. I did not re-check the "red before the fix" claims.
10. Captures: `it::quit_waits::capture_quit_waits` and `conflict_interim::capture_banner_bar_with_q` exist, and
    `docs/shots/m8-quit-waits-80x24.txt` and `m8-banner-bar-q-{80x24,187x50}.txt` match their "what right looks like"
    lines. Held. One caption is stale; see Minor 3.

## Answers to the conductor's focus questions

- **Is the unbounded wait safe?** For data, yes. No path signals a `Stoppable::No` git child. `Live::cancel` and
  `cancel_all` skip the job (`jobs/pool.rs:72-91`), and shutdown waits for it (`runner.rs:499`). For the user, no; see
  Major 1. A hung write now leaves **no way to quit** from inside the app, and an external SIGTERM or SIGHUP also
  blocks.
- **Does `Esc` always get out?** On the graph and the focused banner, yes. On the finish summary while its commit
  runs, no; see Major 2.
- **`Ctrl+C` over a `Stoppable::No` write** boxes and never signals the write (`quit.rs:62-73`). Every modal passes
  `Ctrl+C` down to `keys::graph_key` → `quit::asked`: `resolver.rs:323`, `finish/keys.rs:19`,
  `commit_editor/keys.rs:17`, `rewrite_editor/keys.rs:43`, `banner/focus_keys.rs:102`, `picker.rs:104`,
  `stash.rs:159` and `confirm.rs:170` all return "not mine". Nothing else sets `Quit::Requested` (grep). The keymap
  was not updated; see Major 3.
- **`q` at a stop quits with no box.** Done (`quit.rs:58-81`; `stopped_box` removed), with a real-repo test.
- **`q quit` dropped first at 80 columns.** Test-checked across every width. Whether `q quit` is the right hint to
  drop is the user's call (gap row `M8/13b-80col-quit-hint` stands). At 80 the hint goes because the stop reason
  `merge stopped` claims the room (`m8-banner-bar-q-80x24.txt`), not because the bar itself overflows.
- **Journal and undo.** No new entry kind. The quit happens only after the answer path has applied the outcome
  (`write_ended` is called last in `operate/outcome.rs:323-328`, `banner/resume.rs:149-159` and `undo.rs:592-596`).
  The worker calls `live.finish` before it delivers the event (`pool.rs:190-193`), so a done write is never waited on
  at shutdown.

## Critical

None.

## Major

**1. A write that hangs can no longer be quit at all, and the gap row says otherwise.**
- Inside the app: `q` and `Ctrl+C` both reach `quit::asked`. Over a `Stoppable::No` write that opens the wait box
  (`quit.rs:62-73`, `:180-186`). `y` sets the wait. `Esc` drops it (`remote/in_flight.rs:41-43`), but the write keeps
  running, and the next `q` or `Ctrl+C` opens the same box again. No key ends Graft while that write runs.
- Outside the app: SIGTERM and SIGHUP become a clean quit (`event_loop/signals.rs:47`). That quit calls
  `jobs.shutdown` (`startup.rs:314`) **before** the terminal is restored (`startup.rs:397`), and shutdown now waits
  without a bound (`runner.rs:499`, `pool.rs:96-104`). Closing the window leaves a Graft process behind. `kill` hangs.
  Only `kill -9` works, and that leaves the terminal in raw mode, which is the exact failure `signals.rs:1-5` was
  written to prevent.
- The same unbounded wait also covers an unstoppable job that is still **queued** behind workers stuck in jobs that
  ignore their token (`pool.rs:92-95` says it is waited for). For that case the `Stalled` bound (`SHUTDOWN_GRACE`, 2.55 s) becomes no bound
  at all.
- Before 13b, `Ctrl+C` stopped such a write (SIGTERM) and exited.
- Can a write really hang? Merge, `--continue`, checkout and rebase run git hooks: pre-merge-commit, commit-msg,
  post-checkout, post-merge. Only `commit.rs:60` passes `--no-verify`. A git-lfs `post-checkout`/`post-merge` on a
  stalled network is a plausible daily case for someone working in a team.
- Gap row `M8/13b-unbounded-wait` reads "a hung git leaves the app open until Esc". That is wrong: `Esc` does not
  get him out, and nothing else does either.
- Why it matters: the trade (never cut git off half-way) is right, but with no way out, a hung hook means `kill -9`
  and a broken shell. That is worse than the recoverable half-rebase the ticket's NOTES measured.
- Suggested fix (for the conductor to decide; this is a cause-level design choice, not a builder patch):
  - While waiting, show the elapsed time and, after N seconds, `still waiting for git · ⌃C again stops it half-way`.
    A second `Ctrl+C` gets a box that names the cost (`quit stops <op> half-way · next launch offers continue or
    abort`).
  - Give shutdown a long bound for signals (SIGHUP/SIGTERM), so the terminal is always restored.
  - Correct the gap row's wording now, whatever is chosen.

**2. On the finish summary, `Esc` does not drop the wait, and `q` does nothing.**
- `finish_key` swallows every key while `finish.committing` is set (`update/finish/keys.rs:23-25`). It only lets
  `Ctrl+C` through (`:19-21`).
- So: `Ctrl+C` on the summary opens the wait box, `y` sets the wait, and `Esc` changes nothing. I confirmed this with
  a throwaway probe in my worktree, built on `finish::tests::committing()`. After `Ctrl+C`, `confirm` is set. After
  `y`, `quit_when_done` is `Some`. After `Esc`, `quit_when_done` is **still `Some`**.
- Scope item 3 ("`Esc` while waiting drops the wait") and keymap `:228` are unconditional. The ticks only test the
  graph and the banner.
- Why it matters: this is the screen where the `--continue` commit runs, so its hooks run here. Combined with Major 1,
  this is where the trap is tightest.
- Fix: in `finish_key`'s committing branch, let `Esc` call `quit::drop_wait` first, as `hand_the_keyboard_back` does
  (`banner/focus_keys.rs:58`). Add a test per modal that can be on screen while a `Stoppable::No` write runs:
  finish summary, resolver, rewrite editor.

**3. The keymap still says `Ctrl+C` never boxes, and no gap row tracks the change.**
- `docs/design/02-keymap.md:199` says "`Ctrl+C` quits at once, never boxed". `:233` says "`Ctrl+C` is not changed
  by this". `:289` (confirm box) says "`Ctrl+C` passes through (quits, §2.1)".
- The code now boxes `Ctrl+C` over a `Stoppable::No` write (`quit.rs:62-73`) and does not quit through an open wait
  box (`ctrl_c_over_the_waiting_box_does_not_quit`).
- NOTES say the doc update is "the planner's", but `KNOWN_GAPS.md` has no row for it (grep `Ctrl+C`: only rows 219
  and 720, both unrelated), and no ticket exists.
- The MAP rows are stale too: `crates/graft/MAP.md` `confirm_key` says "`Ctrl+C` passes through (`false`, so the app
  quits)".
- Why it matters: CLAUDE.md says shared namespaces, keys included, are decided in the design doc. The user has not
  been told that `Ctrl+C` changed. It is the key he reaches for when stuck, which is exactly Major 1's case.
- Fix: amend keymap `:199`, `:233` and `:289` (marked Proposed, so the user sees it at the stop), and update the MAP
  row. Until then, add a gap row.

**4. Pipeline: "quit while a write runs" has no prior-art row and no journey count.**
- The design note's Prior art table (`docs/design-notes/m8-polish-and-ship.md:32-82`) has no row for quit-with-work-
  running.
- The `journey-counts` fence (`:140-162`) has no id for it, though `q y` is 2 keys.
- Nothing in `docs/research/` covers how lazygit, VS Code, Sublime Merge or JetBrains handle quitting during a running
  rebase or merge (grep `quit` across `docs/research/*.md`: none on this).
- The keymap paragraph argues from `ux-principles` §3.1 and heuristic 4, not from tools.
- Why it matters: CLAUDE.md says a feature with no prior-art study is Major. Major 1 (what do the best tools do when
  git hangs on quit?) is exactly what that study would have surfaced.
- Fix: a short `researcher` pass on quit-during-operation, covering lazygit, VS Code Git, Sublime Merge, JetBrains
  and gitui. Add the journey id `quit-while-write 2`, and a walk that counts it. This is the note's gap, not the
  builder's.

## Minor

**1. A conflict that lands while the box is up, then `y`, quits, though the box said he would stay.**
- In the `QuitThen::WaitFor(_)` race branch, `quit.rs:110-113` quits when `in_progress.is_none()`. But `in_progress`
  is set only by the later working-tree read (`update/mod.rs:810`), while the conflict answer sets `state.stopped`
  at once (`operate/outcome.rs:197`).
- Probe (throwaway, in my worktree): `merging()`, `q`, then a `Conflicted` answer, gives `confirm` up,
  `stopped=true`, `in_progress=false`. Then `y` gives `quit = Some(Requested)`.
- The window is between the answer and the read. No data is lost (a stop quits with no box by ruling), but the box's
  second line is not kept.
- Fix: test `state.stopped.is_some() || state.in_progress.is_some()`, and add a test.

**2. Stale comments and MAP rows saying "`q` boxes at a stop".**
- `graft-ui/.../hints/lists.rs:419-420`: "`q` boxes from here".
- `update/banner/focus_keys.rs:89-90`: "boxes first while a write runs or git is stopped".
- `update/quit/tests.rs:1-2`: "asks first while a write runs or git is stopped mid-operation".
- `crates/graft/MAP.md` `ConfirmThen` row: "`[ Quit ]` stops that write if still running, then quits". It does not
  mention `WaitFor`.
- Fix: reword all four.

**3. The 80-column capture claim is wrong.** `tests/it/quit_waits.rs:162` and ticket NOTES ("At 80 columns the quit box
wraps its first line after `quit waits for it`") say it wraps. `docs/shots/m8-quit-waits-80x24.txt:9` shows the whole
line on one row (`… · quit waits for it to finish │`). The assertion still passes because it is a substring. Fix: drop
the comment and the NOTES line, or assert the real layout.

**4. `merging_over` is misnamed.** `quit/tests.rs:551-568` says "a second merge launched over it", but it launches
`Operation::Fetch` through `launch_unstoppable`, a state the app never produces. Fix: launch a real `Merge`, or rename
it and say why a fetch is enough.

**5. The progress test can pass when nothing happened.**
- The first event in `progress_while_waiting_keeps_the_waiting_words` (`Progress::Operating`) is a no-op whenever the
  status is `Busy` (`update/mod.rs:721-722`), and it is `Busy` throughout the merge.
- The test only asserts `shown_status`, so it does not show that a progress line reached `status` underneath.
- Fix: assert that `next.status` changed (or use an event that does change it), then assert `shown_status` still
  shows the waiting line.

**6. The new pool tests spin a CPU core.**
- `watching_job` and the trigger job in `jobs/tests.rs` loop on `thread::yield_now()` for up to `PATIENCE` (5 s).
- If `wait_for_unstoppable` ever regressed, `shutdown_waits_for_an_unstoppable_job_and_never_cancels_it` would hang:
  `shutdown` has no bound, and nextest has no `slow-timeout` (`.config/nextest.toml`).
- Fix: block on `recv_timeout` instead of spinning, and run the shutdown on a thread joined with a timeout.

**7. `SHUTDOWN_GRACE` grew from 200 ms to 2.55 s.**
- It is paid only when a git child is slow to take SIGTERM. For a cancelled fetch stuck on a dead network, though,
  that is every quit.
- This is the price of never leaving a git child behind (the cause fix the conductor asked for).
- Not measured. Flagged so the user knows a `q y` over a hung fetch now takes up to about 2.5 s before the shell
  comes back.

## Not findings (checked)

- No `unwrap` or `expect` outside tests in the added code. The pool's locks use
  `unwrap_or_else(PoisonError::into_inner)`.
- The box text is sanitised: `quit_box` runs every line through `sanitize_line`, including the write's description.
- No git work on the UI thread. The wait is state plus a `Command`, and the mark is one `HashSet` insert on the
  runner.
- Every `Stoppable::No` launch goes through `remote::launched`, which is `#[must_use]` and returns
  `Command::Unstoppable`. All five launch sites queue it ahead of the job: `operate.rs:157`, `banner/resume.rs:41`,
  `preview/answer.rs:266`, `undo.rs:503`, and `switch.rs`/`rewrite_editor/after.rs` via `launch_unstoppable`.
- A panicking unstoppable job still calls `live.finish` (`pool.rs:229` `crash::catching`, then `:191`), so shutdown
  cannot hang on a panic.
- `Live::unstoppable` marks by id before or after registration, and the wait only counts ids that are also in
  `tokens`, so a mark whose job never registered cannot hang shutdown.
- The `?` help has no `IN PROGRESS`/`HERE` list to extend (the builder's grep; I did not re-check).
