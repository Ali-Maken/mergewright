# Review: m6-conflicts tickets 11 and 12 (group)

Range `0b5019c..dd2a758`, with `2aeab64` (review fix M1, graft-ops restart) left out. Read-only. Reviewer: Opus, 2026-09-24.

Tickets:
- `.scratch/m6-conflicts/issues/11-preview-verb-confirms.md`: commit `a9bad4d`, merged `c6d26ab`.
- `.scratch/m6-conflicts/issues/12-landing-after-a-stop.md`: commit `fa829ab`, merged `dd2a758`.

**Count: 0 Critical · 3 Major · 11 Minor.**

## Gates

All run one at a time, on `main` at `c72b2a4`. Its `crates/`, `Cargo.toml` and `Cargo.lock` are identical to `dd2a758` (`git diff --quiet dd2a758 HEAD`). The ticket 23 builder was running its own suite (`graft-target-23`) on the same machine throughout.

| Gate | Result |
|---|---|
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass |
| `cargo clippy --all-targets --features heavy-tests -- -D warnings` | pass |
| `cargo nextest run --workspace` | 2754 passed, 1 skipped. **127.5 s** wall (`/usr/bin/time`), nextest's own time 118.3 s. |
| `cargo test --doc --workspace` | 3 passed, 0 failed |
| Heavy tier: `cargo test --features heavy-tests -- --test-threads=2` | 2789 passed, 0 failed, 1 ignored. **327.8 s** wall. Max RSS 1.08 GB (process tree). |
| Release perf run (`frames::` + `heavy::budgets_hold`) | **1 of 13 failed on the first run:** `frames::scrolling_the_100k_fixture_keeps_input_to_frame_under_16_ms_at_the_unicode_tier`. Key 197 took 17.35 ms. It was queued 16.24 ms behind a 1.52 ms window frame, and its own frame was 1.11 ms. I re-ran that one test alone to diagnose it, and it passed: p99 8.34 ms, max 8.75 ms. This matches KNOWN_GAPS row 211 (the release p99 is too noisy to trust) plus the concurrent builder load. Neither ticket touches the scroll path: the only new work there is `GraphView::stopped`'s `None` check in `footer.rs`. **Reported, not waved through:** the first run is a real miss. |
| Fuzz, benchmarks | not run: none is scoped to these tickets. |

Release numbers from the first run:
- `frames::`, Unicode:
  - 100k: p99 8.38 ms, max 17.35 ms (**failed**).
  - 50k refs: p99 8.60 ms, max 9.20 ms.
  - diff scroll: p99 8.58 ms.
  - pane open: p99 8.75 ms.
  - map view: p99 8.30 ms.
- `frames::`, Pixel:
  - 100k: p99 9.13 ms, max 15.15 ms.
  - 50k refs: p99 8.59 ms, max 11.19 ms.
- First paint:
  - 100k with a commit-graph: 25.6 ms.
  - 100k without one: 104.8 ms.
  - 100k with 50k refs: 77.2 ms.
  - realistic, with a commit-graph: 92.5 ms.
  - realistic, without one: 115.2 ms.
  - Peak RSS for all five: 30.6 MB to 113.4 MB, all under the 300 MB bar.

Fast suite:
- The 5 slowest tests, from a second nextest run with `--status-level pass`:
  - `graft-git diff_fixture::parse_unified_matches_numstat_for_every_shape` 9.0 s
  - `graft-ops conflict_ops::whole_file_shapes_resolve_like_git` 8.3 s
  - `graft-ops undo_cancel::cancel_at_every_step_boundary_is_clean` 7.0 s
  - `graft-graph …scrolling_the_whole_history_keeps_the_resident_set_bounded` 6.8 s
  - `graft m5_walk::every_scenario::merge_resolved_with_the_keys` 6.6 s
- Peak RSS 273 MB (`time -l`, over the whole nextest process tree).
- `target/` is 17 GB.

## Tick spot-check

- **Ticket 11:** checked all 11 ticks; 11 held. The unticked `a-merge-to-stop` journey line is left to the acceptance ticket, which is correct.
  - One is weak but held: `the_opening_key_again_confirms` (`preview/tests.rs`) asserts Cancel on one freshly built state, then presses `m` on a second, separately built one. The fixture is deterministic, so they are the same state. It would read better as one state.
  - The real `m m` in `tests/it/preview.rs::m_again_merges_through_the_real_path` moves `HEAD`. Held.
