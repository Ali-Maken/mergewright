# M2 review: details and diffs

Reviewer: Fable (high). Range `183c6ca..HEAD` (210 files, +28,284 / −738). 2026-09-18.

**1 Critical, 11 Major, 16 Minor.** M2 is blocked by the Critical: with many refs the Mistake Radar makes every frame and every update O(refs), and issues one probe job per tracked branch. The two 50k-ref frame tests no longer finish (M1: the whole release perf run took 47 s). On small repositories the promise mostly holds; see the verdict at the end.

Numbers are in `docs/perf/m2-review.txt`; raw logs stay under `target/review/`.

## Gates

| Check | Result |
|---|---|
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass, 1 s |
| `cargo test` (fast tier) | pass, **21 s** wall, peak RSS 336 MB, 1201 passed / 0 failed / 9 ignored |
| `cargo clippy --all-targets --features heavy-tests -- -D warnings` | pass, 18 s, RSS 436 MB |
| Heavy tier `cargo test --features heavy-tests -- --test-threads=2` | **fail on time.** 463 passed, 0 failed, 7 ignored, then `frames::fifty_thousand_refs_*` (Unicode and Pixel) still running after 21 min at 99 % CPU. Killed at **1275 s** (PROMPT 7.1 limit: 10 min). Peak RSS 564 MB |
| Release perf `frames:: heavy::budgets_hold` (ADR-0021) | **fail on time.** First test, `fifty_thousand_refs_*_pixel_tier`, still running after 9 min 19 s at 99 % CPU; killed at 604 s wall. RSS 532 MB |
| Release perf rerun with `--skip fifty_thousand_refs` | 8 passed, 53 s, RSS 279 MB. Frames p50/p99/max: Pixel 100k 4.6/8.3/10.3 ms, Unicode 100k 6.3/8.2/9.4 ms (bar 16). First paint: 100k+50k refs 38.8 ms, 100k cg 86.0 ms (M1: 12.2; cold run, bar 150), 100k no-cg 15.0, realistic cg 13.3, realistic no-cg 15.1. `G` 51 / 55 / 474 / 50 / 522 ms |
| Fuzz `parse_unified`, `parse_file_list` (60 s each, rss 1024 MB) | see the appendix at the end (appended after the run) |
| Bench `graft-ui` (`--warm-up-time 1 --measurement-time 3`) | see the appendix |

Fast tier, 5 slowest tests: 6.42 s `diff_fixture::parse_unified_matches_numstat_for_every_shape`, 5.93 s `layout::stream::tests::scrolling_the_whole_history_keeps_the_resident_set_bounded`, 2.96 s `corpus::two_builds_produce_the_same_object_ids`, 1.13 s `corpus::a_second_build_in_the_same_place_is_a_no_op`, 0.88 s `raster_pixels::reports_per_strip_cost`. `target/` 9.8 GB (was 7.9 GB before the heavy and release builds).

`unwrap()`/`expect()` outside tests: none new. `unreachable!` in three routing arms (Minor 11) and one in `resolve_sides` (Minor 1). `let _ =` once (Minor 9). `drop(Result)` once, commented (Minor 10).

## Critical

### C1. Radar cost is O(refs) per frame and per update, and one probe job per tracked branch

- **Where:** `crates/graft/src/update/radar.rs:132-170` (`request_force_push_probes`), `crates/graft/src/view/radar.rs:133` and `:237` (`findings_for` → `branch_facts`, `radar_lines_for` → `branch_facts` again), `crates/graft/src/view/strip.rs:31-52` (`strip_marks` → `marker_for` per resident row), `crates/graft/src/view/entries.rs:257` (`marker_for` per visible row).
- **What:** `request_force_push_probes` runs at the tail of every `update` (`update/mod.rs:82`). Each call rebuilds a `Vec<(String, Oid)>` over every branch (`refs.branches()` flattens the whole `RefBadges` map) and, the first time, pushes one `Command::ForcePushProbe` per tracked branch: a reflog read plus a `git merge-base --is-ancestor` spawn each. The synthetic 50k-ref fixture tracks every branch, so that is 50,000 jobs and up to 100,000 git spawns on a 2–4 worker pool. `findings_for` then calls `branch_facts(state)`, which allocates a `Vec<BranchFacts>` over every ref, for every row `graph_view` and `strip_marks` paint, every frame; `radar_lines_for` does it a second time for the pane.
- **Evidence:** heavy tier 1275 s and counting (M1 round 2: 177 s); release perf first test 9+ min (M1: 47 s for all ten). A 5 s `sample` of the debug test (`target/review/sample-heavy.txt`) puts the time in `view::radar::branch_facts` (via `findings_for`) and `update::radar::request_force_push_probes` iterating `RefBadges::branches`. `budgets_hold_on_the_100k_fixture_with_50k_refs` still passes because it stops after first paint and one `G`.
- **Why it matters:** PROMPT 7.1's frame budget (16 ms) and heavy-tier limit are gone at the ref counts M1 was built to survive; a real repository with a few hundred tracked branches spawns a few hundred git processes at startup and after every refs refresh, and every keystroke walks all refs.
- **Fix (cause):** (1) compute ref findings once, when refs or the context land, into a `HashMap<Oid, Vec<Finding>>` on `RadarState` exactly like `duplicate_findings`, and make `findings_for`/`marker_for` a lookup; (2) bound the force-push probes to the branches that can show a marker (the checked-out branch, the default branch, and branches whose tip is in the resident window), or batch them into one job with one `git for-each-ref`/`merge-base` pass; (3) stamp `request_force_push_probes` with a refs generation so it is a no-op until refs change. Add a `frames::` case that presses `Enter` and `O` on the 50k-ref fixture so this cannot come back silently (Major 10).

## Major

### M1. `W` (word tint) is a no-op

- **Where:** `crates/graft/src/update/detail.rs:1013-1016`, `crates/graft/src/view/detail.rs:59`, `crates/graft/src/jobs/detail.rs:78`, `crates/graft-ui/src/detail_pane.rs` (no reference to `options` anywhere in the file).
- **What:** `W` flips `DetailState::word_tint`; the view copies it into `DiffOptionsView.word_tint`; the widget never reads `DiffOptionsView`, and the job always assembles with `assemble_diff(&diff, true)`. The tint is always on and the key changes nothing. TESTING.md stop 4 (line 944) tells the user to expect the tint to appear on `W`, which cannot be observed. Ticket 10's deviation note said "display-only"; it is not displayed. This is stubbed work marked done.
- **Fix:** in `line_style_at` (`detail_pane.rs:804`) skip the words branch when `!options.word_tint` (thread `options` through `diff_body`/`SideColumns`), add a snapshot test that `W` changes a cell's style, and fix stop 4's wording. Or drop the key and the config field until it does something.

### M2. The diff body is drawn all `text-muted` unless the Diff region is focused

- **Where:** `crates/graft-ui/src/detail_pane.rs:792-793` (`line_style_at` returns `TextMuted` for every char when `!focused`), `:1199-1206`.
- **What:** `Enter` opens the pane with focus on Files (`update/detail.rs:330`), so the first thing the user sees is a grey diff: no added/removed background, no syntax, no word tint, no sign colour, until `Tab` moves focus to Diff. The design note (New UI, "Eye lands on: the tinted diff lines"; "the focused region's title is accent, others `text-muted`") mutes the title, not the body. DESIGN.md has no rule for dimming an unfocused body. This directly weakens the promise's first half.
- **Fix:** keep line styling independent of focus; mark focus by the region title (and, at most, the gutter). If a dim is wanted, dim one step, never to a single colour.

### M3. The evil-merge probe runs only while the pane is open, so a selected evil merge is never flagged in the graph

