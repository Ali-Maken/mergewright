# G-settle — per-row work starts when the cursor rests

Status: done (2026-10-02). Was: ready (spec revised 2026-10-02 after scout `scouts/03-settle.md`: the first spec's mechanism did not fit the code)
Build with: Opus (conductor, after scout `scouts/03-settle.md` and planner revision; merge 04 first if 04 touched running.rs)
Blocked by: 02 (merged)
Needs: `pace-fence-and-sync` — `Pacer::deadline` (the loop waits on the earlier of the two deadlines); `Wakes::next_wake_by`.

## Why the first spec changed (scout, file:line)
- "Rest = a frame drawn with nothing queued" fails held `j`: keys repeat every 15–90 ms and a paced frame takes a few
  ms, so the loop draws and goes idle **between every repeat**. Each idle frame would count as rest and start a set
  per key, which is the cost this ticket removes. Rest has to be **time since the last motion**, decided in the loop,
  which has the clock. `update` has none (`update/mod.rs:130`).
- Jobs start per event inside `take`, before any draw (`event_loop/running.rs:340-355`). Deferral goes in `settle`
  (`update/mod.rs:179-207`), and the loop decides when.
- Thread emphasis is not a job: the view builds it each frame (`view/entries.rs:40` → `Ancestry::of`, called from
  `view/rows.rs:58` and `view/strips.rs:224`). There is nothing to start at rest or cancel. **It is out of scope**
  (see Out).
- The evil-merge probe runs with the pane hidden (`update/detail/ops.rs:185`). That is deliberate: it marks the graph
  row (MAP `DetailState::probe_evil`, M2 review M3). The comment at `ops.rs:176` is wrong; fix the comment and keep
  the behaviour.
- `ContainedIn` is never cancelled (`ops.rs:250`). Fixed here.

## Read first
- `docs/design-notes/m8-polish-and-ship.md`: Prior art row "Per-row work while moving", Journeys J1–J3, Interfaces
  (`Pacer`, `Rest`)
- `docs/adr/0054-the-loop-never-writes-ahead-of-the-terminal.md`
- `.scratch/m8-polish-and-ship/scouts/03-settle.md` (whole, 1 page)
- `crates/graft/MAP.md`: `AppEvent`, `update`, `selection::follow_selection`, `DetailState::{open, follow, probe_evil,
  evil_probed, files_arrived, contained_arrived}`, `Loading`, event_loop (`Pacer`, paced turn, `joins_frame`,
  `Wakes::next_wake_by`), `drain_sink`
- Source, after the MAP: `update/mod.rs:130-207`, `update/selection.rs:100-155`, `update/detail/ops.rs:170-270`,
  `event_loop/running.rs:250-410`, `view/detail.rs:255-295`

## The rule (decided)
- **Motion** = a wake after which the selection's identity (`FollowKey`) differs from before it. The loop compares
  before and after, so it needs no keymap knowledge. Keys, clicks, `n`/`N`, `g`/`G`, a jump's rows arriving and an
  undo landing all count, through one path.
- **Leading edge.** A motion with no motion in the last `BURST_GAP` (150 ms) follows at once, in the same turn, as
  today. A single `j`, a click or a `G` is unchanged.
- **Burst.** A motion within `BURST_GAP` of the previous one is deferred. `settle` records the selection as owed and
  starts nothing. At the **first** deferred motion it cancels every still-pending job of the last follow
  (`ListFiles`, `ReadFileDiff`, `ContainedIn`, `EvilProbe`), each with `CancelJob`. Their late answers are dropped by
  id.
- **Rest** = owed, and quiet for `clamp(2 × widest of the burst's last 4 gaps, QUIET_MIN 50 ms, BURST_GAP)` since the
  last motion. The loop then applies `AppEvent::Rested`, which runs the follow once for the cursor's row. The burst
  is over, so the next motion is a leading edge again. Why the numbers: macOS and X repeat every 15–90 ms, and their
  initial repeat delay is 225 ms or more. So 150 ms separates a burst from separate presses, and 2× the observed gap
  stops a held key from reading as rest. The 50 ms floor is there because keys applied together in one paced turn
  measure a gap near 0.
- **Another input ends the burst.** An input wake that is not the burst's key (Enter, Tab, a click, `x`, …) while a
  follow is owed gets `AppEvent::Rested` applied **before** it. So the key acts on the cursor's commit, never on the
  pane's stale one.
- **While owed**, the detail pane keeps the last followed commit, and its title row adds a muted `reading…` at the
  right. Graph, cursor row, header and thread emphasis update every frame, as today.
- Pane hidden: no detail jobs (as today). The evil probe follows the same edge, burst and rest rule.
- Unit tests that drive `update` directly see today's behaviour: `defer_follow` defaults to `false`.

## Scope
In:
- `event_loop/rest.rs` (new): `Rest`. The loop owns it beside `Pacer`. Before each update it sets
  `state.defer_follow = rest.in_burst(now)`. After the update it calls `rest.moved` on a motion. It waits on
  `min(pacer.deadline(), rest.deadline() if follow_owed)`, and applies `Rested` when due or when `rest_first` says so.
- `update`: `AppEvent::Rested`, `FollowKey`, `follow_key`, `follow_owed`, `AppState::{defer_follow, followed}`.
  `settle` gates `follow_selection` and `probe_evil` on them, and cancels on the first deferred motion.
- `DetailState`: holds the pending `ContainedIn` id. `load` cancels it when the oid changes, and
  `contained_arrived` drops a stale id. Results stay cached by oid, which is correct data for that commit.
- `settle`'s other calls (`radar::request_context/force_push/duplicates`, `journal::request`, `remote::*`,
  `detail::open_explain`; `mod.rs:188-202`): grep each. Any that reads the cursor joins the gate. NOTES lists which,
  with file:line.