- **Ticket 12:** checked all 14 ticks; 14 held. Two "what right looks like" lines word the row's position wrongly (Minor 6), but their tests assert the right row.
  - `continue_lands_on_the_new_commit` injects the `Resumed(Done)` answer and does not press `c`.
  - The real `c` is covered by `landing_shots::capture_after_continue_on_a_rebase`, which asserts that the cursor is on `git rev-parse HEAD`.

### Deviation 4: `the_stop_line_outlives_a_dismissed_problem`

- **The test does check what the tick now says.** The tick was rewritten at tick time ("As built:"). `banner/tests.rs` runs the stop, then `Resumed(Failed)`, and asserts:
  - the problem takes the row (the control);
  - after `Esc`: `Ready`, `!banner_focus`, `stopped.is_some()`;
  - after one `Tab`: the banner is focused, and the row at 120 starts `merge stopped · 1 file conflicted`.
- **It discriminates.** On the pre-12 code the stop was a `Sticky` that `Failed` overwrote, so there would be no reason after the `Tab`.
- **The contradiction is real, and it is the spec's.**
  - "Today's behaviour for `Esc`" is the pre-12 `hand_the_keyboard_back`, which cleared any `Sticky` for every key and left the banner in the same press.
  - Scope item 3's accepted consequence drops the reason beside a laddered bar at 120.
  - So "the row starts with the reason right after `Esc`" cannot hold, and nothing in the build claims it does.
- **What the user sees:**
  - After a failed resume and `Esc`, the 120-column row is `⇥ banner  m merge  …`, with no reason (`docs/shots/m6/fix-landing-after-esc-120x40.txt`, last row).
  - The header's `MERGING` and the `⇥ banner` lead are the only signs on screen.
  - The stop's words come back after one `Tab`.
- The design note still draws the reason there (Minor 7).

## Pipeline

- **Need:** `docs/design-notes/m6-conflicts.md:222-232`.
- **Prior art:** `:234-248`, citing `docs/research/conflict-resolution.md` (the file exists). Rows cover:
  - landing on a stop;
  - landing on launch;
  - keys at the point of use;
  - the way back after `Esc`;
  - confirming a preview (P1).
- The `y` key was a user ruling (`docs/field-notes.md:58`), filed after the study. That is fine; it is recorded.
- **Journeys:** `:250-279`, and Amendment 2's `journey-counts` at `:473-490`.
- Built counts, checked in tests:
  - `a-merge-to-stop` **2**: the opening `m`, then `m` again (`m_again_merges_through_the_real_path`).
  - `b-launch-to-first-conflict` **1**: one `⏎` (`relaunch::tests::a_merge_begun_in_the_shell_lands_on_the_banner`).
  - `back-after-esc` **2**: `⇥ ⏎`, from the banner (`esc_on_the_banner_leaves_a_way_back_on_screen`).
  - All within the note's counts.
- `a-stop-to-first-conflict` (0, P5) is not in this group.

---

## Critical

None.

## Major

### M1. At the user's 187 columns, the launch row cuts the file count to one digit: `· 1…` for 13 files

- **Where:** `crates/graft-ui/src/graph_screen/hints.rs:1326` (`fit(reason.as_str(), room)` inside `stopped_row`).
- **Evidence:** `docs/shots/m6/fix-landing-b-launch-187x50.txt`, last row:
  ```
   merge in progress · begun outside Graft · 1… · ⏎ resolve  c continue  a abort  M mergetool  e edit  ⇥ graph
  ```
  The pane in the same frame reads `CONFLICTS (13)`.
- **What's wrong:**
  - The launch opens the changes pane (`land_on_the_work`), so the footer is only the graph column's width, about 108 cells at 187.
  - The bar takes about 62 of those, which leaves 43 for the reason.
  - `fit` cuts characters from the end, so the count goes first. The constant words `begun outside Graft` survive.
  - A single digit can survive the cut, and it reads as a different number: `1…` for 13.
- **Why it matters:**
  - 187 is his terminal (`docs/field-notes.md:57`), and a launch with the pane open is the J3 landing's normal case, not an edge.
  - The design note's J3 row (`m6-conflicts.md:260`) promises `1 file conflicted` on that row.
  - A number that disagrees with the picture is worse than no number.
  - The 120-column capture reads `13 files conf…`, which happens to be readable.
- **Fix:**
  - Cut the reason at `·` boundaries. Drop whole parts, lowest value first: `begun outside Graft`, then `from a previous run`, then the step. Keep the count part, or drop it whole; never cut inside it.
  - Test at 108 cells with 13 files.

