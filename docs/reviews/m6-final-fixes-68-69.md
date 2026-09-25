# Review: m6-conflicts group B: 68 (resume words say what happened) and 69 (a box owns the bottom row)

Reviewed at `9d7a5011` (merge of `685f8b07`), branch diff `79553aab...685f8b07`, in a detached worktree with its own
`CARGO_TARGET_DIR`. Source: `docs/reviews/m6-usability-final.md` N2, N5, n3, Q4. Colour is **unverified** (rule 5).

## Verdict

- **68: cause fixed.** `AppState::resuming` keeps the `Resume` that was sent until its answer (`update/state.rs:93`,
  set at `update/banner.rs:596`, taken at `:695-700` under one guard). `closing` keys on the `Resume` sent and the
  entry's status (`banner.rs:777-805`). The failed-continue-reads-as-abort trap the scout found is pinned:
  `jobs/operate.rs` `a_resume_git_fails_answers_failed`.
- **69: cause fixed.** The drift answers end their own `Busy` (`update/undo.rs` `undone` and `reverted_to` `Drift`
  arms). graft blanks the rungs while a modal is open: `view/status.rs:108-139`, read at `view/rows.rs:51,92-94,
  174-176` and `view/frame/bars.rs:33,271-273`. graft-ui is unchanged.
- **Conductor's decision 6 is built and tested.** `update/name_prompt.rs:256-265` asks `preview::preview_refusal`
  before closing the prompt, and routes the refusal to `NamePrompt::refusal`. The typed name is kept and
  `AppState::status` is not written. Verified by `a_fetch_in_flight_refuses_inside_the_stash_branch_prompt`, which uses
  a real `f` launch and passed in my run. The git-mid-operation arm (`preview.rs:295-297`) has no test inside the
  prompt (Minor 6).
- **The `search` rung is blanked** (`rows.rs:92,174`, through `modal_blanked`). No test asserts it (Minor 6).
  `reveal` is left alone, and NOTES explains why.
- **Moved credential captures.** The three in-app captures are right: the bottom row is `⏎ send  esc cancel`, and
  their headers were reworded. However, the m6 contact-sheet caption for one of them still describes the old row
  (Major 2). The M4 pty captures were not regenerated (Minor 5).
