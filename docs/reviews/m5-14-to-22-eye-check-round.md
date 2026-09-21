# Review: M5 tickets 14, 16, 17, 18, 19, 20, 21, 22, 15 (eye-check round)

Reviewed range: `836eb6c..d1bb6c3` (150 files, +12161/−2439). Read-only.
Nine defect tickets built in that order; none had a review round before this.

**No Critical findings.** 0 Critical · 6 Major · 9 Minor.

## What was read, what was checked

- All nine tickets read **in full** (acceptance ticks, mutation tables, NOTES). None skimmed.
- Diff read crate by crate: graft-git, graft-ui, graft (update, view, jobs, tests), docs. graft-ops and graft-core have no source change in the range.
- **Acceptance ticks spot-checked: 22 of 22 held.** Every sampled tick names a test that exists, runs in the fast tier, and asserts the ticked behaviour (not merely that the code ran). Sample: 14 → `the_footer_does_not_call_a_live_merge_interrupted`, `relaunch_into_a_live_conflict`; 16 → `the_banner_and_its_keys_are_on_the_frame_at_every_width`, `every_hint_the_focused_banner_draws_is_answered_by_banner_key`; 17 → `a_preview_scrolled_into_history_draws_the_rows_the_user_was_on`, `no_user_facing_string_says_ghost`, `the_predicted_block_is_labelled_and_named_in_the_users_words`; 18 → `y_after_a_right_click_copies_the_sha_the_menu_named`, `right_click_on_mains_row`, `only_an_explicit_regeneration_rewrites_a_capture`, `every_verb_the_menu_advertises_is_live_while_it_is_open`; 19 → `pane_boundaries.rs` (whole file), `the_lane_cap_comes_from_the_graphs_room_not_the_screens_width`; 20 → `below_a_hundred_columns_the_banner_draws_above_the_tab_line`, `the_tab_layout_banner_costs_one_row_drawn_and_two_focused`, `the_changes_pane_hints_the_keys_that_write`; 21 → `an_unmerged_file_draws_its_conflict_in_the_body`, `the_count_is_never_empty_while_the_row_draws`; 22 → the four `diff_cases.rs` tests + the proptest; 15 → `the_repush_is_leased_and_never_a_bare_force`, `u_after_push_asks_once_then_undoes_locally`, `u_after_push_repush_old_tip_moves_the_bare_back`, `dot_closes_the_menu_it_opened`.
- Not verified, by design: colour (rule 5); "looks right" for any capture (rule 6).

## Gates (run one at a time, warm `target/`)

| gate | result |
|---|---|
| `cargo fmt --check` | ok |
| `cargo clippy --all-targets -- -D warnings` | ok |
| `cargo clippy --all-targets --features heavy-tests -- -D warnings` | ok |
| `cargo test` | ok, **64 s wall**, 2228 passed / 0 failed / 37 ignored, max RSS 345 MB |
| `cargo nextest run` | ok, 57 s, 2226 passed / 37 skipped |
| `cargo test --doc` | ok (2) |
| heavy: `cargo test --features heavy-tests -- --test-threads=2` | ok, 269 s, 2259 passed / 0 failed / 37 ignored, max RSS 958 MB |
| release perf: `cargo test --release -p graft --features heavy-tests --test it -- frames:: heavy::budgets_hold --test-threads=1 --nocapture` | ok, 89 s wall (13 tests, 73 s) |
| `target/` | 14 G |

Release perf numbers: input-to-frame p99 7.7–9.1 ms, max 15.0 ms (Unicode tier, 100k fixture); pixel tier 50k refs p99 8.3 ms. `first_paint_us` 13 954 (100k, commit-graph), 14 322 (realistic), 16 098 / 16 168 (no commit-graph), 39 946 (50k refs). `bottom_ms` 49–53 with commit-graph, 468–517 without. `peak_rss` 31–102 MB. All inside the 7.1 budgets.

Five slowest (fast tier, nextest `--status-level pass`):
1. `graft-git::it diff_fixture::parse_unified_matches_numstat_for_every_shape` 8.69 s
2. `graft-graph layout::stream::tests::scrolling_the_whole_history_keeps_the_resident_set_bounded` 6.68 s
3. `graft-ops::it undo_cancel::cancel_at_every_step_boundary_is_clean` 6.59 s
4. `graft-fixtures::it corpus::two_builds_produce_the_same_object_ids` 3.92 s
5. `graft::it preview::a_preview_scrolled_into_history_draws_the_rows_the_user_was_on` 2.86 s

