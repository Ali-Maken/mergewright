# idle-layout-advance — while no request waits, the history worker lays out ahead to the end; `G` is warm; a request always preempts it

Status: ready (revised 2026-09-26, spec-only pass, after `scouts/12.md` showed the first spec's premises wrong)
Blocked by: 04-slow-key-cause, 04b-harness-input-thread (both done)
Crates: graft (`history::views` / `history::stream` worker loop, `update::history`, the `frames` harness and perf
script; `event_loop` only if Part A's measurement puts the cause there), graft-graph (tests only; no code change
expected — see Part D).
Source: note "Pass 2a" R8 step 4 (`docs/design-notes/m7-power-tools.md:778-807`); DESIGN.md §3.6 ("later calls run in
the background between frames"). The rows, quoted:
- KNOWN_GAPS :149 "Layout advances only as far as the screen asks … `G` cold until it's run once — first paint 13–113 ms,
  `G` on git/git 0.52–1.3 s …) … fast keys can stall at the last laid-out row."
- :161 "The walk answers nothing while a promotion lays out to the anchor … Bound the advance, or answer from the
  provisional layout."
- :169 "A chunk cut back by `truncate` loses its eviction fingerprint — a later rebuild of that chunk goes unchecked
  once."
- PROGRESS.md :102 (the user).

Build with: Opus. Worker-loop scheduling with measurable outcomes. Not Fable: the approach is decided below, and every
outcome is judged by a named test or a perf line (fails the third Fable test).
Scout: done — `.scratch/m7-power-tools/scouts/12.md` (and `scouts/04.md` §3-4 for the loop). No new scout.
Needs: 04's and 04b's perf file (`docs/perf/m7-04-slow-key.txt`, rev 2 = the baseline this ticket must not regress).

## What changed from the first spec (why)
- The layout lives in `Views` on the `ReadHistory` **pool worker** (`history/stream.rs:75`, `views.rs:35-50`); the UI's
  `History` holds only a `Window` (`history/mod.rs:9-10`). So the idle advance runs **on that worker**, not in `pump`.
- The worker needs no "UI is idle" signal. It is its own thread; the UI never touches the layout. "Idle" for it means
  **no `RowRequest` waiting on its channel**. A request that arrives preempts it within one pull.
- No idle-wake scheduling is added to `event_loop` (there is none, `mod.rs:9-10`, and none is needed): idle work sends
  **no event** to the UI, and once caught up the worker parks in `requests.recv()` — zero wakes, zero CPU.
- `advance(budget, stop)` already stops between batches and inside `pull` (`advance.rs:94-108`) and resumes on the next
  call. No `advance_until`: the stop closure carries the preemption.
- **Target is the end of history, not "10 screens ahead"** (note R8 said 10 screens). 10 screens cannot make `G` warm,
  which is this ticket's goal. Memory does not grow with the distance: chunks evict to `resident_chunks`, checkpoints
  thin to `max_checkpoints`, pass-through chunks read no commit objects (§3.6); the one term that grows, `EmittedOrder`,
  is what one `G` already pays. Cost: one walk's CPU per opened walk, on a worker already held (KNOWN_GAPS :148), then
  parked. Conductor: this deviates from note R8 step 4 — flag it at the milestone stop.
- :169 is already fixed in code (`viewport.rs:188-192` → `cache.renote`, since m1/52b) but no test names it: it closes
  with the tests in Part D, not with a code change.

## Decided

### Part A — the startup events ahead of key 1 (carried from 04b; do first, commit its perf rev before Part B)
- Measure, don't guess: the frames harness logs **every non-key wake** from the first `ShowRows` answer to key 1's frame:
  `frame_trace: startup i=<n> kind=<label> sent_us=<t> recv_us=<t> update_us=<t> draw_us=<t>`, and one line
  `frame_trace: startup burst_after_first_paint_us=<t> wakes=<n>` (time from the first painted window to the last
  startup wake). `kind` comes from `Wake::label` (below), never from `Debug` (a `Refs` payload is 50k refs).
- The harness already runs the real `opening` + `JobRunner` into the same `Sender<Wake>` (04b `budget_run::open_into`),
  so the burst is the real app's burst. **Whether the real app's key 1 waits** is answered by `burst_after_first_paint`:
  a key pressed inside that window waits the same way; one pressed after it doesn't. NOTES says which, with the number.
- Fix at the cause, by what the lines show:
  - **Most of the queued time is `draw_us` of wakes that follow each other with nothing between** (a frame drawn per
    event nobody sees): the loop takes every wake already queued (`Wakes::waiting()` true), runs `update` for each, and
    draws once. Bounded: stop taking at a key (a key is always drawn in its own pass) or after 8 ms of updates. This is
    in `event_loop/running.rs:104-149`.
  - **One event's `update_us` dominates** (e.g. a refs answer rebuilding 50k badges): cut that handler's cost at its
    cause; if that needs a design change (moving it to a job), stop and ask.
  - **The events come only from the harness** (a job the real app doesn't start): fix the harness, as 04b did.
  - Anything else: record it, stop and ask (rule 2). No reordering of keys ahead of job events: a key must act on the
    state the events before it built.
- The idle advance (Part B) must add **nothing** to this queue (`idle_steps_send_no_event`).

### Part B — idle advance on the history worker
- `serve` (`views.rs:189`) today blocks in `requests.recv()`. New loop:
  1. Before blocking, `views.idle_step(&stop)` where `stop = || cancelled() || a request is waiting`.
  2. `IdleWork::Advanced` / `Promoting` → `try_recv`; a request → answer it (as today); none → step again.
  3. `IdleWork::Caught` → `recv()` (parks: no wake, no CPU).
- "A request is waiting" uses the same check `answer` uses for a replacing request (`newer()`, `views.rs:241-282`).
  If it needs `&mut`, wrap it in a `Cell`/`RefCell` for the `&dyn Fn` — builder's choice, named in NOTES.
- One `idle_step` = at most one `advance(chunk_rows, stop)` call: ≤ one chunk, stop checked before every pull.
- Order inside `idle_step`: a pending promotion's reach first (Part C), then the frontier toward the end of history.
- Idle work sends **no** `Progress`, no `Window`. One event only: when the frontier reaches the end of history,
  `HistoryUpdate::LaidOut { rows }`, **once per walk** — it lets the UI know the total, and lets tests wait for it
  deterministically instead of sleeping. The UI stores it (`History::laid_out`); no draw change in this ticket.
- The header's "reading" (`History::is_reading`, `walk_is_reading`) is unchanged: idle work is not a request, so the
  header doesn't say reading and `Esc` doesn't cancel it (he isn't waiting on it). Cancel, `Ctrl+R` and shutdown still
  stop it within one pull (`stop` includes the cancel token).
- An inclusion change (`Space` fold, `1`, `H` → `reinclude` → `truncate`) moves the frontier back; the next
  `idle_step` resumes to the end. Builder: name `reinclude`'s caller in graft (the scout grepped only `.truncate(`).

### Part C — promotion answers from the provisional layout (:161)
- `promote` (`views.rs:88-106`) splits: `begin_promotion` opens the promoted layout and leaves it **pending**; `reach`'s
  loop (`views.rs:142-161`) moves into `idle_step`, bounded by the same `stop`.
- Until the anchor has a row, requests are answered from the provisional layout. When it has one: swap, then re-answer
  the last request from the promoted layout (one `Window` event — the screen changes).
- Every `Window` carries the order its rows came from. `History` applies the promotion (order notice cleared,
  re-anchor) when the first `Promoted` window lands, not when `sorted` sends the request (`update/history.rs:49-66`).
  If `Window` already carries the order, reuse it and say so.

### Part D — :169, closed by test
- Two graft-graph tests over today's `truncate` → `cache.renote` path. Expected green on first run (the fix exists);
  that is the finding, stated in NOTES. If either is red, that is the defect: fix it red-first.

### Memory
- Chunks and checkpoints keep their existing limits. `scripts/heap-slope.sh` before/after; the heavy tier's
  peak-memory test (100k fixture, jump to bottom and back, §3.6) stays green.

## Read first
- This ticket; `scouts/12.md` (all), `scouts/04.md` §3-4.
- Note `docs/design-notes/m7-power-tools.md:778-807` (R8).
- `docs/design/03-graph-layout-algorithm.md` §3.6 (lines 260-351: streaming, eviction, pass-through, cold deep jump,
  promotion).
- `docs/perf/m7-04-slow-key.txt`: lines 6-32 (`## cause`) and 547-562 (rev 2 SUMMARY); 04b's NOTES.
- `crates/graft-graph/MAP.md`: `Advance`, `StreamingLayout::advance`/`rows`/`truncate`, `Invalidated`, `fingerprint`,
  `reinclude`. `crates/graft/MAP.md`: `history`, `event_loop`. (MAP.md has no lines for `Views`/`serve`; add them.)

## Signatures
```rust
// graft::history::views (pub(crate))
pub(crate) enum IdleWork {
    /// The frontier moved; `rows` laid out in this step.
    Advanced { rows: usize },
    /// A pending promotion moved toward its anchor.
    Promoting { rows: usize },
    /// Nothing to do: frontier at the end of history, no promotion pending. The caller parks in `recv()`.
    Caught,
}
impl Views {
    /// One bounded step of work nobody asked for: ≤ one `advance(chunk_rows, stop)`. Sends no event except
    /// `HistoryUpdate::LaidOut` once, when the frontier first reaches the end.
    pub(crate) fn idle_step(&mut self, stop: &dyn Fn() -> bool, report: &mut dyn FnMut(HistoryUpdate))
        -> Result<IdleWork, /* the error type `answer` returns today */>;
    /// Open the promoted layout; leave it pending. `idle_step` reaches the anchor; requests meanwhile are answered
    /// from the provisional layout. Takes what `promote` takes today.
    pub(crate) fn begin_promotion(&mut self, /* `promote`'s arguments today */);
}

// graft::history
pub enum HistoryUpdate { /* existing variants */, LaidOut { rows: usize } }
pub enum WindowOrder { Provisional, Promoted }   // on every `Window` answer; reuse an existing field if one exists
impl History { pub laid_out: Option<usize> }      // field

// graft::event_loop — for the startup trace only
impl Wake { pub fn label(&self) -> &'static str; }   // variant name, and the `Product`/`HistoryUpdate` kind inside

// graft-graph: no new API. `StreamingLayout::advance(&mut self, budget: usize, stop: &dyn Fn() -> bool)` as today.
```

## Journey (no UI; the step counts are unchanged, the waits are what change)
- **`G` after he's been reading:** open → (any pause ≥ the walk's catch-up) → `G`. **1 key.** Lands on the last
  commit, cursor on it, header shows no "laying out". Today: 1 key, then 0.52–1.3 s on git/git with a progress line.
- **Scrolling fast past the frontier:** `j`/`PgDn` held. Never stalls at the last laid-out row (:149).
- **After a promotion:** scrolling keeps answering while the promoted layout catches up; the rows switch order once,
  without moving the anchored commit (:161).
- Where he lands after each event: `Ctrl+R` or new refs → a new walk, idle advance restarts; an inclusion change → same
  screen, idle resumes; cancel/shutdown → worker freed within one pull.

## Acceptance (each tick names its test or perf line, at tick time)
- [ ] **Part A** (perf rev before Part B):
  - `frame_trace: startup …` lines in `docs/perf/m7-12-idle-advance.txt`; NOTES names each wake kind ahead of key 1,
    its `update_us`/`draw_us`, the burst length, and whether the cause is in Graft.
  - If in Graft: the fix's red-first test, named (e.g. `queued_wakes_draw_once` for a drain-then-draw loop), and the
    rev line showing key 1 ≤ 16 ms in all 3 runs. If outside: the evidence line, and the builder stopped and asked.
  - `Wake::label` unit test: `every_wake_has_a_label`.
- [ ] **Part B**, graft (`history::views` tests, over a counting source):
  - `idle_steps_reach_the_end_of_history_then_return_caught`;
  - `a_caught_up_idle_step_pulls_nothing` (the parked worker does no work);
  - `a_request_preempts_the_idle_step_within_one_pull`;
  - `a_cancel_stops_the_idle_step_within_one_pull`;
  - `idle_steps_send_no_event` (only `LaidOut`, once);
  - `an_inclusion_change_resumes_the_idle_step`.
- [ ] **Part B**, app level (fast tier, corpus shape of a few thousand commits, through `budget_run`):
  - `g_after_laid_out_lands_without_laying_out`: wait for `LaidOut`, send `G`; no `Progress` between `G` and the
    landing window; the frame shows the root commit.
  - `no_wake_after_laid_out`: after `LaidOut`, no wake of any kind for 500 ms with no input.
- [ ] **Part C:** `a_request_during_a_promotion_is_answered_from_the_provisional_layout`;
  `the_promoted_layout_replaces_the_provisional_once_the_anchor_has_a_row`;
  `a_provisional_window_after_sorted_keeps_the_order_notice` (`update::history` tests).
- [ ] **Part D** (graft-graph): `truncate_keeps_a_fingerprint_for_the_kept_prefix` (mid-chunk cut; the cut chunk has a
  fingerprint whose row count is the kept rows); `a_rebuild_after_truncate_is_checked` (a source whose re-read differs
  in the kept prefix → `RebuildDiffers`, after evicting the cut chunk). NOTES says whether each was red.
- [ ] **Release** — `scripts/m7-perf-key.sh docs/perf/m7-12-idle-advance.txt`, run once at the start (rev 1 = before,
  on this ticket's base) and once after (rev N, 3 runs each, the script appends and computes `before=`):
  - every `spread` line `gate(p99<=16ms)=ok`, and p99 no worse than rev 2 of `m7-04-slow-key.txt` beyond its spread;
  - idle-key latency median no worse than rev 2 (1.1–3.2 ms), idle `asked=0` kept;
  - new scenario `g_after_idle` (FiftyThousand fixture: `ready_to_scroll`, wait for `LaidOut`, 2 s idle, `G`; latency
    from the send to the frame showing the last row): before and after recorded, after asserted ≤ 16 ms. If it can't
    meet it, the number is recorded and the builder stops and asks; no silent threshold;
  - `idle_gap_wakes=<n>` per 600 ms gap after `LaidOut`, asserted 0;
  - `examples/budgets` `G` on git/git (`scripts/clone-git-git.sh`), before/after, recorded, not asserted.
- [ ] `scripts/heap-slope.sh` before/after into the same perf file; slope within the recorded noise. Heavy-tier
  peak-memory test green (named at tick time).
- [ ] Captures: `check_shot` over all, byte-identical (no visual change is intended; a changed capture is a defect
  here).
- [ ] MAP.md: graft `history` gains `Views::idle_step`, `IdleWork`, `begin_promotion`, `HistoryUpdate::LaidOut`,
  `WindowOrder`, `History::laid_out`, and one line for `serve`'s loop; `event_loop` gains `Wake::label`. graft-graph:
  `truncate`'s line says it re-notes the kept prefix. TESTING.md "Performance": `g_after_idle`, `idle_gap_wakes`,
  the startup lines. NOTES proposes the DESIGN.md §3.6 sentence ("on the history worker whenever no request is
  waiting, to the end of history, then parked") for the conductor to apply. The conductor closes KNOWN_GAPS :149,
  :161, :169 by the test names above.

## Touches (check each, report how)
- **`history/views.rs` `serve`/`answer`/`promote`/`reach`, `history/stream.rs`, `jobs/kinds.rs` `ReadHistory` report:**
  the history tests; the worker count unchanged (no new thread).
- **Every `match` on `HistoryUpdate` and every reader of `Window`:** grep, list them in Touches; `update::history`
  tests.
- **Every scroll/jump path (`j k PgUp PgDn G g`, search `Find`, Map seek):** served from the same layout. The graph
  navigation tests; captures byte-identical.
- **Header / `Esc`** (`History::is_reading`, `walk_is_reading`, `escape_key`): unchanged behaviour; their existing
  tests (m6-conflicts/85) green.
- **Cancel, `Ctrl+R`, shutdown, KNOWN_GAPS :148 (worker held):** the pool tests and shutdown tests; the worker is
  freed within one pull.
- **Other readers of the same repository handle while the worker advances** (detail diffs, refs, radar jobs): check for
  a lock shared with the history worker (grep `Mutex`/`RwLock` in `repos`, `repository`); name what was found.
- **Idle rasterise (m1/30h `prefetch`):** UI-thread only, untouched unless Part A changes the loop; its tests.
- **If Part A changes `running.rs`:** every walk that waits on frames (`m3_walk`, `m4_walk`, `m5_walk` incl. M6
  `FrameSettled`), `frames`, the pty capture scripts' outputs via `check_shot`.
- **Battery / idle CPU:** `a_caught_up_idle_step_pulls_nothing`, `no_wake_after_laid_out`, `idle_gap_wakes=0`.
- **Keymap, journal/undo:** none (no key, no operation).

## Rules / Gates
Workspace-wide gates. Release runs only through the script, output to the perf file; read its SUMMARY lines. Part A's
perf rev is committed at the seam before Part B starts.

## Size
Large (~80k). Split seams, in order, if the builder compacts: **A** (trace + cause fix) | **B + D** (worker idle loop,
:169 tests) | **C** (promotion) + the final perf rev.

## Carried from 04b (conductor, 2026-09-26)

With the harness now feeding keys first-in-first-out like the app (04b), the **first key** in 4 scenarios reads
12–23 ms max: it queues behind startup job events (`docs/perf/m7-04-slow-key.txt` rev 2). Which events, and whether
the real app's key 1 waits the same way, are **not identified**. Identify them by measurement (per-event timing in the
trace, the perf script) before this ticket's idle-advance work changes the same queue; say in NOTES whether the cause is
in Graft and, if so, fix it at the cause. Acceptance: one more tick naming the test or perf line that shows it.
→ Specified as **Part A** above. Candidates from `scouts/12.md` "Carried from 04b": `Started` per startup job,
`Product::{Repository, SystemGit, Refs, WorkingTree}`, `HistoryUpdate::{Started, Progress, Window}`, the `settle`
follow-ups (radar, probes, journal, `request_at_launch`, `open_explain`), `PromoteHistory` → `TopologicalOrder`.
