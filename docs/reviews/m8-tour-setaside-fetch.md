# M8 review: tour (19, 19c, 19d), merge set-aside, 21c fetch fix

Reviewer, read-only against `e3838299`. Merges diffed against their first parent, oldest first:
ce6fdfdf, d4ab5f34 (19a/b) · ca2f5484, b8ce4ea7, 706d988f (19c) · 64b7cf93, af480f01 (21c) ·
e663846a (19d-b), ea0c1d22 (debugger, tour task 4), fa0111ca (19d-b2) · 266f8950 (19d-a/a2/a3) ·
5f0b34dc (19d-c).

**Counts: Critical 0 · Major 2 · Minor 5 (2 filed as questions).**

## Gates (own `CARGO_TARGET_DIR`, one cargo command at a time)

The machine was shared: up to 3 other agents' cargo processes ran during the suite and perf
runs. Times and budgets below are **not attributable to this group**.

| Gate | Result | Time | Peak RSS |
|---|---|---|---|
| `cargo fmt --check` | ok | 3 s | — |
| `cargo clippy --all-targets -- -D warnings` | ok | 39 s | — |
| `cargo clippy --all-targets --features heavy-tests -- -D warnings` | ok | 19 s | — |
| `cargo test` (fast tier, workspace) | **pass**, rc 0 | **841 s** wall (build `--no-run` 65 s) | ~691 MB |
| `cargo nextest run` | **6155/6156**, 1 fail (flake, Major 2) | 670 s | ~444 MB |
| `cargo test --features heavy-tests -- --test-threads=2` | **red**: graft `it` 822 pass, 6 fail | 759 s | ~1.66 GB |
| release perf run (`frames:: heavy::budgets_hold`) | **red**: 11 pass, 10 fail | 248 s | ~1.31 GB |
| `target/` size | 9.0 GB (deleted after the run) | | |

Five slowest tests (nextest):

1. `graft-ui graph_screen::hints::tests::the_bar_never_draws_a_fragment_of_itself`: 93.6 s
2. `…hints::tests::a_tail_hint_gains_its_verb_only_once_the_whole_list_is_drawn`: 86.4 s
3. `…hints::tests::no_hint_loses_ground_as_the_room_for_it_grows`: 82.5 s
4. `…hints::tests::the_bare_keys_are_a_tail_not_a_scatter`: 81.6 s
5. `graft-git run::tests::stop::signals::a_slow_read_of_the_group_does_not_spend_gits_grace`: 4.4 s

**Heavy reds (all outside this group):**
- `ghostty_shot::fix_scroll_ghostty_png`: `docs/shots/m7/fix-scroll-ghostty.png` bytes differ.
- Five `held_key` tests (`held_j_stops_on_release_pixel`, `_pixel_free`, `_unicode_free`, `held_j_with_the_pane_shown_stops_on_release_pixel_deflate`, `_unicode_40mb`), each with `frames_after_release=3`. Earlier M8 reviews record the same reds.

**Perf reds (machine busy):**
- The busy control read `busy=true` once and `busy=false` once.
- Frame p99 was 22–34 ms against 16 ms, at Pixel and Unicode, on the FiftyThousand and Fixture repos.
- J find p99 was 31 ms. `G` after idle took 17.9 ms.
- Pixel held-`j` peak RSS was 329 MiB against a 300 MiB budget.
- **This group changed no draw path or graph path**, so the reds are not evidence against it. They are not evidence for it either. Re-run on a quiet machine.

**Fuzzing and benchmarks:** not run. This group touched no fuzzed parser and no benchmarked path.

## Tick spot-check

- **Existence:** I grepped about 41 named verifying tests across 19a/b, 19c, 19d-a/a2/a3/b/b2/c and 21c. All exist at `e3838299`.
- **Content:** I read the bodies of 15 against their ticks:
  - **19d-a set-aside, `merge_set_aside.rs`:** staged-change merge, unstaged put-back, up-to-date filter, stop / finish / continue / abort, put-back clash, undo/redo round trips, "a stop anywhere".
  - **19d-a preview:** `the_merge_preview_forecasts_the_put_back`, and `m6_stash_put_back/set_aside.rs:291` `merge_over_staged_edits_previews_and_puts_back`.
  - **21c:** `a_line_without_fetched_refs_re_renders_byte_identically` (`journal/tests.rs:548`), `a_failed_fetch_that_moved_refs_ends_with_them`, `a_stopped_remote_write_answers…`, `a_stopped_fetch_that_moved_a_ref_says_what_moved`.
  - **19d-b:** the `clock_pin.rs` tests.
  - **19c/19d:** the whole-tour walk's byte checks.
