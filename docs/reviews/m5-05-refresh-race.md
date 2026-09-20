# Review: M5 group 05 — refresh race

Reviewed at `722186a` (merge of `6c8f7c1`), read-only. Main was at `950ab1d` when the
checks ran; `git log 722186a..950ab1d -- crates` is empty, so the source under test is
the fixed hash's. Ticket: `.scratch/m5-history-and-ghost-preview/issues/05-refresh-race.md`.

**Counts: 1 Critical, 4 Major, 3 Minor.**

## What was checked and how

- Read: CLAUDE.md, the ticket (Scope, Acceptance, NOTES), design note G3 block
  (`docs/design-notes/m5-history-and-ghost-preview.md:48-50`), ADR-0037 §6, KNOWN_GAPS rows
  216, 217, 491, 492, the `m3-06` review §M5 (row 216's origin), `crates/graft/MAP.md`
  §update/§jobs, and the whole group diff (`git diff 7e83aa4..722186a`, 20 files). Source
  files opened only to confirm a finding: `jobs/runner.rs` (`queue_as`, `cancel`,
  `row_requests`, the `ShowRows` arm), `jobs/pool.rs` (`run_job`), `update/history.rs`
  (`walk_progress`), `update/mod.rs` (`awaiting` consumers), `update/journal.rs` (`read`,
  `J` open, `close`, `request`), `update/keys.rs` (`graph_key`), `tests/it/app_harness.rs`
  (`press`, `pump`, `PATIENCE`).
- Ran, one at a time: `cargo fmt --check` (clean); `cargo clippy --all-targets -- -D warnings`
  (clean); `cargo clippy --all-targets --features heavy-tests -- -D warnings` (clean);
  `cargo nextest run` — **2006 passed, 31 skipped, 43.3 s** (wall 44.8 s, peak RSS 272 MB);
  `cargo test --doc` — 0 tests (KNOWN_GAPS "M5 gates": the gate credits nothing);
  `cargo test --features heavy-tests -- --test-threads=2` — **all green, 202.9 s**, peak RSS
  702 MB, the graft `it` binary alone 119.4 s; release perf run — **13 passed, 73.0 s**.
- No fuzz targets, no benches exist in the workspace. `target/` is 10 G.

Five slowest fast-tier tests: `graft-git::it diff_fixture::parse_unified_matches_numstat_for_every_shape`
9.15 s; `graft-graph layout::stream::tests::scrolling_the_whole_history_keeps_the_resident_set_bounded`
6.89 s; `graft-fixtures::it corpus::two_builds_produce_the_same_object_ids` 3.50 s;
`graft-git diff::patch::tests::whole_hunk_reproduces_gits_own_bytes_on_every_corpus_shape`
1.92 s; `graft::it m4_walk::every_scenario::u_never_touches_the_remote` 1.61 s. The new
tests: the three pure ones 6 ms each; the four `refresh_race` ones 65–88 ms; `journal_rows`
0.98 s with its workaround removed.

Release perf numbers (`--release`, `--test-threads=1`):

| run | p50 µs | p99 µs | max µs |
|---|---|---|---|
| frames Pixel / 50k refs, pane closed, 420 keys | 5355 | 8684 | 8974 |
| frames Unicode / 50k refs | 4724 | 8581 | 11733 |
| frames Pixel / fixture | 5404 | 8015 | 8404 |
| frames Unicode / fixture | 4821 | 8299 | 9438 |

| `heavy::budgets_hold` fixture | first paint µs | to bottom ms | peak RSS MB |
|---|---|---|---|
| 100k, commit-graph | 13574 | 48 | 30.7 |
| 100k realistic, commit-graph | 13687 | 50 | 35.2 |
| 100k, no commit-graph (provisional) | 15584 | 475 | 94.7 |
| 100k realistic, no commit-graph | 15617 | 518 | 102.0 |
| 100k + 50k refs, commit-graph | 37580 | 52 | 100.1 |

All inside the 16 ms frame and first-paint budgets. Nothing in this group touches the
frame path, so these are a regression check, not evidence about the group.

## Acceptance ticks: 4 ticks, 11 named tests/claims checked, 10 held

Each acceptance tick was checked against what the requirement asked for, not what the code
does. Every named test exists and is green; the question was whether each proves its claim.

| # | claim | verifying test named | held? |
|---|---|---|---|
| 1a | two `Ctrl+R` with no event read between leave one walk; both refreshes' refs + working-tree reads return on a 2-worker pool | `it::refresh_race::two_refreshes_before_started_leave_one_history_job` | **yes** — `App::press` (app_harness.rs:113) dispatches without reading; under the old code the queue order is `w1, ReadRefs1, ReadRemotes1, ReadWorkingTree1, w2, ReadRefs2…` and with `w1`, `w2` both parked the second pair never gets a worker; the loop then dies at the 120 s bound. Failure by starvation is real, not incidental |
| 1b | the replaced walk is told to stop and the pool reports it; status line unchanged | `it::refresh_race::refresh_cancels_the_walk_it_replaces` | **yes** — waits for `JobCancelled(replaced)` from the pool; note it exercises the *queued* cancel (`pool.rs:173`), not the parked one |
| 1c-pure | a `Started` from the replaced walk does not reach the screen | `update::tests::stale_started_is_dropped` | **yes** — stale `Started` fed *after* 40 live rows; under the old "newest wins" `history_started` the rows are replaced. Ordering pinned by construction |
| 1c-it | same, "over the real pool, where the two `Started` events genuinely race" | `it::refresh_race::a_superseded_walks_started_never_reaches_the_screen` | **no** — see C1. Passes with the (c) bug present |
| 1d | `Esc` on a launch read neither cancels it nor makes anything re-request | `update::journal::tests::esc_on_a_pending_journal_read_does_not_rerequest` | **yes** — asserts `on_escape.is_empty()` and that `request` (the only `settle` caller, `update/mod.rs:136`) issues nothing. Placed in unit tests, not `tests/it/` as the ticket wrote (m2) |
| 1e | no `sleep`, no poll loop in the new tests | read `refresh_race.rs` whole | **yes** — every loop blocks on `recv_timeout(PATIENCE)` and exits on an event |
| 1f | the race as a pure `update` sequence | `update::tests::a_refresh_before_the_previous_walk_started_still_cancels_it` | **yes** — asserts `after_first.awaiting == Some(first)` and `CancelJob(first)` in the second refresh's commands; red under the old code (`awaiting` was `None`) |
| 1g | cancelling a parked walk gives its worker back | `it::refresh_race::cancelling_a_parked_walk_gives_its_worker_back` | **yes, as stated** — the NOTES say plainly it passed on `main` too and guards the relocation of the sender-drop into `JobRunner::cancel`; 1-worker pool, exhaustion-pinned |
| 2 | `journal_rows` passes with the `WaitFor(Ready)` workaround removed; all 25 `m4_walk` green | `it::m4_walk::every_scenario::journal_rows` | **yes** — diff removes the step (hangs_and_undo.rs:253-258); 25 `m4_walk` tests passed in the run |
| 3 | every walk `Started` handler asserts `id == awaiting` | `history_started` (mod.rs:373-375), `walk_progress` (history.rs:19) | **yes** — both gate; `ReadHistory` is constructed only at keys.rs:294 and mod.rs:84, both set `awaiting` first |
| 4 | NOTES: cause/symptom, scout, row-216 site list | ticket NOTES | **yes** — present; but the site list answers a different question from row 216 (M1) |

## Critical

### C1. `a_superseded_walks_started_never_reaches_the_screen` cannot fail on the claim it makes

`crates/graft/tests/it/refresh_race.rs:130-176`. The doc comment says it is "the end-to-end
half of `update::tests::stale_started_is_dropped`, over the real pool, where the two
`Started` events genuinely race." They do not race. `JobRunner::queue_as`
(`jobs/runner.rs:344`) delivers `AppEvent::Started` **synchronously on the dispatching
thread**, before it even queues the assignment, and the events channel is FIFO. So the
sequence on the channel is always `Started(replaced)`, `Started(live)`, …, `Window(live)`:
the superseded walk's announcement can never arrive after the live walk's rows.

Consequence: put the (c) bug back — revert `history_started` to "newest wins"
(`update/mod.rs:373-375`) and keep the id minting — and this test still passes:
`Started(replaced)` sets `awaiting = replaced` and an empty `History`, `Started(live)` then
sets `awaiting = live` and a fresh `History`, the live rows land, the loop breaks, and all
three assertions hold. I traced it against the old handler by hand; I did not run the
reverted code.

Why it matters: it is offered at tick time as the real-pool proof of (c). A test that
passes with the bug present is not evidence, and its doc comment claims an interleaving the
runner makes impossible — the next reader will trust it. This is the shape the conductor
asked to have called Critical, and it is: the tick's *requirement* is carried by the pure
test alone.

Fix, either: (1) delete the integration test and reword the tick to name only
`stale_started_is_dropped`, which does prove the claim; or (2) make it pin the ordering it
describes — in the loop, when `Started(replaced)` is read, do **not** feed it to `update`;
hold it; after `rows_arrived(&state)` feed the held event through `update` and assert the
rows survive. That is pinned by construction over the real pool and would go red under
"newest wins". Also correct the comment: the two `Started`s are ordered by dispatch, not by
the scheduler.

## Major

### M1. Row 216 and this fix are two different defects; the ticket and the conductor's framing conflate them

KNOWN_GAPS row 216 (`KNOWN_GAPS.md:216`) is the **`LocalJobs` copy-out/write-back rewind**:
per the `m3-06` review §M5, "a caller copies `local_jobs` out, hands a closure over the copy,
and writes the copy back — so any callee touching `state.local_jobs` is rewound", at 14
listed sites. This group fixed **runner-minted ids learned late from `Started`** — a
different pattern, now recorded as KNOWN_GAPS row 491 with its three siblings. The ticket's
`Build with` line and the review brief both say this is "the pattern row 216 wants
everywhere"; it is not. The group neither applied nor regressed row 216: `opening()`
(`update/mod.rs:76`) mints straight from `state.local_jobs` (good), while `graph_key`
(`update/keys.rs:164-167`) still does the copy-out/write-back around `refresh_key`.

Why it matters: an M7 housekeeping row scoped by "row 216" would refactor the wrong 13
sites and leave rows 491's three where they are, or vice versa.

Fix: no code. In KNOWN_GAPS, keep 216 and 491 as separate rows with one line each saying
what the pattern *is*; correct the ticket's `Build with` note so the log does not carry the
conflation forward.

### M2. The fix is per job kind, not general — and its nearest sibling sits three lines away

Judgement the conductor asked for. The **runner half** is general: `JobRunner::cancel`
(`jobs/runner.rs:379-382`) now sets the token and drops the row-request sender for any
caller, and `pool::run_job` drops a cancelled queued job before it runs. The **update half**
is applied to `Command::ReadHistory` only. `refresh_key` cancels `awaiting` and `promoting`
back to back (`update/keys.rs:286-291`); `promoting` is still learned from `Started` in
`sort_started` (`update/history.rs:39`) and has the identical window. It is less severe — a
sort runs to completion instead of parking, and `sort_started` cancels the older id when the
newer lands — but between the two `Started`s the pool holds two sorts, and `Ctrl+R` twice is
exactly the input that produces it. The ticket put this out of scope, so it is not a builder
defect; row 491 records it for M7.

Fix: pull `PromoteHistory` forward — `Command::PromoteHistory(JobId)` via `queue_as`,
`promoting = Some(id)` at dispatch in `walk_progress`, `sort_started` gated on
`promoting == Some(id)`. About the size of this group's `ReadHistory` change. The
`CommitGraphOffer` and `Search` siblings can stay in M7.

### M3. The design note's Interfaces block now names a signature that does not exist

`docs/design-notes/m5-history-and-ghost-preview.md:48-50` (G3 block) still reads
`Command::OpenRepository(JobId, RepoOpen)`; the code took `Command::ReadHistory(JobId, Order)`
and left `OpenRepository` a unit variant. The builder's NOTES explain the deviation well
(`OpenRepository` is a never-cancelled facts job; the walk is `ReadHistory`) and the reasoning
is right — but the note was not updated, so the milestone's approved design and the code
disagree on the one interface this group exists to change. Group 08 reads that block.

Fix: one-line edit to the G3 block naming `ReadHistory(JobId, Order)`, with
"(deviation from the ticket, see 05 NOTES)".

### M4. The same eight-line incident narrative appears five times

`update/keys.rs:278-285`, `update/events.rs:321-329`, `update/mod.rs:365-372`
(`history_started`), `update/mod.rs:55-62` (`opening`), `update/tests.rs:40-45`
(`walk_started`), and `MAP.md` `AppState::awaiting`: each re-tells "learning the id late left a
window in which a second `Ctrl+R` had no name to cancel, so the walk stayed parked on a
worker and two of those filled the pool". PROMPT.md 6.2: a doc comment says what the thing
is; the incident belongs in one place. A reader of `refresh_key` now scrolls past a paragraph
to find the two lines that changed.

Fix: keep the full account once, on `Command::ReadHistory` in `events.rs` (it is the
interface), and reduce the others to one line each: "named at dispatch so a refresh can cancel
it — see `Command::ReadHistory`."

## Minor

### m1. `J`, `Esc`, `J` during the launch read now leaves two journal reads in flight

`update/journal.rs:149-151`: opening the browser calls `JournalState::read` unconditionally,
which mints a fresh id and sets `reading` to it. Before this group, `Esc` cancelled the
pending read and reset the field, so a re-open started clean. Now `close`
(`journal.rs:211-218`) deliberately leaves the launch read running, so the re-open queues a
second read of the same file without cancelling the first. `journal_event` drops the older
answer by id (`journal.rs:100`), so it is churn, not loss — the row-217 shape moved one key
over, at a lower rate. Fix: in the `J` arm, if `reading.is_some()` keep it rather than
minting; or have `read` cancel the id it replaces.

### m2. Test (d) is in the unit tier, not the `tests/it/` real-pool tier the ticket specified

Ticket Scope item 4 lists (d) under "Tests (headless app harness, real pool, `tests/it/`)".
`esc_on_a_pending_journal_read_does_not_rerequest` is a pure `update::journal` test that
calls `press` and `request` directly. It proves the claim more tightly than a pool test
would, so the tick holds — but the tick should say the tier changed and why. No code change.

### m3. Regression of test (a) is reported only after the full 120 s `PATIENCE`

`refresh_race.rs:63-68`: the pool's verdict is the right one, but under the bug the loop
simply times out. A cheap early signal exists: `JobCancelled(replaced)` must precede the
second refresh's `Refs` product. Asserting that in the same loop turns a 120 s hang into a
sub-second named failure without weakening the starvation check. Optional.

## Fence

Group 07 was fenced to `graft-ops` + `graft-core` (its ticket line 6, NOTES lines 385/518/540:
"the fence kept me out of `graft`"). This group's 20 changed files are all under
`crates/graft/`; `git log 7e83aa4..950ab1d -- crates/graft` shows only `6c8f7c1`. No
crossing either way. The group did touch files outside its own ticket's `Size` list
(`event_loop/mod.rs`, `update/events.rs`, `update/operate.rs` tests, `jobs/tests.rs`, six
files under `tests/it/`), all forced by the `ReadHistory` signature change and the
`opening()` consolidation; within the crate it held.

## Things checked that were fine

- No `unwrap`/`expect` added outside tests; `row_requests()` recovers a poisoned lock with a
  stated reason. No swallowed `Result`. `drop(self.row_requests().remove(&id))` is explicit.
- No lock-order hazard from moving the sender-drop into `cancel`: `live.cancel` does not touch
  `row_requests`; the `ShowRows` arm holds the guard but never calls `cancel`; `shutdown`
  releases its guard before cancelling.
- `read_history` inserts the sender after `queue_as` on the dispatching thread; no
  `CancelJob` for that id can be dispatched before the insert.
- All four `Refresh::Everything` routes reach `refresh_key` (`update/operate.rs:851`), so
  operation-triggered refreshes mint and cancel the same way `Ctrl+R` does.
- The `walk_started` helper makes the 67 pre-existing tests name the walk first, which is the
  new contract; it is not hiding a behaviour change.
- The `opening()` consolidation removed the last four hand-rolled opening lists; the only
  remaining direct `CheckSystemGit` dispatch is a runner unit test.
- Row 217: the "wanted flag" the ticket asked for would, as the NOTES argue, have starved
  `interrupted_notice` for the session; leaving the launch read alone is the right cause fix.
- Row 492 (`row_requests` leak on a walk ending by itself) is recorded.
- No UI-thread git work added; no terminal output touched; nothing visual.
