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
- [x] **Part A** (perf rev before Part B):
  - `frame_trace: startup …` lines in `docs/perf/m7-12-idle-advance.txt` (revs 2 and 3); NOTES names each wake kind
    ahead of key 1, its `update_us`/`draw_us`, the burst length, and that the cause is in Graft. ✔
  - In Graft: red-first tests `event_loop::tests::queued_wakes_draw_once` (red 4≠2),
    `a_key_behind_queued_wakes_is_drawn_in_its_own_pass` (red 5≠4), `update::radar::tests::
    a_force_push_answer_does_not_reclassify_every_ref` (red: "the ref findings were rebuilt"). Rev line: rev 3
    `SUMMARY spread` max 0.8–6.9 ms over every key of all 3 plain runs (key 1 included), `## slow scroll keys (latency
    over 8 ms)` empty, every `gate(p99<=16ms)=ok`. ✔
  - `Wake::label` unit test: `event_loop::label::tests::every_wake_has_a_label`. ✔
- [x] **Part B**, graft (`history::views` tests, over a counting source) — all in `history::idle_tests` (12b), over
  `Counting<WalkSource>` on the Linear shape, `SMALL` chunks of 8:
  - `idle_steps_reach_the_end_of_history_then_return_caught`; ✔
  - `a_caught_up_idle_step_pulls_nothing` (the parked worker does no work); ✔
  - `a_request_preempts_the_idle_step_within_one_pull`; ✔
  - `a_cancel_stops_the_idle_step_within_one_pull` (through `serve`, sender still held); ✔
  - `idle_steps_send_no_event` (only `LaidOut`, once); ✔
  - `an_inclusion_change_resumes_the_idle_step` (FoxtrotMerge, fold → `reinclude` → `truncate`). ✔
  - Extra: `an_idle_step_before_any_request_opens_nothing`, `serve_lays_out_to_the_end_between_requests` (the
    loop on a thread: `LaidOut{200}` with nothing sent on the way, then `G` answered with 0 pulls, no `Progress`). ✔
- [x] **Part B**, app level (fast tier; **not through `budget_run`** — it and `synthetic` compile only under
  `heavy-tests`; instead `it::idle_advance` over `app_harness::App` and a 3,000-commit line built with one
  `git fast-import`, no commit-graph so it is promoted):
  - `g_after_laid_out_lands_without_laying_out`: wait for `LaidOut` in the final order, send `G`; no `Progress`
    between `G` and the landing window; the frame shows the root commit, cursor on row 2,999. ✔
  - `no_wake_after_laid_out`: after `LaidOut` and startup's other jobs (working tree, radar, probes — the walk may
    say nothing meanwhile), no wake of any kind for 500 ms with no input. ✔
- [x] **(12b, M7/04b)** `two_boxes_over_content::capture_two_boxes_over_real_content_for_docs_shots`: `ready()` now
  ends with `captures::radar_answered`. 20-parallel loop (`it` binary, `--exact`): **before** 26/40 failed, all
  `ⓘ` missing on the tip row (`committed ◆ ▏ⓘ [◆ main ↑1]` / `drawn ◆ ▏  [◆ main ↑1]`); **after** 0 `ⓘ` failures in
  120 runs. **Not ticked: the loop is not 0 after** — 5/120 time out in `ready()`'s first pump (a second cause, see
  12b NOTES; the same 5/120 with the idle advance disabled). ✔ (12c) Deterministic test first:
  `history::promotion_tests::a_promotion_before_the_first_window_ends_as_the_normal_path_does` (red: race ended
  `Streaming, 40 rows`, normal path `Complete, 200`; green after `ready_to_swap`). Loop after (`it` binary,
  `--exact`, 6 rounds × 20 parallel): **0 failed of 120**, 0 `ⓘ`, 0 `ready()` timeouts. Workspace nextest
  3687/3687 incl. every `check_shot`: normal-path captures byte-identical.
- [x] **Part C:** `a_request_during_a_promotion_is_answered_from_the_provisional_layout`;
  `the_promoted_layout_replaces_the_provisional_once_the_anchor_has_a_row` (both `history::promotion_tests`, red
  first); `a_provisional_window_after_sorted_keeps_the_order_notice` (`update::history::tests`, green on first
  run — the UI already swapped order only on a promoted window). ✔ (12c)
- [x] **Part D** (graft-graph): `truncate_keeps_a_fingerprint_for_the_kept_prefix` (mid-chunk cut; the cut chunk has a
  fingerprint whose row count is the kept rows); `a_rebuild_after_truncate_is_checked` (a source whose re-read differs
  in the kept prefix → `RebuildDiffers`, after evicting the cut chunk). NOTES says whether each was red. ✔ Both in
  `layout::stream::tests`, green on first run (12b NOTES).