Fuzz (`cargo +nightly fuzz run <t> -- -max_total_time=60 -rss_limit_mb=1024`, own target dir): `parse_unified` 661 558 runs, cov 77 (was 61 last round — the new guards are exercised), no crash, rss 278 MB; `parse_file_list` 652 250 runs, cov 380, no crash; `parse_porcelain_v2` 664 146 runs, cov 160, no crash.

Bench (`--warm-up-time 1 --measurement-time 3`, health check — neither crate's hot path changed): `graft-ui row_render` 25.7 µs (last round 15.3 µs; the bench draws the widget through `GraphColumn::render`, which ticket 19's `commit_graph_segments` / lane-cap changes touch — worth a second run before calling it drift); `graft-graph hot_paths` advance 788 µs, rasterise 631 µs, StripCache hit 1.70 µs / miss 630 µs (unchanged).

## Critical

None.

## Major

### M1. The tab-layout banner row is unknown to the pure geometry — mouse rows and `graph_rows()` are off by one or two under a live operation below 100 columns

- `crates/graft/src/view/frame.rs:436` — `split_for_detail` calls `draw_tab_banner`, which takes one row above the strip (and a second at the bottom while focused, `frame.rs:483-499`) before `split_area` runs.
- `crates/graft/src/update/geometry.rs:89-100` — the `Tab` arm of `detail_split` still puts the content at `y: strip` (= 1) with `height: size.1 - strip`. It does not subtract the banner row(s).
- `crates/graft/src/update/state.rs:277-288` — `graph_rows()` = `CHROME_ROWS + harness.banner_rows() + commit_graph_line + preview_rows_taken`; the operation banner's tab row is not a term.

Why it matters: `geometry.rs` exists to mirror the view for hit-testing (its own doc says so). With a merge stopped at 80 columns and the Detail tab shown, every click lands one row above (two when the banner is focused) the row the user sees; on the Graph tab the cursor can be moved onto a row the graph never drew. This is ticket 19's own principle (m5/19 ADR-0039 amendment: "count from what you were given") applied vertically — the **fourth instance** the conductor asked about is here, in height rather than width.

Fix: give `update::banner` a `tab_rows(state, width) -> u16` (0 / 1 / 2 by `layout == Tab`, `banner_view`, `banner_focus`) and use it in both `detail_split`'s Tab arm and `graph_rows()`. Test: a `pane_boundaries.rs`-style sweep asserting `geometry::detail_split` equals the rects `split_for_detail` actually drew, with and without a focused banner.

### M2. On the Graph tab under 100 columns the operation banner draws twice, and so do its keys

- `crates/graft/src/view/frame.rs:436` — the tab banner draws for `Tab { shown }` whichever side is shown.
- `crates/graft/src/view/rows.rs:40,57` — `banner_op` is set into `GraphView` unconditionally, and `graph_screen::layout.rs` draws it on the header row; `rows.rs:49` also feeds `footer_context` the banner, so the graph's own hint bar draws the banner keys.

Read from the code, not observed: with a live operation, the Detail pane open and the Graph tab shown at 80 columns, row 0 is the tab banner, row 2 (the graph header) carries the same banner text, and while focused the graph footer and the row under it both list `c continue  a abort …`. **No test covers this tab** — `conflict_interim.rs` has no case with `[Graph]` shown under a banner (grep for `[Graph]` / `DetailFocus::Graph` in that file: none); `the_tab_layout_banner_costs_one_row_drawn_and_two_focused` (`conflict_interim.rs:1142`) locates the pane by `"UNCOMMITTED"` (`:1162`), i.e. only the Detail tab. Ticket 16's "drawn once, at every width" tick is verified for the Detail tab only.

Fix: in `split_for_detail`, only draw the tab banner when `shown` (the graph screen already carries it on its header and its footer), or pass `banner_op: None` to the graph view in the Tab layout. Add the missing test either way.

### M3. `docs/design/02-keymap.md` §2.6 and `banner.rs` disagree on what a focused banner does with `u`

