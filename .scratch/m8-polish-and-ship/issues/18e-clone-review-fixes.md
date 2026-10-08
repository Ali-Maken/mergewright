# 18e — clone review fixes: the secret check over events, a pure first-launch line, `esc` in the clone's password box, the splits

Status: **A1 done** (builder 2026-10-08, dispatched early); A2, A3, Part B open, waiting for 19b. Was: ready (conductor 2026-10-08) — was draft (planner spec-only pass 2026-10-08, from `docs/reviews/m8-clone.md` Critical C1 and Majors M1–M3)
Build with:
- **Part A Opus (high)**: M1 moves a per-process switch out of `AppState` across the shell/loop seam; the seam is fixed
  below, but whether a done in one place still hides the line in the next is judged only by a cross-place test. M2's
  test is worth something only if it is seen going red on the reorder. Not Fable: every shape is decided here, and
  each part is judged by a test, not by eye (fails Fable test 2).
- **Part B Sonnet**: move-only splits along seams named below, then four functions cut to ≤ 60 lines by naming steps.
Scout: none needed. The review names every site (`palette/input.rs:85`, `first_launch.rs:177-183, :214-218`,
`places.rs:519-584, :559-572`, `jobs/clone/tests.rs:195-240`, `m8_walk/clone.rs:229-238`). Those lines are at
`ce6fdfdf`; 19b moves them. Re-find each by name (`rg`), never by line. Say this in the report.
Needs: 19b's `Arrival`, `Exit`, `drive` shapes as merged (Part A edits beside them). No new public signatures.
Blocked by:
- **Both parts wait for 19b merged.** 19b (tour, running now) holds `crates/graft/src/event_loop/places.rs`,
  `crates/graft/src/empty_state.rs` and graft-ui `empty_state.rs`; Part A edits the first two and Part B moves all
  three.
- **Part B waits for Part A merged**: same files.
- A1 alone touches only `crates/graft/src/jobs/clone/tests.rs` and the 18c ticket, disjoint from 19b. The conductor
  may dispatch A1 early to close the Critical sooner; then A2–A3 follow 19b. Cap: two cargo helpers.

Crate set:
- Part A: **graft** only — `first_launch.rs`, `update/` (state, `palette/input.rs`, `tests/first_launch.rs`),
  `empty_state.rs` (+ its `palette_tests.rs`), `event_loop/places.rs` (+ `places/tests.rs`), `event_loop/running/carry.rs`,
  `event_loop/startup.rs` (`arrive`), `view/frame.rs` (one read), `jobs/clone/tests.rs`, `crates/graft/MAP.md`,
  and lines 46-47 of `.scratch/m8-polish-and-ship/issues/18c-clone-prompt-and-walk.md`.
