# Review: m6-conflicts ticket 18 (a stop opens the resolver on the first conflicted file)

Ticket `.scratch/m6-conflicts/issues/18-stop-opens-the-resolver.md`: commits `5f3f95f` (partial) and `8196fdb` (continuation 1), merged `0a1cb92`. Diff `git diff 4a6703d 8196fdb`. Read-only against those hashes. Reviewer: Opus, 2026-09-24.

**Count: 1 Critical · 2 Major · 7 Minor.**

Conductor ruling (not re-reported): P5 approved, so the old banner-landing tests are updated to the new landing and none is disabled.

## Gates

All ran one at a time in a detached worktree at `8196fdb`, with its own `CARGO_TARGET_DIR`. `0a1cb92` has no `crates/` change from `8196fdb`.

| Gate | Result |
|---|---|
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass |
| `cargo clippy --all-targets --features heavy-tests -- -D warnings` | pass |
| `cargo nextest run --workspace` | 2849 passed, 1 skipped. Two runs: 107.6 s and 96.3 s (build excluded). Under 7.1's 3 minutes. Peak RSS 274 MB. |
| `cargo test --doc --workspace` | 3 passed |
| Heavy: `cargo test --features heavy-tests -- --test-threads=2` | 2884 passed, 0 failed, 1 ignored. 309 s wall, max RSS 410 MB. |
| Release perf (`frames::` + `heavy::budgets_hold`, `--test-threads=1`) | **Run 1: 12 passed, 1 failed.** `frames::fifty_thousand_refs_keep_input_to_frame_under_16_ms_with_the_map_view_open` failed on 1 key of 421: key 132 took 16.31 ms. It was queued 16.1 ms behind key 131, and its own frame took 0.17 ms. **Run 2 (same command, alone): 13 passed.** Frame p99 7.9–9.0 ms, max 8.6–9.6 ms. First paint: 15.1–37.3 ms in run 2, 20.5–118.6 ms in run 1. Load average was about 4, and other agents' cargo processes were running. See Minor 7. |
| Fuzz, benchmarks | Not run. This ticket touches neither the parsers nor the graph rows. |

`target/` is 18 GB (shared) and 9.5 GB (the review target dir).

The five slowest fast-suite tests:
- `graft-git::it diff_fixture::parse_unified_matches_numstat_for_every_shape`: 8.9 s
- `graft-ops::it conflict_ops::whole_file_shapes_resolve_like_git`: 7.9 s
- `graft-ops::it undo_cancel::cancel_at_every_step_boundary_is_clean`: 7.0 s
- `graft::it m5_walk::every_scenario::merge_resolved_with_the_keys`: 6.9 s
- `graft-graph layout::stream::tests::scrolling_the_whole_history_keeps_the_resident_set_bounded`: 6.7 s

## Ticks

**5 checked, 4 held.** I read every named test against its box.
- `operate::tests::a_conflicted_merge_lands_in_the_resolver`: **false (overstated)**. See Critical 1.
- `banner::tests::a_rebase_stopping_again_lands_in_the_resolver` (`banner/tests.rs:1012`): held. It checks the path is `b.rs`, `Banner`, focus and `OpenConflict`.
- `banner::tests::esc_from_a_landed_resolver_shows_the_banner_keys` (`:1048`): held. It checks the resolver is closed, `⏎ resolve` is on the row, and the row starts `merge stopped · 1 file conflicted ·`. Its `banner_focus` assert is vacuous (Minor 2), but the bottom-row asserts carry it.
- `operate::tests::an_edit_stop_with_nothing_unmerged_keeps_the_banner` (`operate.rs:2258`): held. It checks no resolver, `stopped`, and no `OpenConflict`. It does not assert `banner_focus`, but `operated` sets focus before the landing (`operate.rs:816`) whatever the paths are.
- Capture `landing_shots::capture_a_merge_that_stops`: held. It asserts `MERGING` on row 1, `▸ <first>` and a bar starting `o yours`, at 80, 120 and 187.

