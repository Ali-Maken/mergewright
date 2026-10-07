# Stop review fixes, part 3: a terminal hang-up mid-write leaves the operation paused, never orphaned

Status: done
Build with: **Opus** (high). It's the exit path during a git write. The cause of the 101 is unknown, and whether git
is left orphaned shows only in a process test, not in the diff. The journal and undo semantics don't change: the stop
is 13c's and the answer is 13d's. Not xhigh, not Fable.
Needs: 13f's `Forced { …, started }` and the compiling heavy tier.
Blocked by: **13f merged** (and so 11b).
**Crate set: graft only**, plus two doc lines. Files:
- `crates/graft/src/event_loop/{startup,exit_wait,input,signals}.rs`, and whichever file holds the panicking line
  (named in NOTES)
- `crates/graft/tests/it/quit_waits/` (new `hangup.rs`, helpers shared with `signal.rs`/`stuck.rs`)
- `crates/graft/MAP.md`, `docs/design/02-keymap.md:242`, `docs/design-notes/m8-polish-and-ship.md:141`
- `docs/shots/m8-hangup-relaunch-120x40.txt`

Scout: required (brownfield, exit path). Size it to `wind_down` → `Pool::stop`, and to the input and signal threads
on EOF/EIO/SIGHUP.
Source: `KNOWN_GAPS` `M8/hangup-exit-101-orphans-git` (a 13c regression: `graft-git run.rs:478` `process_group(0)`),
review m8-13cd "Known gaps" and Question 2, Minor 6; open point `M8/13d-open-points` (the 80x24 capture, ruling R3
in `13e-stop-review-fixes.md`).

## Decision (planner, 2026-10-07; the user's ruling, field notes :91: a stop leaves it paused, nothing lost)
- **A hang-up is "Quit from outside"** (keymap §2.2), the same as SIGTERM.
  - The write gets up to `EXIT_WRITE_WAIT` to land.
  - Past that, the forced stop runs: hook-first, so git pauses a merge, pick or revert; a rebase stays paused at its
    step.
  - A second signal stops it at once.
  - Why: a write about to land should land. A stuck one ends paused, the same state `q` `y` leaves. One rule for
    every outside quit.
- **Terminal read EOF or `EIO` is the hang-up too.** It ends the loop as `Exit::Signal(SIGHUP)` would. It is never a
  `Fatal` that sits silently, whichever of the two arrives first.
- **Nothing written to a dead terminal may end the process.** Every exit-path write or restore after the hang-up is
  fallible. Handle its error (rule 3), with a comment saying the words have nowhere to go: the journal and the
  relaunch banner carry them.
- **Never exit 101. Never leave git running.** Exit with the code the SIGTERM path exits with today (read it from
  `quit_waits::signal`'s assert; NOTES).
- **Where he lands:** relaunch → paused: the banner `MERGING … c finish  a abort` (R1); landed: the graph with the
  merge on it.
- **Trade-off accepted:** for up to 60 s after a hang-up, a relaunch can find the old Graft's git still running. git's
  own locks refuse the overlap, and nothing is lost. A single-instance guard isn't in scope; the conductor files a row
  if the walk shows it.

## Scope
**In:**
1. **Find the 101 first.** Reproduce 13d's observation as a test: the real binary as a pty session leader, a merge
   with a hung `pre-merge-commit`, and the master closed. It is red with exit 101.
   - Find the panic's message and line: `RUST_BACKTRACE=1`, stderr to a file.
   - If sending stderr to a file makes the 101 go away, that is evidence it was a stderr write. Find which write by
     grep, and say so.
   - Record the line in NOTES (rule 7). 13b's guess (ratatui `Terminal::drop`'s `eprintln!`) is unverified.
   - **Fix that cause.** No `catch_unwind`, no panic hook as the fix.
   - If the cause can't be found or removed at its line, stop and report `blocked: <what>`.
2. Item 1's fix, plus the EOF/`EIO` rule above.
3. **Minor 6:** the waiting line is printed whenever `writes_running()` is non-empty on any exit, not only after a
   signal. The write is fallible, as above.