- `view/detail.rs`: the muted `reading…` on the title row while owed.
- Fix the stale comment at `ops.rs:176`.

Out:
- **Thread emphasis.** It is paint, not started work, and deferring it would make the lit thread lag the cursor he
  is watching. Its per-frame cost (`Ancestry::of` ×2 per frame) is unmeasured. Ticket 05 (trace follow-up) judges it
  from `draw_us`. The likely fix, building it once per frame and sharing it between rows and strips, touches
  `view/strips.rs`, which ticket 04 holds.
- Keeping the old diff body visible while the at-rest load is pending. At rest the body reads `reading…` until the
  files arrive, as today.
- Per-frame search-hit set (`view/strip.rs:38-42`, KNOWN_GAPS 414): ticket 05.
- Part B (per-turn cost against history and ref count): ticket 05.

## Signatures
```rust
// graft::event_loop::rest (pub(crate))
pub(crate) const BURST_GAP: Duration = Duration::from_millis(150);
pub(crate) const QUIET_MIN: Duration = Duration::from_millis(50);
pub(crate) struct Rest { /* last: Option<Instant>, gaps: [Duration; 4], key: Option<_> (the burst's key) */ }
impl Rest {
    pub(crate) fn new() -> Rest;
    pub(crate) fn in_burst(&self, now: Instant) -> bool;
    pub(crate) fn moved(&mut self, now: Instant, by: Option<&Wake>);   // the wake just applied changed FollowKey
    pub(crate) fn rest_first(&self, wake: &Wake) -> bool;              // an input wake that is not the burst's key
    pub(crate) fn deadline(&self) -> Option<Instant>;                  // last + clamp(2 × widest gap, QUIET_MIN, BURST_GAP)
    pub(crate) fn rested(&mut self);                                   // burst over; next motion is a leading edge
}
// graft::update
pub(crate) enum AppEvent { /* … */ Rested }
pub(crate) enum FollowKey { Commit(Oid), WorkingTree }
pub(crate) fn follow_key(state: &AppState) -> Option<FollowKey>;      // what follow_selection reads
pub(crate) fn follow_owed(state: &AppState) -> bool;                  // follow_key(state) != state.followed
// AppState gains: pub(crate) defer_follow: bool (written only by the loop), pub(crate) followed: Option<FollowKey>
```
`Rest` takes `now` as a parameter, never reads the clock (as `Pacer` does). Exact private names are the builder's;
MAP.md records them.

## Acceptance
- [x] `Rest` unit tests (`event_loop::rest::tests::*`, 6 tests) with given `Instant`s: a lone motion is not in a burst; 60 motions 16 ms apart are in a burst
      from the 2nd, deadline = last + 50 ms; gaps of 90 ms give a deadline of last + 150 ms; `rest_first` is true for
      another key and false for the burst's key and for job wakes; `rested` makes the next motion a leading edge.