- **Where:** `crates/graft/src/update/detail.rs:397-404` (`probe_evil` requires `self.shown`), `crates/graft/src/update/selection.rs:106`, `crates/graft/src/view/radar.rs:210-214`.
- **What:** PROMPT §1.4: "computed lazily, only for the selected merge". The implementation probes only the commit the pane is showing. Cursor on an evil merge, pane closed: no `⚠`, no radar line. TESTING.md stop 10 documents exactly this ("`evil-merge · Enter`"). The design note (line 175) says "evil per selected merge".
- **Fix:** key the probe on `AppState::selected_spec()` (or the cursor's commit) regardless of `shown`; keep `evil_cache` and the cancel-on-move. `evil_at` already carries the id, so the view needs no change.

### M4. Working-tree file list lists stat-dirty, unchanged files as `M +0 −0`

- **Where:** `crates/graft-git/src/diff/read.rs:145-160` (`list_files_working_tree`, `git diff-index -z --raw --numstat HEAD`).
- **What:** `diff-index` is plumbing: it does not refresh stat information. A tracked file whose mtime changed but whose content did not (branch switch, editor save, `make`) is reported as `:100644 100644 <old> 0000000 M` with numstat `0 0`. Verified in a throwaway repo: `touch f.txt` → `diff-index` lists `M f.txt`, `git diff --raw --numstat HEAD` lists nothing. The pane's header counts come from `git status` (which refreshes), so the header says "0 unstaged" while the list shows modified files.
- **Fix:** use porcelain `git diff --raw --numstat -z -M -C HEAD` (same output shape; refreshes stat in memory, and `GIT_OPTIONAL_LOCKS=0` keeps it from writing the index), or drop entries whose new oid is all zeros and numstat is `0 0`. Add a test with a touched-but-unchanged file.

### M5. `L` (load all) sticks for the rest of the session

- **Where:** `crates/graft/src/update/detail.rs:609-612` (`load_all` sets `max_lines = usize::MAX`); nothing in `load` (`:443`) or `show_cursor` (`:517`) resets it.
- **What:** after one `L`, every file of every later commit is read without a line cap, up to the runner's 64 MiB output cap, then turned into `LineView`s with word and syntax spans on the job thread (and kept in the 8-entry cache). A generated file or a vendored tree turns a cursor move into seconds of CPU and hundreds of MB.
- **Fix:** reset `max_lines` to `MAX_LINES` in `load()` and when the file cursor moves; `L` applies to the file it was pressed on. `DiffKey` already carries `max_lines`, so the cache stays correct.

### M6. `--remerge-diff` calls every conflict-resolved merge "evil", and Explain does not say so

- **Where:** `crates/graft-ops/src/mistake_radar/probes.rs` (`probe_evil_merge`: any path in `git show --remerge-diff --name-only` → `Evil`), `crates/graft-ops/src/mistake_radar/explain.rs:158`.
- **What:** a remerge diff is non-empty for every merge whose conflicts were resolved by hand — the common case on git/git and on any busy repository. Each such merge gets `⚠ EVIL MERGE` (warning severity) and an explanation saying the merge "changes N files in ways found in neither parent", with no hint that a conflict resolution looks identical. A warning that fires on ordinary work trains the user to ignore it (CLAUDE.md, status rule).
- **Fix:** say what was measured ("N files differ from a clean re-merge; a hand-resolved conflict looks the same — check the listed files"), and downgrade to Info when the same paths appear in both parents' diffs (a conflict) — or keep Warning only for files touched by neither parent (the fixture's `evil.txt` case).

### M7. ARCHITECTURE.md was not updated for M2 (PROMPT 6.1)

- **Where:** `ARCHITECTURE.md` (no change since `183c6ca`); lines 206-207 say `MistakeRadarKind` is "produced by `ops::MistakeRadar`, attached to a `graph::Row`".
- **What:** the radar is a paint-time lens (`graft/src/view/radar.rs`) over a context job (`graft/src/jobs/radar.rs`) and probes in `ops`; nothing is attached to a `Row`. Undocumented: the typed runner `graft_git::run` (every M2 git spawn goes through it), `graft_git::contained`, `graft_git::diff`, the detail jobs and their `LOCAL_JOB_BIT` id split, `graft/src/map.rs`, the `update/` purity guard, the `view`-side `Cell` (Minor 7).
- **Fix:** a doc-only ticket: one paragraph each for the runner, the diff reads, the radar lens and probes, the detail job ids; fix the `Row` claim.

### M8. Files far over the ~400-line rule (PROMPT 6.2)

Non-test lines: `crates/graft-ui/src/detail_pane.rs` 1330; `crates/graft/src/update/detail.rs` 1045; `crates/graft/src/update/mod.rs` 846; `crates/graft/src/map.rs` 669; `crates/graft-git/src/diff/unified.rs` 558; `crates/graft-ui/src/map_screen.rs` 507; `crates/graft-git/src/run.rs` 484; `crates/graft-git/src/diff/read.rs` 457. Functions stay under clippy's 60-line gate but `draw_side_row` (78), `mouse_event` (88), `core_event` (83), `walk_branch` (94) and `build_map` (81) are at the edge. Suggested seams: `detail_pane` → `header.rs`, `files.rs`, `body.rs`, `side_by_side.rs`; `update/detail` → `state.rs` (DetailState, caches, pane size) and `keys.rs`; `update/mod` → move `detail_hit*`/`detail_split`/`context_menu_bounds` into `update/geometry.rs`; `map.rs` → `walk.rs` + `rows.rs`.

### M9. No perf or frame test covers what M2 added

- **Where:** `crates/graft/tests/it/frames.rs` (8 tests, all M1 shapes), `crates/graft/tests/it/heavy.rs` (unchanged in M2).
- **What:** nothing measures a frame with the detail pane open, the strip on, the radar at many refs, or a 5,000-line diff scrolling. C1 was caught only because the M1 tests happen to use a 50k-ref fixture. No test asserts `W` changes output (M1 would have failed it), none covers M4's stat-dirty case, and the force-push probe has never run against a rewritten remote (TESTING.md admits it).
- **Fix:** `frames::` cases with a prelude of `Enter`, `Enter Tab v`, `O` on the 50k-ref and 100k fixtures; a fixture whose `refs/remotes/origin/x` was moved backwards after a fetch (reflog entry present) for the force-push probe; a stat-dirty working-tree test; a style assertion for `W`.

### M10. `strip_marks` does O(rows × search hits) per frame

- **Where:** `crates/graft/src/view/strip.rs:44-50`.
- **What:** for every resident row, `for index in 0..search.hits()` calls `search.hit(index)` — with a search that matches thousands of commits on git/git, every frame with the strip shown compares rows × hits. It also decodes every row's message (`from_utf8_lossy`) per frame just to pass a subject to `marker_for`.
- **Fix:** build a `HashSet<Oid>` of hits once per search result (or index hits by row) and use `Commit::subject()` without allocating.

### M11. The evil verdict and radar lines are recomputed from allocation-heavy paths on every frame the pane shows

- **Where:** `crates/graft/src/view/radar.rs:194-249` (`radar_lines_for`), `crates/graft/src/view/detail.rs:113-114`.
- **What:** the pane header rebuilds `radar_lines_for` every frame: `findings_for` (C1) plus a second full `branch_facts` walk at `:237`, plus `sanitize_line` of every title. It is the same shape as C1, listed separately because the fix is different: the header is a view model that only changes when `detail.spec`, `evil`, `force_push` or refs change — cache it on `AppState` (a `detail_header: Option<Arc<DetailHeader>>` invalidated on those events).

## Minor

1. `crates/graft-git/src/diff/read.rs:331` — `unreachable!` in `resolve_sides` for `WorkingTreeVsHead`. Split `DiffSpec` into the two-sided specs and the working tree so the type rules it out.
2. `crates/graft-git/src/diff/read.rs:350-375` — `first_parent` spawns `git rev-list --parents -1` for every file diff read; the parents are already in `AppState` (`row.commit.parents`). Pass `against` from the caller; one spawn fewer per file.
3. `crates/graft-ops/src/mistake_radar/probes.rs:134` — `_branch: &str` unused parameter on a public function; drop it or use it in the `NotCheckable` text.
4. `crates/graft/src/jobs/radar.rs:32` — doc says the reflog tips feed `Evidence::Backward`; they feed foxtrot (`previous_default_tips`).
5. `crates/graft-core/src/mistake_radar.rs:70-72` — `name()` doc says config spells kinds `foxtrot-merge`; `config/load.rs:199-201` accepts `foxtrot`, `backward`, `accidental_pull`, … Doc is wrong.
6. `crates/graft-git/src/history/read.rs:131` — `first_parent_chain` returns a partial `Ok(chain)` when stopped; a truncated chain reads as a success to the radar context (a commit past the cut looks "off the default chain"). Return `Err(Stopped)`.
7. `crates/graft/src/update/state.rs:136`, `crates/graft/src/view/entries.rs:263` — `radar_column_seen: Cell<bool>` is mutated from `view`. View is meant to be pure; make the reserved-column decision an `update` fact set when the context lands, or a value the view returns.
8. `crates/graft/src/update/detail.rs:462` — `for loading in [&self.files]` is a one-element loop; `if let Loading::Pending(id) = self.files` reads plainly.
9. `crates/graft-git/src/diff/file_list.rs:173` — `let _ = old;` inside `parse_numstat`; bind `_old` in the pattern instead.
10. `crates/graft/src/event_loop/running.rs:146-149`, `crates/graft/src/update/mod.rs:654-656` — the layout file is written to disk on every mouse-move during a divider drag, on the UI thread, and the `Result` is dropped. `create_dir_all` runs first, so "missing directory" is not the swallowed case; an unwritable one is. Save on release (or debounce) and surface a failure once.
11. `crates/graft/src/jobs/runner.rs:270,553,558` — three `unreachable!` routing arms. Split `Command` into `JobCommand` and loop-side commands so the match is total.
12. `crates/graft-git/src/run.rs` — `judge` reuses `commit_graph::first_line` (made `pub(crate)` for it); move it to a small shared text helper rather than coupling the runner to the commit-graph module.
13. `crates/graft/src/update/detail.rs:708-712` — `open_explain` finds facts only when the explained commit is `History::selected`; otherwise the request is dropped without a word. Fine for the `.` menu today; carry the row's parents and subject in `explain_requested` so it cannot silently no-op.
14. Explain has no gallery stop: `docs/shots/m2-explain-panel.txt` is a TestBackend capture; stop 7 shows the menu, never the opened panel. Add `foxtrot-merge · . Enter` so the "explained" half of the promise is in the pty walk.
15. TESTING.md line 944 (stop 4) claims `W` adds the stronger tint; the tint is on by default and `W` changes nothing (M1). Rewrite once M1 is fixed.
16. Release perf: 100k commit-graph first paint 86.0 ms this run vs 12.2 ms in the M1 review (bar 150; cold run 1, ADR-0027). Re-measure ×3 in M7's perf pass; if it stays, bisect M2.

