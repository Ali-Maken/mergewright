# fix round: fold rules — what may wake the app at rest, and rows vs commits under the fold

Status: done (17/17 boxes; one timing flake recorded, see NOTES)
Blocked by: 18-lag-and-idle-wakes (merged), 17-stacked-branch-lanes (merged)
Build with: Sonnet (`builder-sonnet`, model sonnet); fails twice → `builder-opus`. Why: both rules are decided below and pinned by unit tests that need no fixture; the code is one command's shape, one harness classification and three test assertions. The only judgement left (wake attribution in the harness) is spelled out case by case.
Needs: none. 18's `refs::branch_list` (merged) is not touched.

From review Major 4 (`docs/reviews/ease-pass-07-14.md:164-177`: "fix the cause or change the rule by decision, not by loosening"; "the user-visible answer being rows, not commits"), 16 NOTES table (`16-fix-heavy-tier.md:27-29`), 18 NOTES cause 2 (`docs/perf/18-lag.txt:28-35`).

## Need
No field note: he hasn't hit this. Two things hurt. (1) The app's promise "a quiet app has nothing to say" (`frames.rs:373-377`) can't be tested: 4 frame tests stop at `frames.rs:393`, so their p99 never prints. (2) Seven heavy tests (5 budgets + `after_idle`, `16-heavy.txt:47-55`) never reach the first-paint and RSS budgets, so the 100k budgets (PROMPT §1.2, §7.1) have had no measurement since 06c. He keeps the app open all day on a laptop, so a wake at rest costs battery.

## Evidence (rule 7: each claim with its line)
- **Wakes.** `update/fold_count.rs:28-43`: one `CountFolded` per rest, "every answer is an event whose settle asks for the next". So N folding merges on screen give N answers in a chain, each one a wake and a frame. 18's probe saw 3 of them land 10/18/26 ms after `Rested` (`18-lag.txt:30-31`). The harness counts every job event except a key's first `Window` as unasked (`tests/it/frames/readings.rs:239-262`, the arm at `:257-258`). The design asks for these counts: "fold counts asked at rest, the number absent until it lands" (`docs/design-notes/ease-pass.md:165`).
- **Rows ≠ commits.** `tests/it/heavy.rs:452-456` (the line 16 cited as `:585` has moved) asserts `rows == scale.commits()`, "every commit laid out". `tests/it/frames/after_idle.rs:61` takes `rows = SCALE.commits()` (and per 16, the wait under `:58` waits for 100 000 rows). Both tests were written in m1, before the fold. The fold is the default (`crates/mergewright/MAP.md:29`, `MergedBranches::Folded (default)`) and is decided (`ease-pass.md:226-243`, Hard choice 2). 79 195 of 100 000 is the fold working.
- `heavy.rs:300` (`ask_and_lay_out`, `View::default()`) also asserts rows == commits. It passed in 16's run (it is missing from the failure list at `16-heavy.txt:46-60`), so `View::default()` does not fold. Leave it as it is.

## Decisions
**D1. What may wake the app at rest.** An input is a key, a paste, a mouse event, a resize, a focus change or a signal. An input may cause reads. Each read must be asked by one of:
- (a) the update handling the input itself;
- (b) the loop's own `Rested` after it;
- (c) the first `Window` answering that input's `ShowRows`. The window has to land before the app knows what is on screen.