- [x] Loop test (`event_loop::tests::settle::a_held_j_follows_the_first_row_and_the_row_it_rests_on_and_nothing_between`; also `rest_comes_the_quiet_window_after_the_last_key_not_before`) (fast tier, `event_loop/tests/settle.rs`, the `pump` seam with a counting job double): pane shown,
      60 `j` 16 ms apart over a range that includes a merge row. Follow commands started: one set on the first key,
      **none for keys 2–60**, exactly one set after the quiet window, for the final row. The first set's pending ids
      each get a `CancelJob` at key 2.
- [x] (`event_loop::tests::settle::a_late_file_list_for_a_row_the_cursor_left_changes_nothing_shown`, `…a_late_contained_in_…`, `…a_late_evil_verdict_…`) A late `ListFiles` / `ContainedIn` / `EvilProbe` answer for a row the cursor left changes nothing shown
      (update-level test per kind).
- [x] (`event_loop::tests::settle::after_rest_the_pane_shows_the_cursors_commit_in_the_first_frame_after_its_files`) After rest, the pane shows the cursor's commit in the first frame after its `ListFiles` answer (loop test).
- [x] (`event_loop::tests::settle::a_single_j_follows_in_the_same_turn`, `enter_after_a_burst_acts_on_the_cursors_commit`, `tab_after_a_burst_acts_on_the_cursors_commit`) A single `j`: follow commands in the same turn as the key (loop test). `j j j ⏎` and `j j j Tab` act on the
      cursor's commit, not the pane's old one (loop test).
- [x] (`event_loop::tests::settle::with_the_pane_hidden_only_the_evil_probe_runs_on_the_leading_edge_and_at_rest`) Pane hidden: no `ListFiles`/`ContainedIn` ever; `EvilProbe` only on the leading edge or at rest (loop test).
- [x] (workspace `cargo nextest run`: 5223/5223 passed) Existing `update` tests unchanged and green (they run with `defer_follow = false`); the gate run proves it.
- [x] (`event_loop::tests::settle::capture_the_pane_following_a_burst`) Capture `docs/shots/m8/settle-following.txt`: a mid-burst frame. Cursor row 6+ rows below the pane's commit,
      pane title ending `reading…`. Muted colour **unverified** (rule 5).
- [x] (`scripts/m8-pace.sh docs/perf/m8-settle.txt`, J1 lines) J1 tail ≤ 100 ms — **held**: Pixel deflate 40 MB/s 36 ms
      (`docs/perf/m8-settle.txt:78`), under load 25–35.
- [ ] J1 unchanged or better — **unverified** (review C1): load 25–35 vs 10–22 at the baseline, so the numbers are not
      evidence either way; measured at `d7ee721d`, before 04's merge; and `held_key` ran with the pane hidden, where
      this ticket changes almost nothing. Re-measured in the M8 review's release perf run (`scripts/m8-pace.sh`), on a
      tree with 03 and 04, at matched load. The changed path now has its own J1 lines: `held_key::held_j_with_the_pane_shown_stops_on_release_unicode_40mb`
      and `…_pixel_deflate` (scenarios `A-pane`, `A-deflate-pane`; `j1_line` rows in the script). Not run here (heavy
      tier, measurement belongs to the review).

## Touches
- Detail pane follow, compare follow (`compare_followed`), the evil-merge mark on the graph row, Explain
  (`open_explain` in settle). Check each still fires at rest.
- Every exhaustive match on `AppEvent` (grep). The frame trace (`FrameRecord::commands` now drops during bursts;
  check `trace::tests`).
- Harness helpers that wait for a followed pane (`prelude::settled`, walk `ready`). A scripted run that ends inside a
  burst never rests; tests that need the pane apply `Rested` or wait out the window. Grep `held_key`, `pixel_scroll`
  and `pace` tests for detail assertions.
- Captures that show the pane right after a burst of keys (grep the acceptance walks in `docs/shots/m7`, `m6`).
- DESIGN wording on the pane following "every move" (grep `docs/design/` for `follow`). If it disagrees, note it as a
  Proposed change in NOTES; do not edit DESIGN.
- MAP.md graft. KNOWN_GAPS row 1093: give its fate in NOTES.

## File set (disjoint from 04 and 14)
`crates/graft/src/event_loop/{rest.rs (new), running.rs, mod.rs}`, `event_loop/tests/settle.rs` (new, plus the tests
mod line), `update/mod.rs`, `update/selection.rs`, `update/detail/{ops.rs, state.rs}`, the files defining `AppEvent`
and `AppState` (grep), `view/detail.rs`, `crates/graft/MAP.md`, `docs/shots/m8/settle-following.txt`,
`docs/perf/m8-settle.txt`.
Not touched: `event_loop/graphics.rs`, `view/{strips,strip,rows,entries}.rs`, `update/{keys,mouse,search,history}.rs`,
`tests/it/held_key.rs`.

