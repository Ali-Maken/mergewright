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
