# The clock pin draws only; the whole-tour walk shoots a settled screen; two set-aside questions answered

Status: draft (planner spec-only pass, 2026-10-09; rules on review `docs/reviews/m8-tour-setaside-fetch.md` M1, M2,
m1, m2, m3 and its "Proposed changes").
Build with: **Opus, xhigh.** Part D is the stop-outcome path in graft-ops (`apply/set_aside/answer.rs`,
`apply/stopped.rs`), the path 19d-a3 fixed for `Clashed`: a wrong mark tells him a merge that is in has failed, and no
diff shows it. Part A is one production line, and its proof is a sweep of what the session clock writes. Not Fable: no
new structure, and every claim below has a test that can show whether it is right.
Needs: 19d (all parts merged), 21b-5 not required. The signatures called are shipped: `session::launch_clock`,
`JobRunner::with_clock`, `App::open_with_clock`, `graft_fixtures::PINNED_NOW`, `ApplyError::stopped_at`.
Scout: **none.** The review names every line (`startup.rs:355`, `session.rs:116,145-146`, `tour_walk.rs:104`,
`:393-407`, `:645-656`, `answer.rs:86`, `stopped.rs:231-257`). NOTES 19d-b, 19d-b2 and 19d-a3 hold what a scout would
report. Part A's first box is the sweep a scout would do, so it is done by the builder, written to NOTES, before code.
ADR: **none.** Reversing it is one call site. This is recorded as an amendment to 19d §R3 (text in §R1, appended by the
builder).

## Need
- He opens his real repos every day, and `u`, J and the time machine read the journal's stamps to order what he did
  (J merges graft entries with the reflog by time, `graft-ops/src/journal_list/rows.rs:5,47`; the switch window,
  `journal_list/steps.rs:41,339`; `state_at`, `graft-ops/src/journal.rs:344-355`). Today one stray `GRAFT_NOW` in his
  shell stamps every operation in any repo he opens at one fixed millisecond. That history is false and permanent, and
  later launches order it wrongly (review M1). A test variable must never do that.
- He judges the M8 captures by eye. The whole-tour walk can commit a capture taken mid-sort, and its gate goes red
  one run in two under load and green on retry (review M2). A gate that goes green on retry teaches everyone to retry.
- No journey changes. No key, screen or word changes.

## Rulings
### R1. M1: `GRAFT_NOW` pins the drawn clock only; journal stamps are always the wall clock (the reviewer's option a)
- **Chosen over (b), a harness-only switch with an on-screen state.** (b) still ships an env var that writes false
  history; it only adds a second var and a status line. A cargo feature cannot gate it: the gates run plain
  `cargo nextest run`, which builds the pty tests' binary with default features. `cfg(debug_assertions)` cannot either:
  the heavy release run uses the binary, and he may `cargo run`. Under (a) the binary writes nothing from the pin.
- **Not git's precedent.** `GIT_COMMITTER_DATE` writes false dates too, but git orders history by parents, not dates.
  Graft's journal readers order by the stamp, so a false stamp is a wrong order.
- **No on-screen state.** Under (a) a stray pin persists nothing: ages are wrong for one session and right again when
  the var is unset. A status marker would change every pty capture. Trade-off accepted, and written in TESTING.md.
- **Build:** `startup::run_repo` stops passing the session clock to `JobRunner::with_clock`. The binary's runner keeps
  its default, `Clock::Wall`. `Harness::clock` (the drawn frame) and the practice repo's `now` (a temp repo, removed
  on every ending) keep the session clock. Any other writer that Part A's sweep finds taking the session clock (the
  state dir, the recent list, first-launch) moves to the wall clock too.
- **Unchanged:** the in-process seams `App::open_with_clock` and `JobRunner::with_clock` still pin stamps. No env var,
  flag or config reaches them, and they write only into test fixtures.