### M2. File size: ticket 12 put a new concern into `update/banner.rs`, which was already 60% over the limit

- **Where:** `crates/graft/src/update/banner.rs`. PROMPT.md 6.2 says files should be under about 400 lines. Growth over this range:
  - `update/banner.rs`: 636 → **796**.
  - `update/preview.rs` (ticket 11): 1181 → **1241**.
  - `update/state.rs`: 729 → 783.
  - `view/rows.rs`: 655 → 683.
  - `graph_screen/hints.rs`: 1256 → 1357.
- **What's wrong:**
  - None of `banner.rs`, `preview.rs`, `state.rs` or `rows.rs` is named in a size row. KNOWN_GAPS M5/18, M6/06, M6/07 and M6/10+22 cover other files; `hints.rs` is in M6/10+22.
  - About 170 new lines in `banner.rs` (`:574-793`) are a separate concern from the banner's keys: `stopped_again`, `finished_lands_on_head`, `refs_answered`, `walk_started`, `land_on_head`, `stop_reason`, `stop_line`, `one_tab_to_banner`.
- **Why it matters:** readability is a hard rule (CLAUDE.md), and every group has so far added to the same overlong files.
- **Fix:**
  - Move the stop's wording and the head landing to `update/banner/stop.rs`, or into `update/relaunch.rs`, which is 185 lines and already owns `LiveOrigin`.
  - Name `preview.rs`, `state.rs` and `rows.rs` in a size row, or split them with `codebase-design`.

### M3. Readability and docs drift in the touched code

- **`update/mod.rs:539-542`:**
  - `took_fact`'s doc comment ("A fact about the repository beside the graph arrived: …") now sits stacked on top of `working_tree_read`'s own two lines.
  - `took_fact` (`:580`) has no doc comment at all, and `working_tree_read`'s rustdoc opens by describing a different function.
  - Fix: move the first two lines back above `took_fact`.
- **`crates/graft/MAP.md:224`:** it still says that `Conflicted` "sets a **sticky** line worded as a state rather than an error (`conflicted_line`)".
  - `conflicted_line` is gone. `Conflicted` now sets `AppState::stopped` and leaves `Status::Ready` unless git gave a stalled reason.
  - The same MAP's `stalled_line` entry is correct, so the file contradicts itself.
- **`tests/it/conflict_interim.rs` `capture_banner_for_docs_shots` (~`:1534-1540`):** the `Tab` round trip is now "kept so the shot is taken the way it always was".
  - These are dead steps whose stated purpose no longer exists.
  - Fix: remove them and re-check the shot, or say what they still guard.

## Minor

1. **`y confirm` is drawn on boxes where `y` does nothing.**
   - Where: `update/preview.rs:459-468` (`footer_keys`) sets `confirm_key` and `again` for every `Operation` subject. That includes:
     - `closing_box` (preview failed, or undrawable; its only choice is `Close`);
     - a plan that cannot act.
   - `confirm_by_key` (`:452`) swallows `y` in both cases.
   - It breaks `02-keymap.md:111`: "A key the menu displays must actually work when pressed". The ticket's scope item 4 asked for it, so this is a spec gap, not a builder error.
   - **Proposed change:** set `confirm_key`/`again` only when `acts()` or `planning()`. While planning the key is only waiting, not dead.
