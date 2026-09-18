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
