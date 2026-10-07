# Stop review fixes, part 3: a terminal hang-up mid-write leaves the operation paused, never orphaned

Status: draft
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
- [ ] The cause: NOTES name the panicking line and its message, observed. Red first:
      `it::quit_waits::hangup::a_hangup_mid_write_never_exits_101` (exit ≠ 101, = the SIGTERM path's code).
- [ ] Quick hook lands: `it::quit_waits::hangup::a_hangup_mid_merge_lets_a_quick_hook_land`.
      - Hook `sleep 2; exit 0`; close the master once the hook's pidfile exists.
      - Asserts: Graft exits within `EXIT_WRITE_WAIT`; `HEAD^2` = the source tip; no `MERGE_HEAD`; no `*.lock` under
        `.git`; git and the hook gone (`kill -0`); the journal entry `Committed`.
- [ ] Stuck hook, second signal: `it::quit_waits::hangup::a_hangup_then_a_second_hangup_leaves_the_merge_paused`.
      - Hook `sleep 600`; close the master, then `SIGHUP` Graft's pid.
      - Asserts: Graft exits within `FORCED_WAIT` of the second signal; `MERGE_HEAD` = the source tip; `HEAD` unmoved;
        no lock; git and the hook gone; the entry `Pending`.
      - Then relaunch in a new 120x40 pty: the banner reads `c finish  a abort`, and `a` gives back `HEAD`, `git status`
        and his file as before (reuse `stuck.rs`'s relaunch helper).
- [ ] Stuck hook, no second signal (heavy, `heavy-tests`):
      `it::quit_waits::hangup::a_hangup_with_a_stuck_hook_is_stopped_after_the_wait_and_left_paused`. Same asserts,
      bound `EXIT_WRITE_WAIT` + `FORCED_WAIT`. Run this one test alone, once; record its time in NOTES. The heavy
      tier itself stays for the review.
- [ ] EOF/`EIO` = hang-up: `event_loop::tests::a_dead_terminal_read_ends_the_loop_as_a_hangup` (an `EIO` and an EOF
      both give `Exit::Signal(SIGHUP)`).
- [ ] Minor 6: `event_loop::exit_wait::tests::a_fatal_exit_with_a_write_running_prints_the_waiting_line`, and a dead
      stderr there doesn't panic: `…::the_waiting_line_to_a_closed_stderr_is_not_fatal`.
- [ ] Every hang-up test holds a `Drop` guard that kills its pidfile pids. `ps -A | grep -c "sleep 600"` = 0 after
      the run (NOTES).
- [ ] Capture `docs/shots/m8-hangup-relaunch-120x40.txt` (≤ 200 KB), banner row quoted in NOTES with
      `from a previous run`. Colour **unverified**; whether it looks right is the user's call.
- [ ] The doc blocks in item 5 are replaced (grep in NOTES).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
`cargo clippy --all-targets --features heavy-tests -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
One context, about 70k tokens. Item 1 is the unknown. If it eats the context, hand off after item 1 with the cause in
NOTES; the conductor splits the rest.
