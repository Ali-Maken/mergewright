# Ease pass — group review, groups 07–14

Reviewer, 2026-10-20. Read-only against **55cfdbc5** (main), diff from the ease-pass start 9cb03ce8, per-group diffs
taken merge by merge (07a d1de1a59, 07 953dc667, 07c f62e5a61, 07b b7529299 + 85a890e9, 13 bcc58b40 + b7593aec,
09 59b290d1, 09c 9db702e9, 09p c137936d, 09b b31508a7, 12 aac386dc, 11 22c92a05, 11b 4fc96acf, 12b c34ad595,
14 8b9fbbc6, 14b 373046f2, 14c 919cbfe7 + 37395041). Sampled, not read whole: about 10k lines of product code changed
in these groups.

Companion files: `docs/reviews/ease-pass-usability.md` (the whole-app walk), `docs/perf/ease-pass-review.txt` (the
release perf run).

## Gates (run here, in a worktree off 55cfdbc5, own `CARGO_TARGET_DIR`, one cargo process at a time)

| Gate | Result |
|---|---|
| `cargo fmt --check` | ok |
| `cargo clippy --all-targets -- -D warnings` | ok |
| `cargo clippy --all-targets --features heavy-tests -- -D warnings` | **fails to compile**: `tests/it/heavy.rs:321,347` non-exhaustive match, `HistoryUpdate::FoldedHits` (Major 4). With the two arms added locally: 3 more errors, `too_many_lines` at `tests/it/frames/journal_find.rs:253` (64), `tests/it/frames/run.rs:207` (63), `tests/it/pieces_shot.rs:70` (62) |
| `cargo test` | 7207 passed, 0 failed, 16 ignored; **519 s** wall (peak RSS 540 MB) |
| `cargo nextest run` | 7199/7199, 16 skipped; **481 s** (peak RSS 419 MB). No flaky retries |
| `cargo test --doc` | ok |
| Heavy tier `cargo test --features heavy-tests -- --test-threads=2` | **does not compile on main** (same two arms). Run with the arms added locally, uncommitted: see "Heavy tier" below |
| Release perf run | same: does not compile on main; run with the local two-arm patch, numbers in `docs/perf/ease-pass-review.txt` |
| `target/` | 9.1 GB after the debug gates (before the release build) |

Machine load average 5.8–6.0 during the fast-tier runs (`uptime`), so the times above are loaded times, not idle ones.
5 slowest tests: not captured (nextest is configured `status-level = "fail"`, so no per-test times were printed; I did
not spend another 8 minutes re-running it). Fuzzing / benchmarks: not run (none wired for these groups).

**Heavy tier** (with the two arms added locally, uncommitted; 861 s, peak RSS 1.18 GB, `--test-threads=2`): unit
binaries all green (2690 + small ones); `tests/it` **1080 passed, 18 failed**, 6 ignored. Two of the 18 are mine:
`file_size::every_file_over_400_lines_is_listed_with_its_count` (my two added lines grew `heavy.rs` 638→640) and
`published_repo::nothing_a_commit_would_carry_names_this_machine` (my perf file, mid-write, held home paths; scrubbed
before commit). **The other 16 are the pass's, never seen because the tier could not compile:**
- `heavy::budgets_hold_*` ×5 — `heavy.rs:585` `rows 79195 != 100000` "every commit laid out": the default fold (06)
  draws fewer rows than commits; the test stops before any budget is judged.