- **Result:** 15 of 15 held. **No false tick found.**

## What was checked clean

- **Undo and journal safety:**
  - The merge set-aside follows ADR-0045's order: `stash create`, then `begin` with `put_back: SetAside`, `stash store`, `reset --hard`, the merge, `apply --index`, and finally drop.
  - D1 is fixed: `stopped::mark` now runs over `closed_set_aside` (`apply/close.rs`).
  - D2 is fixed: `set_aside::Failed { error, reason }` with `PlanFailure::local_reason`.
  - `stopped_at` is carried on `Clashed` (19d-a3, `stopped.rs:252-256`).
  - `fetched_refs` never reaches an undo path. I grepped the undo modules; only `wrote_anything` reads it.
  - Old journal lines still parse and re-render byte for byte (`journal/tests.rs:548`).
- **The M5 promise:** the merge preview names the put-back before you confirm (`MERGE_MAY_STOP`, `the_merge_preview_forecasts_the_put_back`).
- **Other set-aside kinds:** `undone_with_edits` (`update/clash.rs`) is an exhaustive match over `OperationKind`. A new kind can't fall through silently.
- **Keymap row `m`:** amended in `docs/design/02-keymap.md:257`.
- **Tour:**
  - The path guard `inside` and `remove_practice_repo` refuse anything outside the practice root.
  - `tour_progress` writes objects only into the practice repo.
  - In `event_loop/places.rs`, a removal failure is said through `say`, not dropped.
- **Rule 3:**
  - No new `let _ =`, empty `Err(_) => {}` or `unwrap_or_default()` standing in for a failure in the diffed src.
  - No new `unwrap()`/`expect()` outside tests.
- **Sanitising:** new notices go through `say` / `sanitize_line`. `launch_clock`'s error is sanitised before stderr (`session.rs:140-146`).
- **Captures:**
  - All are 200 KB or less. The largest in the repo is the m7 sheet at 173,931 B.
  - No home paths leak.
- **Feature pipeline:**
  - 19d's note has Need, Prior art (cites `docs/research/commit-stash.md`) and Journeys.
  - 21c's note says it adds no interaction. See Minor 5.

## Critical

None.

## Major

### M1. `GRAFT_NOW` pins real journal stamps in the shipped binary, and is documented only in MAP.md

- **Where:**
  - `crates/graft/src/session.rs:116,145-146`: `NOW_VAR`, `launch_clock`.
  - `crates/graft/src/event_loop/startup.rs:355`: `JobRunner::…with_clock(harness.clock)`.
- **What:** any user (or a stray shell export) who runs `graft` with `GRAFT_NOW` set gets every operation in **whatever real repo he opens** journalled at one fixed millisecond. The stamps are persistent and false.
- **Why it matters:** several readers order or match journal entries by that stamp.
  - J merges graft entries with reflog steps by time (`graft-ops/src/journal_list/rows.rs:5,47`).
  - J matches switches to entries within `CHECKOUT_WINDOW_MS = 60_000` (`journal_list/steps.rs:41,339`).
  - The time machine's `state_at` partitions and sorts by `at_unix_ms` (`graft-ops/src/journal.rs:344-355`).
  - With every entry on one millisecond, the ULIDs inside that millisecond carry random bits, so id order is random. That is the root of the tour task-4 flake the debugger fixed in ea0c1d22 by moving the tour to journal order.
  - A pinned launch writes history that later wall-clock launches order wrongly.
- **Documentation:** `crates/graft/MAP.md:92,1118` and `graft-fixtures/MAP.md:12` only. Not in `TESTING.md`, the README, `--help`/`cli.rs` or `KNOWN_GAPS.md`.
- **Spec status:** this behaviour was the 19d R3 ruling, so it is a spec question, not a build defect. See Proposed changes.
- **Suggested fix (pick one):**
  - (a) Pin only the **drawn** clock (ages, "now") and keep journal stamps on the wall clock. The tour's ordering is already journal-order since ea0c1d22.
  - (b) Keep the pin, but gate it behind a test-only build feature or a `GRAFT_TEST_HARNESS` pairing. Draw a status line while it is active (by state: `clock pinned <date>`). List it in TESTING.md and KNOWN_GAPS.

### M2. The whole-tour walk shoots before the screen settles, so the gate flakes

- **Where:**
  - `crates/graft/tests/it/tour_walk.rs:645-656`: the "home, finished" and "cursor back on" waits gate the `m8-tour-finished-120x36.txt` shot on the cursor subject only. Neither calls `settled()` (defined at `tour_walk.rs:104`, which requires no `sorting…`).
  - `tour_walk.rs:393-407`: the step-7 conflict shot is also taken without `settled`.