## Disclosed deviations, judged

| Ticket | Deviation | Verdict |
|---|---|---|
| 04 | Rule B narrowed to `of <remote>` with `into` = the branch, or no `into` on the default branch | Fine; fewer false accidental-pull findings. Foxtrot via the *local* default reflog means the clone that pulled never sees its own foxtrot (KNOWN_GAPS row stands). |
| 06 | `LOCAL_JOB_BIT`, `CancelJob`, `Product::Files`, `DiffOptions: Hash` | Fine; the bit split is documented and tested. Belongs in ARCHITECTURE.md (M7). |
| 07 | `DetailPane` takes the theme | Fine. |
| 08 | `diff_body` takes `truncated` | Fine. |
| 10 | `W` flips `word_tint` directly | **Not fine:** it flips a flag nothing reads (M1). |
| 12 | `OpenDetails(oid)`, caller-painted menu background | Fine. |
| 13 | `Radar::policy(self)`, marker column always reserved | Fine; no layout jump when the context lands. |
| 13b | Dispatch at the end of `update`, `head_facts` from `AppState` | Design fine; the per-update fan-out it enables is C1. |
| 14 | `probe_evil` from `follow_selection`, gated by `evil_at` | Half right: the gate on `shown` breaks "selected merge" (M3). |
| 15 | `ForcePushProbe` carries a `String`, not `RefName` | Fine. Unbounded fan-out is C1. |
| 16 | `ⓘ` for duplicates | Fine. |
| 19 | `sanitize_path_bytes` | Fine; every path reaching `ui` is sanitised. |
| 20 | Even-split strip bucketing | Fine. |
| 21 | `refresh(&self)`, unconditional re-anchor | Fine. |

Acceptance-walk facts: the force-push probe is unverified against a real rewrite (M9); the evil probe did run live at stop 10; no corpus commit has two files (stop 2 proves nothing about the file cursor — add one); the compare header still lacks `· N files +A −B` (accepted gap); stop 1 opening on `three: in progress` is the fixture's true row 0, not a bug.

## Does the promise hold?

*"What a commit changed is readable without leaving the graph, and a bad merge is flagged and explained."*

- **Readable without leaving the graph:** yes in mechanism — `Enter`, files tree, unified and side-by-side diffs, compare, working tree, three breakpoints, all from the graph — but the diff arrives grey (M2) and `W` does nothing (M1), so what the user first sees is not the designed view.
- **A bad merge is flagged and explained:** foxtrot, backward and accidental-pull are flagged in the graph from the corpus fixtures and Explain opens with the fade over the rewritten first-parent line (DoD met, `docs/shots/m2-explain-panel.txt`, `m2-06-*`, `m2-07-*`). Evil merges are flagged only after opening the pane (M3), and flagged too often (M6). Force-push has never been seen to fire (M9).
- **At scale:** not usable — C1 stalls the frame loop at 50k refs and spawns a job per tracked branch on any repository.

Verdict: the promise holds on the fixtures and small repositories once M1–M3 are fixed; C1 blocks the milestone.

## Appendix: fuzz and bench

| Check | Result |
|---|---|
| Fuzz `parse_unified` (`-max_total_time=60 -rss_limit_mb=1024`) | 693,819 runs in 61 s, no crash, RSS 660 MB |
| Fuzz `parse_file_list` (same) | 671,299 runs in 61 s, no crash, RSS 552 MB |
| Bench `graft-ui --bench row_render` (`--warm-up-time 1 --measurement-time 3`) | GraphColumn render 15.65 µs (M1: 15.1 µs, +3.6 %, under the 10 % bar), RSS 90 MB. `cargo bench -p graft-ui` without `--bench` fails: the `--lib` libtest target rejects criterion flags; TESTING.md's command should name the bench. `graft-graph` benches not rerun: no M2 change in that crate |

## Fixes, round 1a (2026-09-18, builder-opus)

The reviewer's findings above are untouched; this records what changed
against each of the cost-and-scheduling ones. Numbers: `docs/perf/m2-review-fix-1a.txt`.

**C1 — fixed at the cause, two separate costs.**

1. *Per frame.* `view::radar::findings_for` called `branch_facts` and
   `classify_refs` for every row, so each frame walked every ref once per
   row. The ref-derived findings are the same for the whole frame, so
   `update::radar::refresh_ref_findings` now works them out once, at the
   tail of `update`, into `RadarState::ref_findings`,
   `force_push_findings` and `force_push_not_checkable` (by commit), and a
   row looks itself up. The rebuild is skipped unless a stamp changed:
   `refs` and `context` by `Arc::ptr_eq`, plus `HEAD`, the default branch,
   `force_push_revision` and the policy. `branch_facts`/`head_facts` moved
   from `view::radar` to `update::radar` (the only caller now), which keeps
   `update` from depending on `view`.
2. *Per update.* `request_force_push_probes` asked one `ForcePushProbe`,
   and its `git` spawn, per upstream-tracked branch: 25,000 jobs on the
   50k-ref fixture. It now asks only about branches that could be on
   screen — the branch `HEAD` is on and the branches on resident rows,
   found by `RefBadges::on` hash lookup, never by walking the refs — and at
   most `MAX_FORCE_PUSH_PROBES` (32) per session.

   *What it costs the user:* a force-push verdict now appears for branches
   they can see, not for every branch at once. A branch first seen after 32
   others have been probed is not probed until `Ctrl+R` clears the map. A
   `Rewritten` badge on a branch that has never been on screen is the one
   thing lost; the marker appears as soon as the branch is scrolled to.

**M3 — fixed at the cause.** `DetailState::probe_evil` took the pane's own
spec, so nothing was probed with the pane closed. It now takes
`selected: Option<Oid>`, and `selection::follow_selection` passes the
selected merge whether or not the pane is open, so a selected evil merge is
flagged on the graph row. Cost: one `git` probe per selected merge even
when the pane is closed, cached by commit as before.

**M9 — partly covered.** Added: `frames::fifty_thousand_refs_keep_input_to_frame_under_16_ms_with_the_detail_pane_open`
(the 50k-ref scroll with the pane open, so every frame draws the header,
the radar lines and the file list), and a unit test that 100 tracked
branches off screen cost one probe. Not covered: a Map-view frame test and
a diff-scroll frame test — left for round 2.

**M10 — fixed at the cause.** `strip_marks` collects the search hits into a
`HashSet<Oid>` once per frame and tests each row against it, instead of
scanning the hit list per row.

**M11 — judged, not done as suggested.** Once C1's ref walk became a map
lookup, the pane's residual per-frame cost is a few small allocations for
one commit, so the suggested `detail_header: Option<Arc<DetailHeader>>`
cache was not added: a cache with its own invalidation is a real cost in
code, and the new pane-open frame test is how we would know it was needed.
If that test shows the header on the hot path, the cache goes in then.

**Numbers after the fix** (`docs/perf/m2-review-fix-1a.txt`): heavy tier
91 s (was 1275 s and killed); release perf 59 s, 11 passed, worst frame
9.6 ms against the 16 ms budget (was 604 s and killed). The heavy tier's
exit code is still 101, for one failure this fix did not cause and did not
fix: `heavy::slopes_per_commit_with_a_commit_graph` is 76 B/commit against
a 50 B/commit bar, and was 69 B/commit before this fix
(`docs/perf/m2-review-fix-1a-baseline-slope.txt`). The review never saw it
because its heavy tier was killed before reaching that test. It is a
separate finding and needs its own ticket.

## Fixes, round 1b (2026-09-18, builder-sonnet)

The reviewer's findings above are untouched; this records what changed
against M1, M2, M5 and M9's remainder. Numbers: `docs/perf/m2-review-fix-1b.txt`.