- `frames::after_idle::g_after_idle` — waits for 100 000 rows, gets 79 195, times out at 60 s (`after_idle.rs:58`).
- `frames::{fifty_thousand_refs,scrolling_the_100k_fixture}_…_at_the_{pixel,unicode}_tier` ×4 — `frames.rs:393`:
  5 wakes with no input after `LaidOut`, all `Produced::FoldedCounted` (06's fold counts asked at rest).
- `held_key::{held_j_stops_on_release_pixel, …_pixel_free, j_then_journal_opens_at_once_pixel}` ×3 — byte verdict
  `bytes_per_frame_held<=512KiB` FAILED: 1 569 311 / 3 978 454 / 1 818 474 bytes per held frame (debug; bytes are
  judged in debug too). Which group raised it is not established (13's role strokes are the likeliest: its own
  `ease-pixel.txt` shows focus-walk bytes ×16).
- `ghostty_shot::fix_scroll_ghostty_png`, `pieces_shot::emphasis_pieces_png` — committed PNG captures differ from what
  the code draws (13 changed the strips; heavy-gated captures were not re-shot).
- `resolver::frame_budget::inline_at_187_and_200_columns_keeps_input_to_frame_under_16_ms` — `frame_budget.rs:140`
  "no taken block's band on screen": 11b re-pointed it to the inline view without running it (`ease/11b-frame-budget`
  → **broken**).

**Release perf run** (same local arms; `docs/perf/ease-pass-review.txt`, 208 s, load ~4): 21 run, **8 passed, 13
failed**. Over the 16 ms input-to-frame budget (p99), 50 000 refs: `scrolling_the_diff` **293 ms**,
`with_the_detail_pane_open` **292 ms**, `with_the_map_view_open` **977 ms** — keys queued ~270 ms behind "other event"
frames whose own draw is 20–80 ms (Major 6). The other 10 fail as in debug (5 rows ≠ commits, 4 idle `FoldedCounted`
wakes, `g_after_idle` timeout), so the four core `frames::` scroll budgets and every first-paint budget are **not
judged**; first paint as printed: 62 ms (50k refs), 74 ms (commit-graph), 111 ms (none), 82 / 114 ms (realistic).
Passed: `journal_find` p99 5.5 ms, `budgets_hold_at_pixel_on_monorepo_with_j_held_{40mb,free}` (peak RSS 148 / 160 MB),
`readings` ×2, `sink` ×2, `controls`.

## Heavy tier on main without the scratch patch (conductor's question)

**No, it does not compile.** `cargo clippy --all-targets --features heavy-tests` and `cargo test --features
heavy-tests` both stop at `crates/mergewright/tests/it/heavy.rs:321:24` and `:347:24`: `match update` does not cover
`HistoryUpdate::FoldedHits { .. }` (the variant 06c added, `crates/mergewright/src/history/state.rs:220`). The old
scratch worktree's `| HistoryUpdate::FoldedHits { .. }` arms are exactly the fix; they are not on main. I added them in
my own worktree, uncommitted, only to get the heavy and perf numbers below, then removed them. Nothing of it is in my
commit.

## Feature pipeline

Design note `docs/design-notes/ease-pass.md` has Need (:32), Prior art (:40, cites `graph-readability.md`,
`commit-stash.md`, `conflict-resolution.md`, `conflict-visuals-and-finish.md`, `merge-preview.md`, all present in
`docs/research/`) and Journeys (:73, a fence of 9 counted rows). Built step counts: all 9 walked at their count —
`ease_walk::journeys::*` each `counted(...)`, all green in my nextest run; `one_key_over_fails_both_counted_journeys`
green. The count is a literal per walk, not derived from the keys sent (Minor 2). Mocks: ADR-0063 "built to the mocks";
departures found are Major 2 and Minor 3.

## Ticks spot-checked

| Group | Checked | Held | How |
|---|---|---|---|
| 07 / 07a / 07b / 07c | 13 | 13 | 07 :77–81, :84 test names found; 07a :51, :52, :56 found; 07b :66, :67, :72 found, :68 `row_bar_table` read in full (`view/rows/row_bar.rs:111-158`) — every `MergeFold` × {own, elsewhere, none} as claimed |
| 09p / 09b | 6 | 6 | 09p :131, :132 found; 09b :68, :69 found, :70 read (`branch_pane/picker_on.rs:30-45`: `C` then `⏎`, switch asserted), :71 read — its own text says the "cleared by refs read" part is a state test on `Reads`, not a drawn row: honest, holds |
| 11 / 11b | 6 | 6 | 11 :61, :63, :67 found (:67 itself says red never seen; already gap `ease/11-tdd-order`); 11b :42, :43, :47 found |
| 12 / 12b | 7 | 7 | 12 :57, :60, :62, :70, :71 found; 12b :53, :54, :55 found. 12 keeps 3 boxes unticked (:55, :58, :63) — Minor 8 |
| 13 | 4 | 4 | :68, :69, :70, :73 found |
| 14 / 14b / 14c | 7 | 7 | :142 `one_key_over_fails_both_counted_journeys` (`ease_walk/mod.rs:204`) and `counted` (`mod.rs:171-177`) read; :143 IDs counted by grep: G 14, C 15, M 19 (`merge.rs` 10 + `merge_finish.rs` 9), Q 17, fold 4; :145 three `shots.rs` tests found (:65, :89, :128); :149 `q4_pixel_lag_summary_recorded` found; :150 m3 waiter at `tests/it/m3_walk.rs:1020` reads `Nothing goes in. Put a file in with space.` |

**43 checked, 43 held. No false tick.** "Found" means the named test exists at the named place and passed in my run;
"read" means I read the body against the box's words. One weak-but-true tick: 14 :143's M18 test was loosened to fit
the build (Minor 3).

---

## Critical

None.

## Major

### 1. A pushed branch level with its remote is "not on a branch": pane, `C`, `not merged` and `(you)` all miss it
- **Where:** `crates/mergewright/src/history/seeds.rs:44-51` seeds every `refs/heads/*` *and* `refs/remotes/*` tip;
  `crates/mergewright-graph/src/layout.rs:229-234` collects them into one `HashMap<Oid, BranchIdentity>`, so when
  `ops/ci` and `origin/ops/ci` share a tip the later ref (the remote — refs come sorted, `heads` before `remotes`) wins
  and the lane is `origin/ops/ci`. `crates/mergewright/src/update/branch_pane.rs:204-206` (`focused`) then looks for a
  candidate whose `identity_of` equals that lane; the candidates (`:136-151`) hold the local `ops/ci` and only
  *remote-only* remotes, so none matches → `view/branch_pane.rs:28-37` draws `<sha7> · not on a branch`.
  The branch column survives only because `view/entries/branch_column.rs:112-120` patches exactly this case ("its tip
  may have seeded the lane: name that lane by the local branch"); the bar survives through the same patch
  (`view/rows/row_bar.rs:87`). Everything that goes through `focused()` does not: the right pane, `C`'s target
  (`update/picker/open_on.rs:28` — the bar says `C switch to <b>` while `C` opens on the first row), the branch status
  read, `not merged`. HEAD's `(you)` is matched by the local name (`branch_column.rs:110`), so it is lost the same way.
- **Seen:** 14's walk on the mock fixture, `ops/ci` rows (in sync, `ease_walk/fixture.rs:259`): pane `<sha7> · not on a
  branch` while the second line says `made on ops/ci` (14 NOTES; row `ease/14-ops-ci-not-on-branch`). HEAD's case is
  reasoned from the same lines, not observed. The G12 tests all pick branches with no upstream or one that is behind
  (`bar_and_pane.rs:93` integration, `team.rs:151` on `team`, which has no remote), so none covers it.
- **Why it matters:** every branch he has just pushed is level with its remote. After `P push` the pane on his own
  branch — the "who did what / pushed? merged?" answer the pass promises at 0 keys — would read `not on a branch`, and
  `C ⏎` on an in-sync teammate branch stops being 2 keys.
- **Fix (cause):** one identity per tip, chosen once. In `seeds_for`, give a local branch's identity to a tip a local
  branch holds (remotes only where no local branch sits at that tip), so lane, column, pane and `C` all key on the same
  name; then drop the patch at `branch_column.rs:112-120`. Test first: in-sync `ops/ci` row → pane heading `ops/ci`;
  HEAD in sync → `wallet-release (you)` and `(you're here)`.

### 2. A branch stacked on another ordinary branch has no lane of its own: wrong `made on`, no column name, drawn as "this"
- **Where:** `crates/mergewright-graph/src/layout/assign_row.rs:62-63` — a commit some lane is waiting for takes that
  lane (`winning_lane`), with its identity; the branch-tip lookup (`known_identities`) is read only for a commit
  nothing waits for (`open_new_lane`, `:209-214`). Reserved lanes (trunk, integration) are the only exception. So when
  `feature/receipts` is cut from `wallet-release`'s tip, `wallet-release`'s commits continue `feature/receipts`' lane,
  and `made_on` (`mergewright-graph/src/role.rs:96-97`, the row's lane) names `feature/receipts` for them.
- **Seen:** `docs/shots/ease-pass/main-cursor-own-120.txt:8-9` — `Merge branch 'sara/fix-toc' into wa…` and `Start the
  wallet release` (wallet-release's commits) have a blank branch column and sit on the `┃` lane (role This) of
  `feature/receipts`; G1 says a built-on branch draws `│`. `main-pane-sibling-120.txt:12` — Sara's `toc: fix the
  anchors` reads `bob/x` because bob branched from it. The mock gives each stacked branch its own lane and row branch
  (`prototypes/ease-mocks/mergewright-main-screen.html:110,117-118`: W on its own lane 2 beside I; rule at `:168-174`).
- **Why it matters:** he works on dependent branches daily (memory `works-in-a-team-on-dependent-branches`). On a stack,
  the pass's "which branch is this row on / who did what at a glance" answer names the top branch for every commit
  below it. A departure from the accepted mock (ADR-0063) that was never put to him. It settles three open questions as
  one cause: `ease/07b-made-on-lane`, `ease/07c-wallet-no-column`, `ease/14-sibling-bob-x` → **broken**.
- **Fix:** a layout decision (DESIGN §3, ADR-0001), so planner first, not a builder: e.g. when a continuing lane reaches
  a known branch tip, the rows from that tip down take that branch's identity (what the mock's rule says: made on X at
  or below where X left). Cost and the merge-parent case (`assign_row.rs:148-155`) need the planner's look. Tests:
  stacked fixture where the lower tip is the fork point; lower branch's rows read `made on <lower>` and draw `│`.

### 3. A branch merged *into* yours is offered as what yours is built on
- **Where:** `crates/mergewright-ops/src/branch_status.rs:121` — an ordinary candidate is an eligible base whenever
  `base_only == 0` (wholly under the walker). A branch merged into W through a second parent is wholly under W too;
  nothing asks whether it is on W's first-parent line.
- **Seen:** 14 on the mock fixture: sara's pane `built on main ─◆ dana/receipts ─◆ wallet-release ─◆ sara/fix-toc`,
  then `✓ up to date with dana/receipts` (14 NOTES). Also in 07's capture `main-cursor-own-120.txt:12-16`: `built on
  main ─◆ sara/fix-toc ─◆ wallet-release ─◆ feature/receipts`, where sara/fix-toc was merged into wallet-release
  (`:8`). That capture's "what right looks like" line does not mention the route, so nobody judged it.
- **Why it matters:** G12's `built on` route is the pane's answer to "behind what it was built on?"; it names the wrong
  branch and reports `up to date with` a branch that has nothing to do with it. Confirms `ease/14-merged-branch-as-base`
  (**broken**).
- **Fix:** an ordinary base must be reachable from the walker along first parents (or: its tip is not reached only
  through a merge's second parent). Keep 09p's budget in mind (`docs/perf/branch-pane.txt`): one `rev-list
  --first-parent` per contender top is affordable; per candidate is not. Test on 14's mock fixture: sara's route
  `main ─◆ wallet-release ─◆ sara/fix-toc`.

### 4. The heavy tier has not compiled since 06c — no heavy run, no heavy clippy, no release budgets for ten groups
- **Where:** `crates/mergewright/tests/it/heavy.rs:321,347` (missing `HistoryUpdate::FoldedHits { .. }` arm); behind
  it `tests/it/frames/journal_find.rs:253`, `frames/run.rs:207`, `pieces_shot.rs:70` over clippy's 60-line limit (the
  rename's longer names, 8da8b6c3, rewrapped them — invisible to the plain clippy gate).
- **Seen once compiled:** 16 heavy tests fail (list under Gates → "Heavy tier"): first-paint tests stop on the fold's
  rows ≠ commits, scroll-budget tests on idle `FoldedCounted` wakes, three Pixel byte verdicts over 512 KiB per held
  frame, two PNG captures stale, the resolver frame budget looking for 11b's removed band. Every one is a knock-on of a
  pass group that no builder could see.
- **Why it matters:** FoldedHits landed in 06c (40a10255). From then to 55cfdbc5 — 07a, 07, 07c, 07b, 13 (pixel tall
  row), 09, 09c, 09p, 09b, 11b, 12b, 14 — no heavy test, no frame budget and no first-paint budget could run, and
  `ease/11b-frame-budget` was waiting on exactly that run. 07a's builder reported the break (log :148); it was filed
  `unverified` (`ease/13-heavy-foldedhits`), not broken. The conductor's per-merge gate list (fmt, clippy, nextest,
  doc) has no `--features heavy-tests` line, so a heavy-only compile error can live for days.
- **Fix:** the two arms (cause: an exhaustive match in a feature-gated file) and the three function splits; then each
  of the 16 failures to its owner: the rows ≠ commits and after_idle expectations to the fold's rule (06 — what
  "every commit laid out" means under the default fold, the user-visible answer being rows, not commits), the idle
  `FoldedCounted` wakes (06: a wake with no input is what `frames.rs:393` exists to catch — fix the cause or change the
  rule by decision, not by loosening), the Pixel bytes (13), the two PNGs (13, re-shot only after the bytes are
  understood), the resolver budget (11b). Add
  `cargo clippy --all-targets --features heavy-tests -- -D warnings` to the gates run after every merge (it costs ~10 s
  warm); this is the KNOWN_GAPS row "The heavy code is not in the standard clippy gate" biting.

### 5. Fast tier at 481–519 s against PROMPT 7.1's 3 minutes
- **Where:** my runs above: `cargo nextest run` 481 s, `cargo test` 519 s, 7199 tests, load ~6.
- **Why it matters:** 7.1's "Under 3 minutes" is a hard rule for a laptop he uses while it runs. The open row
  `M7/r1007-q1` (266 s for 4687 tests, "unverified") has grown with the pass by ~2500 tests. Loaded runs overstate it;
  but even half this is over.
- **Fix:** an idle run from a script into `docs/perf/` with `--status-level pass` to name the slowest 20; move whole-app
  capture/walk tests that cost seconds each (the `ease_walk` and capture families are the likely bulk) behind a
  `walks` filter or the heavy tier, by the user's call. Not measured per test here.

### 6. Release input-to-frame budget blown with 50 000 refs: p99 292–977 ms against 16 ms
- **Where:** `docs/perf/ease-pass-review.txt`: `frames::fifty_thousand_refs_keep_input_to_frame_under_16_ms_scrolling_the_diff`
  p99 293 ms (59 of 422 keys over), `…_with_the_detail_pane_open` 292 ms (93 of 421), `…_with_the_map_view_open`
  977 ms (86 of 421). Slow keys wait ~240–280 ms in the queue (perf file :74), then draw their own frame in 8–80 ms.
  The same runs record `idle_prefetch_us` max 548 / 561 / 593 ms (perf file :39, :55, :70) — one idle step longer
  than the whole wait.
- **Why it matters:** ADR-0021's frame budget is the "no lag" promise; he uses Ghostty daily. Whether these held before the pass is unknown — the tier has not run since 06c.
- **Cause: not established — question, with a candidate line.** `view/entries/branch_column.rs:86-139` (`names_of`)
  walks every local and remote ref and hashes each name, "one pass over the refs … per frame", and is reached more than
  once per frame (`branch_name` from the second line, `second.rs:104`, and from the bar, `row_bar.rs:87`, beside the
  column). At 50 000 refs that is a candidate for the 8–80 ms own frames; the 0.5 s idle prefetch step (or the fold
  counts, `FoldedCounted`, Major 4) is the candidate for the queueing. Measure before fixing: a per-frame timer around
  `names_of` and per idle step in this test, then bisect against 9cb03ce8.

## Minor

1. **A lane named after an upstream of a different name stays blank.** `view/entries/branch_column.rs:116` rebuilds the
   remote lane name as `<remote>/<local name>`; a branch tracking `origin/other-name` never matches its lane id.
   Settles `ease/07c-upstream-name` → **broken** (edge case). Goes away with Major 1's fix.
2. **The journey count is a literal, not the keys sent.** `ease_walk/journeys.rs:30,58,77,95,…` pass a hand-written
   list to `counted`, separate from the `app.key` calls inside `previewed`/`stop_after_y`/`pick`/… (`flow.rs:39-95`).
   Today each helper sends exactly the key it names (read), but an extra key added inside a helper would not fail the
   count. Fix: have the walk record the keys it sends (a counting wrapper on `App::key`) and count those.
3. **Landing bars carry words the mocks don't, and M18's test was loosened to allow it.** Merge landing reads `u undo
   the merge · P push · a amend · space show …` (`merge-landed-120.txt`); mock `mergewright-merge-screen.html:198` has
   no `a amend`. Commit landing reads `. explain · u undo … · P push · c commit again · a amend`
   (`commit-landed-120.txt:45`); mock `mergewright-commit-screen.html:179` has neither `. explain` nor `a amend`.
   Source: `graph_screen/hints/lists.rs:139-149` (`a amend` on any amendable HEAD row) and `hints/filters.rs:131`
   (`.` explain leads a marked row). `ease_walk/merge_finish.rs:221-224` asserts only `starts_with("u undo the merge
   · P push · ")` and `contains("space show…")`, so the departure passes. ADR-0063 says a departure goes to him first.
   Fix: ask him (one line: "keep `a amend` / `. explain` on the landing bars?"), then tighten the test to the answer.
4. **`b take` at 90 columns.** `graph_screen/hints/lists/clash.rs:73` short form of `take both` is `take`, so the
   90-column clash bar reads `o take yours (left) · t take theirs (right) · b take · e edit` (`merge-clash-90.txt`) —
   `b take` says nothing. Short form `both`.
5. **Two separators.** Settles `ease/14-preview-footer-spaces`: the preview footer uses `footer::join` with two spaces
   (`preview_box/merge.rs:256-270`), as every preview box and the commit screen's bar do; the graph and clash bars use
   ` · `. The mock draws spans with no text separator, so neither is "the mock's". Textual; app-wide consistency call
   (usability file).
6. **ARCHITECTURE.md is silent on the pass's new modules.** No mention of `ops::branch_status` (09/09p), `git::contained`
   (09p) or the `ease_walk` acceptance walk; its walks section (`:405-436`) still says every walk judges by
   `repo_truth`, never `AppState` — `ease_walk` reads `AppState` (`journeys.rs:54`, `picker_on.rs:43`).
7. **Ratchet raised on 5 test files** against "split, don't raise": `over_400.txt` rows `event_loop/tests.rs`
   1647→1648, `event_loop/tests/settle.rs` 714→715, `update/preview/tests.rs` 2152→2157,
   `tests/it/m7_walk/rewrite_stops.rs` 437→438, `tests/it/name_and_drag.rs` 683→686 (06h base 0e00b383 → 55cfdbc5).
   Already gap `ease/07b-ratchet-raised` (broken).
8. **Ticket 12 still has three open boxes** (`12-finish-review-and-look.md:55, :58, :63`) though 14b's `m15_…`,
   `m11_…`, `m18_…` now walk those screens. Either tick them naming those tests (after checking :55's order and :58's
   exact 120-col words, which the M tests do not assert) or close them into the open rows `ease/12-file-order`,
   `ease/12-bar-120`.
9. **High-contrast / `NO_COLOR` on the inline resolver are asserted only through `Theme::tint`.** 11b dropped
   `band_in_high_contrast_has_no_background`, `band_under_no_color_is_glyphs_and_modifiers`; the inline view's
   16-colour case is covered (`conflict_resolver/tests/card_colours.rs:153`) but no resolver-level test draws high
   contrast or `NO_COLOR` (grep of `conflict_resolver/` for them: only `finish_row` tests). `ease/11b-dropped-tests`
   stays **unverified**, narrowed to those two.
10. **Pixel tier focus walk got 16× the bytes.** `docs/perf/ease-pixel.txt`: focus walk `bytes_p95` 2 326 (base) →
    36 771 (roles) at 120×40; input age p95 23.5 → 39.3 ms (tall, open). Already the user's question `ease/13-pixel-lag`;
    noted because it is the Ghostty lag he complained about twice.

## Questions (no line proves them)
- `ease/14-accept-in-process`: 14 :25-26 asked for a pty walk; 14c built in-process (`scripts/ease-accept.sh:11-14`,
  TESTING.md says so). Nothing on the ease screens is walked through real terminal key decoding (`⏎`, `esc`, `⇥` in his
  Zed). Accept with a line in 14, or add one pty smoke of `commit-all-subject` and `merge-one-clash`?
- `ease/07b-zeroed-hash` — answered, not a defect: `tests/it/captures.rs:190-200` replaces only the cursor row's own id
  (the run-made one); a wrong or missing hash on the second line still shows as drift.

## KNOWN_GAPS `ease/14-*` rows, settled

| Row | Settled as | Line |
|---|---|---|
| `ease/14-merged-branch-as-base` | **broken**, confirmed (Major 3) | `mergewright-ops/src/branch_status.rs:121` (the path in the row says `mergewright-git`; it is `mergewright-ops`) |
| `ease/14-ops-ci-not-on-branch` | **broken**, cause found (Major 1) | `history/seeds.rs:44-51`, `graph/layout.rs:229-234`, `update/branch_pane.rs:204-206` |
| `ease/14-sibling-bob-x` | **broken**, same cause as `07b-made-on-lane`, `07c-wallet-no-column` (Major 2) | `graph/layout/assign_row.rs:62-63, 209-214`; `graph/role.rs:96-97` |
| `ease/14-review-settle-hang` | **close**: a test not waiting, fixed by `ease_walk::flow::read_in` (`flow.rs:100`), used in `m17_…`; green in my run | — |
| `ease/14-preview-footer-spaces` | **close** as a defect; the separator question moves to the usability file (Minor 5) | `preview_box/merge.rs:256-270` |
| `ease/14-refutes` | **close all three**: `ease/06-no-space-offer` (producers `view/rows/row_bar.rs:57,61`), `ease/08u-justmerged` (`update/finish.rs:207-223`), `ease/12-landing-bar` (bar now `u undo the merge · P push · …`); the left-over `a amend` is Minor 3 | as listed |
| `ease/14-pixel-ghostty` | stays **unverified** (his run) | — |
| `ease/14-accept-in-process` | stays a **question** (above) | — |
| `ease/14-m3-sheet-stale` | stays **unverified** (old sheets not rebuilt, user 2026-10-19) | — |
| `ease/13-heavy-foldedhits` (not 14, but asked) | **broken**, confirmed by building (Major 4) | `tests/it/heavy.rs:321,347` |

## Checked and fine
- No `unwrap`/`expect` added outside tests in the sampled groups; the three `unwrap_or_default` / `.ok()` hits are
  layout fallbacks with a comment (`mergewright-ui/src/branch_pane.rs:213`, `update/picker/open_on.rs:17-20`,
  `finish_summary/list.rs:104`), not swallowed errors.
- Git off the UI thread: ahead counts are in the refs read job (`jobs/runner/reads.rs:99`); branch status is a
  `Command::AtRest` job (`update/branch_pane.rs:237`); the look's numstat is a job (`jobs/finish/look`). `user.email`
  is read once at open (`repository.rs:119` → `repo/mod.rs:162-166`).
- Escapes: the second line (`view/entries/second.rs:92-93`), the merge look (`SanitizedText` throughout
  `mergewright-ui/src/merge_look.rs`) and branch names (`row_bar.rs:96`) are sanitised.
- No data-loss path found in the sampled diffs: 11/11b/12/12b change look and routing; the merge write paths are 05's
  (reviewed in `ease-pass-05.md`). 12's read-only message removed message editing (user's pick, field-notes 2026-10-18).
- 09p's dropped shared-history check is sound: `branch_status/measure.rs:25-29` — anything under an orphan trunk is
  itself unrelated, so R1 drops it anyway; the equivalence oracle test covers it.
- Crate rules hold: `mergewright-ui` imports `mergewright-git` only as a dev-dependency; no new runtime dependency in
  these groups.