- `docs/design/02-keymap.md:245-246` — "**`u` and `U` are refused while an operation is live**, focused or not: `finish or abort the <kind> first · ⇥ banner`".
- `crates/graft/src/update/banner.rs:246-248` — with the banner focused, `u`/`U` fall into `_ => {}` and `banner_key` returns `true`; nothing is drawn. `keys.rs:78` runs `banner_key` before the undo path, so `undo.rs:84` `in_progress_refusal` (which does draw the doc's words) is reached only when the banner is **not** focused.

The doc says one thing for both focus states; the code says it in one and stays silent in the other. Ticket 15 edited §2.6 and ticked "doc and code agree". The tick's test (`u_refuses_mid_operation` family) drives the unfocused case. Either the doc drops "focused or not" (and says the focused banner swallows), or the swallow arm answers `u`/`U` with the same `Status::Sticky`. The second is the better product: the user pressed `u` on purpose and gets nothing back.

### M4. `undo_after_push` and `pushed_since` count a push of **any** branch as "this commit is on the remote"

- `crates/graft/src/update/remote.rs:527-535` — the push filter is `kind == Push && status == Committed && id > target.id`; no test that the push touched the undone entry's branch.
- `crates/graft/src/update/remote.rs:814-832` — `pushed_since` uses the same filter, and `pushed_to(push)` (`:836`) then names **that** push's remote side.

Scenario (the user works on dependent branches, see memory): commit on `feature`, push `main`, `u`. The box opens saying `commit "…" is on origin/feature` (remote_side comes from the head's upstream, `:540-544`), which is false — nothing on `feature` was pushed. After a local undo the status reads `origin/main still has it`, which is also false. The error is on the conservative side (asks when it need not, never skips an ask), so no data risk, but the words are wrong and the M5 promise is "you see the result before you confirm". Fix: filter on `entry.touches.refs` containing the undone branch's tracking ref (or the push's `branch` field), and derive `remote_side` from the matching push.

### M5. A failed or still-loading remote read silently turns "undo after push" into a plain undo

- `crates/graft/src/update/remote.rs:517` — `let facts = facts(state).ok()?;`. `facts()` (`:237-245`) returns `Err` for `Loading::Failed`, `Idle` and `Pending`.

When the remote facts failed to load (`git remote show` refused, network down), `undo_after_push` answers `None` and `undo_key` proceeds as if no push had happened: no box, no `still has it` line. Rule 3 shape: an error stands in for "not pushed". The journal itself knows the push happened (the filter at `:527` needs only the journal); only `old_tip`/`expected` need facts. Fix: when facts are not `Ready`, still open the box but with `Re-push old tip` disabled and a line saying why (`remote not read: <error>`), or refuse `u` with a sticky naming the failed read. Test: `Loading::Failed` facts + a committed push → `u` does not undo silently.

### M6. A capture that contradicts ticket 20 still ships, and its caption says so

- `docs/shots/m5-ui-banner.txt:17` (and the two other width blocks) — ` graft /code/api    ⚠ REBASING 3/7 — conflict src/auth/token.rs (+1)`.
- `crates/graft-ui/src/graph_screen/tests.rs:1333-1334` — `capture_for_docs_shots` caption: "`⚠ REBASING step 3 of 7 — conflict in <path> (+N)` — spelled out at >=100 cols, abbreviated (`3/7`, path only) under 100".

Ticket 20 removed the `⚠`, the filename and the width tiers (`layout.rs` `banner_line(banner)` is width-less now), and re-captured `m5-ui-banner-conflict-80x24.txt` — but not this file, and not the caption that generates it. Rule 6: the user reviews by eye from the contact sheet; a stale capture is a false picture. Regenerate with `capture_for_docs_shots` after fixing its caption (and check whether that `#[ignore]` capture still compiles the same banner: it must, since `banner_line` is shared).

## Minor

1. `crates/graft/tests/it/hint_bar.rs:238` — `assert!(drawn > 4)` at 160×48 for "the changes pane hints the keys that write". Four is well under what the bar draws; the test would still pass if half the write keys vanished. Assert the named keys (`s stage`, `u unstage`, `c commit`, …) are present instead.
2. `crates/graft/src/update/context_menu/verbs/tests.rs:479` — `const LIVE_KEYS: [&str; 2] = ["m", "r"];` is a hand-kept list for "a trailing parenthesis on a verb label is always a live key". A new verb with a `(x)` label that is not live passes until someone edits the list. Derive the set from `bound_actions`/the menu itself.
3. `crates/graft-git/tests/it/diff_cases.rs:383` — the proptest re-implements `is_bare_object_id` (`unified.rs:94`) as its own `is_a_bare_object_id`. A property that mirrors the code's predicate agrees with the code by construction; it proves only that the two copies match. Keep it, but add one non-property case per guard with a hand-written expected error string (the four existing tests do this — say so in the proptest's doc comment so nobody later "simplifies" them away).
4. `crates/graft-ui/src/graph_screen/footer.rs:137` — `.unwrap_or_default()` on the narrowest-segment index. It is a legitimate "index 0 when nothing fits", not a swallowed failure, but it has rule 3's shape; one comment saying why it is safe.
5. `docs/shots/m5-ui-undo-after-push-80x24.txt:30,57` — header row shows the raw scratch path `/var/folders/…/graft-remote-pJ1SLR/wor…`. Ticket 20 substituted `/code/api` for the banner capture so the header is laid out at a real width; this capture (and the M4 push-box captures it copies) did not get the same. Cosmetic, but the contact sheet is what the user reads.
6. `docs/shots/m5-ui-preview-rebase-80x24.txt` / `m5-ui-preview-scrolled-80x24.txt` — header reads ` graft /code/api  8…` / `7…`: the path was substituted **after** layout, so the walk status is truncated against the temp path's width and the short path leaves a dangling `8…`. Same fix as ticket 20's: substitute before the frame is drawn.
7. `docs/shots/m5-ui-banner-conflict-80x24.txt:25-27` — caption says "only one side of the tab strip is on screen and this frame stands on the graph", but the pane is closed in the capture (no strip row drawn, rows 74-94 are all graph). The words describe the Tab layout; the picture is the no-pane layout. Say which.
8. `crates/graft/tests/it/conflict_interim.rs:1142-1175` — `the_tab_layout_banner_costs_one_row_drawn_and_two_focused` proves the "two while focused" cost only through `pane_top == 2` for both states plus a comment "the second row is taken off the end"; nothing asserts the pane's **height** shrank by one. Add `pane_bottom(&focused) == pane_bottom(&unfocused) - 1`.
9. `crates/graft/src/update/banner.rs:242-246` — the swallow arm's comment ("`u` cannot undo the entry *before* the one still running") is the design reason; it should also say the swallow is silent and point at §2.6 so M3 above is a one-line fix either way.

## The six attention points

1. **Ticks:** 22 checked, 22 held. No false tick.
2. **Tests that agree with the code rather than the requirement:** three found — Minor 1, 2, 3 above. `pane_boundaries.rs` **is** a genuine property: it sweeps widths 20..=240 × heights {3, 8, 24, 40, 48} × focus × override and checks overlap and `right.x <= left.right()` from the geometry alone, not from any drawn string.
3. **Fourth width instance:** none in width. The vertical analogue is M1 (the tab banner's rows unknown to `geometry.rs` and `graph_rows()`). `draw_tab_banner` uses its own `area.width`; `preview_box_area` docks in the graph's own area; the confirm box is centred on the screen (already filed last round, not repeated).
4. **Ticket 22:** `parse_unified` returns an empty `Ok` only for empty input and for lines that are all bare object ids (`unified.rs:94`, `split_lines` `:458-469` drops only the final newline). `reject_line_outside_a_file_section` and `reject_combined_diff` name what they cannot read. Holds.
5. **Ticket 15:** `PushParams` (`graft-ops/src/operation/remote.rs:46-67`) has `lease: Option<Lease>` and no force field; the command is built with `--force-with-lease=` at `:297`; a re-push without a lease is refused at `:228` (`Refusal::RepushNeedsLease`). A bare `--force` is unreachable by construction — holds. Passwords: `PushParams`, `ConfirmThen::UndoAfterPush`, the journal entry and the two captures carry only branch, remote, oids and a URL; no credential field exists in the range. The `Debug` derives on those types print nothing secret because nothing secret is in them. Verified by reading the types, not by a test that feeds a password through — there is none in the range, and the M4 credential path was not touched.
6. **Keymap doc vs code:** §2.1 (`.` closes the menu it opened) agrees with `context_menu.rs` (`Escape | '.'` → `None`) and `dot_closes_the_menu_it_opened`. §2.6 disagrees with the focused-banner path — M3.

## Not reported (known, deliberate)

Colour unverified; visual results not ticked; `hints_for` width tiers; combined diffs refused; `(pushed 2m ago)` absent; `(forecast)` trailing paren; 80-column clipping of the push command.

## For KNOWN_GAPS.md

| item | state | expiry |
|---|---|---|
| Mouse rows off by one/two under a live operation below 100 columns (M1) | **broken** (read from `geometry.rs:89-100`; no test either way) | M5 acceptance group |
| Banner and its keys drawn twice on the Graph tab below 100 columns (M2) | **unverified** (read from code; no test covers the Graph tab under a banner) | M5 acceptance group |
| `u` after a push of another branch asks with the wrong branch named (M4) | **broken** | M5 acceptance group |
| Failed remote read turns "undo after push" into a silent local undo (M5) | **broken** | M5 acceptance group |
| `docs/shots/m5-ui-banner.txt` predates ticket 20 (M6) | **broken** (stale capture) | before the contact sheet |