- **What the tests rely on now:**
  - Pty drawn ages: `GRAFT_NOW` (frame only), the fixtures' dates and the pty git-date defaults (19d §R5). Unchanged.
  - Pty journal stamps: wall time. No pty test may assert a stamp's value, and no committed pty capture may draw a
    journal stamp's age. If one does, it reads `future`. Part A's sweep finds any such capture; then stop, don't mask.
  - The tour: journal append order (ea0c1d22), never stamp equality.
  - Ties within one millisecond still happen in-process (pinned runner). Part C proves `u` is safe under one.
- **Amendment text for 19d §R3,** appended verbatim under it by the builder:
  `**Amended 2026-10-09 (19e, review m8-tour-setaside-fetch M1):** GRAFT_NOW pins the drawn clock only (the frame and
  the practice repo's dates). The binary stamps the journal from the wall clock, always: a shipped env var must never
  write false history into a real repo, and J, the switch window and state_at order by the stamp. In-process tests
  still pin stamps through App::open_with_clock / JobRunner::with_clock, which no env var reaches. Pty tests rely on
  the drawn pin, fixture dates and §R5's git dates; the tour relies on journal order.`

### R2. M2: the capture helper refuses an unsettled screen (cause), not a retry (symptom)
- `tour_walk.rs`'s local `shot` waits for `settled` (`:104`, no `sorting…`) **inside itself**, before it compares or
  writes. Then every capture in the file is gated by construction, including future ones, and not only the two waits
  the review names. The waits at `:393-407` and `:645-656` also add `settled` to their predicates, so the assertions
  after them read a settled screen.
- If `settled` asks for graph-only text that the resolver (step 7) never draws, use the resolver's own loaded marker
  for that shot. Never a sleep, a frame-count heuristic or a retry.
- Other pty capture producers outside this file with the same gap are listed in NOTES with file:line, not fixed here.
  The conductor files them.

### R3. m2: prove `u` under a tie by construction, not by a 50× loop
- Under R1 the pty test no longer ties, so looping it proves nothing. A hand-built journal holds two Committed entries
  on one `at_unix_ms`. The one appended last has the **smaller** id, so id order and append order disagree. Graft-ops'
  newest-undoable read (`next_undo`, or the name graft-ops MAP gives it) must return the one appended last.
  Deterministic, no loop.
- **Red → stop and report with the line.** Undo order is a ruling, not a build decision.
- J's merge (`rows.rs:5,47`) and `state_at` (`journal.rs:344-355`) under a tie: not read here. Filed by the conductor
  as a **question** (no line proves wrong logic).