4. **R3 capture:** relaunch after the paused hang-up at 120x40 →
   `docs/shots/m8-hangup-relaunch-120x40.txt`.
   - Its banner row must contain `from a previous run` and `c finish  a abort`.
   - If `from a previous run` is missing at 120, stop and report it as a graft-ui question with the capture line.
     Don't fix it here.
5. **Docs.** Replace each with the block below, only after the tests hold.
   - `docs/design/02-keymap.md:242`:
     ```
     - **Quit from outside** (SIGTERM, SIGHUP, or the terminal closing; **Proposed**, M8/13b, 13g): the terminal is given back first. A write that can't stop gets up to 60 s to land; then Graft stops it as `q` `y` does (its stuck hook first, so git pauses it), and the same signal again stops it at once. The shell gets the line saying where git was left. When the terminal has closed nothing is printed: the next launch shows where git was left (paused: the banner offers continue or abort; landed: the graph).
     ```
   - `docs/design-notes/m8-polish-and-ship.md:141`:
     ```
     | SIGTERM / SIGHUP / terminal closed while a write runs | terminal given back at once (closed: nothing printed); the write lands within 60 s → exits; else stopped as the stuck write's `q` `y` (hook first, git pauses it), with that row's exit line; a second signal stops it at once; never exit 101, never git left running (13g) |
     ```
   - The graft MAP `Exit` and `exit_wait::Pool` rows: hang-up = `Signal(SIGHUP)`; the waiting line on any exit with
     writes running.

**Out:**
- A panic-proof stop (a `Drop` on the pool for every panic). If item 1 finds a cause that can't be fixed at its line,
  report it; don't add this.
- The single-instance guard.
- Linux.
- Any graft-ui change.

## Signatures
No public signature changes expected. `Exit` keeps its variants; a hang-up is `Exit::Signal(libc::SIGHUP)`. If the
builder finds a new variant necessary, report it before building.

## Read first
- This ticket's Decision; `13e-stop-review-fixes.md` Rulings
- `docs/reviews/m8-13cd-stop.md` "Known gaps" (`M8/hangup…`), Minor 6, Question 2
- `13d-stop-part-c.md` NOTES "Hang-up observation"; `13b-quit-waits-for-a-write.md` NOTES on hang-up
- `crates/graft/MAP.md` `event_loop` (:942-953), `## jobs` `shutdown`/`EXIT_WRITE_WAIT`/`FORCED_WAIT` (:1045-1066)
- `docs/design/02-keymap.md` §2.2 "Quit from outside", "A stuck write"
- Then source: the scout's file, then `startup.rs` `wind_down`, `exit_wait.rs`, `input.rs`, `signals.rs`

## Touches (check each; say how)
- `Exit::Fatal` callers: is any `Fatal` other than EOF/`EIO` changed? It must not be (grep `Exit::Fatal`).
- The `quit_waits::signal` and `quit_waits::stuck` tests (same exit path): the workspace nextest.
- `headless` (no-tty) path: it doesn't read a terminal. Check that EOF on its stdin, if it reads stdin, isn't turned
  into a hang-up (grep `run_headless`).
- Examples that match `Exit` (`gallery`, `m3_walk`, `m4_walk`, `m5_walk`): the compiler.
- The journey fences `quit-stuck-write 4` and `quit-while-write 2`: walked unchanged.
- Captures: one new one; no existing capture changes.

