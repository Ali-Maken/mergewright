# G-settle — per-row work starts when the cursor rests

Status: ready (spec revised 2026-10-02 after scout `scouts/03-settle.md`: the first spec's mechanism did not fit the code)
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
- [ ] `Rest` unit tests with given `Instant`s: a lone motion is not in a burst; 60 motions 16 ms apart are in a burst
      from the 2nd, deadline = last + 50 ms; gaps of 90 ms give a deadline of last + 150 ms; `rest_first` is true for
      another key and false for the burst's key and for job wakes; `rested` makes the next motion a leading edge.
- [ ] Loop test (fast tier, `event_loop/tests/settle.rs`, the `pump` seam with a counting job double): pane shown,
      60 `j` 16 ms apart over a range that includes a merge row. Follow commands started: one set on the first key,
      **none for keys 2–60**, exactly one set after the quiet window, for the final row. The first set's pending ids
      each get a `CancelJob` at key 2.
- [ ] A late `ListFiles` / `ContainedIn` / `EvilProbe` answer for a row the cursor left changes nothing shown
      (update-level test per kind).
- [ ] After rest, the pane shows the cursor's commit in the first frame after its `ListFiles` answer (loop test).
- [ ] A single `j`: follow commands in the same turn as the key (loop test). `j j j ⏎` and `j j j Tab` act on the
      cursor's commit, not the pane's old one (loop test).
- [ ] Pane hidden: no `ListFiles`/`ContainedIn` ever; `EvilProbe` only on the leading edge or at rest (loop test).
- [ ] Existing `update` tests unchanged and green (they run with `defer_follow = false`); the gate run proves it.
- [ ] Capture `docs/shots/m8/settle-following.txt`: a mid-burst frame. Cursor row 6+ rows below the pane's commit,
      pane title ending `reading…`. Muted colour **unverified** (rule 5).
- [ ] J1 tail ≤ 100 ms, unchanged or better: `scripts/m8-pace.sh` → `docs/perf/m8-settle.txt`, held_key summary
      lines against `docs/perf/m8-pace.txt`.

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