2. **No test for the `!fits` branch of a confirm key** (`update/preview.rs:446-448`; scope 2's first bullet). Add `y` and `m m` below `PREVIEW_MIN`: no command, box open.
3. **`land_on_head` drops the landing silently in two places, with no comment saying why that is safe (rule 3).**
   - `banner.rs:717-719`: no history, so `return`.
   - `banner.rs:721-723`: `awaiting` is `None`, so the seek's request is dropped.
   - A related question (rule 7: the lines are given, the reachability is not shown): `history_started` returns at `mod.rs:515-517` for a superseded walk id, before `banner::walk_started` (`:536`). The only other clearer is a refs failure (`mod.rs`, `Refs(Err)` arm).
   - So a second refresh that replaces the walk before it starts (for example `Ctrl+R` straight after `c`) should leave `land_on_head` owed for good, and the landing silently lost.
   - Is that reachable? If it is, point `head_landing.walk` at the newest walk, or drop the landing when `awaiting` changes.
4. **Question: the landing also runs after `--abort` and a final `--skip`.**
   - `resumed`'s `Done` arm (`banner.rs:604-609`) serves all three resumes (`:591`: "A `--continue`/`--skip`/`--abort` came back").
   - So `a`, confirmed, also focuses the graph and seeks `HEAD`. Scope item 7 names only `c`.
   - It is probably right, but it is untested and unstated. Confirm it, and pin it with a test.
5. **The stop row's colour is not recorded as unverified.**
   - Where: `graph_screen/footer.rs:104` draws the whole row, the bar included, in `Token::Text` instead of `TextMuted` while stopped.
   - Rule 5: colour ships unverified. The ticket's gaps list only the reverse video.
   - Fix: add it to the colour-unverified gap row.
6. **Two "what right looks like" lines say "the last row", but the row they mean is higher up the frame.**
   - The 120x40 frames are stacked.
   - In `fix-landing-after-continue-rebase-120x40.txt` the graph footer is row 24 of 40, and the frame's last row is blank.
   - In `fix-landing-b-launch-120x40.txt` the last row is `zdiff3.txt    both modified`.
   - The tests find the row by its content, so they are right. The captions would send the user to the wrong row.
   - Fix: say "the graph's footer row". Ticket 12's capture files also carry no caption header; ticket 11's do.
7. **The design note is behind the build.**
   - `m6-conflicts.md:369` draws the reason after `Esc` at 120. The build drops it (scope 3's accepted consequence; `fix-landing-after-esc-120x40.txt`).
   - `:378` draws the preview footer without `y confirm`.
   - `:582-583` says "`.`-opened previews are unchanged". The `y` ruling superseded that.
   - `:305-315` still shows `Status::Stopped` and the old `stop_reason(state, origin, again)`. The ticket says those are superseded; the note does not.
   - Fix: mark each one as superseded in the note.
8. **The kill-9 relaunch captures and checklist show a row the app no longer draws.**
   - `TESTING.md:2219-2221` (scenario 14) says the line is "exactly `rebase still in progress from a previous run · ⏎ resolve  c continue  a abort`". `TESTING.md:2250` says the same.
   - `docs/shots/m6/m6-06-relaunch-sticky-{120x40,80x24}.txt` and `m6-contact-sheet.txt:419-497` show it too.
   - The walk now checks `… from a previous run · step …` (`m5_walk/resolved_operations.rs:420`), and the row gains the step, the count, `M mergetool  e edit  ⇥ graph`.
   - Fix: regenerate these captures in the acceptance run and update the two TESTING.md lines. The contact sheet is what the user judges by eye (ADR-0037).
9. **The builder's gaps, to file (ticket 12 NOTES "Gaps").**
   - **Broken, pre-existing:** the `ChangesPane` bar drawn with the diff focused offers `space stage` and `d discard`, which do nothing there.
   - **Missing:** there is no way-back hint while the pane's file list has focus.
     - This is path B's case: the launch opens and focuses the pane (the `m5_walk/gestures.rs` comment at ~`:234`), so after `Esc` on a shell-begun merge nothing names `⇥`.
     - `⏎` on the pane's conflicted row still reaches the resolver, so he is not stranded.
   - **Missing:** a merge begun in the shell mid-session lands only on an owed landing.
   - **Unverified:** the colour of the focused banner and the pane cursor. Add Minor 5 here.
10. **Not test-first.** The graft-ui half of ticket 12 (`stopped_row`, `OperationLive`, the footer rung) was written in the same pass as its tests. The builder disclosed it. No action beyond noting it.
11. **`the_opening_key_again_confirms` builds two states.** It asserts Cancel on one fresh state and presses `m` on another (`preview/tests.rs`, the first test under "confirm keys"). Fix: assert and press on one state.

## Architecture

- **Crate rules hold.** `graft-ui` gained `stopped_row` and `OperationLive` with no dependency on `graft`. The `graft` MAP names `view::rows::footer_context` as the one place that wraps.
- **Refs read off the UI thread.** `ReadRefs(JobId)` goes through `queue_as` on a worker (`jobs/runner.rs`).
- **Nothing blocks the UI thread.** `land_on_head` is pure: `History::seek` returns a request.
- **Tests only.** `unwrap`/`expect` appear only inside `#[cfg(test)]`.
- **Sanitised.** The stop's words (from `InProgress::name` and counts) go through `sanitize_line` in `view::rows::stopped_line`. `footer_keys` sanitises `m`/`merge`.
- `ARCHITECTURE.md` names none of the changed items. `dependency_edges::internal_dependency_edges_match_architecture_md` passed in the heavy run.