Each such read wakes the loop **at most twice**: its `Started` (if the job sends one) and its one final answer. **Nothing else wakes the app.** In particular, nothing is asked by another read's answer (no chains), there are no timers or polls, and no background count runs. Once the last answer has landed, there are zero wakes until the next input.
- Rejected: **count with the window** (18's option b). It puts git walks on the 16 ms input-to-frame path, and a held `j` would count every merge it passes. Both go against `fold_count.rs:28-32` and the design note `:165`.
- Rejected: **count every fold in the background after `LaidOut`**. That is a wake at rest by definition, plus work for merges never shown.
- Rejected: **widen the harness alone and keep the chain** (18's option a as worded). A harness that accepts any answer to any ask can't catch an ask loop that keeps itself going.

**D2. The fold count becomes one read per rest.** `at_rest` asks for **every** uncounted folding merge on screen in one `CountFolded`, listed from the top row down. The job counts them one by one in one job and sends **one** answer with every count. That is one wake and one frame where today there are N. All tails get their numbers in one frame, as the mock shows them (G4, `ease-pass.md:117-118`; ADR-0063). The rest is unchanged:
- the number is absent until it lands;
- `?` on each merge that failed, and the status line says why, once, with the first failure's words;
- never recounted (`fold_count.rs:6-7`);
- a merge that is `Counting` is never asked again;
- a burst asks nothing (`update/mod.rs:260-263`).

Batch size is bounded by the folding merges on screen (≤ screen rows), so no cap is needed. Cost: the first number now shows once the whole batch has been counted, not after the top merge alone. On the 100k fixture that is 3 counts in ~26 ms (`18-lag.txt:30`). Movement never cancels a batch, because counts are facts.

**D3. Rows vs commits under the fold (fix the tests, not the code).**
- Under `Folded`: one row per **shown** commit. A hidden commit has no row; it sits behind its merge's tail (`Whereabouts::Folded { behind }`).
- Once the walk is done: `walked == every commit the walk reaches` and `rows + hidden == walked`, where `hidden = walked − shown` from the view's `Included` (graph MAP `Included::walked / shown`).
- Under `Shown`: `hidden == 0` and `rows == walked`.

The budgets are measured in the mode he runs (the default, `Folded`), **plus one run with every branch shown**. He can pick that mode in `?` (`ease-pass.md:20`), it lays out the most rows, and PROMPT §7.1's 300 MB applies to the app whatever the mode.

**D4. The harness keeps `frames.rs:393` word for word.** Only what `unasked_wakes` treats as asked changes, to D1. If D1 flags a wake other than `FoldedCounted` (for example a branch-pane answer asked from a count answer's settle: `update/mod.rs:263-265` asks both in the same settle), **stop and report `blocked`** with the wake label and the line that asks it. Don't widen the rule.

## Scope
In (crate `mergewright`; `mergewright-graph` only if `HandleSet` has no count, see Signatures):
- the batched `CountFolded` (update, command, job, answer arm);
- `Window.walked` / `Window.hidden`;
- the harness classification;
- `heavy.rs` `check` and the new every-branch test;
- `budget_run` mode parameter;
- `after_idle`;
- MAP.md entries (`jobs::fold_count`, `update::fold_count` if listed, `history` `Window`);
- the script and the perf file.

Out:
- p99 budgets, the idle prefetch, the Pixel byte verdicts and the two PNGs (16's other questions);
- `frames.rs:393`'s assertion text;
- any DESIGN/ADR text (the conductor's; see ADR draft below);
- the branch pane's reads (D4: report, don't change).

## Signatures
```rust
// crates/mergewright/src/update/events.rs. Today: RestRead::CountFolded { merge: Oid } (fold_count.rs:42).
pub(crate) enum RestRead {
    /* other variants unchanged */
    /// Every folding merge on screen with no count yet, top row first. One per rest.
    CountFolded { merges: Vec<Oid> },
}

// The answer. Keep today's per-merge result type; only the cardinality changes.
// Produced::FoldedCounted { counts: Vec<(Oid, <today's per-merge result>)> }

// crates/mergewright/src/update/fold_count.rs: same names, new contract.
/// At rest: one command asking for every uncounted folding merge on screen; marks each `Counting`.
/// Asks nothing when none is uncounted.
pub(super) fn at_rest(next: &mut AppState, commands: &mut Vec<Command>, next_id: &mut dyn FnMut() -> JobId);
pub(crate) fn owed(state: &AppState) -> bool;          // unchanged meaning
pub fn settled(state: &AppState) -> bool;              // unchanged meaning (captures/walks wait on it)

// crates/mergewright/src/history: Window gains two counts (keep existing fields).
pub struct Window {
    /* existing, incl. laid_out */
    /// Commits the walk has read so far, shown or hidden by the fold.
    pub walked: usize,
    /// Of those, the commits the fold hides (walked − shown). 0 when not folding.
    pub hidden: usize,
}
// Filled from the view's `Included::walked()` / `shown()`. Cost bound: no per-row work and no allocation
// per answer. If `HandleSet` has no count, add `HandleSet::len(&self) -> usize` in mergewright-graph
// (one fn, unit-tested). That is the only graph change allowed; say so in the report.

// tests/it/budget_run.rs (the shared runner behind `budgets --gate`)
pub fn run_in(repo: &Path, mode: MergedBranches) -> Result<Numbers, String>; // `run(repo)` = `run_in(repo, MergedBranches::default())`
// Numbers gains `walked: usize, hidden: usize`; `budget_child` prints `walked=` `hidden=`.
```
Harness (test-private; fit names to the code): every job event the paced harness hands carries its job id (extend the seam that already gives `Kind::Window(id)`). Every command the loop sends is recorded with its id and the wake it was sent from: `AskedFrom::{Input, Rest, InputWindow, Answer}`. `unasked_wakes` counts a job wake as asked only when its id was asked from `Input | Rest | InputWindow` and it is that id's `Started` or its first final answer. If that seam can't carry a job id without an app change beyond the existing test hook, stop and report `blocked` with the line.

## Acceptance (tick each with its test)
App, fast tier:
- [x] 3 folding merges on screen, none counted, at rest → exactly one `CountFolded { merges }`, top row first, all three `Counting`.  <- `update::fold_count::tests::one_rest_asks_every_uncounted_folding_merge_on_screen_in_one_read`
- [x] The batch's answer lands with every on-screen merge asked → the settle sends no `CountFolded` (no chain).  <- `update::fold_count::tests::a_count_answer_asks_no_count`
- [x] A second rest with one new merge on screen asks only that merge; a `Counting` one is never asked again.  <- `update::fold_count::tests::a_merge_being_counted_is_not_asked_again`
- [x] Answer with one failure among three → that tail `?`, the others their numbers, one status line with the failure's words.  <- `update::fold_count::tests::one_failed_count_marks_only_that_merge`
- [x] A burst (held `j`) asks no count (today's rule kept; name the existing test if one exists, else write it).  <- `update::fold_count::tests::a_burst_asks_no_count`
- [x] The job counts each merge as git does: fixture with 2 merges; each count equals `git rev-list --count <m>^1..<m>^2` run in the test.  <- `jobs::fold_count::tests::one_read_counts_each_merge_as_git_does`
- [x] Row contract on a small fixture (one merged branch of k commits): `Folded` → `hidden == k`, `laid_out + hidden == walked == git rev-list --count --all`; `Shown` → `hidden == 0`, `laid_out == walked`.  <- `it history_rows::folded_rows_and_hidden_commits_add_up_to_every_commit`

Harness, `cargo nextest run --features heavy-tests -E 'test(unasked)'` (no fixture):
- [x] key → its window → count asked at `Rested` → its answer: 0 unasked.  <- `frames::readings::tests::unasked_a_count_asked_at_rest_is_asked`
- [x] count asked while handling the key's first window: 0.  <- `…::unasked_a_count_asked_by_the_keys_window_is_asked`
- [x] count asked while handling another count's answer: 1.  <- `…::unasked_a_count_asked_by_an_answer_is_counted`
- [x] a second final answer to one ask: 1; an answer to an id never asked: 1; a later `Window` (not the first): 1 (today's rule kept).  <- `…::unasked_repeats_strangers_and_later_windows_are_counted`

Heavy, release, run by script (`scripts/19-fold-rules.sh` → `docs/perf/19-fold-rules.txt`, `--test-threads=1`, these tests only, no other cargo helper running; read SUMMARY lines only):
- [x] `frames::scrolling_the_100k_fixture_…_{pixel,unicode}_tier` and `frames::fifty_thousand_refs_…_at_the_{pixel,unicode}_tier`: `idle_gap_wakes=0` with `frames.rs:393` unchanged, each `woke=[]` line quoted. Their p99 line printed and stated. A p99 over 16 ms is recorded as a finding, never loosened.  <- the four named tests
- [x] `heavy::budgets_hold_*` ×5: `walked == scale.commits()`, `rows + hidden == walked`, `hidden > 0`; then the RSS and first-paint budgets run. State each fixture's rows/hidden.  <- the five named tests
- [x] Every branch shown, 100k without a commit-graph: `hidden == 0`, `rows == scale.commits()`, the same budgets.  <- `heavy::budgets_hold_on_the_100k_fixture_with_every_branch_shown`
- [x] `frames::after_idle::g_after_idle`: waits on `LaidOut`, not a row count. `G` lands on row `laid_out − 1`, and `walked == SCALE.commits()`.  <- `frames::after_idle::g_after_idle`
- [x] Every other `SCALE.commits()` / `scale.commits()` used as a row count in `tests/it/frames*` and `heavy*` is listed with its verdict (row count → fixed to D3; commit count → kept).  <- grep, in NOTES
- [x] Captures: none expected to change (same tails, same words). Re-run the capture/walk tests that wait on `fold_count::settled`; any byte change is listed with why, and no snapshot is accepted blind.  <- `cargo nextest run` (capture families)

## Read first
- This ticket. Then `docs/perf/18-lag.txt:28-35` and `16-fix-heavy-tier.md:24-29` only.
- `docs/design-notes/ease-pass.md:165` and `:226-243` (Hard choice 2) only.
- `crates/mergewright/MAP.md` (`history`, `jobs::fold_count`, `update::branch_pane`), and `crates/mergewright-graph/MAP.md` (`inclusion`: `Included`, `FoldWalk`). MAP before source.
- Source: `update/fold_count.rs`, `update/mod.rs:255-267`, `jobs/fold_count.rs`, `tests/it/frames/readings.rs:239-280`, `frames.rs:373-398`, `heavy.rs:280-302,399-480`, `frames/after_idle.rs`, `tests/it/budget_run.rs`. Scout anything beyond these.

## Touches (check each, say how)
- **Callers of `RestRead::CountFolded` and `Produced::FoldedCounted`:** the job dispatch (`jobs/kinds.rs`), the answer arm (`update/answer.rs`/`answers`), `rest_owed` (`event_loop/running/turn.rs:344-346`), `fold_count::settled` users (captures, walks, `ease_walk`). Grep both names.
- **Tail drawing:** `FoldMark { count }` (ui) is unchanged; `?` and status words are unchanged.
- **Branch pane:** it is asked in the same settle after the fold count (`update/mod.rs:263-265`; MAP `update::branch_pane` "asked at rest after the fold count"). Confirm its ask still goes out on the same rest; D4 if the harness flags it.
- **`Window` literals** across app and tests: grep `Window {`.
- **The `budgets --gate` example** still calls `run` (default mode): same numbers.
- `held_key`'s `fold_changes_held=unread` (`16-heavy.txt:63`): reads no count; confirm.
- **No other changes:** search's `· N in folded merges`, keymap, journal, undo, other crates (except the optional `HandleSet::len`), PNGs.

## Gates
Workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo clippy --all-targets --features heavy-tests -- -D warnings`, `cargo nextest run`, `cargo test --doc`. Heavy: only the script above. Say which heavy tests were not run.

## Size
~55k tokens: one command shape plus 5 unit tests, two `Window` counts plus 1 fixture test, the harness classification plus 4 tests, three heavy assertions plus 1 new test, the script.

## ADR (filed as docs/adr/0065-at-rest-only-what-an-input-asked-may-wake-the-app.md)
**Title: At rest, only what an input asked may wake the app; under the fold, rows are shown commits.**

Context:
- 18 cause 2: fold counts chained one wake per merge after `Rested`.
- 16: heavy tests assumed one row per commit, but the fold is the default.

Decision:
- D1 (an input's reads are asked by its update, its `Rested` or its first window, and are answered once each; no chains, timers, polls or background reads).
- D2 (one batched fold count per rest).
- D3 (rows + hidden == walked; budgets in both modes).

Consequences:
- Every future background read must be one of D1's three. `frames.rs:393` enforces it through `unasked_wakes`.
- A batch shows its first number later than a chain did, but all at once.
- The design note's `:165` reads "asked at rest, in one read per rest".

**Conductor:** my search tools were down this pass, so I couldn't grep `docs/adr/` for an existing rest or idle-wake ADR (m7-power-tools/12 introduced `idle_gap_wakes`). Check before numbering, and amend that ADR if one exists.

## NOTES (builder-sonnet)
No compaction. Scout: none (every file was named in the ticket and read directly; no wide sweep needed).

**Boxes.** App 7/7 and harness 4/4 pass in the fast tier (`cargo nextest run`: 7225/7225, plus the 4 `unasked` + 2 `idle_gaps`/`batched_key` tests under `--features heavy-tests`). Heavy 6/6 by `scripts/19-fold-rules.sh` -> `docs/perf/19-fold-rules.txt` (release, `--test-threads=1`; other builders were running, load 3.5-7):
- 4 frames tests: `idle_gap_wakes=0`, `woke=[]`, `pauses_after_laid_out=5 of 5`, `frames.rs:393` unchanged. p99: 100k pixel 14.7 ms (4 keys over 16 ms, max 35 ms: recorded), 50k pixel 11.7 ms, 50k unicode 12.2 ms; 100k unicode 2.5 / 3.6 ms in two reruns. **The first run of the 100k unicode test failed a different check**: `verdict.rs:62` "key 262 (PageDown): 0 of 34 rows on hand" (the layout-behind-keys timing check, `frames.rs:28-29`), not the wake rule, with its `idle_gap_wakes=0 woke=[]` line printed. Not reproduced in two reruns. Unverified cause (load); not loosened.
- 5 `budgets_hold_*`: `walked=100000`, `rows+hidden==walked`, `hidden>0` on all five: rows=79195 hidden=20805 (100k, 100k realistic, 50k refs; each with/without commit-graph as named). Peak RSS 34-158 MB, first paint 17-138 ms (50k refs 138 ms, budget 150).
- Every branch shown (100k, no commit-graph): rows=100000 walked=100000 hidden=0, RSS 101 MB, first paint 17 ms.
- `g_after_idle`: waits on `LaidOut`; `rows=79195 walked=100000 laid_out=79195`, `idle_gap_wakes=0`.
- Not run: the other heavy tests (`held_key`, `ghostty_shot`, `pieces_shot`, `resolver::frame_budget`, 1M, the slope guards, `fifty_thousand_refs_*_{detail_pane,diff,map_view}`).
- Captures: full nextest passes, no file under `docs/shots/` changed, no snapshot accepted.

**Idle wakes before/after.** Before (18's run, `docs/perf/18-lag.txt:28-35`, not re-measured): 5 x `Produced::FoldedCounted` per closed test, from `update/fold_count.rs:27-43` (one ask per answer). After: 0.

**Causes, not symptoms.** D1/D2 fix the chain at its cause (`update/fold_count.rs::at_rest`, one `CountFolded { merges }`, `Product::FoldedCounted { counts }`); the harness (`tests/it/frames/unasked.rs`, `watching.rs`) now says what D1 allows instead of accepting any answer. No D4 stop: the branch pane's ask (`update/mod.rs:263-265`) was never flagged. Cost to the user: the first fold number shows once the whole batch is counted.

**Grep of `.commits()` as a row count (verdicts).** `heavy.rs:104` commit slope: commit count, kept. `heavy.rs:177` (`the_table_and_the_order_...`), `:259` (`cold_jump`), `:275` (`order_bytes`): these lay out `View::default()` (not folded), rows == commits holds: kept, as the ticket said. `heavy/check.rs` and `after_idle.rs:72`: `walked`, a commit count: kept. The row uses in `check` and `after_idle` (`rows == commits`) were the two fixed. No other use in `tests/it/frames*` or `heavy*`.

**Deviations / things to know.**
- Test-side name change: `a_count_answered_mid_burst_asks_the_next_at_rest` (`event_loop/tests/settle/fold.rs`) described the chain D1 removes; rewritten to assert the key's settle asks the whole batch and the answer asks none. `every_folding_merge_on_screen_is_counted_one_job_each` and `no_count_is_asked_mid_burst` (`update/fold_count/tests.rs`) replaced by the ticket's named tests.
- `AppState` was exactly 8192 bytes; `Window` +16 B tipped it over, and clippy `large_stack_arrays` failed five tests that build `[state, state]`. Fixed at the size, not the lint: `History::folded_ask` is now `Option<Box<FoldedAsk>>` (rare, like `Sought`).
- `Window` now derives `Default`; six test files build windows with `..Default::default()` instead of repeating four fixed fields, so no `over_400` row rose. `over_400.txt`: rows updated down (settle.rs, history/tests.rs, amend_into, preview, rewrite_editor, strips/tests, view/tests.rs, budget_run.rs, paced.rs, held_key.rs, events.rs, heavy.rs), `frames/readings.rs` removed (now 316). Splits: `budget_run/child.rs`, `heavy/check.rs`, `frames/{unasked,watching}.rs`, `frames/readings/tests.rs`, `view/tests/strip_state.rs`. No graph-crate change (`HandleSet::len` existed).
- `Window.walked/hidden` are filled in `history::serve::walk_counts` (two counter reads); the idle `refreshed` windows carry them too.
- Harness limit: `command_id` lists the id-carrying reads that can wake at rest; any other command's wakes count as unasked (honest direction), so a new background read must be added there.
- Design note `ease-pass.md:165` still says "asked at rest" (conductor's text per ADR-0065).

**Touches:** `RestRead::CountFolded` / `Product::FoldedCounted` callers (`jobs/runner/{dispatch,lists}.rs`, `update/answer.rs`, `event_loop/label.rs` by grep and compile; `update/facts.rs` matches `{ .. }`); `rest_owed` (`event_loop/running/turn.rs`): `owed()` unchanged meaning, `a_count_answered_mid_burst_asks_no_further_count` and the rest-owed tests pass; `fold_count::settled` users (captures, ease_walk, fold_by_default): full nextest, no shot changed; branch pane ask: `branch_pane/tests.rs:191` position test passes, the harness never flagged it; `Window {` literals: grep, 40 sites compile and pass; `budgets --gate` example: still `run` (default mode), prints the two new fields; `held_key` `fold_changes_held=unread`: reads no count (`held_key` not re-run, no code path of it changed); search's folded count, keymap, journal, undo, PNGs: no file touched.