**Overlap with 04 (building now):** none by file. 04 holds `event_loop/graphics.rs`, `view/strips.rs`, graft-graph
and graft-term. One risk: if 04's idle-prefetch change reaches `running.rs`, both edit it. 03 confines its
`running.rs` change to the per-wake hook and the wait deadline. The conductor merges 04 first and 03 rebases. J1
tail is measured after both are merged.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide. The perf run goes through the script, never in the builder's context.

## Size
One context, ~60k tokens.

## NOTES (builder, 2026-10-02)
Scout used: `scouts/03-settle.md` (planner's), plus own reads of the named ranges. No compaction.

Deviations, each for a reason:
- **Clock seam `Wakes::now()`** (default `Instant::now()`): `Run` reads every time from it (pacer, batch budget, rest). Loop tests run on a virtual timeline (`Timed` in `tests/settle.rs`) — deterministic, no sleeps. Touches 7 `Instant::now()` sites in `running.rs` beyond the per-wake hook and the wait; 04 may conflict there (mechanical).
- **`Rest::moved(now, Mover)`** instead of `Option<&Wake>`: the wake is consumed by `update` before the loop knows it moved; `Mover::of(&wake)` is read first.
- **`follow_waits` holds the follow only when it would start work** (pane shown, or a merge under the cursor to probe). Pane hidden off a merge, the follow runs at once (starts nothing; cancels a running probe). Without this, every held key with the pane hidden waited for a rest that does nothing and broke `pace::three_hundred_held_keys…` (fence/frame pattern). Also never held when nothing was followed yet (`G` before its rows land, first window): keeps "a `G` is unchanged".
- **`reading…` title needs graft-ui**: the title row is drawn in `graft-ui/src/detail_pane/header.rs`, not in `view/detail.rs`. Added `DetailView::following` + `draw_following` (+ test `a_pane_waiting_for_the_cursor_to_rest_ends_its_title_row_in_reading`, 2 fixtures). graft-ui held by no other builder.
- Outside the file list, forced by the new variant/fields: `update/commit_graph.rs`, `update/detail/control.rs` (+ `contained_arrived` id), `event_loop/label.rs`, `update/tests.rs` (`every_field_spelled_out`).
- `view/detail.rs`: `commit_header` now finds the pane's commit anywhere in the window (was: only the cursor's row), else mid-burst the header went blank. Once the commit leaves the window the header is empty (not seen in tests; a held key past the window edge would show it).
- Abandoned reads draw `reading…` in the body (not `select a commit`); same spec followed again re-reads (`DetailState::abandoned`). Returning to that exact row mid-burst with only the diff pending re-lists the files (file cursor to top) — edge, accepted.

`settle`'s other calls, grep: `radar::request_context` (radar.rs:330), `request_force_push_probes` (:402), `request_duplicates_probe` (:534), `refresh_ref_findings` (:196), `remote::request_at_launch` (remote.rs:372), `request_leaves` (:78), `forget_stale_divergence` (:307), `journal::request` (journal.rs:238): none reads `history.cursor`/`selected()`/`on_working_tree` — not gated. `detail::open_explain` (control.rs:118) reads `History::selected` but only on a menu choice — an input, which `rest_first` rests before. Not gated.

Stale comment at `ops.rs:176` fixed (the probe runs with the pane hidden, on purpose).