**Journey `a-stop-to-first-conflict` (unticked):** this is correct. The box says "(acceptance ticket)". `21-acceptance-conflict-journeys.md:42` owns it: "`a-stop-to-first-conflict`: 0 keys. The frame right after the stop is the one above." The design note's count is 0 (`m6-conflicts.md:486`).

## The ~30 old-landing tests

I sampled 11 change sites. None was disabled or deleted. Ten were updated with their assertions intact. One had its assertions replaced (Minor 1).
- `conflict_interim.rs:152-158` `stopped_on_the_conflict`: it now lands, presses `Esc`, and asserts the resolver is closed. Its ~21 callers are unchanged, and so are their assertions. I read `conflict_focuses_the_banner_and_lists_the_files` (`:288`): all its asserts are still there.
- `banner/tests.rs:1064` `a_stop_keeps_the_banner_keys_on_the_bottom_row`: `Esc` first. Its asserts at 80, 120 and 200 are unchanged.
- `banner/tests.rs:1089` `the_stop_line_outlives_a_dismissed_problem` and `:1147` `esc_on_the_banner_leaves_a_way_back_on_screen`: `Esc` first. Nothing else changed.
- `resolver/file_list.rs:454-461` `at_step_two`: it waits for the landed resolver instead of Tab-ing to it. At `:489`, "may or may not have opened the resolver; cope with either" became `assert!(state.resolver.is_some())`. That is **stronger**.
- `m5_walk/conflicts.rs:43-46`, `:377-380`, `:388-391` and `regressions.rs:443-446`: `WaitFor(ResolverLanded), Esc` before `BannerFocused`. Every later step and `See` is unchanged.
- `landing_shots.rs:49-85` `capture_a_merge_that_stops`: this was **repurposed**. Its banner assertions were removed, not moved (Minor 1).

**The new `Esc` versus counted journeys.** At `8196fdb` nothing in `crates/` parses `journey-counts`: `git grep journey-counts -- crates` is empty. So no m5_walk scenario counts a journey. Each added `Esc` sits on the harness's path to the banner, not inside a journey. The design note accepts the one real cost: "abort from there is `Esc a`, 2 keys, for the rarer act" (`m6-conflicts.md:368`). `back-after-esc` (J7, 2) starts from the graph, so the change does not touch it. None exceeded.

## Critical

### 1. False tick: `a_conflicted_merge_lands_in_the_resolver` does not check two of its box's four claims
- **Where:** `crates/graft/src/update/operate.rs:2235` and `:2254`. Box: ticket 18, Acceptance line 1.
- **What:** the box says the test checks four things after `m m`:
  - the resolver opens on `first_conflicted`;
  - `opened_from == Banner`;
  - **the cursor is on block 0**;
  - **`file_cursor` is that file's index**.
- The test does not check the last two:
  - For the cursor, it asserts `open.cursor_block == None` (`:2235`), the opposite of the box, with a comment deferring to `answers.rs`.
  - For `file_cursor`, it asserts `0` before any list exists (`:2254`). That is `unwrap_or(0)`'s default (`resolver/files.rs:159`), so any path would pass.
  - It also calls `operated` directly rather than driving `m m`.
  - On `first_conflicted`: the state has no working tree, so the test compares against the answer's path. The two are equal by code: `graft-ops/src/apply/resume.rs:130-139` and `update/banner.rs:78-88` both filter `status.entries` by `unmerged`.
- **The behaviour holds. I verified it by running** a throwaway probe in my worktree, not committed. On the real two-file fixture, after `landed_in_the_resolver` and the set read, it gave `path=notes.md cursor_block=Some(0) file_cursor=0 listed=["notes.md","shared.rs"] unmerged=["notes.md","shared.rs"]`. The fault is the tick, not the code.
- **Why it matters:** CLAUDE.md says a false tick is Critical whatever its size. "Overstated" is the largest class in the false-tick sort (`docs/log/m6.md:87`).
- **Fix:** add `cursor_block == Some(0)` and `file_cursor == position of first in listed_files` to an integration test after the read and the set land. `capture_a_merge_that_stops` already waits for the read; its frame shows the set has landed too, but pump on `listed_files` to be sure. Re-tick naming that test.