- **Swallowed errors in code:** none found. The new early returns (`resumed` stale guard, `refuse`'s `if let`) are
  guards on states that cannot hold, and each is commented. **At the screen, an error can now go unseen:** see Major 1.

## Spot-check of ticks

- **68:** 9 of 11 ticks checked, 9 held. Checked: all seven test ticks, both real-repo ticks and the capture. For
  each, the test exists, asserts what the tick says and passed in my run. MAP and NOTES were not checked.
- **69:** 12 of 13 ticks checked, 12 held. Checked: every test tick, the Q4 pin, both capture ticks, and the
  `fix-box-takes-y` assertion at `remote_by_key.rs` (now `assert_eq!` on the box's keys). MAP was not checked.
  `a_key_that_keeps_a_modal_open_leaves_the_status_alone` holds as ticked, but it is weaker than its Touches line
  claims (Minor 6).
- **False ticks: none found.**

## Gates (run one at a time, worktree at `9d7a5011`)

| Check | Result |
|---|---|
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass (22 s) |
| `cargo clippy --all-targets --features heavy-tests -- -D warnings` | pass |
| `cargo nextest run` | 3286 passed, 1 skipped. **183 s** wall; a second run took 204 s. Build 49 s |
| `cargo test --doc` | pass |
| Heavy tier `cargo test --features heavy-tests -- --test-threads=2` | pass, 485 s wall, peak RSS 978 MB |
| Release perf run (`frames::` + `heavy::budgets_hold`) | 13 passed, 147 s. Worst frame p99 **8.47 ms**, max **9.16 ms** (budget 16 ms). First paint: 20.8 ms with a commit-graph, 130 ms without, 76.6 ms with 50k refs. Peak RSS at paint ≤ 114 MB |

- **Slowest 5 tests:** `m5_walk::every_scenario::merge_resolved_with_the_keys` 12.8 s;
  `conflict_ops::whole_file_shapes_resolve_like_git` 12.3 s; `diff_fixture::parse_unified_matches_numstat_for_every_shape`
  11.8 s; `history_ops::every_preview_refuses_exactly_what_check_refuses` 11.5 s;
  `undo_cancel::cancel_at_every_step_boundary_is_clean` 10.5 s. None of them come from this diff; the new tests
  take milliseconds, and the slowest is `a_modal_owns_the_bottom_row` at 0.26 s.
- **Peak memory:** fast suite 272 MB largest single process; build 977 MB. **`target/`:** 6.5 GB (debug plus
  release, my own directory).
- **Fast-suite time is at or over the 3-minute budget** (TESTING.md tier table). A builder (ticket 76) was running on
  the same machine throughout, so both figures are contaminated and neither is a clean measurement. This diff did not
  cause it.
- No fuzz or bench targets were touched by this diff; I did not run them.

## Critical

None.

## Major

**1. A status that arrives while a modal is open is never drawn. A failure is replaced unseen; a `Done` is
acknowledged unseen.** (Verified by a throwaway test in my scratch worktree, since removed.)
- **Where:** `view/status.rs:133-139` (`bottom_row_problem`) and `view/frame/bars.rs:271-273` (`footer_notice`)
  hide every status while a modal is open. `update/keys.rs:122` calls `acknowledge` before any modal claims the key,
  and `:106-111` turns a `Done` into `Ready`. `update/operate.rs:128` (`launch_with`) writes `Busy` over whatever
  status stands.
- **Observed:** with the steps `f` (fetch running), `b` (name prompt open), then the fetch's
  `Operated::Failed { "fetch failed: could not resolve host" }` arriving, the state is
  `Sticky { "fetch failed: …" }`, but the drawn row is `⏎ create  ⇥ toggle  esc cancel`. Typing `topic` and then `⏎`
  leaves the status as `Busy("branch topic at 0303030")`. The fetch failure was never on screen. A `Done` that arrives
  (`fetched`) is turned into `Ready` by the first key typed into the prompt, so it is never shown either.
- **Why it matters:** Decided 2 says the modal "hides; it never clears". That holds only for a status that stood
  *before* the modal opened. A status that *arrives* under it is cleared unseen, either by the any-key
  acknowledgement or by the modal's own act. Before 69 that failure was drawn on the row. This breaks CLAUDE.md
  rule 3 at the screen and H1. The builder's own scenario (`b` over a running `f`, `a_busy_of_another_job_survives_a_box`)
  is exactly how to reach it, and no test covers a job answering while the modal is up.
- **Fix (spec, via the conductor):** two options.
  - Do not acknowledge a status the user cannot see: `acknowledge` skips while `modal_owns_bottom_row`.
  - Or keep a failure that arrived under a modal and draw it once the modal closes, even when the modal's act writes
    `Busy`.

  Add a test: a job answering `Failed` and then `Done` while the name prompt is open, then `esc`: the line is shown.

**2. The m6 contact sheet's caption for `m4-credential-prompt-in-app` now says the opposite of the capture.**
- **Where:** `docs/shots/m6/m6-contact-sheet-captions.tsv:122`, repeated at `docs/shots/m6/m6-contact-sheet-3.txt:463`.
  The caption says the `80x24 · username` frame's bottom row "reads `push main → origin/main · esc cancels`" and
  that "the two 120x40 frames are unchanged".
- **The capture:** since 69 all three frames read `⏎ send  esc cancel` (`docs/shots/m4-credential-prompt-in-app.txt:52,
  94,136`).
- **Why it matters:** the user reviews by that line (ADR-0037, rule 6). A "what right looks like" line that contradicts
  its picture is how a wrong picture gets approved, or a right one gets rejected. It also reverses M6 fix F9 without
  saying so. F9 put the push's line on that row on purpose, so the user could see which push the question belongs
  to. The border now says only `esc cancel the push`, and not which push (`main → origin/main`). 69's Touches line
  ("m6 captions: … rows for `fix-box-keeps-on-y` and `fix-box-takes-y-120x40` reworded") missed this row.
- **Fix:** reword row 122 to the new row, say F9 is superseded by 69 Decided 1, and rebuild the sheet. Ask whether
  the prompt's title or border should name the push, since F9's information is now gone from the screen.

**3. Pre-existing, found this round: `⏎` in the plain branch or tag name prompt launches a second write over a
running fetch and takes `operating` from it.** (Verified by a throwaway test, since removed.)
- **Where:** `update/name_prompt.rs:277-296` calls `operate::launch`, with no in-flight check. `update/operate.rs:127`
  sets `operating = Some(new id)` unconditionally. `operate.rs:769-771` then drops the fetch's answer, because
  `operating` no longer names it. Decided 6's guard (`name_prompt.rs:260`) covers `BranchFromStash` only.
- **Observed:** after `f`, `b`, `topic`, `⏎`: `fetch = JobId(…809)`, `operating = Some(JobId(…810))`,
  `commands = [Operate(…810, BranchCreate { name: "topic", … })]`.
- **Why it matters:** two git writes run at once. Both can take ref locks. The fetch's result line and its refresh are
  lost, and `Esc` now cancels the branch create rather than the fetch. The keymap and state docs say one write at a
  time (`AppState::operating`: "every other write key is a no-op while this is `Some`").
- **Fix:** in `create`, refuse every kind with `operation_in_flight` into `NamePrompt::refusal`, as Decided 6 does for
  the stash branch, keeping the typed name. Add a test per kind. Not introduced by 68 or 69; route it through the
  conductor.

## Minor

1. **`, last commit skipped` is untrue when the skip that finishes a rebase was not the rebase's last commit.**
   - **Where:** `update/banner.rs:788-790` words every `(Committed, Skip)` this way. `graft-ops/src/apply/resume.rs:
     351-352` closes `Committed` for any skip after which git finished.
   - **Example:** three commits, a stop on the first, `s y`, and the other two apply cleanly. The line says
     `…, last commit skipped ✓`; the first commit was skipped.
   - `a_final_skip_says_it_skipped` uses `TwoStopRebase` at step 2 of 2, where "last" happens to be true.
   - **Fix:** `…, 1 commit skipped`, or name the skipped subject.
2. **The name prompt's refusal outlives its cause.** `wait for fetch … to finish` stays drawn as a problem line after
   the fetch has ended.
   - **Where:** `NamePrompt::refusal` is cleared only by a key or a paste (`name_prompt.rs:187,230`), and
     `modal_problem` (`view/status.rs:121-127`) draws it unconditionally.
   - This breaks CLAUDE.md "a status must never read as an error when nothing is wrong". Combined with Major 1, the
     fetch's `fetched` never shows either.
   - **Fix:** clear a `preview_refusal`-sourced refusal when `operating` or `in_progress` clears, or recompute it in
     the view.
3. **The commit-graph offer row is not blanked under a modal.** The row names keys the modal swallows.
   - **Where:** `view/rows.rs:95,177` (`commit_graph: state.commit_graph_line()`).
   - **Captures:** `docs/shots/m6/fix-box-owns-the-bar-80x24.txt` frame 3 shows `no commit-graph file · B build one ·
     X never here` above the drift box's keys; `docs/shots/m5-ui-credential-over-graph-160x48.txt:87` shows the same
     over the credential prompt.
   - The confirm box swallows `B` and `X` (`update/confirm.rs:138-139`). Under the credential prompt, `B` would be
     typed into the password.
   - Out of Decided 1's literal list, but it breaks the same principle (UX §3.2). **Fix:** `modal_blanked` it, or say
     why not.
4. **The credential prompt's keys are now in two places, and the two lists disagree.** The border reads
   `⏎ send  ⌃U clear  esc cancel the push`; the bar reads `⏎ send  esc cancel`
   (`docs/shots/m4-credential-prompt-in-app.txt:50,52`). This extends n3 (left out by Decided 5) to a fourth modal.
   **Fix:** name the credential prompt in n3's KNOWN_GAPS row.
5. **The M4 pty captures show the pre-69 row.**
   - **Where:** `docs/shots/m4-03-credential-masked-{80x24,160x48}.txt` and
     `m4-04-credential-username-{80x24,160x48}.txt` (row 26 or 50), plus `docs/shots/m4-contact-sheet.txt:247,279,335,
     367`, still show `push main → origin/main · esc cancels` under the prompt.
   - They come from the `m4_walk` example over a pty, not from a test, so 69's `git status docs/shots` check could not
     catch them.
   - The rebuilt m4 sheet now shows the same prompt with two different bottom rows.
   - **Fix:** re-run the M4 pty walk at the acceptance ticket, or mark the four files as predating 69 in their captions.
6. **Test gaps.**
   - (a) In `a_key_that_keeps_a_modal_open_leaves_the_status_alone`, the preview is a `Pending` plan
     (`view/tests/bottom_row.rs:225-247`), so 67's refused-confirm path is not exercised there, despite the Touches
     line "Covers the preview after 67". The test also runs at 120x40 only, and its `checked > 0` bound (`:412`)
     would pass with one key.
   - (b) No test shows the `search` rung blanked under the preview. That is the one modal where it would show, since
     a box's bar covers the footer.
   - (c) Decided 6's git-mid-operation arm (`preview.rs:295-297` → `live_refusal_words`) is not tested inside the
     prompt.
7. **File sizes (PROMPT.md 6.2, about 400 lines).**
   - Grew further: `update/banner.rs` 804→865, `view/rows.rs` 739→759, `update/preview.rs` 1284→1292,
     `update/state.rs` 904→911, `update/undo.rs` 1496→1511.
   - New: `view/tests/bottom_row.rs` 413.
   - `jobs/operate.rs` 348→400, because a 52-line test module was added inline rather than as `operate/tests.rs`.
   - KNOWN_GAPS rows M6/31 and M6/11+12 now carry stale numbers. **Fix:** update the rows; the split stays in M7.

## Q4 (already filed as Major M6/q4; new facts only)

- **The drift box promises what `u` then denies.** The box says "A backup is taken first, so this is itself
  undoable." (`update/undo.rs:500,527`; capture `fix-box-owns-the-bar-80x24.txt` frame 3). Once the undo has run, the
  pinned second `u` answers `nothing to undo`, drawn with a tick (`tests/it/undo_over_drift.rs:80-89`). The Q4 spec
  should decide which is true, and word both the box and that answer to match.
- `nothing to undo ✓` draws a success tick for a key that did nothing. Worth a line in the same spec.

## Touches, checked

- **68:**
  - `grep -n "entry.description" banner.rs` finds one hit, inside `closing` (`:780`). True.
  - `operating`'s other clearers: `undo.rs:291,429`, `operate.rs:45,772` and `remote/in_flight.rs:86` match the
    NOTES. True.
  - Shell-merge capture unchanged: true (not in the diff).
- **69:**
  - The captures listed as moved are the ones in the diff. True.
  - Missed: the m6 caption row 122 (Major 2) and the M4 pty captures (Minor 5).
  - "graft-ui: none" is true; the diff touches no graft-ui file.