- Part B: **graft** (`empty_state*`, `event_loop/places*`, `update/credential*`), **graft-ui** (`empty_state*`), both MAP.md.
- **Not:** graft-ops (m5's `clone::target` is a gap row), `docs/shots/**` (no capture may change), ADRs, ARCHITECTURE.md,
  keymap, KNOWN_GAPS.

## Rulings (planner)
- **C1: make the box true, cheaply.** `jobs::clone::tests::child_asks_through_the_pool_and_clones` already sees every
  event of a clone that asks for a password. It gains the secret check over each event's `Debug`. `no_secret_leaked`
  (a `tests/it` helper) can't be called from a unit test, so the amended 18c box names what is scanned, not the helper.
- **M1: the switch lives in the shell, the states hold a plain value.** As the review proposes:
  - `AppState::first_launch` and `OutsideState::first_launch` become `Option<PathBuf>`: `Some(dir)` while the line
    shows. Cloned with the state and compared by value, so a snapshot can't change under a later `update`.
  - The first palette open sets it `None` and asks the shell to end the line: the graph pushes the existing
    `Command::FirstLaunchDone(dir)`; the empty state returns a new `OutsideStep::FirstLaunchDone(dir)`. Neither
    `update` nor `empty_state.rs` touches a `FirstLaunch` or the file.
  - The process-wide `FirstLaunch` (the `Arc<AtomicBool>` + dir) is held by `drive` only. The review names its
    cross-place state `places::Carried`; confirm the name and say it in NOTES. `drive` lends `&FirstLaunch` to the
    repo place's loop through the context `pump_with` already takes, so `running::carry` can reach it. No global,
    no thread-local. `places` uses its own copy for the empty state.
  - Carrying either one: `FirstLaunch::end()`. On `Some(dir)`, write `done` with `first_launch_done`. A failed write
    is `Status::Failed(words)` on the graph (as today) or the problem line on the empty state. On `None` (already
    ended), do nothing.
  - Every `Arrival` is seeded with `FirstLaunch::line()`, so a done in one place hides the line in every later place.
    This is the reason 18d gave for sharing the `Arc`. It still holds, now without the state owning it.
  - Rejected: keeping the `Arc` in `AppState` but never ending it from `update`. `PartialEq` would still compare a
    live atomic, and ARCHITECTURE.md:286 ("`AppState` … must stay pure") would still be broken in letter.
- **M2: a `places::tests` case on a real pool, seen red on the reorder.** Not a pty step. The order is inside
  `carry_out`, and the pty can't tell a reorder's `authentication failed` from timing.
- **M3: move-only, along the seams below.** Each file ends at ≤ 400 production lines (`wc -l` up to the first top-level
  `#[cfg(test)]`). Apply the seams in the order given, and stop at the first that brings the file under. If every named
  seam leaves a file over, **stop and report the counts**. A new seam is a spec question.

## Scope
**In:**
- **A1** (C1): the event-level secret check, and the 18c box re-ticked.
- **A2** (M1): the first-launch shape above. Behaviour, words and captures are unchanged.
- **A3** (M2): `esc` in the clone's password box tested on a real pool.
- **B1** (M3): move-only splits, one commit per file, public paths unchanged (re-exported from the parent module):
  - graft `empty_state.rs` (838 at review) → `empty_state/` beside its existing `tests.rs`, `palette_tests.rs`,
    `sweep_tests.rs`:
    1. `empty_state/clone.rs`: `CloneState`, `CloneRun`, `clone_request`, `clone_started`, `outside_job`,
       `clone_prompt_key`, `clone_view` (≈ 260);
    2. `empty_state/open_prompt.rs`: `o`'s path resolution (`~`, relative to the folder) and `⇥` directory completion.
  - graft `event_loop/places.rs` (607 at review, more after 19b):
    1. `places/outside.rs`: `OutsideJobs`, `carry_out`, `Left`, `leave_outside` (the empty-state place's loop);
    2. `places/practice.rs`: 19b's `Practice` slot and the `Exit::Practise` / `Exit::LeaveTour` arms' helpers.
  - graft-ui `empty_state.rs` (763 at review):
    1. `empty_state/clone_prompt.rs`: `CloneView`, `CloneShown`, `window`, `into_line`, `draw_clone_rows`;
    2. `empty_state/bar.rs`: `bar_keys` and its shedding (19b's `t` first-shed rule with it).
  - graft `update/credential.rs` (467): `credential/core.rs` takes the host-agnostic core: `PromptKey`, `open_prompt`,
    `close_orphaned`, `prompt_key`, `prompt_paste` (MAP `## update::credential` last row). The four wrappers stay.
- **B2**, a **separate commit** (m6): `carry_out`, `outside_job`, graft-ui `bar_keys` and `draw_prompt` each ≤ 60
  lines, by naming their steps as functions in the same order. No behaviour change.

**Out** (rows in NOTES, for the conductor): minors m1–m5 and m7, questions Q1–Q2. Line counts in those rows are at
`ce6fdfdf`. Part B moves them; the builder lists the new file:line for each in NOTES.

## Signatures
```rust
// graft::first_launch — the per-process switch; after 18e it lives in the shell only
pub(crate) struct FirstLaunch { /* Arc<AtomicBool> + dir, unchanged */ }   // loses PartialEq: no state holds it
impl FirstLaunch {
    /// `Some(dir)` while the line still shows this process: what every place's state is seeded with.
    pub(crate) fn line(&self) -> Option<PathBuf>;
    pub(crate) fn end(&self) -> Option<PathBuf>;   // unchanged: `Some(dir)` once, then `None`
}

// graft::update — AppState field (was FirstLaunch)
pub(crate) first_launch: Option<PathBuf>,         // Some(dir) = line shown; plain value
// graft::empty_state — OutsideState field (was FirstLaunch) and one new step
pub(crate) first_launch: Option<PathBuf>,
pub(crate) enum OutsideStep { /* … */ FirstLaunchDone(PathBuf) }   // first palette open; `places` carries it
// graft::event_loop — Arrival field (was FirstLaunch)
pub(crate) first_launch: Option<PathBuf>,
// Command::FirstLaunchDone(PathBuf): unchanged. Its carry calls the shell's `FirstLaunch::end()` first.
```
`view` reads `state.first_launch.is_some()` where it read `is_shown()`. Part B: no item changes, only paths of private
modules.

## Read first
- `docs/reviews/m8-clone.md`: C1, M1, M2, M3, minor m6.
- `ARCHITECTURE.md` :279-305 ("`AppState` … must stay pure").
- `.scratch/m8-polish-and-ship/issues/18d-first-launch-line.md` NOTES "Deviations" (why the `Arc` was shared).
- `.scratch/m8-polish-and-ship/issues/18-rulings.md` R6 (the `esc` rows, the words `clone stopped · nothing kept`).
- `crates/graft/MAP.md`: lines 6-7 (`empty_state`, `first_launch`), `## event_loop` `places::drive` (:105),
  `running` split (:991), `## update::credential` (:416-426), `jobs` `clone` (:1081). `crates/graft-ui/MAP.md`
  `empty_state`.
- 19b's NOTES in `.scratch/m8-polish-and-ship/issues/19-tour-practice-repo.md` (what it added to `drive` and `Arrival`).
- Then source: `first_launch.rs`, `update/palette/input.rs`, `event_loop/places.rs`, `running/carry.rs`,
  `jobs/clone/tests.rs`. Part B: the four files it splits.

## Touches (check each; say how)
- **A1**: the 18c ticket box (:46-47) and the gap `18c-secret-events-level` (the conductor closes it, naming A1's
  test). `m8_walk/clone.rs` itself is unchanged.
- **A2**:
  - every `first_launch` site: `rg -n 'first_launch|FirstLaunch' crates/graft/src`. List them in NOTES.
  - the places that read it: `view/frame.rs` `status_row_notice`, `startup::arrive`, `running::carry` (the
    `FirstLaunchDone` arm), `places::drive` (`for_this_launch`, each `Arrival`, 19b's practice arrival).
  - graft-ui `EmptyStateView::first_launch: bool` is unchanged.
  - Captures `docs/shots/m8/first-launch-120x36.txt` and `…-empty-state-120x36.txt` must pass **unchanged**, with no
    `GRAFT_UPDATE_SHOTS`. A diff is a defect: stop.
  - The pty seed (`pty::Isolated`, `graft_sandbox.sandbox_env`) is untouched.
  - A tour arrival (19b) seeds from `line()` like any other place. Check that it does not show the line again after
    `done`.
- **A3**: `HttpRemote` (graft-fixtures, read-only use). `m4_walk` `password_prompt_esc_cancels` is untouched.
- **B**:
  - `cargo nextest list --workspace` is identical before and after each move commit.
  - Every `use` path outside the moved modules is unchanged: `git diff --stat` touches only the split files, their
    new children, and the MAPs.
  - Gap rows, 19b's ticket and the review cite old file:lines. List the new location of each m1–m6 site in NOTES.
  - Captures are unchanged.
- No keymap, journal, undo or graft-ops change. No change to ARCHITECTURE.md: this restores its rule.

## Acceptance (each tick names its test or check inline)
- [x] A1, events: `jobs::clone::tests::child_asks_through_the_pool_and_clones` asserts that, for every event it
      receives (the credential ask included), `format!("{event:?}")` doesn't contain the test password. **Seen red**:
      a probe event whose payload string holds the password fails it. Record the failing line in NOTES, then remove
      the probe. (`jobs::clone::tests::child_asks_through_the_pool_and_clones`, run via its parent
      `asks_through_the_pool_and_clones`; red line in NOTES "A1")
- [x] A1, 18c box 2: amended to name what is scanned, then re-ticked with both tests. (18c :46-50, naming
      `jobs::clone::tests::child_asks_through_the_pool_and_clones` and `m8_walk/clone.rs::clone_asks_for_a_password`)
      The text: "secret absent from
      every event's `Debug` (`jobs::clone::tests::child_asks_through_the_pool_and_clones`) and from pty frames,
      stderr and the new `.git` (`m8_walk/clone.rs::clone_asks_for_a_password`)". The box no longer says
      `no_secret_leaked`.
- [ ] A2, purity: `update::tests::first_launch::update_leaves_its_input_state_alone`. With `s0.first_launch =
      Some(dir)`, `let before = s0.clone()`, `update(&s0, ?)` gives `None` plus `Command::FirstLaunchDone(dir)`, and
      `s0 == before`. **Written first against today's code, where it fails** (the shared atomic flips `s0`). Record
      the failing line in NOTES.
- [ ] A2, across places: `event_loop::places::tests::a_line_ended_in_one_place_stays_gone_in_the_next`. The palette
      opened in a repo place, then `Exit::Open` → next `Arrival::first_launch == None`. The same from the empty
      state into a repo.
- [ ] A2, empty state: `empty_state::palette_tests::opening_the_palette_ends_the_first_launch_line` asserts
      `OutsideStep::FirstLaunchDone(dir)` on the first open and no such step on the second.
- [ ] A2, guard: no `atomic` under `crates/graft/src/update/**` or in `empty_state*`. Extend
      `update_cannot_reach_the_world` if it scans source; else add `update_holds_no_shared_switch`. Name the test in
      NOTES.
- [ ] A2, unchanged and passing: `first_launch::tests::count_launch_table`;
      `update::tests::first_launch::{shown_until_the_palette_opens, gone_after_five_launches,
      a_problem_or_notice_wins_the_row, a_broken_file_warns_once_and_never_stops_the_app}`;
      `event_loop::tests::the_loop_writes_first_launch_done_and_says_a_failure`;
      it `first_launch::{a_terminal_launch_counts_and_shows_the_line_until_the_palette, headless_never_counts,
      first_launch_captures}`, the last with no `GRAFT_UPDATE_SHOTS` and `git status` clean.
- [ ] A3: `event_loop::places::tests::esc_in_the_clone_password_box_stops_the_clone`.
      - Setup: an `HttpRemote` that needs Basic auth, and a clone started through `OutsideJobs` on a real pool.
      - Wait for the credential ask, bounded and event-driven, never a sleep. Then
        `carry_out(OutsideStep::Credential(job, None))`.
      - The prompt reads `clone stopped · nothing kept` (R6) and never `clone failed`. The target folder is absent and
        `OutsideState::credential` is `None`.
      - **Seen red**: swap the cancel and the refusal in `carry_out` and the test fails. Record the assertion line in
        NOTES, then restore the order.
- [ ] B1: each split file and each new child is ≤ 400 production lines (`wc -l` to the first `#[cfg(test)]`, every
      count in NOTES, plus which seams were needed). `cargo nextest list --workspace` before and after each move
      commit gives `diff` empty. A line-set check as 13l A1 did shows moves only.
- [ ] B2: `carry_out`, `outside_job`, `bar_keys` and `draw_prompt` are each ≤ 60 lines (counts in NOTES). The
      workspace `cargo nextest run` passes with no test file edited in that commit (`git diff --stat` of the
      commit shows no test file).
- [ ] B, MAPs: graft `empty_state` (:6), `first_launch` (:7, the new shape), `places::drive` (:105),
      `## update::credential`; graft-ui `empty_state`. Each names its new file. Checked by `rg` of each moved item in
      the MAPs (NOTES).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
One context per part:
- Part A: about 50k tokens.
- Part B: about 45k tokens (reading four files is most of it).

## NOTES — A1 (builder, 2026-10-08)
- Scout: none. The ticket names the one test; the change is in one test file.
- Shape: `jobs/clone/tests.rs` gains `next_watched(events, id, deadline, watch)`, which shows `watch` **every** event
  received, another job's and `Core`/`Rested` included, before the id filter. `next_for` now delegates to it with a
  no-op watch, so its other callers are unchanged. The child test passes a closure asserting
  `!format!("{event:?}").contains(PASSWORD)`. The scan runs before the `match`, so the `CredentialNeeded` ask and the
  `Cloned` answer are both scanned. The loop breaks on `Cloned`, so events after it are not received and not
  scanned.
- **Seen red:** a probe sending `Progress(id, RemoteProgress { brief: "leaked <PASSWORD>" })` on a clone of the
  sender failed at `crates/graft/src/jobs/clone/tests.rs:243:9` with `event 1 carries the password: Progress(JobId(1),
  RemoteProgress { brief: "leaked correct-horse-battery-staple" })`. The parent `asks_through_the_pool_and_clones`
  failed with it. Probe removed. The assertion is now at `:234`. Green after removal.
- The 18c box's second half was checked by reading, not run here beyond the workspace suite:
  `m8_walk/clone.rs::clone_asks_for_a_password` scans the pty output (`leaks(&output)`) and every file under the new
  `.git`. stderr is covered by `quit()` asserting it empty (`m8_walk/clone.rs:84`), which is stronger than a scan.
- Touches: `jobs/clone/tests.rs` (other `next_for` callers unchanged in behaviour: the watch is a no-op, and the
  suite passed); the 18c box (:46-50); gap `18c-secret-events-level` is for the conductor to close, naming
  `jobs::clone::tests::child_asks_through_the_pool_and_clones`. `m8_walk/clone.rs` unchanged. No production code,
  no MAP item (test-only helper).
- Gates (worktree's own target): fmt ok; clippy `--workspace --all-targets -D warnings` ok; doctests ok; nextest
  6020/6021. **The one failure is not A1's**: `published_repo::nothing_a_commit_would_carry_names_this_machine` names
  `docs/log/m8.md:330` (a home-dir path), a file last changed by base commit `a9bf9e0c` (conductor, "gaps+log: 20
  remainder"). Outside this ticket's files, so not fixed here. Fix: `cargo run -q -p graft-fixtures -- redact
  docs/log/m8.md`.

## NOTES — proposed KNOWN_GAPS rows (planner, for the conductor)
Lines are at `ce6fdfdf`. 19b and 18e B move them, and 18e's builder lists the new location of each.

| Id | Row | Status | Route |
|---|---|---|---|
| M8/clone-open-fails-stuck | m1: a clone that lands but can't be opened leaves the prompt `Running`. `outside_job` returns `Cloned` without changing `run` (`graft/src/empty_state.rs:242-250`); a failed open only sets `problem` (`event_loop/places.rs:575-580`); `Running` swallows every key but `esc` (`empty_state.rs:644-650`), which cancels a finished job. Way out is `⌃C` only. Fix: failed open → `run = Ended { words }`, plus a `places::tests` case | broken (untested) | 21, after 18e B |
| M8/palette-offers-in-missing | m2: `outside_source` has no `missing` gate (`update/palette/listing.rs:21-31`), so the palette offers `c Clone…` and `i Init` in a folder that does not exist and `⏎` does nothing. Breaks R6 and §1.15 (`01-view-mockups.md:482`). `no_clone_in_a_missing_folder` checks bar and key, not the palette | broken | 21 |
| M8/j15-literal-count | m3: `tests/it/m8_walk/clone.rs:150` passes literal `3` to `assert_within_the_note`; siblings count the sent array (`outside_repo.rs:40-46`, `palette_find.rs:38`) | broken (test weakness) | 21 |
| M8/first-launch-notice-arm | m4: `a_problem_or_notice_wins_the_row` (`update/tests/first_launch.rs:97-117`) never hits the footer-notice arm of `status_row_notice` (`view/frame.rs:135-149`). Add a `config_notice` case | unverified (coverage) | 21 |
| M8/clone-target-twice | m5: "usable URL" and "free target" are each defined twice: `empty_state.rs:155` vs graft-ops `is_empty_folder` (`graft-ops/src/clone.rs:209-211`); the `-` refusal in `folder_name` (`empty_state.rs:169-171`) and `clone()` (`clone.rs:149`). Fix: `graft_ops::clone::target(folder, url) -> Result<PathBuf, CloneError>` used by both | broken (duplication) | 21 |
| M8/clone-long-fns | m6: `carry_out` (`places.rs:519-584`, ~65), `outside_job` (`empty_state.rs:210-271`, 62), graft-ui `bar_keys` (~70), `draw_prompt` (~65) | broken | **18e B2**: close when 18e B2 is ticked |
| M8/note-clone-interfaces-stale | m7: design note `m8-polish-and-ship.md:385` still reads `clone(params, hooks: RemoteHooks, stop: &StopFlag)` and `:327` `EmptyPrompt { Open(FieldView), ConfirmInit } // … adds Clone(..)`. R1–R3 and 18c's `EmptyStateView::credential: bool` replaced both | broken (textual) | 23 docs pass |
| M8/first-launch-hides-hints | Q1: the first-launch line replaces the graph's key hints for up to five launches (`view/frame.rs:135-149`; `docs/shots/m8/first-launch-120x36.txt` last row), which are the launches where he knows the fewest keys. The note says only "status row, muted" (:472) | unverified (question) | user, by eye, on the M8 contact sheet |
| M8/clone-url-token-shown | Q2: `https://user:token@host/x.git` is drawn unmasked in `clone › …` and kept in the field after a failure. Git writes it to the new `.git/config` (git's own behaviour). Whether git's failure words echo it is not checked. Planner's lean: mask the `:secret@` part in the drawn field only | unverified (question) | conductor → planner ruling, then 21 |

**Closing (the conductor's job, not rows):**

| Row | Closed by |
|---|---|
| `18c-secret-events-level` | 18e A1, naming `jobs::clone::tests::child_asks_through_the_pool_and_clones` |
| `M8/11-files-over-size`, the graft `empty_state.rs` part | 18e B1, with the counts |
| `M8/look-file-sizes`, the graft-ui `empty_state.rs` part | 18e B1, with the counts |