## Major

### 1. A box opened while the operation runs turns the stop into a refusal nobody asked for, and the comment says this cannot happen
- **Where:** `crates/graft/src/update/banner/stop.rs:37-46`. The comment reads: "Its refusals (`operating`, a box) cannot hold: … a box would have refused the launch."
- **What:** a box can open **after** the launch. `c` on the graph opens the commit editor with no `operating` guard (`update/keys.rs:249-251` → `commit_editor::open`, `commit_editor.rs:279`). `operated` itself expects the editor to be open when the stop arrives ("The editor and its draft are left alone", `operate.rs:817-819`). `open_resolver` then refuses on `box_open` (`resolver/open.rs:61`, `:77-79`). `land_in_resolver` ignores the result.
- **Verified by running:** a throwaway probe in my worktree, not committed:
  1. `operating = Some(7)`.
  2. Key `c` gives `editor_open_during_merge=true`.
  3. A `Conflicted` answer for `a.rs` then gives `resolver_open=false status=Sticky { reason: "finish the box first", keys: None } stopped=true`.
- **Why it matters:**
  - CLAUDE.md rule 3: the ignored result is justified by a comment that is false.
  - "A status field must never read as an error when nothing is wrong": the user pressed no key, and the line tells him to finish a box.
  - The P5 landing is silently lost. The window is as long as the merge or `--continue` runs.
- **Fix:** in `land_in_resolver`, return early when `resolver::box_open(state)`. That keeps today's banner landing with no words. Alternatively, defer the landing until the box closes. Either way, test it. Correct the comment.

### 2. Test names and messages still state the old landing (PROMPT.md 6.2, names)
- **Where:**
  - `crates/graft/tests/it/conflict_interim.rs:288`: `conflict_focuses_the_banner_and_lists_the_files`, with the message "the thing that stopped takes the keyboard" (`:299-302`).
  - `tests/it/m5_walk/conflicts.rs`: `conflict_focuses_banner_and_lists_files`, with the step `See("the banner has the keyboard and names the file", …)`.
  - The capture stem `m5-06-banner-conflicts`, which now shows the resolver (`docs/shots/m5-06-banner-conflicts-*.txt`).
- **Why it matters:** after P5, a conflict focuses the **resolver**. A reader takes these names as the spec and learns the wrong landing. The walk's capture file says "banner" and shows no banner.
- **Fix:**
  - Rename the tests to what they now check, for example `esc_from_the_landing_shows_the_banner_and_lists_the_files`.
  - Reword the messages.
  - Rename the capture stem, for example `m5-06-stop-lands-in-resolver`, and update `TESTING.md:2019` and the captions TSV (Minor 4) with it.

## Minor

1. **`capture_a_merge_that_stops` lost its banner assertions instead of moving them, and ticket 12's tick now points at deleted files.**
   - **Where:** `tests/it/conflict_interim/landing_shots.rs:49-85`, `.scratch/m6-conflicts/issues/12-landing-after-a-stop.md:127`.
   - **What:** the real-repo asserts on the frame after a stop were dropped. They were:
     - `⏎ resolve  c continue  a abort` at 80, 120 and 187;
     - no `esc dismiss`;
     - `merge stopped · 2 files conflicted ·` at 120 or more.
   - What remains:
     - the unit test at 80, 120 and 200, on a synthetic one-file state;
     - `conflict_interim.rs:1127`, which asserts `merge stopped · `;
     - `regressions.rs:401`, which asserts the rebase keys at 80.
   - Nothing now checks the plural count against a real repo. Ticket 12's box names `fix-landing-a-stop-*`, which was removed, and this test for "the row text below".
   - **Fix:**
     - Assert those strings on the `Esc`'d state from `stopped_on_the_conflict` at the three sizes.
     - Annotate ticket 12:127 "superseded by 18".