Perf (`docs/perf/m8-settle.txt` vs `m8-pace.txt`): J1 Unicode 40MB/s 2 ms (was 1), free 1 (1); Pixel deflate 40MB/s 36 (29), free 14 (8): all pass. Load 25–35 vs 10–22 at baseline; difference not attributable. Same three pre-existing Pixel non-deflate failures as the baseline (row 1089, 04's area). held_key runs with the pane hidden and no merges near, so this ticket barely touches J1 there.

Not done / could not:
- KNOWN_GAPS row 1093: builders may not read KNOWN_GAPS.md (CLAUDE.md); its fate is the conductor's call.
- Real-terminal feel of rest timing (150/50 ms) unverified — only virtual-timeline tests.
- Muted colour of `reading…` unverified (rule 5).
- DESIGN: no wording about the pane following "every move" found (`grep -ni follow docs/design/*.md`); no Proposed change.

## NOTES (builder, review fixes C1/M1/M3, 2026-10-03)
Scout: `scouts/03-settle.md` existed; not re-sent (fixes confined to sites the review named with lines, all re-read). No compaction.

- **C1** — J1 tick split above: "≤ 100 ms" held (36 ms, `m8-settle.txt:78`); "unchanged or better" **unticked, unverified**, owed to the M8 review's release perf run. New J1 scenario for the changed path: `held_key::held_j_with_the_pane_shown_stops_on_release_{unicode_40mb,pixel_deflate}` (pane opened `⏎ Tab Tab`, held `j` down MONOREPO's merge-rich `main`), with `j1_line … A-pane` / `A-deflate-pane` in `scripts/m8-pace.sh`. **Compiled (clippy `--features heavy-tests`), never run** — heavy tier.
  - Harness change it needed: `frames::paced` now records the loop's own `Rested` turn as a `Kind::Rest` entry (an `Updated` mark after the last wake was applied and drawn) instead of writing its frame over the last wake's. `held_key` A then ends the graph's tail at `written[rest]` and reports the pane's catch-up apart (`rest_frames`, `rest_ms`, unjudged). This also changes the **existing** A scenarios when a rest fires there (pane hidden, cursor stopping on a merge): their tail no longer includes the probe's rest frame. Consistent with J1 being about the graph (review Minor 5); the M8 review should read the A lines knowing it.
- **M1** — one predicate `update::pane_follows` (shown, not a menu compare) read by `followed`, `follow_waits` and `view::detail::following`. Mid-burst with the pane not following, `settle` calls the new `DetailState::abandon_probe` (probe only); `abandon` cancels everything as before. Cause fix. Test `event_loop::tests::settle::a_burst_leaves_a_menu_compares_pending_files_alone` — **red before the fix** (the compare's `ListFiles` id was in the `CancelJob`s, `settle.rs:518`), green after.
- **M3** — test `event_loop::tests::settle::a_pinned_compare_follows_at_rest`: base pinned, 10 held `j`: one `ListFiles` on the leading edge, none mid-burst, one at rest; spec `Compare { base, head: row 10 }`.

Touches:
- Pane follow / compare follow (`compare_followed`) / evil mark / Explain: `followed` refactor is equivalent (same two guards, now via `pane_follows`); covered by the settle loop tests (held `j`, pinned compare, menu compare, hidden pane probe) and the `update` selection/compare tests (160 tests matching `settle|selection|detail|compare` green). Explain: `open_explain` answers a menu choice (an input, `rest_first`), unchanged — not re-tested.
- Hidden pane mid-burst on a merge: now cancels only the probe; files/diff/contained-in left pending (were cancelled before). `with_the_pane_hidden_only_the_evil_probe_runs_…` still green (asserts the probe cancel).
- `AppEvent` matches: no new variant in this round (grep not needed; it compiles).
- Frame harness (`frames::paced`, `readings`, `after_idle`, `journal_find`, `held_key`): `Kind::Rest` added to `kind_name` and `readings` groups; `unasked_wakes` skips it like a key; `journal_find` replays `Rested` as the loop applied it; `after_idle` matches only `Key`/`Window`. Fast-tier frames tests in the workspace run.
- `trace::tests`, walk `ready`/`settled` helpers, m6/m7 acceptance captures: workspace run (below); `git status` after it shows no `docs/shots/` file rewritten.
- `view::detail::following` (title `reading…`): same answer as before for every state (shown ∧ ¬fixed ∧ behind); capture `docs/shots/m8/settle-following.txt` not rewritten by the run.
- MAP.md graft: `pane_follows`, `abandon_probe` added to the G-settle entry.
- Not touched: design note, research (M2 is the researcher's), KNOWN_GAPS (conductor: Minor 6 rows).

Gates (workspace, own target dir): `cargo fmt --check` ok; `cargo clippy --all-targets -- -D warnings` ok, also with `--features heavy-tests`; `cargo nextest run --workspace` 5309/5309 passed (9 skipped), 624 s; `cargo test --workspace --doc` ok (3). After the run `git status` shows nothing under `docs/shots/` rewritten.
Not verified: the two pane-shown `held_key` scenarios have never run (heavy tier), so the `Kind::Rest` split has no test at MONOREPO scale yet. If it fails to find the rest there, the A tail falls back to the last byte, as before.
