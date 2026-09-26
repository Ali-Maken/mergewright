# 12b idle-advance-worker — Part B + Part D of 12, plus the 04b capture fix

Status: partial — Part B and Part D done; the 04b loop is not 0 (second cause, below)
Blocked by: 12 Part A (merged 7f64f5c0)
Crates: graft (`history::views`, `history::stream` worker loop, `update::history` apply arm, `event_loop/label.rs`,
`tests/it/heavy.rs`, `tests/it/two_boxes_over_content.rs`), graft-graph (tests only).
Build with: Opus (unchanged from 12).
Split by: conductor, 2026-09-26, along the builder's seam (12 compacted after Part A). No new decision.

## Scope
- **12 "Decided" → Part B and Part D**, exactly as written there. Signatures from 12 "Signatures": `Views::idle_step`,
  `IdleWork`, the `serve` loop, `HistoryUpdate::LaidOut` (+ its arms in `update` apply and `event_loop/label.rs`),
  `History::laid_out`. The 12 builder's NOTES "What remains (for the split)" lists what is ready.
- **12 "Acceptance" boxes for Part B (both) and Part D.** Tick them in 12's file, in place, each naming its test.
- **Carried fix, KNOWN_GAPS M7/04b:** `two_boxes_over_content::capture_two_boxes_over_real_content_for_docs_shots`
  fails ~50% under load (debugger: 30 of 31 fails were `ⓘ` missing on the tip row). Cause: `ready()`
  (`crates/graft/tests/it/two_boxes_over_content.rs:102-124`) doesn't wait for the radar job. Proposed, untested: end
  `ready` with `crate::captures::radar_answered(app, …)`. Prove it: a loop at 20 parallel that failed before, 0 after;
  say the counts. Add a tick for it under 12's Acceptance, marked "(12b, M7/04b)".

## Not in scope
Part C, the final perf rev, heap slope, heavy peak memory, `frames` additions — those are 12c.

## Rules
12 "Touches", "Rules / Gates". Gates workspace-wide. Release runs only through the script.

## NOTES (builder, 2026-09-26)

No compaction. Scout: used `scouts/12.md` + 12's NOTES; no mismatch found. Ticks are in 12's Acceptance.

### Part B — what was built
- `history/views.rs`: `serve` → `next_request`: before blocking, `Views::idle_step` a chunk at a time; parks in
  `recv()` on `IdleWork::Caught`. "A request is waiting" = `try_recv` inside the `stop` closure; a channel can't be
  peeked, so what it takes is kept in a `RefCell<Option<Arrival>>` (builder's choice, as 12 allowed) and taken next
  (then skipped to the newest, as before). `try_recv` also sees the senders gone → `Ended::Cancelled`, so a walk
  whose sender is dropped does no idle work (all existing `serve` tests unchanged).
- `Views` remembers the view last asked for (`shown`, set in `layout()`); idle work is in that view. Nothing asked →
  `Caught`, nothing opened (`an_idle_step_before_any_request_opens_nothing`).
- `LaidOut` once per (view, total): again after a fold (new total) and after a promotion swap (`announced` reset in
  `promote` — a new, cold layout). Spec said "once per walk"; per layout is what makes "wait for `LaidOut`, then `G`
  is warm" true after a promotion. The app test waits for the `LaidOut` sent in the final order for that reason.
- **Deviations:** `idle_step`'s `report` returns `Delivery` (as `serve`'s does), and `IdleWork` gains
  `NobodyListening` for it, so a gone loop is not swallowed (rule 3). `IdleWork::Promoting` is not added: nothing
  builds it until Part C (12c) and it would be dead code under clippy.
- `reinclude`'s caller in graft: `Views::layout` (`history/views.rs`, the `inclusion() != &view.inclusion` branch),
  reached from `serve` → `views.layout(&view)` for every request, and now from `idle_step`.
- `History::laid_out` is stored as heard, never cleared (a `LaidOut` carries no view, so one sent just before a view
  change can land after it). Nothing reads it but tests; 12c should decide if it drives the header.
- Red: the six worker tests and two app tests were written first against the spec's signatures (compile-red: no
  `idle_step`/`LaidOut`). `jobs::tests` (the ShowRows walk test) now asserts the `LaidOut{200}` that follows its done
  window, before `JobCancelled` (same worker, so ordered).

### Part D — :169
- Both tests green on first run: the fix exists (`viewport.rs` `renote`). Mutation check: with the `renote` call
  disabled both go red (`truncate_keeps_…`: `check` passes with no fingerprint; `a_rebuild_…`: panics in `note`
  at `cache.rs:151`, "continue no chunk growing at the frontier"). Reverted; `git diff` shows no `viewport.rs` change.

### 04b — `ⓘ` fixed, but the loop is not 0 (second cause)
- Fix (cause): `ready()` ends with `captures::radar_answered`. Counts (20-parallel, `--exact`, script in my
  scratchpad): before 26/40 failed, every one `ⓘ` missing on the tip row; after 0 `ⓘ` failures in 120 runs.
- **Second cause, 5/120 after, and 5/120 with the idle advance disabled** (temporary `recv`-only `next_request`,
  reverted): `ready()`'s first pump times out (120 s). Traced one (GRAFT_TEST_TRACE): the second app's
  `TopologicalOrder` landed **before** the first `Window`; the promoted window came back `laid_out: 180, done: false`
  and nothing asks again, so `phase` stays `Streaming` (`history/scroll/apply.rs:41` sets `Complete` only on a
  `done` window) while `ready()` waits for `Phase::Complete` (`tests/it/two_boxes_over_content.rs:119`). With the idle
  advance, `LaidOut{200}` does arrive but changes no phase. I traced only an idle-on run; that the baseline timeouts
  follow the same path is **unverified**.
- **Question for the conductor:** should `LaidOut` complete the walk in `History` (`phase = Complete`,
  `rows = total`)? It is true after it, and would close this hang, but changes the header ("N rows so far" →
  "N commits") — a draw change 12 rules out for this ticket. Otherwise `ready()` needs another settle condition.

### Touches
- `HistoryUpdate` matches: `history/scroll/apply.rs` (stores `laid_out`), `event_loop/label.rs` (+ label test),
  `tests/it/heavy.rs:315` (compiled with `--features heavy-tests`, clippy clean), `history/tests.rs:200`,
  `jobs/tests.rs`; `update/history.rs` and `frames.rs:1190` use `if let`/pattern on one variant (grep). All compile.
- `History` literals: all 8 use `..History::started()` (grep) — unaffected.
- Scroll/jump paths, header/`Esc`, cancel/shutdown: workspace nextest 3670/3670 (incl. m6-conflicts/85's
  `esc_and_the_walk`, pool/shutdown tests); no `docs/shots` file changed after the run (captures byte-identical).
- Shared locks: `repos.rs` mutexes are taken only at walk start (`stream.rs:39-45`); during idle the walk takes the
  `CommitTable` overflow `RwLock` per lookup (`graft-git/src/history/table.rs:36`), as a `G` does. No long hold.
- Worker count unchanged; no new thread. Keymap, journal/undo: none.
- MAP.md: graft `history` (`serve`, `Views::idle_step`, `IdleWork`, `LaidOut`, `History::laid_out`); graft-graph
  `truncate`. Not done here (12c): `begin_promotion`/`WindowOrder` lines, TESTING.md, DESIGN §3.6 sentence.