**M1 — fixed at the cause.** `line_style_at` computed `word_tinted` from
`line.words` alone; no code path read `DiffOptionsView.word_tint` or
`DetailState::word_tint`, so `W` changed a field nothing consumed. Fixed by
threading the flag in: `diff_body` and `render_diff_body` now take a
`DiffFocus { focused, word_tint }` (bundled into one struct — a fifth bare
bool would have pushed `diff_body` over clippy's 7-argument limit), and
`line_style_at` gates the words branch on `style_ctx.word_tint`. `W` now
turns the tint off; the default (`word_tint: true`) keeps today's on-by-default
look. Verified with a new unit test,
`word_tint_false_leaves_the_word_at_the_plain_row_background`
(`crates/graft-ui/src/detail_pane/tests.rs`), asserting the word's cell and
a plain cell on the same row get the same background when the flag is off.

*What it costs the user:* none — the default behaviour is unchanged; `W`
now does what its help text always claimed.

*Caption stale, not fixed (out of scope):* gallery stop 4
(`crates/graft/tests/it/gallery.rs`, "word tint" stop) still describes the
old always-on behaviour ("stop 1 with W: only the changed words darken");
same wording problem the reviewer flagged for TESTING.md under Minor 15.
Left for whoever picks up Minor 15, since fixing one without the other
leaves them contradicting each other.

**M2 — fixed at the cause.** `line_style_at` early-returned `TextMuted` for
every character when `!focused`, so the whole diff body went one flat
colour until `Tab` reached the Diff region. Removed that branch; diff-line
colour (added/removed/context) no longer depends on focus. Focus now shows
on the gutter only, via a new `gutter_style` helper (accent when focused,
muted otherwise) — there is no separate "Diff" region title to carry the
cue instead, so the gutter is the one focus indicator left. Verified with a
new unit test, `unfocused_diff_body_keeps_its_added_removed_colour`, which
renders the same hunk focused and unfocused and asserts the body's
foreground colour is identical and not `TextMuted`, while the gutter's
colour still differs between the two.

*What it costs the user:* the Diff region no longer has a full-body dimmed
state; the only unfocused cue left is the gutter. If that turns out to be
too subtle in practice, the next visual pass should say so — this is a
judgement call, not something the diff or the tests can settle by
themselves (flagged for the user, not silently decided).

**M5 — fixed at the cause.** Nothing reset `DetailState::max_lines` after
`load_all` (`L`) set it to `usize::MAX`, so the cap stayed off for the rest
of the session. Reset to `MAX_LINES` in two places: `load()` (new
commit/spec selected) and `move_files_cursor()` (cursor actually moves to a
different file), both in `crates/graft/src/update/detail.rs`. Verified with
a new unit test, `moving_the_file_cursor_resets_load_all`, which sets
`max_lines` as if `L` had just been pressed, moves the file cursor, and
asserts the cap is back to `MAX_LINES`.

*What it costs the user:* `L` now applies to the file it was pressed on
only, matching the reviewer's expected behaviour; a large file two files
over no longer inherits an unrelated load-all.

**M9 — remainder covered.** Added `Pane::DiffScroll` (`Enter`, `Tab`, then
the scroll — round 1a's pane-open test left focus on Files, so its
scroll moved the file cursor, not the diff) and `Pane::Map` (`M`, then the
scroll, on the same 50k-ref fixture) to
`crates/graft/tests/it/frames.rs`, plus one test each:
`fifty_thousand_refs_keep_input_to_frame_under_16_ms_scrolling_the_diff`
and `..._with_the_map_view_open`. Both pass under the 16 ms budget in
release: DiffScroll p50 6.3 ms / max 8.8 ms, Map p50 5.7 ms / max 8.8 ms
(`docs/perf/m2-review-fix-1b.txt`).

**Visual captures.** Regenerated the gallery captures that exercise M1 and
M2 (`.scratch/m2-details-and-diffs/pty-gallery-m2.py`): stop 1
(`m2-01-feature-branches-enter`, the baseline M2 was about — Enter, no
Tab) at 80x24 unicode/ascii, 100x30 unicode and 160x40 unicode; stop 4
(`m2-04-feature-branches-enter-w`, the `W` toggle M1 was about) at 80x24
and 160x40 unicode. All five files are byte-identical to what was already
committed, and that is expected, not a null result: the gallery's captures
are `pyte.Screen` character grids (`text()` in the pty script joins
`screen.buffer[y][x].data`), which carry no colour attributes, so a
colour-only fix cannot show up in them. They still confirm the fix did not
change what's drawn structurally (row layout, text, truncation markers).
Confirming the colours themselves — the actual point of M1 and M2 — needs
a real terminal; that was not done here. Recorded plainly rather than
implied by "captures refreshed": no capture in this repo's pipeline can
currently show a colour-only fix, and that gap applies to any future
colour-only finding, not just this one.

**Numbers** (`docs/perf/m2-review-fix-1b.txt`): fast tier 36 s, all green.
Heavy tier 112 s, exit 101 for the same pre-existing failure round 1a
documented and did not cause or fix,
`heavy::slopes_per_commit_with_a_commit_graph` (76 B/commit against the 50
B/commit bar) — confirmed by reading `target/m2-fix-perf-1b/heavy-tier.log`,
not assumed. Release perf 80 s, 13 passed, 0 failed, including the two new
frame tests.

## Fixes, round 1c (2026-09-18, builder-opus)

The reviewer's findings above are untouched; this records what changed
against M4 and M6, the two git-semantics findings. Numbers:
`docs/perf/m2-review-fix-1c.txt`.

**M4 — fixed at the cause.** `list_files_working_tree` ran plumbing `git
diff-index`, which trusts the index's stat cache and never refreshes it, so
a tracked file whose mtime moved without its content changing (a branch
switch, an editor save, a build) came back as `M` with a `0 0` numstat while
the pane's header — from `git status`, which does refresh — said nothing was
unstaged. Swapped to porcelain `git diff --raw --numstat -z -M -C HEAD`:
same output shape, but it refreshes the stat cache in memory and compares
content. `GIT_OPTIONAL_LOCKS=0` is already on every `GitInvocation`, so the
refresh never writes the index. Dropping the `0 0` entries instead would
have hidden the symptom and still mis-sorted a real binary change.

*Verified:* a new test,
`a_touched_but_unchanged_file_is_not_listed_as_modified`
(`crates/graft-git/src/diff/read/tests.rs`), on a writable copy of the
`Linear` corpus fixture: it writes a tracked file's own bytes back, asserts
`git diff-index --raw HEAD` still names it (without that assertion the test
would pass for the wrong reason if the setup stopped being stat-dirty), and
asserts `list_files(WorkingTreeVsHead)` lists nothing.

*What it costs the user:* nothing lost. `git diff` is porcelain, so it obeys
the user's `diff.*` config where `diff-index` did not; `--no-color`,
`--no-ext-diff` and explicit `-M -C` already pin everything that would
change the parsed output.

**M6 — fixed at the cause; detection narrowed, and Explain says what it
cannot see.** `--remerge-diff` being non-empty is not evidence of an evil
merge: it is also exactly what a hand-resolved conflict leaves behind.

The dividing line is a git semantic that is exact in one direction: a
conflict can only arise in a file *both* sides changed since the merge base.
So when the remerge diff is non-empty, `probe_evil_merge` now computes the
merge base and each side's changed paths (`merge-base`, then one `diff-tree
-r --no-renames --name-only -z` per side) and splits the named files:

- a file at most one side touched could not have conflicted, so the merge
  invented the edit — `EvilVerdict::Evil`, still a Warning;
- everything else — `EvilVerdict::Resolved`, a new variant that is never a
  finding. The pane shows it as `evil merge: clean (N files hand-resolved)`
  (`RadarLine::EvilResolved`), so the line reports the fact without reading
  as a problem;
- no merge base (unrelated histories) — `NotCheckable` with the reason, not
  a guess in either direction.

Cost: three extra `git` spawns, and only for a merge whose remerge diff is
non-empty. A clean merge still costs the one spawn it always did, and the
verdict is cached per commit as before.

*What the check still cannot distinguish, stated in Explain rather than
guessed at:* a file both sides changed. A real evil edit hidden inside a
conflict resolution lands in `Resolved` and is not flagged. The `EVIL MERGE`
explanation now says so in full — "Files both sides changed are not listed:
a hand-resolved conflict looks the same there, so the check stays quiet
rather than warn on ordinary work" — and points at `git show
--remerge-diff`, which shows those files too. Three paragraphs still (the
explain test caps it at three); the first now says *why* the listed files
could not have conflicted.

*Why narrowed rather than downgraded to Info:* an Info finding still puts a
radar marker on every conflict-resolved merge in the graph, which on a busy
repository is most merges. That is the same "trains the user to ignore it"
failure one severity down. A false negative the explanation admits to is
honest; a marker on ordinary work is not.

*Verified, two ways.*
1. A new test, `a_hand_resolved_conflict_is_not_an_evil_merge`
   (`crates/graft-ops/src/mistake_radar/probes.rs`), builds a real conflict
   in a writable copy of the `Trivial` fixture — both sides edit
   `shared.txt`, the merge is asserted to actually fail, the conflict is
   resolved by hand — and asserts `Resolved { files: ["shared.txt"] }` and
   no finding. `the_evil_fixture_names_the_file_the_merge_invented` still
   passes: `evil.txt` is in neither parent, so it is still `Evil`.
2. Against real history, independently of Graft's Rust:
   `scripts/m2-fix-1c-evil-audit.sh 60` over the 60 newest two-parent merges
   of the git/git clone (`~/.cache/graft/git-git.git`). 53 clean either way;
   the old check warned on 7, the new one warns on 1. The 6 that fell away
   are hand-resolved conflicts. The 1 that remains is
   `c1ab07b8 Merge branch 'master' of https://github.com/j6t/gitk`, which
   changes `gitk-git/README.md` although neither side touched it since the
   base — a genuine evil merge, not an artefact of the narrowing. Counts in
   `docs/perf/m2-review-fix-1c.txt`.

*Not observed:* nothing here was judged by eye. The pane's new
`evil merge: clean (N files hand-resolved)` line has no capture — the
gallery's pyte grids carry no colour, but this is a text change, so it is
capturable; it simply was not, because no gallery stop lands on a
conflict-resolved merge and the corpus has no such shape. Adding one is a
fixture change, out of this round's scope.

*Gates:* `cargo fmt --check`, `cargo clippy --all-targets -D warnings` and
`cargo test` (fast tier) all pass — 28 test binaries green, 0 failed. No
heavy tier or release perf run this round: nothing here touches the frame
path.

## Fixes, round 1d (2026-09-18, builder-sonnet)

The reviewer's findings above are untouched; this round is M7, M8 and
Minor 15 — the last before re-review. No git-semantics or runtime behaviour
changed: every fix here is either documentation or a pure file split with
public signatures held fixed. Full gates, no heavy tier or perf run
(nothing here touches the frame path or a hot loop).

**M7 — fixed at the cause.** `ARCHITECTURE.md`'s "Shared types" table still
described the M1-era design: `MistakeRadarKind` "attached to a `graph::Row`
as plain input data". M2 never built that — Mistake Radar classification
lives entirely in `app` (`update::radar`/`view::radar`), against a
`RadarContext` a background job builds once per refresh, and `graph` never
sees a finding at all. Rewrote that entry to say what M2 actually built,
including commit `92c35ac`'s move of the radar's default-branch chain onto
a `HandleSet` over its own `Arc<CommitTable>` (ADR-0026, housekeeping/02) —
the current tree, not the review's snapshot. Also added three "Where to
start reading" bullets M2 introduced no entry for: the detail pane/diff
views/Explain panel, the Map view/overview strip, and the Mistake Radar's
detection vs. its background jobs — each naming the crate and module a
change would actually touch, the same shape as the existing bullets.

*Verified:* read against `crates/graft-ops/MAP.md` and the relevant
sections of `crates/graft/MAP.md` (`update::radar`, `view::radar`,
`jobs::radar`, `map`) — not source, per the ticket's read-first rule. No
code changed; this is a documentation-only fix, so there is nothing to
gate beyond the fixes below (same `cargo fmt`/`clippy`/`test` run covers
the whole round).

**M8 — fixed at the cause; all 8 files split, none needed a design
decision.** Every file the review named was a single-responsibility file
that had grown past the ~400-line rule by accretion, not by mixing
unrelated concerns — each split follows a seam the file's own structure
already implied (a byte-parsing layer under a grammar layer, a walk layer
under a row-building layer, view-model types under their drawing code,
child-process plumbing under `GitInvocation`'s own methods). None of the
eight needed a new interface or a behaviour change; every public path,
signature and doc comment is unchanged, only relocated. `codebase-design`
skill's language: each split moves an *internal* seam that already existed
inside the file's own code into a file boundary — no new seam was invented.

- `crates/graft-ui/src/detail_pane.rs` (1330 → 43 non-test lines) → `body.rs`
  (`DetailPane` widget, Explain panel), `header.rs`, `files.rs`,
  `layout.rs`, `line_render.rs`, `unified.rs`, `side_by_side.rs`.
- `crates/graft/src/update/detail.rs` (1045 → 31) → `detail/state.rs`
  (`DetailState` and its caches), `detail/ops.rs` (`impl DetailState`),
  `detail/control.rs` (`LocalJobs`, `detail_event`, `effective_layout`,
  `open_explain`/`close_explain`), `detail/keys.rs` (`detail_key`).
- `crates/graft/src/update/mod.rs` (846 → 366) → `geometry.rs` (pure screen
  geometry a mouse event needs), `mouse.rs` (what a mouse event does once
  `geometry` says where it landed).
- `crates/graft/src/map.rs` (669 → 286) → `map/walk.rs` (refs, span, default
  chain, `walk_branch`/`RowWalk`, one bounded walk per branch), `map/rows.rs`
  (`RowContext`, `build_row`/`build_rows`, one row per local branch).
- `crates/graft-git/src/diff/unified.rs` (558 → 415) →
  `unified/primitives.rs` (numbers, percentages, quoted-path unquoting — no
  header or hunk grammar).
- `crates/graft-ui/src/map_screen.rs` (507 → 126) → `map_screen/render.rs`
  (`MapScreen`'s `Widget` impl and every draw helper).
- `crates/graft-git/src/run.rs` (484 → 358) → `run/exec.rs` (the
  child-process machinery `spawn`/`supervise` are built on: pipe draining,
  kill/reap, `GitArg` → `OsString`/shell-quoted text).
- `crates/graft-git/src/diff/read.rs` (457 → 316) → `read/raw.rs` (revision
  resolution and raw/numstat splitting: `Sides`, `resolve_sides`,
  `first_parent`, `split_raw_and_numstat`).

*A real gap this uncovered, fixed alongside it:* `update`'s purity guard
(`update_cannot_reach_the_world`, `crates/graft/src/update/tests.rs`) reads
its own source with `std::fs::read_dir` on `update/` directly — not
recursive. Splitting `update/detail.rs` and `update/mod.rs` into
subdirectory files (this round's fix, and the earlier round that produced
`update/detail/*.rs`) put four files — `detail/state.rs`, `detail/ops.rs`,
`detail/control.rs`, `detail/keys.rs` — outside that scan entirely: the
test kept passing, but it was no longer checking them for `ratatui`,
`std::io`, threads or a clock. A green gate was not evidence the guard
still covered the module (rules-rationale's point exactly). Fixed at the
cause: the scan now walks subdirectories, still skipping every `tests.rs`
wherever it sits. Extracted into a helper (`update_sources`) rather than
inlining the walk, since the walk plus the assertions together tripped
clippy's line-count lint. *What it costs the user:* nothing — this closes
a silent coverage gap the file splits opened; no behaviour changed.

*Verified, per split:* `cargo build -p <crate> --quiet`, then
`cargo clippy -p <crate> --all-targets --message-format=short -- -D
warnings`, then `cargo test -p <crate> --quiet`, comparing pass counts
against the pre-split baseline (`graft-ui` 186+1+2+1+2+1, `graft` 466+14,
`graft-git` 134+49) — identical every time, so no test lost coverage and
none newly failed. `wc -l` confirms every non-test file landed under 400
lines. After all eight splits, `cargo fmt` reflowed a handful of import
lists and one over-long signature (pre-existing formatting drift as well
as new splits); `cargo fmt --check`, `cargo clippy --all-targets -D
warnings` and `cargo test --quiet` then ran clean across the whole
workspace in one pass (every crate, not just the eight touched) — 466+0
(graft), 134+49 (graft-git), 186+1+2+1+2+1 (graft-ui), 97/97, 29+2 ignored,
31, 2, 150, 2 (doctests) — 0 failed anywhere. Each crate's `MAP.md` got a
one-line note per split; no public item's line changed, since none of the
signatures moved.

*Not run:* the heavy tier, fuzzing, benchmarks, or a release perf run —
none of these files are on a hot path this round touches, and the ticket
excludes them explicitly.

**Minor 15 — fixed at the cause.** `TESTING.md`'s "Diff body" paragraph and
gallery stop 4's two captions (`crates/graft/tests/it/gallery.rs`) still
described word tint as always-on with `W` doing nothing observable — the
wording from before round 1b's fix (M1 above) made `W` actually toggle it.
Rewrote both to say what the code now does: word tint is on by default,
`W` turns it off (line colour — added/removed/context — unaffected either
way), pressing it again turns it back on. Stop 4's caption no longer claims
`W` is what makes the words darken; it says `W` removes the darkening.

*Verified:* read against round 1b's own description of the fix (above) and
`crates/graft-ui/src/detail_pane/tests.rs`'s
`word_tint_false_leaves_the_word_at_the_plain_row_background` (name only,
not the body — this round did not need to read the test itself to confirm
the direction the flag now runs). The new captions also had to fit
`concrete_and_fits`'s per-line cell-width budget (79 cells including the
`"shows: "`/`"differs: "` prefix) — took four rewrites to land both lines
under it. `cargo test -p graft --test it
gallery::tests::the_details_group_walks_the_19_stops_with_their_keys_and_fits`
passes; no snapshot depends on the caption text, so nothing else needed
re-running. Not judged by eye: this is a caption/doc wording fix, not a
rendering change.

*What it costs the user:* nothing — the two docs now agree with each other
and with the code; no behaviour changed anywhere in this round.

*Gates:* `cargo fmt --check`, `cargo clippy --all-targets --message-format
short -- -D warnings`, `cargo test --quiet` and `cargo build --quiet`, all
run workspace-wide (not just the touched crates) after every file split —
all pass, 0 failed. The `update` purity guard specifically re-run and
confirmed green after being made recursive. No heavy tier, no release perf
run, per the ticket's scope.

*Turn count:* this round ran long — 8 file splits across 4 crates, each
needing its own read/write/build/clippy/test cycle, plus the purity-guard
fix the splits themselves uncovered. The session compacted once before
this round started (context handed off via summary, per CLAUDE.md); from
resumption to this write-up was on the order of 90 tool calls, well past
the old 60-turn flag. Per ADR-0029 this is reported rather than used to
cut the work short: the ticket was one coherent unit (all 8 M8 files plus
M7/Minor 15 for one re-review pass) and stopping partway would have left
`docs/reviews/m2-details-and-diffs.md` describing a mix of split and
unsplit files, which is worse than one long round. Nothing here was
rushed to fit a number.

## Round 2 (2026-09-18, reviewer, Fable high)

Range `183c6ca..d994f38`; the fix rounds themselves are `6a9c579..HEAD`
(51 files, +6,524 / −5,067). Read in the CLAUDE.md order: stat, MAP.md
files, then the five fix commits' diffs crate by crate; whole files opened
only to confirm a finding. Nothing under `docs/shots/` was written by this
round; two throwaway repositories under `/tmp` were used to check git
behaviour (git 2.54.0, Apple).

**0 Critical, 3 Major, 17 Minor.** The Critical is gone. Two of the four
git-semantics fixes (M4, M6) each fixed the symptom and rest on a claim that
is false on this machine; M3 is fixed in code and contradicted by the
acceptance docs. Nothing blocks the milestone; the three Majors are all
narrow and cheap.

### Gates (this round)

| Check | Result |
|---|---|
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass (cached, 0.2 s, RSS 95 MB) |
| `cargo clippy --all-targets --features heavy-tests -- -D warnings` | pass, 4.6 s, RSS 495 MB |
| `cargo test` (fast tier) | pass, **24.3 s** wall (was 21 s), RSS 350 MB, **1209 passed / 0 failed / 9 ignored** |
| Fuzz `parse_unified` (45 s, its file was split in 1d) | 521,642 runs, no crash, RSS 664 MB |
| Heavy tier, release perf, benches | **not rerun** (conductor); latest on record `docs/perf/housekeeping-02.txt`: heavy tier exit 0 in 161 s, release perf exit 0 in 83 s, heap slope 31.5 B/commit (bar 50). Round 1b: worst frame 9.6 ms of 16 with the pane open, diff scrolling and Map on the 50k-ref fixture |

Fast tier, 5 slowest: 7.76 s `diff_fixture::parse_unified_matches_numstat_for_every_shape` (was 6.42),
5.99 s `layout::stream::tests::scrolling_the_whole_history_keeps_the_resident_set_bounded`,
3.30 s `corpus::two_builds_produce_the_same_object_ids`, 1.18 s `corpus::a_second_build_in_the_same_place_is_a_no_op`,
0.92 s `raster_pixels::reports_per_strip_cost`. `target/` 12 GB (was 9.8).

`unwrap()`/`expect()` outside tests: none new (every hit in the post-review
diff is inside an inline `#[cfg(test)]` module). No new `let _ =`. The
`unreachable!` in `resolve_sides` moved to `read/raw.rs` (Minor 1 stands).
Every new string reaching `ui` is sanitised: `NotCheckable` reasons via
`sanitize_line`, Explain paragraphs via `view::explain`, `EvilResolved`
carries a count.

### Did the fixes fix it?

| Finding | Verdict | Notes |
|---|---|---|
| C1 | **confirmed** | `refresh_ref_findings` rebuilds three maps only when the stamp (`refs`/`context` by `Arc::ptr_eq`, `HEAD`, default branch, `force_push_revision`, policy) changes; a row is a hash lookup. Probes bounded to `HEAD`'s branch plus resident rows, 32 per session; `probes_are_bounded_by_what_is_on_screen_not_by_the_ref_count` covers it. Numbers above. Residual: `force_push_candidates` still walks the resident window on every `update` (R2 Minor 16). |
| M1 `W` | **confirmed in code** | `line_style_at` gates the words branch on `DiffRowStyle::word_tint`, fed from `DiffOptionsView.word_tint`; `word_tint_false_leaves_the_word_at_the_plain_row_background`. Colour unverified by construction (KNOWN_GAPS "Unverified"); not ticked, not failed. |
| M2 unfocused body | **confirmed in code** | `!focused → TextMuted` early return removed; focus now on the gutter (`gutter_style`); `unfocused_diff_body_keeps_its_added_removed_colour`. Same unverified status. |
| M3 evil probe follows selection | **fixed, new problem** | `probe_evil(selected, …)` from `selected_merge(state)` regardless of `shown`; the view reads `evil_at` without `shown`. Unit-tested only; never observed. TESTING.md stop 8 and `gallery.rs:145` now assert the opposite (R2 Major 3). |
| M4 stat-dirty files | **fixed, new problem** | The listing is right and tested. The claim it rests on — `GIT_OPTIONAL_LOCKS=0` keeps `git diff` from writing the index — is false (R2 Major 1). |
| M5 `L` sticks | **confirmed** | `max_lines = MAX_LINES` in `load()` and `move_files_cursor()`; `moving_the_file_cursor_resets_load_all`. |
| M6 evil false positives | **narrowed, new problem** | Hand-resolved *content* conflicts are `Resolved` and silent (tested, and 7 → 1 on git/git's 60 newest merges). The dividing line is wrong for renames: a hand-resolved rename/modify conflict is still `Evil` (R2 Major 2). The false negative is disclosed in Explain and the probe's doc, nowhere in KNOWN_GAPS.md or TESTING.md (R2 Minor 2). |
| M7 ARCHITECTURE.md | **partly** | The `graph::Row` claim is fixed and three "where to start" bullets are right. Still undocumented: the typed runner `git::run` (every M2 spawn goes through it), `git::contained`, the `LOCAL_JOB_BIT` id split, the `update/` purity guard (R2 Minor 12). |
| M8 file sizes | **confirmed, with caveats** | 7 of the 8 named files are under 400 non-test lines; `diff/unified.rs` is 417 and the note says all are under (R2 Minor 9). Public `pub` sets are identical before and after every split except `ramp_glyph` (R2 Minor 6); several `pub(super)` items widened to `pub(crate)` (R2 Minor 7). Boundaries judged under "Round 1d" below. |
| M9 tests | **partly** | Three frame tests (pane open, diff scroll, Map) on the 50k-ref fixture, heavy-gated; probe-count, stat-dirty, `W` style and unfocused-colour unit tests. Still nothing on: the force-push probe against a real rewrite (accepted gap, user 2026-09-18), `Resolved` in the pane, the pane-closed evil marker. |
| M10 strip | **confirmed** | `HashSet<Oid>` of hits once per frame; `subject()` instead of decoding the message. Residual: the set is rebuilt every frame (R2 Minor 5). |
| M11 | **won't-fix (user)** | Recorded at `6eab12b`; the pane-open frame test is the tripwire. |

### Major

#### R2 Major 1. Porcelain `git diff` writes `.git/index` regardless of `GIT_OPTIONAL_LOCKS=0`; the M4 fix's central claim is false

- **Where:** `crates/graft-git/src/diff/read.rs:148-166` (`list_files_working_tree`, `git diff --raw --numstat -z -M -C HEAD`), doc comment at `:150-157`; `crates/graft-git/src/run.rs:12` ("a read never takes the index lock from under the user's own git"); review round 1c above.
- **What:** verified twice in a throwaway repository on git 2.54.0: `touch f.txt`, copy `.git/index`, run the exact command with `GIT_OPTIONAL_LOCKS=0` *and* `--no-optional-locks` — the listing is empty (correct) **and `.git/index` is rewritten** (`cmp` differs; `git diff-index --raw HEAD` goes from 1 entry to 0). `git diff`'s `refresh_index_quietly` takes `index.lock` non-blocking and writes if it gets it; only `git status` honours the optional-locks switch. With `index.lock` already held the write is skipped and the listing is still correct — so the correct output never depended on the write, only `diff.autoRefreshIndex`'s in-memory `skip_stat_unmatch` did.
- **Why it matters:** M2 is a read-only milestone and Graft now writes the user's index (stat cache only — no content at risk) on every working-tree listing: selecting the working-tree row, every `Ctrl+R`, every re-follow. It briefly holds `index.lock`; a `git commit`/`git add` the user runs at that instant fails with "index.lock exists". The runner's module doc, the function's doc and the round-1c note all state the opposite. A viewer that writes the repository is exactly what `GIT_OPTIONAL_LOCKS` exists to prevent.
- **Fix (cause):** go back to plumbing `diff-index --raw --numstat -z -M -C HEAD` and drop entries that are `M`, same mode on both sides, new oid all zeros, numstat `0 0` — `--numstat` is computed from content, so `0 0` is exactly "identical after conversion"; mode changes keep differing modes and binaries print `- -`, so nothing real is lost. Add to the M4 test: the index bytes are unchanged after `list_files`. Fix the two doc comments either way.

#### R2 Major 2. A hand-resolved rename conflict is still an `EVIL MERGE`

- **Where:** `crates/graft-ops/src/mistake_radar/probes.rs` `split_remerge_files` (partition on "path in both sides' `diff-tree --no-renames --name-only` sets"), `changed_paths` (`--no-renames` on purpose).
- **What:** the invariant "a conflict can only arise in a file both sides changed since the base" is true for paths git *reports*, not for paths the remerge diff *names*. Built in a throwaway repository: base has `foo`; side A `git mv foo bar` and edits line 1; side B edits line 1 of `foo`; `git merge` conflicts in `bar`; resolved by hand. `git show --remerge-diff --name-only` names `bar`; side A's set is `{foo, bar}`, side B's is `{foo}`; `bar` is not in both → `EvilVerdict::Evil { files: ["bar"] }`, `⚠ EVIL MERGE`, and Explain says "no conflict could have arisen there" about the file that did conflict. Rename/rename (both sides rename `foo` differently, one name kept) fails the same way. Merges after a file move are ordinary work on any repository that refactors.
- **Why it matters:** it is the failure M6 was about — a warning on ordinary work — surviving on a common shape, and the new Explain text now asserts a falsehood in that case. The git/git audit of 60 merges did not happen to contain one.
- **Fix (cause):** compute each side's set with rename detection (`diff-tree -r -M --name-status -z`) and put **both** names of an `R` entry in that side's set, so the post-rename path counts as "changed by both" whenever its pre-rename path was; keep `--no-renames` out. Add the rename/modify case beside `a_hand_resolved_conflict_is_not_an_evil_merge`, and rerun `scripts/m2-fix-1c-evil-audit.sh` with the same change in the shell so the two agree.

#### R2 Major 3. The M3 fix was never observed, and the acceptance walk now asserts the opposite

- **Where:** `TESTING.md:704-708` (graph stop 8, "Nothing on screen marks the merge as evil … this stop is the control"), `crates/graft/tests/it/gallery.rs:145` ("the control: evil is a tree fact (M2), so it must look like stop 3"), `docs/shots/35-evil-merge-{ascii,unicode}.txt` (0 `⚠`, M1 era), `docs/shots/m2-10-evil-merge-enter-*.txt` (last regenerated at `71d28ab`, before the review).
- **What:** since round 1a the selected merge is probed with the pane closed and the marker is painted from `evil_at` alone; the gallery runs the real job runner with the radar on and the cursor lands on the merge at stop 8. So stop 8 now shows `⚠` — the "control" text tells the user that is wrong. No capture after the fix exists in either group. This round tried to take one with the M1 pty script into `target/` and could not: `pty-gallery.py` no longer walks the current gallery ("footer never appeared within 60 s"), so the M1 procedure TESTING.md still points at is dead too.
- **Why it matters:** CLAUDE.md's acceptance rule — a checklist saying concretely what right looks like — is now wrong at the stop that demonstrates the second half of the promise; the only evidence for the fix is a unit test that a `Command::EvilProbe` is emitted.
- **Fix:** rewrite stop 8 (graph group) and its caption to expect `⚠` on row 0 after the probe lands (stop 10 keeps the pane); regenerate `35-evil-merge-*` and `m2-10-*` with the M2 script; either repair `pty-gallery.py` or point TESTING.md's M1 walk at `pty-gallery-m2.py`.

### Minor

1. `crates/graft-ui/src/detail_pane/header.rs:55` — `evil merge: clean (N files hand-resolved)`: `clean` is `EvilVerdict::Clean`'s word ("the tree is what merging its parents gives"), and this verdict is the case where it is not. Say what was found: `evil merge: none found · N hand-resolved files not checked`.
2. The `Resolved` false negative is disclosed only inside the Explain paragraph and the probe's doc comment. CLAUDE.md puts open items in `KNOWN_GAPS.md`; TESTING.md's radar section should also say a both-sides evil edit is not flagged. One row, expiry when rename-aware sets land.
3. `probes.rs` `merge_base` — takes the first line of `git merge-base`; with criss-cross history there are several bases and the recursive/ort strategy merges against a virtual one, so a path "changed by one side" against the first base can be both-changed against the real base (or vice versa). Not reproduced; note it in the doc comment or use `--all` and union the sides' sets over every base.
4. `update/selection.rs` `selected_merge` → `probe_evil` on every cursor move: holding `j` through git/git (a merge every few rows on the default view) spawns a `git show --remerge-diff` — a full in-memory merge — per merge row and kills it ≤50 ms later; the 16-entry `evil_cache` thrashes. Spec-compliant ("only the selected merge") and the reviewer's own suggestion, but the cost was not stated in round 1a. Probe when the cursor rests (the next tick after a move with no further move), not on the move.
5. `view/strip.rs:38-42` — the `HashSet<Oid>` of search hits is rebuilt every frame the strip is shown: O(hits) allocation per frame, which for a search matching most of git/git is the cost M10 moved rather than removed. Cache it beside the search result (rebuild when `state.search` changes).
6. `crates/graft-ui/src/map_screen/render.rs` — `ramp_glyph` went `pub` → `pub(crate)`; `crates/graft-ui/MAP.md:151` still lists it as a public item while `:152` says "now `pub(crate)`", and the 1d note claims no public signature changed. Pick one: restore `pub` or drop the MAP.md line.
7. Visibility widened where narrowing would do: `detail_key`/`detail_event` `pub(super)` → `pub(crate)` (`update/detail.rs:25-27`); 23 `pub(crate)` items in `detail_pane/*` (17 in `line_render.rs`), 11 in `map/{walk,rows}.rs`, 11 in `unified/primitives.rs`, 11 consts in `update/detail/state.rs` — every one reachable crate-wide when only the parent module needs it. `pub(super)` throughout, as `run/exec.rs` and `diff/read/raw.rs` already do.
8. `crates/graft/src/update/detail/{state,ops}.rs` — the seam is "the struct" versus "its `impl`" (`ops.rs`: 20 methods spanning file loading, the evil probe, focus/options, scrolling and pane resizing, 395 lines). That is a cut, not a boundary. Split `ops.rs` by concern: `loading.rs` (`open`/`close`/`load`/`files_arrived`/`show_cursor`/`diff_arrived`/`contained_arrived`/`cancel_pending`), `evil.rs` (`probe_evil`/`evil_probed`), `pane.rs` (`cycle_focus`/`set_options`/`load_all`/`scroll_*`/`resize_pane`/`set_pane_cells`). Same shape in `detail_pane/line_render.rs`: draw helpers, `DiffRow`/`RowSlot`/`DiffRowStyle` types and style functions in one 17-item file, with `percent` off in `layout.rs`.
9. `crates/graft-git/src/diff/unified.rs` — 417 non-test lines after the split; the 1d note says `wc -l` confirmed every file under 400. `ops.rs` 395 and `map_screen/render.rs` 393 are at the edge.
10. `detail_pane/side_by_side.rs` `diff_body_side_by_side(…, focused: bool, word_tint: bool)` beside `diff_body(…, focus: DiffFocus)` — the struct was introduced to avoid two bare bools and then one of the two callers kept them. Pass `DiffFocus` to both.
11. `crates/graft/src/update/tests.rs:1480-1557` purity guard. It is recursive now and scans 20 files; **nothing slipped while it was blind**, because no subdirectory ever existed under `update/` in a committed tree before `9982783` (`183c6ca` and `9982783^` are both flat) — the 1d note's "the earlier round that produced `update/detail/*.rs`" is wrong; the blind window lived only inside that builder's working copy. Residue: the floor is `>= 7` against 20 real files (assert the list, or `>= 20`, so a broken walk fails loudly); the substring check misses `use std::{time, thread}` and `core::time`; the comment "every other file is pure end to end and is kept whole" is false — six files (`context_menu`, `detail`, `map`, `selection`, `radar`, `strip`) carry inline `#[cfg(test)]` modules the guard also scans (safe direction: a test mentioning `Instant` trips it).
12. `ARCHITECTURE.md` — M7's residue: no mention of `git::run` (the typed runner every M2 spawn uses, and where `GIT_OPTIONAL_LOCKS`/`LC_ALL` are set), `git::contained`, the `LOCAL_JOB_BIT` split of job ids, or the `update/` purity guard as a stated rule.
13. `crates/graft-git/src/run.rs:12` and `diff/read.rs:155` — both state that a read never takes the index lock / never writes the index. False for `git diff` (Major 1); fix the text with the code.
14. `crates/graft/src/jobs/radar.rs` `impl PartialEq for RadarContext` — tables compared by `Arc::ptr_eq`, so two `of_ids` contexts with identical members are unequal, and `Eq` promises more than delivered. Say so in the doc, or compare the chain by ids when the tables differ.
15. `scripts/m2-fix-1c-evil-audit.sh` re-implements the probe's rule in shell. Fine as evidence once; it will drift from `split_remerge_files` the first time the rule changes (Major 2 changes it). Either keep the two in step in the script's header or have the audit call the Rust probe.
16. `update/radar.rs:397` `force_push_candidates` walks the resident window (`refs.on(id)` per row, allocating a `Vec`) on every `update`, including key repeat, even after the 32-probe cap is reached before the loop (the early return at `:375` covers only that case once the map is full). Stamp it on the window's generation like `refresh_ref_findings`, or return early when nothing new could be resident.
17. Fast tier is 24 s (was 21); `parse_unified_matches_numstat_for_every_shape` grew 6.4 → 7.8 s. Under the 7.1 budget; watch it in M3.

### Round 1a–1d code, reviewed for the first time

- **`92c35ac` (ADR-0026 handles):** sound. `RadarContext` holds a `HandleSet` plus the history's own `Arc<CommitTable>`, so `on_default_chain(&Oid)` is `handle_of` + a bit test and a rebuilt table cannot alias the handles; `of_ids` gives tests and `context_menu` a self-contained table; `HandleSet: PartialEq` handles trailing zero words from `remove`. Slope 76 → 31.5 B/commit measured outside any helper's context. Minor 14 is the one rough edge.
- **`9982783` splits, boundaries judged:** good — `run/exec.rs` (child-process plumbing under `GitInvocation`), `diff/read/raw.rs` (revision resolution and raw/numstat splitting), `diff/unified/primitives.rs` (byte parsers under the grammar), `map/{walk,rows}.rs`, `update/{geometry,mouse}.rs` (where a click landed / what it does), `map_screen/render.rs`. Weak — `update/detail/{state,ops}.rs` and `detail_pane/line_render.rs` (Minor 8). `pub` sets identical for every file but `ramp_glyph` (Minor 6); visibility otherwise widened rather than narrowed (Minor 7). The purity guard fix is right and the history claim around it is wrong (Minor 11).
- **`32845d3` (C1/M3/M10):** as tabled above; `RefFindings::same_as` is the right shape for the stamp.
- **`f52e1b2` (M1/M2/M5):** as tabled; Minor 10.
- **`ca120c0` (M4/M6):** Majors 1 and 2; the two new tests are the right tests for what they claim, they just do not claim the thing that is false.

### Does the promise hold?

*"Details and diffs, and bad merges get flagged and explained."*

- **Details and diffs:** yes. `Enter`, files, unified and side-by-side, compare, working tree, three breakpoints, from the graph; the diff arrives in colour and `W` does something (by test — colour itself is unverified by construction, KNOWN_GAPS). At 50k refs the frame stays under 10 ms with the pane open, the diff scrolling and the Map up; the heap slope is under its bar. The one wart is that reading the working tree writes the index (Major 1).
- **Bad merges flagged and explained:** foxtrot, backward, accidental-pull and duplicates — flagged in the graph from the corpus and explained, unchanged since round 1. Evil merges — flagged for the selected merge with the pane closed (in code; unobserved, and the acceptance walk says otherwise, Major 3); hand-resolved content conflicts no longer warn, hand-resolved rename conflicts still do (Major 2); a both-sides evil edit is not flagged, and Explain says so. Force-push — bounded to what is on screen, never seen firing (accepted gap).
- **At scale:** now holds on the numbers on record.

Verdict: **the promise holds** on the fixtures and on real repositories of any size the tests reach, with two narrow false claims (index write, rename conflicts) and one contradicted acceptance stop to close. No Critical; M2 can ship on the fixes above.

## Fixes, round 1e (2026-09-19, builder-opus)

The reviewer's findings above are untouched; this round is the three Majors
of Round 2. Full gates green (`cargo fmt --check`, `cargo clippy
--all-targets -D warnings`, `cargo build`, `cargo test` — 1216 passed, 0
failed, 9 ignored; was 1209). No heavy tier or perf run: nothing here is on
the frame path, and round 2's figures stand.

**Every git claim in this round was checked in a throwaway repository on
git 2.54.0 (Apple) before any code changed**, which is what both earlier
rounds skipped.

**R2 Major 1 — fixed at the cause.** Confirmed the reviewer's finding first:
with a stat-dirty file, `GIT_OPTIONAL_LOCKS=0 git --no-optional-locks diff
--raw --numstat -z -M -C HEAD` leaves `.git/index` **changed** (`cmp`
differs), while the same arguments to `diff-index` leave it **identical**.
So `list_files_working_tree` is back on plumbing `diff-index --raw --numstat
-z -M -C -r HEAD`, and Graft no longer writes the user's index or takes
`index.lock` on a working-tree listing. Cost to the user: none — the listing
is the same, and one class of "index.lock exists" failure in their own
terminal is gone.

The M4 bug it was meant to solve is filtered instead, but **not in the shape
Round 2 proposed**: on this git, `--numstat` does not print a `0 0` record
for a stat-dirty file, it prints *no record at all* (`--raw` names it,
`--numstat` does not). A filter on `0 0` would therefore never have fired,
and the positional raw/numstat join would have failed with a parse error —
which is exactly what the first run of the existing test showed. So the two
streams are now walked together in step (`join` in `diff/file_list.rs`), and
the new `parse_worktree_file_list` drops a `--raw` record that has no
`--numstat` record beside it *and* is shaped like the stat cache lying (`M`,
one mode on both sides, all-zero new id). Every other gap is still a
`ParseError`, and `parse_file_list` (tree-to-tree) accepts no gap at all.
`-w` is unaffected: `diff-index -w` reads content itself, prints real ids,
and still does not write the index (checked).

*Verified:* `listing_the_working_tree_does_not_write_the_index` asserts the
`.git/index` bytes are identical before and after `list_files` on a
stat-dirty fixture — the thing that must not change, not a proxy for it.
Round 1c's `a_touched_but_unchanged_file_is_not_listed_as_modified` still
passes. Four parser tests cover the drop and the three things that must
survive it (mode change, binary `- -`, a missing record for a real status is
still an error). The two false doc comments (`run.rs`, `diff/read.rs`) now
say what git does.

**R2 Major 2 — fixed at the cause.** Reproduced first: base `foo`, side A
renames it to `bar` and edits line 1, side B edits line 1 of `foo`; git
conflicts in `bar`, resolved by hand. With `--no-renames` the sides' sets
are `{foo, bar}` and `{foo}`, their intersection is `{foo}`, and the remerge
diff names `bar` — hence `Evil { ["bar"] }`. Note that adding both names of
a rename to each side's set is *not* enough on its own (`bar` is still
outside the intersection): the rename has to carry the verdict. So
`changed_paths` now reads each side with `diff-tree -r -M -C --name-status
-z`, returns `SideChanges { changed, renames }`, and `split_remerge_files`
intersects the two `changed` sets and then, for every rename `old -> new` on
either side, adds `new` when `old` is in the intersection. Cost to the user:
the probe's existing blind spot widens the same way — an evil edit to the
post-rename path lands in `Resolved` — which is the direction it already
errs in for every file both sides touched, and is documented on the
function.

*Verified:* new `a_hand_resolved_rename_conflict_is_not_an_evil_merge`
builds that repository and asserts `Resolved { ["renamed.txt"] }`; it uses a
40-line file because git's 50% similarity threshold does not find the rename
in a 2-line one (the first attempt at the fixture produced modify/delete
instead of a rename conflict, and would have proved nothing). Round 1c's
`a_hand_resolved_conflict_is_not_an_evil_merge` and the fixture test
`the_evil_fixture_names_the_file_the_merge_invented` both still pass.
`scripts/m2-fix-1c-evil-audit.sh` carries the same rule in shell (checked
against the rename repository by hand: it agrees). **Not rerun** — the audit
spawns four git processes per merge over git/git and its header forbids
running it from an agent's context, so the counts in
`docs/perf/m2-review-fix-1c.txt` are still the old rule's and need a rerun.

**R2 Major 3 — text corrected; one capture knowingly left stale.**
TESTING.md graph stop 8 and `gallery.rs`'s caption said "nothing marks the
merge"; they now say a `⚠` appears beside the merge's `◆` once the probe
lands, that no other row carries one, and that the shape is otherwise stop
3's. Before writing that, the claim was made observable: new
`the_evil_marker_paints_on_the_graph_row_with_the_pane_closed` draws the
state with the pane **closed** and an `Evil` verdict in, and asserts the
`⚠` is on the merge's row and not on an ordinary one. That is the half
round 1a never showed (its test only proved a `Command::EvilProbe` was
emitted).

*Captures, stale and labelled as such:* `35-evil-merge-{ascii,unicode}` have
no `⚠` at all (M1 era) and cannot be retaken — `.scratch/m1-the-graph/
pty-gallery.py` no longer walks the gallery, as the reviewer found.
`m2-10-evil-merge-enter-160x40-unicode` does show the `⚠`, but with the pane
open, which always worked; the pane-closed marker has no capture in either
group. Retaking needs `pyte`, which is not importable in this environment,
so no new capture was taken and none was faked. TESTING.md now says all of
this in place of the old assertion. **The user judges stop 8 by eye on the
next walk.**