- [ ] **Release** — `scripts/m7-perf-key.sh docs/perf/m7-12-idle-advance.txt`, run once at the start (rev 1 = before,
  on this ticket's base) and once after (rev N, 3 runs each, the script appends and computes `before=`):
  - every `spread` line `gate(p99<=16ms)=ok`, and p99 no worse than rev 2 of `m7-04-slow-key.txt` beyond its spread;
  - idle-key latency median no worse than rev 2 (1.1–3.2 ms), idle `asked=0` kept;
  - new scenario `g_after_idle` (FiftyThousand fixture: `ready_to_scroll`, wait for `LaidOut`, 2 s idle, `G`; latency
    from the send to the frame showing the last row): before and after recorded, after asserted ≤ 16 ms. If it can't
    meet it, the number is recorded and the builder stops and asks; no silent threshold;
  - `idle_gap_wakes=<n>` per 600 ms gap after `LaidOut`, asserted 0;
  - `examples/budgets` `G` on git/git (`scripts/clone-git-git.sh`), before/after, recorded, not asserted.
  **(12e) Not ticked — one sub-item fails.** Rev 4 of `docs/perf/m7-12-idle-advance.txt` (HEAD 7f9c42c9, runs
  17:40–17:50Z, no foreign process > 40% CPU, load 5.2 → 1.6 falling off the build), per sub-item:
  - spread gate: ✔ all 7 `SUMMARY spread … gate(p99<=16ms)=ok`; `SUMMARY runs: 6 of 6 exit 0`; script exit 0 —
    exit status and SUMMARY agree (12d's first real release run).
  - p99 vs m7-04 rev 2: ✘ **worse beyond its spread in 5 of 7** (e.g. Unicode 50k Closed 3147–3221 vs 2798–2838 µs;
    Map 1195–1343 vs 431–1079); within it for both Pixel scenarios. Control D in the same run: `machine: busy=false
    work_warm_us` 2199–6677 µs vs 1026–1738 in rev 3 — the same fixed work, no Graft code, 2–5× slower. The machine
    was slow, as 12g found for key 37; not rerun (rule). Whether any of it is code is **not shown**.
  - idle median: ✔ `SUMMARY idle control=Plain` medians 1155–2897 µs (inside 1.1–3.2 ms), `asked=0/15` every row.
  - `g_after_idle`: ✔ `frames::after_idle::g_after_idle` — rev 4 `SUMMARY g_after_idle` 8156–11938 µs over 6 runs;
    `scripts/m7-12-final-perf.sh` (18:01Z) `SUMMARY g_after_idle side=before latency_us=51051(max 52076)
    wakes_between=201` vs `side=after 8663(max 8928) wakes_between=1`.
  - `idle_gap_wakes`: ✔ `frames::check_idle_gaps` — every rev 4 `SUMMARY idle_gap … idle_gap_wakes=0
    pauses_after_laid_out=5 of 5`, 42 lines; g_after_idle's 2 s gap 0 too. Reading unit-tested by
    `frames::readings::idle_gaps_count_what_nothing_asked_for_after_laid_out`.
  - budgets `G` on git/git (85,649 commits, cg): `SUMMARY budgets_G` before 148.1/141.1 ms, after 148.5/141.0 ms.
  **(12e) Rev 5, the re-measure (conductor's terms: accept it only if control D is back in 1.0–1.7 ms).** HEAD
  8024c850, 18:08–18:18Z, load 1.3–1.7, only `launchd` > 40% twice (43–45%). Control D `machine: busy=false
  work_warm_us` **2072–4417 µs — still slow**, so rev 5 is not accepted either; box stays unticked. For the record:
  all 7 spreads `ok`, `SUMMARY runs: 6 of 6 exit 0`; p99 still above m7-04 rev 2's spread in 5 of 7 (e.g. Unicode
  50k Closed 2855–2887 vs 2798–2838; Map 1083–1098 vs 431–1079), within it in both Pixel scenarios; idle medians
  1124–3316 µs, `asked=0/15`; `SUMMARY g_after_idle` 8033–10303 µs. Both revs kept in the perf file. Timestamps:
  rev 4 2026-09-30T17:40–17:50Z, rev 5 2026-09-30T18:08–18:18Z.
- [x] `scripts/heap-slope.sh` before/after into the same perf file; slope within the recorded noise. Heavy-tier
  peak-memory test green (named at tick time).
  ✔ (12e) `scripts/m7-12-final-perf.sh` section of 18:21Z (HEAD 8024c850, `SUMMARY steps: 16 of 16 exit 0`,
  `SUMMARY quiet: 16 of 16`): `SUMMARY heap_slope side=before footprint_peak=34.41,28.22 live_at_end=5.19,5.19`,
  `side=after footprint_peak=33.86,31.31 live_at_end=5.19,5.19` B/commit — after inside before's own run-to-run
  spread (28.2–34.4); live identical; housekeeping-02's recorded after was 31.49. (Prototype needed a compile fix
  first: see 12e NOTES.) Heavy: `SUMMARY heavy_debug exit=0 … 12 passed` — `heavy::budgets_hold_*` (5, peak RSS),
  `heavy::slopes_per_commit_with_a_commit_graph`, and heavy.rs:217's
  `heavy::the_table_and_the_order_stay_at_the_commit_count_after_g_and_after_promotion ... ok`; release
  `SUMMARY heavy_release_budgets exit=0 … 5 passed`.
- [x] Captures: `check_shot` over all, byte-identical (no visual change is intended; a changed capture is a defect
  here). ✔ (12e) Workspace `cargo nextest run` 4979/4979 (5 skipped) at 7f9c42c9 incl. every `check_shot`; `git
  status` shows no `docs/shots` change.
- [x] MAP.md: graft `history` gains `Views::idle_step`, `IdleWork`, `begin_promotion`, `HistoryUpdate::LaidOut`,
  `WindowOrder`, `History::laid_out`, and one line for `serve`'s loop; `event_loop` gains `Wake::label`. graft-graph:
  `truncate`'s line says it re-notes the kept prefix. TESTING.md "Performance": `g_after_idle`, `idle_gap_wakes`,
  the startup lines. NOTES proposes the DESIGN.md §3.6 sentence ("on the history worker whenever no request is
  waiting, to the end of history, then parked") for the conductor to apply. The conductor closes KNOWN_GAPS :149,
  :161, :169 by the test names above.
  ✔ (12e) Checked by grep, no test: graft MAP.md `serve` :55, `Views::idle_step` :56, `begin_promotion` :57,
  `IdleWork` :58, `WindowOrder` :59, `HistoryUpdate::LaidOut` :60, `History::laid_out` :61, `Wake::label` :894
  (12c/12f wrote them; 12e confirmed); 12e added the `frames` submodules, `after_idle`, `idle_gaps` and the perf
  script's exit status to the frame-trace line, and the budgets promote-swap label. graft-graph MAP.md :111
  `StreamingLayout::truncate` "re-notes the kept prefix's fingerprint". TESTING.md heavy tier: exit status, the
  three idle-advance readings, `scripts/m7-12-final-perf.sh`. The DESIGN §3.6 sentence is in 12e's NOTES.
- [x] **(12c, review 04) Major 1 — the perf script never passes a failed run.** `scripts/m7-perf-key-selftest.sh`
  (canned outputs via `M7_PERF_KEY_CANNED`, no cargo): 7 cases, 21 checks. Red first on the old script: 17 of 21
  failed (a plain run exiting 101 read `gate(p99<=16ms)=ok`, script exit 0). Green after: `SUMMARY runs: FAILED 1 of
  6: plain2 exit 101`, `gate(p99<=16ms)=FAIL(plain2 failed)`, exit 1; a 2-cell spread reads `MISSING(2 of 3)`, exit
  1; a failed control, a missing output and the rev comparison all checked. `frames:` p99 line now printed by
  `frames::verdict::judge` after the pixel and rows-on-hand asserts, before the gate's own assert. Not run on a real
  release run (no perf runs in 12d). ✔ (12d)
- [x] **(12c, review 04) Major 3 — `frames.rs` split.** 1706 → `frames.rs` 292 (tests, script, `check_scale`) +
  `frames/{run 361, paced 348, sink 286, readings 259, controls 160, verdict 122}`; `scroll` 60 lines, the `Run`
  built by `run::measured`. No behaviour change beyond Major 1's print order: `cargo test --features heavy-1m --test
  it -- frames:: --list` gives the same 11 tests before and after (two now under `frames::controls::` /
  `frames::sink::`, still under the script's `frames::` filter); `frames::sink::the_sink_counts_…` passes; clippy
  `--features heavy-1m --all-targets -D warnings` clean. Scroll tests not run (heavy, release). ✔ (12d)

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

## NOTES (builder, 2026-09-26)

**I compacted mid-ticket.** Stopped at the first split seam: **Part A is done and committed; B, C, D and the final
release/heap/budgets/docs items remain** (list at the end).

Scout: used `scouts/12.md` (+ 04/04b NOTES). Its premises match this respec; no new mismatch found.

### Part A — what was ahead of key 1 (rev 2, plain3, FiftyThousand Unicode Closed, the clean run)
- 68 wakes from the first `Progress::History::Window` to key 1: `Produced::RadarContext` (update 4.2 ms),
  `Produced::Journal`, then 32× `Started::ForcePushProbe` + `Started::DuplicatesProbe` (draw 0.3–7.8 ms each, update
  ~0), then 32× `Produced::ForcePushProbe` (**update 1–5 ms each**, draw 0.3–1 ms). All sent within ~36 ms of first
  paint (`burst_after_first_paint_us` 16–55 ms across runs); the loop took ~170 ms to drain them. Fixture refs: 3–5
  wakes, no burst.
- Real app: the same `opening` + `JobRunner` feed the same channel, so a key pressed within ~20–55 ms of first paint
  waited behind this burst; one pressed later did not.
- **Cause in Graft, two parts, both fixed at the cause:**
  1. `update/radar.rs` `refresh_ref_findings`: every probe answer bumps `force_push_revision`, which re-ran
     `classify_refs` over all 50k refs. Now `ref_findings` rebuilds only when refs/context/HEAD/default/policy move;
     the force-push maps rebuild alone, O(verdicts ≤ 32), each verdict placed at the tip it was probed at
     (`Product::ForcePushProbe(branch, tip, verdict)`, `RadarState::force_push: HashMap<String, Loading<Probed>>`).
     Behaviour change: a verdict whose branch has since moved marks nothing (before: a `NotCheckable` line moved to
     the new tip). Test `a_verdict_for_a_branch_that_has_moved_marks_nothing` (written after the change, not run red;
     under the old code it would fail — the old loop placed `NotCheckable` at the branch's current tip).
  2. `event_loop/running.rs`: one draw per queued job answer. New `Wakes::job_event_waiting` (default `false`;
     `ChannelWakes` peeks its `seen`); the loop runs job events queued back to back and draws once, bounded by
     `BATCH_BUDGET` = 8 ms of updates; a key is never folded in (drawn in its own pass). The 8 ms bound has **no test**
     (it needs a clock seam); verified only by reading. Tests `queued_wakes_draw_once`,
     `a_key_behind_queued_wakes_is_drawn_in_its_own_pass`.
- Evidence, rev 3 (same scenario): `Produced::ForcePushProbe` update 1–5 **µs**; 66 of 68 startup wakes undrawn, one
  draw (286 µs) at the end; last startup wake handled at 20.8 ms after the first window (was ~170 ms). Every plain run
  max 0.8–6.9 ms; controls B and C, which failed at base (rev 1), exit 0.
- Load caveat (rule 1): rev 2's plain runs failed with seconds of latency under other agents' load (load avg 14–18,
  other worktrees' test binaries at 100% CPU) — not the harness. Rev 3 ran at load 6.1 → 1.8 (uptime before/after,
  not in the perf file). Rev 1 (base) ran at unrecorded load; the before/after `SUMMARY rev3` lines compare against
  rev 2, which is contaminated — compare rev 3 with rev 1 by eye.
- Harness change: `frames` `Paced` implements `job_event_waiting` without `stamp` (the stamp marks a drawn frame).

### Extra item (conductor, mid-ticket): `m5_walk::parse_counts` read only one fence
- `crates/graft/tests/it/m5_walk/journeys.rs`: reads every ```` ```journey-counts ```` fence; a duplicate id across
  fences is refused; one-fence notes (M5/M6) unchanged. Tests `two_fences_are_read_as_one_block`,
  `a_duplicate_id_across_fences_is_refused`, `no_block_is_an_error`; `m7_walk::restart_after_done` green; all 148
  m5/m6/m7 walk tests green. Commits 4e1ea128, 60716460.

### Touches (Part A)
- `update::radar` callers of `force_push_probed` / readers of `RadarState::force_push`: grep → `radar.rs`,
  `view/radar.rs` tests, `view/tests.rs` capture fixture, `jobs/runner.rs`, `event_loop/label.rs`, `update/mod.rs`
  (pattern `..`, unchanged). All compile; radar/view tests green.
- `running.rs` loop: every walk that waits on frames (m3/m4/m5/m6/m7 walks), `frames`, event-loop tests — workspace
  nextest 3596/3596 green; captures byte-identical (no `docs/shots` change after the run).
- Prefetch (m1/30h): its tests green (`event_loop::` 73/73 + 2 new).
- MAP.md: graft `Wake::label`, `Wakes::job_event_waiting`, `is_job_event`, `Probed`, `force_push_probed`,
  `refresh_ref_findings`, `force_push_revision` lines updated.

### What remains (for the split)
- **B + D:** `Views::idle_step`/`IdleWork`, `serve` loop, `HistoryUpdate::LaidOut` (+ arms in `update` apply,
  `event_loop/label.rs` — ready in the builder's scratch as a two-line add, `tests/it/heavy.rs:315`),
  `History::laid_out`; the six worker tests and two app-level tests; graft-graph Part D tests.
- **C + final:** `begin_promotion`, the three `update::history` tests, `g_after_idle` + `idle_gap_wakes` in `frames`,
  final perf rev, heap-slope before/after, heavy peak-memory, `examples/budgets` `G` on git/git, TESTING.md
  "Performance", MAP.md history lines, DESIGN §3.6 sentence proposal.