## Acceptance (each tick names its test inline)
- [x] The cause: NOTES name the panicking line and its message, observed. Red first:
      `it::quit_waits::hangup::a_hangup_mid_write_never_exits_101` (exit ≠ 101, = the SIGTERM path's code).
- [x] Quick hook lands: `it::quit_waits::hangup::a_hangup_mid_merge_lets_a_quick_hook_land`.
      - Hook `sleep 2; exit 0`; close the master once the hook's pidfile exists.
      - Asserts: Graft exits within `EXIT_WRITE_WAIT`; `HEAD^2` = the source tip; no `MERGE_HEAD`; no `*.lock` under
        `.git`; git and the hook gone (`kill -0`); the journal entry `Committed`.
- [x] Stuck hook, second signal: `it::quit_waits::hangup::a_hangup_then_a_second_hangup_leaves_the_merge_paused`.
      - Hook `sleep 600`; close the master, then `SIGHUP` Graft's pid.
      - Asserts: Graft exits within `FORCED_WAIT` of the second signal; `MERGE_HEAD` = the source tip; `HEAD` unmoved;
        no lock; git and the hook gone; the entry `Pending`.
      - Then relaunch in a new 120x40 pty: the banner reads `c finish  a abort`, and `a` gives back `HEAD`, `git status`
        and his file as before (reuse `stuck.rs`'s relaunch helper).
- [x] Stuck hook, no second signal (heavy, `heavy-tests`):
      `it::quit_waits::hangup::a_hangup_with_a_stuck_hook_is_stopped_after_the_wait_and_left_paused`. Same asserts,
      bound `EXIT_WRITE_WAIT` + `FORCED_WAIT`. Run this one test alone, once; record its time in NOTES. The heavy
      tier itself stays for the review.
- [x] EOF/`EIO` = hang-up: `event_loop::tests::a_dead_terminal_read_ends_the_loop_as_a_hangup` (an `EIO` and an EOF
      both give `Exit::Signal(SIGHUP)`).
- [x] Minor 6: `event_loop::exit_wait::tests::a_fatal_exit_with_a_write_running_prints_the_waiting_line`, and a dead
      stderr there doesn't panic: `…::the_waiting_line_to_a_closed_stderr_is_not_fatal`.
- [x] Every hang-up test holds a `Drop` guard that kills its pidfile pids. `ps -A | grep -c "sleep 600"` = 0 after
      the run (NOTES).
- [x] Capture `docs/shots/m8-hangup-relaunch-120x40.txt` (≤ 200 KB), banner row quoted in NOTES with
      `from a previous run`. Colour **unverified**; whether it looks right is the user's call.
- [x] The doc blocks in item 5 are replaced (grep in NOTES).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
`cargo clippy --all-targets --features heavy-tests -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
One context, about 70k tokens. Item 1 is the unknown. If it eats the context, hand off after item 1 with the cause in
NOTES; the conductor splits the rest.

## NOTES (builder, 2026-10-07)

No compaction. No scout: the ticket's line held (`graft-git run.rs:478` `process_group(0)` unchanged) and the exit path
is five files I read directly (startup `run_repo`/`wind_down`/`finish_to`, exit_wait, input, signals, turn).

**The cause (observed, rule 7).** Reproduced as a session-leader pty (`setsid` + `TIOCSCTTY` in `pre_exec`; the master
fd closed in the child, else Graft's inherited copy keeps the pty alive), stderr on the pty, merge hook `sleep 600`,
master closed: exit status 25856 = **101 after 0.05 s**, git and hook alive (orphaned). Graft's own crash report
(`$HOME/.local/state/graft/crash-*.log`, isolated home) said:
`message: failed printing to stderr: Input/output error (os error 5)`, backtrace
`ratatui-core-0.1.2/src/terminal.rs:483:17` (`Terminal::drop`'s `eprintln!("Failed to show the cursor")`) ←
`graft/src/event_loop/startup.rs:349` `drop(terminal)` in `run_repo`. So 13b's guess was right. With stderr to a
file the 101 goes away and the file holds `Failed to show the cursor: Input/output error (os error 5)`: same write.
The panic happens before `Pool::stop`, so nothing stopped git (its own process group since 13c, so the kernel's
`SIGHUP` never reached it).

**Fix (cause).** `startup::put_away` shows the cursor itself before the `Terminal` drops; on failure it marks the
backend (`RowClear::cursor_given_up`), so ratatui's `Drop` retry is answered `Ok` and never `eprintln!`s. Cost to the
user: none; a live terminal gets the same bytes as before. Plus: dead-terminal read (`EOF`/`EIO`, `input::hung_up`)
→ `Exit::Signal(SIGHUP)` in the repo loop and the places loop (headless unchanged); `finish_to` lets a hang-up die of
`SIGHUP` though the restore fails (`EIO`), and `wind_down` drops a stalled-jobs line it cannot print after a hang-up;
`Pool::stop` takes `again` **before** the waiting line and, when signalled, counts the ending as one signal
(`Watch::again_after_one`) so a hang-up's own late-counted `SIGHUP` is not "the signal again".
Exit code: the `SIGTERM` path dies of the signal (`quit_waits::signal` asserts `status.signal() == Some(15)`); a
hang-up now dies of `SIGHUP` (`status.signal() == Some(1)`), asserted in all four hang-up tests.

**Ticks' tests:** `it::quit_waits::hangup::{a_hangup_mid_write_never_exits_101, a_hangup_mid_merge_lets_a_quick_hook_land,
a_hangup_then_a_second_hangup_leaves_the_merge_paused, a_hangup_with_a_stuck_hook_is_stopped_after_the_wait_and_left_paused}`;
`event_loop::tests::a_dead_terminal_read_ends_the_loop_as_a_hangup`; `event_loop::exit_wait::tests::{a_fatal_exit_with_a_write_running_prints_the_waiting_line,
the_waiting_line_to_a_closed_stderr_is_not_fatal}`; also `event_loop::signals::tests::*`,
`event_loop::row_clear::tests::a_cursor_given_up_is_not_asked_for_again`,
`event_loop::startup::tests::finish_after_a_hangup_dies_of_it_whatever_the_restore_said`.
Red first: the 101 was observed by the probe (above) before the fix; the never-101 test was written against that run.
The EOF/EIO loop test was written alongside its mapping, not seen red.

**Timings.** Quick hook: hang-up to exit 2.03–2.10 s. Second hang-up to exit: 125–177 ms (≤ `FORCED_WAIT`).
Heavy test, run alone once: **60.09 s** hang-up to exit (≤ `EXIT_WRITE_WAIT` + `FORCED_WAIT`).
`ps -A | grep -c "[s]leep 600"` = 0 after the quit_waits run, after the heavy run and after the workspace nextest.

**Capture.** `docs/shots/m8-hangup-relaunch-120x40.txt` (2.0 KB). Banner row:
` merge still in progress from a previous run · c finish  a abort  ⇥ graph  q quit`. Colour **unverified**;
whether it looks right is the user's call. The pty relaunch (120x40) shows the banner, `a` `y` aborts, `HEAD`,
`git status --porcelain` and `base.txt` match before the merge.

**Docs.** `grep -n "terminal closing" docs/design/02-keymap.md` → :242 replaced. The design note's row was at **:171**,
not :141 (the note grew); replaced there: `grep -n "terminal closed while a write runs" docs/design-notes/m8-polish-and-ship.md`.
graft MAP: `Exit`, `waiting_line`, `exit_wait::Pool`, `signals::watch` rows; new rows `input::hung_up`, `startup::put_away`.

**Touches.** `Exit::Fatal`: none changed (6 uses in `crates/graft/src`, none edited; the diff touches no `Fatal`).
`quit_waits::signal`/`stuck`: green in the workspace nextest (5725/5725). Headless: `headless.rs` untouched; its
`InputFailed` still returns the error. Examples matching `Exit`: compile (`clippy --all-targets`, both feature sets).
Journey fences `quit-stuck-write 4`, `quit-while-write 2`: walked by `stuck.rs`, green. Captures: one new; every
existing `check_shot` green. `stuck.rs`: `pid`/`alive`/`gone_within`/`locks`/`entries` made `pub(super)` for
`hangup.rs` (no behaviour change). `nix` dev-dependency gains the `poll` feature (no new package; `Cargo.lock`
unchanged). Test-only `unsafe` in `hangup.rs::session_leader` under a scoped `#[allow(unsafe_code)]` with a reason.

**Findings (not fixed, out of scope).**
- The places screen (`event_loop/places.rs:241`, plain `Terminal<CrosstermBackend<Stdout>>`) still drops ratatui's
  `Terminal` the old way: a hang-up **on the places screen** would still exit 101 by the same `terminal.rs:483`
  `eprintln!`. No write runs there, so nothing is orphaned. Unverified (not reproduced); filed here as a gap.
- After `a` `y` on the relaunch, `q` sometimes opens `Quit?` ("abort merge is still running") because the abort's job
  has not answered yet though `MERGE_HEAD` is gone; the test answers `y` as he would. Behaviour, not a defect claim.
- Real-world double `SIGHUP` (kernel to the foreground group + the shell forwarding `SIGHUP` to its jobs) may count as
  "the signal again" and stop the write at once instead of waiting. Not observed here (one kernel `SIGHUP` in the test
  setup, where Graft is the session leader); a question for the walk, not verified.

## NOTES (debugger, 2026-10-07): `a_hangup_mid_write_never_exits_101` flaky under load

**Cause (test, ordering assumption; mechanism observed).** The test closed the master, waited a fixed 2 s, then sent
"the signal again" (old `hangup.rs:432-440`). The product counts "again" from the exit's waiting line:
`exit_wait.rs:89-95` takes the baseline in `Pool::stop_to`, as `max(seen now, 1)` (`signals.rs:99`). A `SIGHUP` counted
before `stop_to` runs goes into the baseline, so it is never heard. Graft then waits `EXIT_WRITE_WAIT` (60 s), and the
test's `FORCED_WAIT + 5 s` bound fails with "the second hang-up ends it". With stderr on the dead pty, the test cannot
see the line, so it has no moment at which a second signal is sure to count.
- Instrumented (`[DEBUG-13gx]`, all removed). Unloaded order: kernel `SIGHUP` seen=1 → loop ends on `InputFailed`
  (EOF) → `stop_to` baseline=1 → test `SIGHUP` seen=2 → stop. From hang-up to the waiting line: 22–49 ms, 10 runs, load
  average 37.
- Red, by forcing the order: a 2.5 s sleep injected before the baseline (temporary, env-gated). Order: seen=1, seen=2,
  then baseline=2. The unchanged test failed **3 of 3** at `hangup.rs:440`, in 16.3–18.6 s. The flake on main took
  20.7 s, which fits a slower setup under load.
- **Not reproduced naturally.** 10 loop runs with a 6-thread load beside them, plus 2 full loaded workspace runs: all
  green. Under load the 50 ms window never reached 2 s here.
- **Not excluded:** a `PATIENCE`/`HOOK_STARTS` (20 s) setup timeout also fits 20.7 s. The failure output was not kept,
  so the two can't be told apart. A longer timeout would be a symptom fix, so none was added.

**Product: not changed. This is a question, not a defect.** Counting from the line is documented
(`exit_wait.rs:89-90`), and the window is what stops a shell-forwarded duplicate `SIGHUP` from counting as "again".
Counting from the loop's end instead would make a real window-close stop the write at once. That is a design call.

**Fix (cause, test-side).** `a_hangup_mid_write_never_exits_101`:
- Its hook now waits for `.git/release` and then exits 0.
- After the 2 s check (no exit, git alive), the test creates the file. The merge lands, and the test asserts: Graft
  exits within `EXIT_WRITE_WAIT`, ≠ 101, dies of `SIGHUP`, no crash report, git and the hook gone.
- No signal-ordering race is left.
- Still red on 13d's bug: with the old `drop(terminal)` put back temporarily, it fails at 48 ms with status 25856
  (exit 101).
- Green 3 of 3 with the 2.5 s injection, and 1 of 1 without it.

The second signal stays covered by `a_hangup_then_a_second_hangup_leaves_the_merge_paused`, which waits for the line
first: green 3 of 3 under the injection.

Coverage moved: the stuck hook, force-stopped with stderr dead, is now
`a_hangup_with_a_stuck_hook_is_stopped_after_the_wait_and_left_paused` (heavy), switched from `Stderr::File` to
`Stderr::Pty`. It never read the file. Run alone once: 60.08 s from hang-up to exit, PASS. The cost: the fast tier no
longer force-stops a write with stderr dead. Only the heavy tier does.

**Touches.**
- `hangup.rs` only. No product code changed (`git diff --stat`).
- `FORCED_WAIT` and `hang_up_again` are still used by the paused test.
- No capture changes.
- Gates: see the commit.
