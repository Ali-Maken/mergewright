# Review: M8 ticket 13, daily landing (pull nothing / `u` `U` / Incomplete / `q` box)

Fixed hash: `c72a9dbe` ("merge: m8 13 daily landing"), diffed against its first parent `08754847`.
Ticket: `.scratch/m8-polish-and-ship/issues/13-daily-landing.md`. Design note: `docs/design-notes/m8-polish-and-ship.md`
(Journeys J18, J20; "Where he lands"; 2b Proposed on `q`).

**0 Critical · 3 Major · 10 Minor.**

## Gates (own worktree at `c72a9dbe`, own `CARGO_TARGET_DIR`, one command at a time)

| Gate | Result |
|---|---|
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass |
| `cargo clippy --all-targets --features heavy-tests -- -D warnings` | pass |
| `cargo nextest run --workspace` | **5177/5177 pass**, 9 skipped, 2 slow |
| `cargo test --doc` | pass |
| Heavy tier, release perf run, fuzz, bench | **skipped on the conductor's instruction** |

- Fast suite: **616 s** wall clock (622 s including the `time` wrapper). Peak RSS 541 MB. `target/` 4.8 GB. The machine was
  shared with concurrent builders (748 s system CPU), so this time is not a baseline.
- Slowest tests: nextest flagged only 2 as slow (> 60 s): `graft-ui graph_screen::hints::tests::the_bar_never_draws_a_fragment_of_itself`
  at 82.6 s and `…::the_bare_keys_are_a_tail_not_a_scatter` at 80.1 s. I did not get the 3rd to 5th: this run did not
  record per-test times, and I did not re-run the 10-minute suite to get them.
- Every capture under `docs/shots/` is checked on every fast run (`graft-fixtures` `capture`, "asserted on every run").
  So the green run shows that no checked capture is stale at `c72a9dbe`. See "Captures" below for the ones the run does not check.

## Spot-checked ticks: 5 checked, 5 held (1 with a disclosed deviation, see Minor 7)

1. J18. `daily_landing::j18_a_pull_with_nothing_new_leaves_the_cursor` exists and asserts the cursor stays, `land_on_head` is
   false and the status is `Done{"up to date"}`. Held.
2. J20. All 11 named tests exist (`daily_landing` ×2, `land_on_result` ×2, `undo::landing::tests` ×7). Held. One caveat:
   "one test per operation kind" is met only at entry level. That is enough here because `landing::of` reads nothing but the
   journal entry (`undo/landing.rs:23-36`), so the operation kind cannot change where it lands.
3. Incomplete. Both named tests exist and assert landing, tagging and the warning. Held.
4. `q`. All six named `quit::tests` and `daily_landing::capture_quit_while_a_fetch_runs` exist. Held for the fetch half. The
   stopped-rebase half is ticked while it deviates (disclosed inline), and "relaunch offers continue" names no test.
5. Captures. Both files exist and both named generators exist. Held.

## Focus answers

- **Landing rules vs J18/J20.** These match the note's "Where he lands" rows. A pull whose entry is equal on HEAD, refs, index
  and worktree lands nowhere and says `up to date ✓` (`operate/outcome.rs:343-357`, `:332`). After `u`/`U` the cursor goes to
  HEAD if HEAD moved, else to the first branch or tag (in name order) that moved and still exists, else to HEAD for a ref that
  has no row (`undo/landing.rs:23-36`). Each walk sends one key per journey (`daily_landing.rs:62`, `:136`, `:143`), which fits
  the note's counts (`pull-nothing-new 1`, `undo-see-result 1`). But the walks do not read the note's `journey-counts` fence
  (Minor 9).
- **Quit box cancel in the real event loop.** Verified **by reading the code, not by observing it**:
  - `carry_out` runs a frame's commands before the loop reads `state.quit` (`event_loop/running.rs:197-213`).
  - Its default arm sends `CancelJob` to the runner (`running.rs` `command => jobs.dispatch(command)`;
    `jobs/runner.rs:1268` → `cancel`).
  - After the loop ends, `jobs.shutdown(SHUTDOWN_GRACE)` cancels every job anyway (`event_loop/startup.rs:205`).

  I tried a pty run against a remote with a deliberately slow upload-pack, to observe the git child being stopped. The
  permission system refused it. The timing risk that remains is Minor 3.