2. **`banner_focus` assertions that cannot fail.**
   - **Where:** `conflict_interim.rs:156` and `banner/tests.rs:1053`.
   - **What:** both follow the `Esc`, but `banner_focus` is already true while the landed resolver is open (`operate.rs` asserts it at the landing).
   - The m5_walk `BannerFocused` predicate (`m5_walk/run.rs:518-524`) does not require `resolver.is_none()`. So the walk never checks that its new `Esc` closed the resolver.
   - **Fix:** add `state.resolver.is_none()` to `BannerFocused`. The two assert sites already check it or can.
3. **Stale caption.**
   - **Where:** `docs/shots/m5-contact-sheet-captions.tsv:10-12`.
   - **What:** the captions still describe the banner and a sticky `… resolve it and s… · esc dismiss`. The regenerated m5-06 captures show the resolver. This is a textual finding.
   - **Fix:** reword the captions from `TESTING.md:2019`.
4. **MAP.md is stale for this change.**
   - **Where:** `crates/graft/MAP.md`.
   - **What:**
     - `update/banner/stop.rs`'s item list (`:299-306`) lacks `land_in_resolver`.
     - `operated` (`:225`) describes `Conflicted` with no landing.
     - `resumed` (`:295`) says "the banner **stays focused**" and does not mention the resolver.
   - **Fix:** one item line, plus a clause on each of the two entries.
5. **At 187x50 the second file's name is cut while its column has room (a question for the user's eye).**
   - **Where:** `docs/shots/m6/fix-stop-lands-in-resolver-187x50.txt`, row 5.
   - **What:** the row reads `   …ared.rs  …` in a 24-column list. `shared.rs` plus its `…` state is 15 columns. The name looks cut to the width `notes.md` left after `1 of 1 open`.
   - This is the ADR-0043 list column (ticket 13's), not this ticket's code. But this capture's box says "the strip lists both files".
   - **Fix:** the owner of the list column decides whether names take the width a short state leaves.
6. **The file keeps growing past the size rule.**
   - **Where:** `update/operate.rs`, now 2432 lines (991 non-test).
   - **What:** this ticket adds 4 code lines and 63 lines of inline tests. The file is already tracked in KNOWN_GAPS (M6/06, the `operate.rs`/`undo.rs` 6.2 row). Noted only.
7. **Release perf run 1 missed one frame.**
   - **What:** `frames::…with_the_map_view_open`, key 132, 16.31 ms. Almost all of it was queue time behind key 131: 16.1 ms queued against a 0.17 ms own frame. Run 2 passed.
   - This ticket touches no frame or Map code. The run was under load: average about 4, with other agents' cargo processes running. **Not verified as load.** I did not check whether a gap row already covers queue-time misses under load.

## Also checked, no finding
- **Rule 3:** the diff has no `let _`, no `unwrap_or_default` and no `unwrap`/`expect` outside tests. The one ignored result is Major 1.
- **Git work:** none is on the UI thread. `land_in_resolver` only pushes `Command::OpenConflict`, and `open_resolver` pushes the set read.
- **Terminal output:** no new text reaches the terminal unsanitised. The path goes through the resolver's existing drawing.
- **Scope 3 (unchanged cases):**
  - A launch into a live operation still lands on the banner. `capture_a_launch_into_a_shell_merge` asserts `banner_focus`, and it passes.
  - A stash pop cannot reach either arm: `Operated::Conflicted.kind` is `InProgress`, which has no stash variant. I checked this by reading the code, not by running it.
- **Captures against "what right looks like"** (`fix-stop-lands-in-resolver-{80x24,120x40,187x50}`):
  - Row 1 is `MERGING` at all three sizes.
  - The strip or column lists both files with `▸ notes.md`; at 187 the second name is cut (Minor 5).
  - The identity row reads `▸ notes.md · 1 conflict · 1 open`.
  - The bar starts `o yours`.
  - `TESTING.md:2019` holds against the three m5-06 captures.
  - Colour is unverified (rule 5). By-eye judgement is the user's.
- **Pipeline:** Amendment 2 has Need (`:414`), Prior art (`:427`, which cites `docs/research/conflict-resolution.md` and the layout audit) and Journeys (`:439`, J1/J2 = 0). `02-keymap.md:285` carries the rewritten §2.6a sentence.