- **What happened:**
  - In this review's nextest run, `tour_walk::the_whole_tour_in_a_pty_ticks_all_eight_and_comes_home` failed.
  - Line 3 of the drawn frame still read `date order, sorting…` against the committed capture.
  - The same test passed in the `cargo test` run, so this is a race, not a regression.
- **Why it matters:**
  - A red gate that goes green on retry teaches everyone to retry.
  - The capture the user judges by eye can be committed mid-sort.
- **Suggested fix:**
  - Fold `settled(s)` (or at least `!s.has("sorting…")`) into the predicate of the waits before each `shot`.
  - Audit the other `shot(` calls in the file for the same gap.
  - This is the cause fix. A retry would be the symptom fix.

## Minor

### m1. Question: does a landed set-aside whose put-back is `Kept` without a clash lose `Landed`?

- **Where:**
  - `graft-ops/src/apply/set_aside/answer.rs:86`: `(Some(failed), _) | (None, Ok(failed)) => ApplyError::Git { … paused: None, … }`.
  - `stopped::mark` (`apply/stopped.rs:231-257`) passes `Err(other)` through unchanged (`:256`).
- **What it looks like from the code:**
  - If the merge landed and `apply --index` then failed without a clash, the caller gets a plain `Git` error.
  - It does not get one carrying the landed state, so the screen may say "failed" for a merge that is in.
- **Status:** read, not run, so filed as a question (rule 7). `M8/landed-clash-exit-line` tracks the clash case only.
- **Suggested fix:** a test that makes `apply --index` fail with no clash after a landed merge, and asserts what the user is told.

### m2. Weak test: `u_under_a_pinned_clock_undoes_the_newest`

- **What:** under a pinned clock, all entries share one millisecond, so the "newest" pick depends on whichever ordering the code uses.
- **Why it matters:** if that is id order, the test passes by chance roughly half the time (the M1 mechanism).
- **Suggested fix:** confirm it asserts journal (append) order. Run it in a loop (`--test-threads=1`, 50×) to show it can't flip.

### m3. The fast suite is far over the 7.1 budget

- **What:** 841 s for `cargo test` and 670 s for nextest, against PROMPT.md 7.1's minutes.
- **Mitigating:** the run was under concurrent load.
- **Outside this group:** four `graft-ui` `graph_screen::hints::tests` take 81–94 s each (last touched by 21b-1/21b-2, `592d3620`, `5259a592`).
- **Suggested fix:** re-time on a quiet machine. If the hints tests still exceed 60 s, move their exhaustive width sweeps to the heavy tier, or sample the widths.

### m4. 21c changes what old pull rows say (by design; noting the knock-on)

- **What:** `wrote_anything` now reads `fetched_refs`.
- **Old lines:** they have no `fetched_refs` field, so they read as before (`journal/tests.rs:548` proves byte identity for rendering).
- **New lines:** a pull that only fetched now counts as having written, so new pull rows move out of "nothing written".
- **Status:** consistent with ADR-0062; flagged so the usability walk checks that J reads the same way for both.

### m5. Question: is there prior art for the merge set-aside and the 21c J wording?

- **19d:** cites `docs/research/commit-stash.md`, but says outright that no study covers *merge over local changes*. Several of the tools the user named (JetBrains "smart checkout/merge", Sublime Merge, lazygit) have exactly this behaviour.
- **21c:** says it adds no interaction, so it needed no study. That is defensible for a fix.
- **The rule:** CLAUDE.md makes a missing study Major. Because a related study exists, I file it as a question: should a `docs/research/merge-over-local-changes.md` be done before the next merge-flow change?

## Already tracked in KNOWN_GAPS (cited, not re-filed)

`M8/merge-finish-drops-edits-back` (`banner/resume.rs:447-449`), `M8/19d-a3-80x24-no-stash-row`,
`M8/landed-clash-exit-line`, `M8/m4-07-future-rows`, `M8/tour-step7-capture-bare`,
`M8/21c-row-cuts-moves`.

## Proposed changes

- **`GRAFT_NOW` scope (from M1):** change the 19d R3 ruling so the pin covers the drawn clock only, not journal stamps.
  - **Trade-off:** the tour's journal-dependent captures would need journal order (already true since ea0c1d22) rather than stamp equality.
  - **Gain:** no shipped env var can write false history into a real repo.
  - **Alternative:** keep the pin and make it a test-harness-only switch with an on-screen state.
- **Fast-tier budget enforcement:** add a per-test ceiling (nextest `slow-timeout` with `terminate-after`) so a 90 s unit test fails the fast tier instead of silently passing.