### R4. m1: answer it with the seam that exists
- A stop injected before the put-back step (`a_stop_anywhere_in_a_set_aside_merge_keeps_the_edits`' seam) is exactly
  "the merge landed, then the put-back did not run". The review reads `answer.rs:86` as giving `Git { paused: None }`,
  and `stopped::mark` as passing it through (`stopped.rs:256`), so the mark is `None`. graft maps that to
  `BeforePause` ("nothing changed") for a merge that is in.
- **Right:** the entry is `Committed`, `HEAD` is the merge, the edits are in `S` (listed) or back in the tree, and
  `ApplyError::stopped_at()` is `Some(StoppedAt::Landed)`.
- **Red → fix the cause** in the shared answer/mark, the way 19d-a3 carried the mark onto `Clashed`. Every set-aside
  kind gets it, never a merge-only branch. If a public variant needs the field, add `stopped_at: Option<StoppedAt>`
  as a3 did. No new words: graft's one caller (`jobs/operate.rs:424` → the quit exit line) shows the shipped Landed
  line. If a caller would need new words, stop and report.
- **Green as found** → the question is closed; say so with the test.
- A put-back that fails **without** a stop and without a clash: build it only if an existing seam reaches it. Never add
  a production hook. Otherwise NOTES says "unreachable by any test seam; unverified" and m1's remainder stays a
  question.

### R5. m3 and the fast-tier ceiling: shape ruled here, built later, not in 25
- **Not ticket 25.** Its scope puts tests and performance in the M8 review (`25-fable-architecture-pass.md:26`).
- **Not 19e,** for three reasons:
  - The ceiling's number needs a quiet-machine timing. The review's 841 s / 670 s ran beside three other agents'
    cargo jobs ("not attributable").
  - A ceiling landed before the `graft-ui` hints tests are cut makes the gate red on arrival, and graft-ui is outside
    this crate set.
  - A ceiling tuned under load is the M2 flake class again.
- **Shape (decided, so the later ticket has no decision left):**
  - In `.config/nextest.toml`: `[[profile.default.overrides]] filter = 'kind(lib)'` with
    `slow-timeout = { period = "30s", terminate-after = 2 }`. That ends a unit test at 60 s.
  - Integration tests (pty walks) get no ceiling: they are long by design and wait on a terminal.
  - It lands in **one ticket** with the hints width sweeps (`graph_screen::hints::tests`, 81–94 s each) sampled, or
    moved to `heavy-tests`.
  - Before that ticket: a quiet re-time, a script writing to `docs/perf/<ticket>.txt`.
- The conductor files that ticket for the M8 review's slot.

## Signatures
```rust
// No public signature changes, unless part D is red (then as 19d-a3's Clashed field, below).

// graft, event_loop/startup.rs — run_repo: the JobRunner is built without the session clock (Clock::Wall default).
//   JobRunner::with_clock(self, session::Clock) -> Self   // unchanged; callers after 19e: tests and examples only.
// graft, session.rs — launch_clock() -> Result<Clock, String>  // unchanged; its doc says "drawn clock only".

// graft tests/it/tour_walk.rs (test code)
fn ready_to_shoot(screen: &str) -> bool; // = settled(screen), or the resolver's loaded marker (R2); pure
// `shot` calls it in a wait before Captures::check; no other path in the file reaches Captures::check.

// graft-ops (only if part D is red) — apply/error.rs, as 19d-a3:
//   the variant a landed set-aside answer returns gains `stopped_at: Option<StoppedAt>`;
//   `ApplyError::stopped_at(&self) -> Option<StoppedAt>` (signature unchanged) returns it.
```

## Crate set
- `crates/graft`:
  - src: `event_loop/startup.rs` (the `with_clock` line), `session.rs` (doc only), and any writer Part A's sweep finds.
  - tests: `tests/it/clock_pin.rs`, `tests/it/tour_walk.rs`.
  - MAP lines :91, :92, :1118.
- `crates/graft-ops`:
  - tests: `tests/it/merge_set_aside.rs` and a new `tests/it/journal_tie.rs` (one `mod` line in `main.rs`).
  - src, only if part D is red: `apply/set_aside/answer.rs`, `apply/stopped.rs`, `apply/error.rs`, plus compiler-forced
    `..` in graft matches.
  - MAP, if changed.
- `crates/graft-fixtures/MAP.md:12`: "(drawn clock only)". Doc line only.
- `TESTING.md` (one entry), and the 19d ticket (the §R3 amendment, appended).
- **Not:** graft-ui, `.config/nextest.toml`, `tests/it/pty.rs`, `quit_waits/*`, examples.

## Read first
- This file.
- Review M1, M2, m1, m2 (`docs/reviews/m8-tour-setaside-fetch.md:91-143`).
- 19d §R3, §R5, NOTES 19d-b and NOTES 19d-a3 "Step 1" (`19d-merge-dirty-tree-and-tour-order.md:126-137`, `:169-183`,
  `:604-615`, `:756-776`).
- graft MAP :91-92, :109 (`places::drive`, `Carried::clock`), :1118; graft-fixtures MAP :12.
- graft-ops MAP: the `next_undo`, `ApplyError`, `stopped_at` and `StoppedAt` lines (grep).

## Acceptance
**A. The pin draws only (M1, R1).** Order: the sweep before any code.
- [ ] **Sweep.** Every place in `crates/graft/src` that hands the session clock (`launch_clock`'s result,
      `harness.clock`, `Carried::clock`) to something that **writes** is listed in a NOTES table: site · writes what ·
      into where. Expected: `startup::run_repo`'s `with_clock` (journal) and the practice repo's `now` (temp). Any
      other writer into a real repo or the state dir is moved to the wall clock in this box. Test: the grep command,
      named in NOTES, and `clock_pin::graft_now_never_stamps_the_journal` below.
- [ ] **No false stamp.** Pty, `GRAFT_NOW` = `PINNED_NOW`, one in-app commit, quit. Every new journal entry's
      `at_unix_ms` lies between the wall ms read before spawn and after exit, and none equals `PINNED_NOW * 1000`.
      **Red before.** Test: `clock_pin::graft_now_never_stamps_the_journal`.
- [ ] **The drawn pin still holds.** These stay green with expectations unchanged: `graft_now_pins_the_drawn_age`,
      `a_bad_graft_now_refuses_to_launch`, `an_in_app_commit_under_the_default_pin_draws_an_age`,
      `open_with_clock_pins_the_drawn_frame`, `no_capture_producer_runs_on_the_wall_clock`. Test: those, in the full
      run.
- [ ] **The old test's claim is updated.** `u_under_a_pinned_clock_undoes_the_newest` still undoes the second commit.
      Its "every `at_unix_ms` = pin" assertion becomes the wall-range check above. NOTES names the changed
      expectation. Test: that test.
- [ ] **No pty capture drew a stamp.** Full nextest green. `git status --short docs/shots` is empty.
      `grep -rl future docs/shots` gives the same 6 prose hits NOTES 19d-b2 lists. Any capture that changed → stop and
      report, naming it and its producer. Test: the full run plus those two commands, outputs in NOTES.
- [ ] **No production caller of the stamp seam.** `grep -rn "with_clock(" crates/graft/src` shows only `JobRunner`'s
      definition and its own `#[cfg(test)]` uses. Test: that grep, in NOTES.
- [ ] **Docs say it.** graft MAP :91 ("the journal's `at_unix_ms` comes through it too": in-process only), :92 and
      :1118 ("the shipped binary passes the session clock" removed); graft-fixtures MAP :12; TESTING.md gains "Switches
      the binary reads for tests": `GRAFT_NOW` draws only and writes nothing, `GRAFT_PRACTICE_EPOCH` dates the temp
      practice repo; the §R3 amendment is appended to 19d. Test: `grep -n GRAFT_NOW TESTING.md crates/*/MAP.md`, output
      in NOTES.

**B. The walk shoots settled (M2, R2).**
- [ ] **The pure predicate.** `ready_to_shoot` is false on a frame whose line 3 reads `date order, sorting…` (the
      review's failing frame), and true on the bodies of the committed `m8-tour-finished-120x36.txt` and
      `m8-tour-step7-conflict-120x36.txt`. Test: `tour_walk::a_screen_still_sorting_is_not_ready_to_shoot`.
- [ ] **Gated by construction.** `shot` waits on `ready_to_shoot` before `Captures::check`. The waits at `:393-407`
      and `:645-656` include `settled`. `grep -n "Captures::check\|\.check(" tests/it/tour_walk.rs` shows one call,
      inside `shot`. Test: that grep, plus `the_whole_tour_in_a_pty_ticks_all_eight_and_comes_home` and
      `the_tour_captures_match` green.
- [ ] **Holds under repeats.** The whole-tour walk runs 10× in a row from a script writing to
      `docs/perf/19e-tour-walk-repeat.txt`, with one summary line `pass=<n>/10`. Expected 10/10. The builder reads only
      that line. Red-before is not claimed: the race can't be forced. Test:
      `the_whole_tour_in_a_pty_ticks_all_eight_and_comes_home`, via the script named in NOTES.
- [ ] **Captures unchanged.** `git status --short docs/shots` is empty, and `grep -c sorting` on both tour captures is
      0. Test: those commands, in NOTES.
- [ ] **Audit.** Every other pty capture producer under `crates/graft/tests/it` that shoots without a settled wait is
      listed in NOTES with file:line, or NOTES says "none" with the grep. Test: the grep, in NOTES.

**C. `u` under a one-millisecond tie (m2, R3).**
- [ ] A hand-built journal (two Committed entries, equal `at_unix_ms`, the later-appended with the smaller id) → the
      newest-undoable read returns the later-appended. Red → stop and report with the line. Test:
      `journal_tie::u_on_a_one_millisecond_tie_takes_the_last_appended` (graft-ops).

**D. A landed set-aside stopped before its put-back (m1, R4).**
- [ ] Through the stop-injection seam, a set-aside merge is stopped after the merge lands and before `stash apply
      --index`. The entry is `Committed`, `HEAD` is the merge, the edits are in `S` (listed) or in the tree (never
      neither), and `stopped_at() == Some(StoppedAt::Landed)`. Red before → fixed per R4. Green as found → say so.
      Test: `merge_set_aside::a_stop_after_the_merge_landed_before_the_put_back_says_landed`.
- [ ] If fixed: the cherry-pick twin holds the same mark.
      Test: `cherry_pick::a_pick_over_edits_stopped_before_its_put_back_says_landed`.
      Also, NOTES lists each graft caller of `stopped_at()` and what it shows now, with no new words. The full run
      covers `update::quit::tests` (exit-line table unchanged).
- [ ] A put-back failure with no stop and no clash: a test exists only if an existing seam reaches it
      (`merge_set_aside::a_put_back_that_fails_without_a_clash_keeps_the_merge_and_the_edits`). Otherwise NOTES says
      "unreachable by any seam; unverified" and names the seams tried. Test: that test, or the NOTES line.

**Gates.**
- [ ] `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
      workspace-wide. Counts in NOTES.

## Touches (check each, say how)
- **The binary's session clock:**
  - every consumer in `crates/graft/src` (Part A's sweep): the frame, the focus debounce (`now_millis`), the practice
    repo, any state-dir writer;
  - `places::drive`'s `Clock` param, which is still passed and still drawn.
- **Journal readers that order by stamp:**
  - J's merge, the switch window, `state_at`, `next_undo`;
  - in pty runs they now see real gaps, not one ms;
  - `tour_progress` reads append order (`tour_tasks.rs`, `tour_walk.rs` green).
- **`quit_waits/pty_session.rs` `Session::spawn`** (sets git dates to `FORK_NOW`):
  - not edited;
  - its two captures must stay byte-equal under wall stamps (Part A's sweep box).
- **The in-process seams:** `App::open_with_clock` and `JobRunner::with_clock` (unchanged; their tests green).
- **The m4 example's `future` rows (19d-c C+2):**
  - not here;
  - under R1 the fix there is the in-process `JobRunner::with_clock(Clock::Fixed(PINNED_NOW))`, a test seam;
  - the conductor's open item.
- **The set-aside answer (part D, if red):**
  - every kind that sets aside: run `set_aside`, `cherry_pick`, `pull_set_aside`, `absorb_staged`, `merge_set_aside`,
    `stop_hooks`, `stash_clash`, with no expectation changed beyond those named;
  - every `ApplyError` match (the compiler);
  - the quit exit line.
- **Captures:** none expected to change (A and B boxes). The M8 contact sheet (ticket 28) is unaffected.
- **Docs:** G-rename 27's `GRAFT_*` list (same var, same name); `TESTING.md`; graft and graft-fixtures MAPs; 19d §R3.
- **KNOWN_GAPS:** the conductor closes M1 and M2, and files R3's J/`state_at` tie question and R5's ceiling ticket.
- **Keymap, journeys, words:** none.

## Size
About 50k: A ~15k, B ~10k, C ~5k, D ~20k. One context. If D's fix grows past the shared mark (a new public type, or a
caller needing words), stop at D, commit A-C, and report. If you compact, say so in NOTES.