- **Stopped rebase isolated?** In the code, yes. It is one function (`update/quit.rs:73-84`) with one test, and the module doc
  says how to flip it. Two leftovers would need changing with it: the graft-ui doc of `ConfirmYes::Quit` ("while a write runs
  or git is stopped", `hints/context.rs:421-422`) and the `update::quit` row in MAP.md. **But the branch is mostly unreachable
  (Major 1).**
- **graft-ui `ConfirmYes::Quit` coverage.** Covered:
  - it is in `ALL`, and `from_label("Quit")` resolves through `ALL` (`context.rs:479-481`);
  - the coverage sweep counts it (slot 111);
  - `keys/bound.rs` has `("y","quit")`;
  - `quit/tests.rs` asserts `yes() == Some(ConfirmYes::Quit)`, and the capture's bottom row reads `y quit`.

  The slot number is a special case (Minor 6).
- **Stale captures.** The checked captures are clean (green run). The builder re-shot the m6 drift pair and rebuilt
  `m6-contact-sheet-4.txt`. The caption of `fix-undo-drift-way-back` (`m6-contact-sheet-captions.tsv:144`) and its TESTING.md
  row (`TESTING.md:1466`) do not mention the cursor, so neither went stale. No other contact sheet contains the two changed files.

## Critical

None.

## Major

**1. With the banner focused, `q` does nothing at all, so a stopped operation gets neither the new box nor the ruling's quit.**
`banner_key` swallows every key it does not name (`update/banner/focus_keys.rs:161`, `_ => {}`). Its own doc names this
ticket's job: "Quitting straight out of a live operation wants its own confirm, which is M8's" (`focus_keys.rs:83-84`). That doc
is still there, and `q` was not routed. Three of the stop landings focus the banner:
- a conflict stop (`operate/outcome.rs:246`);
- a launch into a stopped operation (`relaunch.rs:85`);
- an undo back into one (`undo.rs:284`).

So the `q` he presses where he actually lands is swallowed silently. It does nothing until `Tab` hands the keyboard back. The
same applies to a `c` continue launched from the banner while it runs. The test for this path builds its state with
`banner_focus` false (`quit/tests.rs:144-152`), so it never meets the swallow.
- Why it matters: ruling 4 is about this exact moment, and a key that silently does nothing breaks UX §1 "visibility of
  system status". He reads it as a hang.
- Fix: in `banner_key`, before the catch-all, send plain `q` to `quit::asked`, the same way `u`/`U` are already sent to their
  global path (`focus_keys.rs:139-153`). Add a test from a banner-focused stop. Update the stale doc at `:79-84`.

**2. The box promises "quit stops it" for writes that cannot be stopped, then refuses to quit.**
`running_box` words the line the same way for every running write (`update/quit.rs:56-68`). Merge, rebase, cherry-pick,
revert, time-machine and J put-back writes are launched `Stoppable::No` (`operate.rs:116-131`, `banner/resume.rs:46`,
`preview/answer.rs:271`, `undo.rs:507`). For those, `[ Quit ]` reaches `stop_write`. `stop_write` puts `… runs to the end · a
conflict stops it on the banner, where a aborts` on the status row (`remote/in_flight.rs:41-44`), and `confirmed` returns
without quitting (`quit.rs:47-50`).
- Why it matters: the box states a consequence that will not happen, which goes against the M5 promise "you see the result
  before you confirm". He presses Quit and the app stays open. The builder's NOTES (ticket :74) say this path has no test.
- Fix: in `running_box`, read the running write's `Stoppable`. For `No(_)`, word the box `<op> runs to the end · quit
  when it finishes` and either offer only `[ Cancel ]` or make `[ Quit ]` quit once the write ends. Add a test for each.

**3. One file, two things called `landing` (readability, PROMPT 6.2).**
`undo.rs:28` imports `consequence::landing`, a function that words the status line (used at `:367` and `:543`). `undo.rs:26`
adds `mod landing`, the cursor landing (used at `:378` and `:381`). In `undid`, `landing(what, …)` and `landing::of(…)` sit
eleven lines apart and mean different things. It compiles only because modules and functions live in separate namespaces.
- Fix: rename the module (`undo/lands_on.rs`, giving `lands_on::of`) or the function (`done_words`). Update the MAP.md row.

## Minor

**1. When an annotated tag comes back, the cursor lands on the tag object, not a commit.** The journal records a ref's raw
target unpeeled (`graft-ops/src/facts.rs:355-372`, `RefTarget::Object(id)`). For `refs/tags/*`, `landing::of` returns
`Landing::Commit(that id)` (`undo/landing.rs:27-31`). An annotated tag's id is a tag object, which has no row. So
`History::seek` sends a `Find` that walks the whole history (`history/scroll/query.rs:85-99`; 17,694 commits on his repo) and
lands nowhere. `came_back` tags an id no row has. The synthetic tests use bare commit ids, so they do not catch it. Fix: skip
`refs/tags/*` landings whose value is not a commit row, or peel in `landing`; add a real-repo `u` after deleting an annotated tag.

**2. `has_a_row` leaves out remote-tracking refs, though the graph draws them.** The doc says "a ref the graph draws a row
badge for: a branch or a tag" (`undo/landing.rs:83-86`), but the graph draws `[origin/main]` (e.g.
`m6-contact-sheet-4.txt:1626`). An undo that moves only a `refs/remotes/*` ref lands on HEAD rather than on that ref's row. Fix:
include `refs/remotes/`, or correct the doc and say why it is left out.

**3. Question: does `y` during a fetch exit cleanly?** `SHUTDOWN_GRACE` is 200 ms (`jobs/mod.rs:111`). The git child gets
`SIGTERM` and up to 2 s before `SIGKILL` (`graft-git/src/run/exec.rs:202`, `:236-258`). If git, or a job between token checks,
takes longer than 200 ms, two things follow:
- the exit prints `graft: 1 background job(s) did not stop in time and were abandoned` and returns failure
  (`event_loop/startup.rs:300-303`);
- a child that has not yet been signalled outlives Graft.

I have not observed either; I am filing it as a question because the pty run was refused. Fix: observe it once (a slow
`remote.<name>.uploadpack` wrapper, `f q y`, then `ps`), and record the result in KNOWN_GAPS as verified or broken.

**4. The stopped box's words do not fit bisect or merge.** `"{kind} is stopped · next launch offers continue or abort"`
(`update/quit.rs:76-80`):
- for a bisect, which is a session rather than a stop and has no continue or abort, it reads `bisect is stopped · …`;
- for a merge, the app's own verb is `c finish` (graft-ui MAP `LiveC::Finish`).

This is textual. Fix it when the ruling lands: word it per kind, or leave bisect out.

**5. A comment now sits above the wrong match arm.** The `RestartConflict` guard's comment ("A write that began while the box
was up owns the index …", `update/confirm.rs:238-242`) now sits above the new `ConfirmThen::Quit` arm (`:243`). Its own arm is
`:244`. Fix: move the `Quit` arm above the comment.

**6. The coverage slot is a hardcoded special case.** `confirm_slot` returns 111 for `Quit` because `54 + 9 = 63` collides
with the changes pane (`graft-ui/src/graph_screen/tests.rs:2339-2342`). The next `ConfirmYes` collides again at 64. The sweep
would catch that (the slot count drops), so nothing is wrong today. Fix: give the confirm box a contiguous block at the end,
sized from `ConfirmYes::ALL.len()`.

**7. Tick 4 claims a clause the code does not do.** The tick says "`q` with a stopped rebase → quits, relaunch offers
continue". The code boxes, with a disclosed deviation, and no test is named for "relaunch offers continue" (ticket :54-62).
It is not a false tick, because the deviation is stated where the tick is. But a reader of the checkbox gets the wrong answer.
Fix: split it into its own `[ ]` line, pending the ruling. The conductor adds KNOWN_GAPS rows for the pending ruling and for
Minor 3. Neither is in `KNOWN_GAPS.md` today (grep `quit`).

**8. J18's status line has no capture.** `up to date ✓` is a new line he will see daily, and no file under `docs/shots/`
shows it (CLAUDE.md rule 6). Fix: add a frame to an existing generator, e.g. `daily_landing::j18_…`.

**9. The J18/J20 walks do not check their counts against the note.** They send one key (`daily_landing.rs:62`, `:136`,
`:143`) but do not read the note's `journey-counts` fence (`pull-nothing-new`, `undo-see-result`). Pipeline step 7 wants the
acceptance walk to fail when a journey goes over its count. Fix: the M8 acceptance ticket maps both ids to these tests, as
`m7_walk::every_m7_journey_id_has_a_walk` does.

**10. The quit box draws three empty rows.** `quit_box` passes an empty command (`update/quit.rs:90`). The box keeps the
command slot, so blank rows sit between the message and the buttons (`docs/shots/m8-quit-while-running-80x24.txt:11-13`). Other
boxes print the git command there. Whether it looks right is the user's call. Flagged so he looks at it.

## Not findings (checked)

- No `unwrap`/`expect` outside tests in the added code (`quit.rs`, `undo/landing.rs`, `outcome.rs` hunks).
- Box text is sanitised: `quit_box` runs every line through `sanitize_line` (`quit.rs:86-91`), including the running write's
  description.
- No git work on the UI thread. The landing is computed from the answered entry, and the cancel is a `Command`.
- Incomplete keeps everything it did before. `amend_into::landed` takes `asked` the way the removed `forget` did
  (`amend_into.rs:325-331`). `commit_editor::committed` still runs, through `landed` (`outcome.rs:174`).
- Ctrl+C never boxes (`quit.rs:28-33`). The box passes Ctrl+C through, so the app quits, and `shutdown` cancels the job.
- `pulled_nothing` skips `remote_done`, which has a side effect only for `Push` (`remote/in_flight.rs:123-140`). Nothing is lost
  for `Pull`.
