# m1-the-graph/27b — git/git without a commit-graph misses the 300 MB budget

Status: done — profile + fix B here; remainder completed in 27c, 27d
Build with: **Opus** (high) — split from 27's remainder; same scope and model
Blocked by: 27
Leaves runnable: yes — git/git stays under 300 MB peak RSS with and without a commit-graph.

Why: 27 measured git/git (85,649 commits, release) at **300.5–305.9 MiB without a commit-graph** (over PROMPT §1.2's 300 MB) and 254–256 MiB with one (too close). Nearly all of it arrives on `G` (+278 MiB no cg, +175 MiB cg). The 100k synthetic fixture passes (158/179 MiB) because its commits are small.

## Read first
- Ticket 27: "Results", "Decisions", "Handoff (what remains)" and "Suspects" — **the suspects are unmeasured**
- PROMPT.md §1.2 (the two budget bullets), §6.3 "Performance budgets"
- DESIGN.md §3.6 (budget and memory paragraphs only); ADR-0008, ADR-0015, ADR-0018
- `MAP.md` of `graft-git`, `graft-graph`, `graft`
- Scouts (Explore, haiku/sonnet) for source beyond a few lines.

## Scope
In:
1. **Measure first:** heap profile (or allocation counting) of `examples/budgets` on a no-cg copy of git/git (`cp -R` the clone from `scripts/clone-git-git.sh --where` into `$CLAUDE_JOB_DIR/tmp` or `target/`, delete `objects/info/commit-graph`). Name what holds the memory with numbers before changing anything. Profile output goes to files; summarise, don't dump into context.
2. **Fix the cause** inside M1 so git/git peaks under 300 MB **with a real margin** both ways; record before/after per suspect confirmed.
3. If possible, make the heavy tier catch this class of regression (e.g. a synthetic fixture option with realistic message/identity sizes), without generating large fixtures outside the heavy tier. If that is its own ticket's worth of work, say so and stop at that seam.
4. Update 27's results table rows for git/git and the KNOWN_GAPS rows 27 touched.

Out:
- Compact `EmittedOrder` index, checkpoint thinning, background advance, 1M claim (→ 30).
- Changing behaviour to meet the budget (e.g. no full `G`, dropping promotion): **stop and ask** through the conductor.

## Signatures
Expose: none public. Keep any seam `pub(crate)`. If a public item changes, update that crate's MAP.md.

## Acceptance
- [ ] Profile finding written in this ticket: top holders with sizes, which of 27's suspects were real.
- [ ] git/git release, 3 runs each, cg and no cg: peak RSS and first paint, before/after table. Both under 300 MB.
- [ ] 100k heavy-tier budgets still green (run once, alone, at the end).
- [ ] No behaviour change without the user's OK.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`; heavy tier once, alone, at the end. Gotcha from 27: zsh doesn't word-split `$VAR` commands; spell cargo commands out.

## Size
One context, under ~80k tokens. Seam if it grows: (a) profile + finding + first fix, (b) remaining fixes + heavy-tier guard. Stop at the seam with a handoff note rather than compacting.

## Profile finding (2026-09-13, builder-opus)

Tool: `prototypes/heap-profile-27b` (outside the workspace, no deps): `budget_run.rs` plus a counting global allocator (live and peak heap), a sampler writing `ms heap MiB rss MiB` to a file, and an optional self-`SIGSTOP` (a live-heap MiB threshold as arg 2, or `STOP_AT_TOP=1`) so `vmmap`, `heap` and `malloc_history` can run on the paused process. Build it with its own `Cargo.lock` copied from the root, `cargo build --release --offline`. Raw outputs were in `target/27b/`, which is not committed.

Headline: **RSS is not heap.** git/git without a commit-graph, release, before any change:

| Measure | Value |
|---|---|
| peak RSS (`getrusage`) | 298–301 MiB |
| peak memory footprint (`time -l`) | 211–213 MiB |
| peak live heap (counting allocator) | 145–146 MiB |
| live heap after `G` lands | 78 MiB |
| RSS at paint, heap 5 MiB | 24 MiB (fixed cost ≈ 19 MiB) |

The RSS-minus-heap gap grows during `G`, from 78 MiB at 211 ms to about 150 MiB at the peak (1.3 s). The difference is pages macOS malloc has freed but still keeps resident ("reusable": out of footprint, still counted by `ru_maxrss`), plus clean mapped pages (the 36 MiB pack). **Churn volume drives the gated number as much as live data does.**

Live heap at the peak (`MallocStackLogging=lite` + `malloc_history -callTree`, process stopped at 145 MiB live; cg run stopped at 119 MiB):

| Holder | no cg | cg | Suspect |
|---|---|---|---|
| Promotion `topological_order` → `Topological` maps and heaps | 47.8 MiB | — | 1, 2 (real) |
| Stream walk's `Topological` (cg orders topologically at once) | — | 44.4 MiB | 1 (real) |
| of which `known` table 15.1, `Node.parents` Vec growth 12.9 (84k allocs), `indegree` 7.6, `expanded` / `emitted` 4.3 each | | | 1 |
| gix object cache (`MemoryCappedHashmap::put`), per handle | 19.6 (stream) + 16.5 (promotion, via `hydrate`) | 20.8 | 5 (real, ×2 without cg) |
| `LayoutCursor::extend` rows in `RowCache`; `Row.lanes` alone | 55.8; lanes 34.8 MiB in 7,806 allocs ≈ 4.5 KB/row | 53.7; lanes 34.8 | new: `lanes` sized to `active`'s widest-ever length, trailing `None`s trimmed but capacity kept |
| `EmittedOrder::note` | 10.3 | 8.2 | 6 (as designed) |
| `Included` walked-set | 4.3 | 4.3 | 6 |

Suspect 3 (`promote` doubling) is not visible at the heap peak, which comes before promotion lands. Suspect 4 (`hydrate` churn) is real as churn: it isn't live at the peak, but it feeds the RSS gap. Not yet attributed per call site, because full MallocStackLogging would not start (below).

### Tried: fix A, collect `Row.lanes` only up to the last drawn column (reverted)

Rows came out identical. Measured 3–5 runs, no cg:

| | peak RSS | footprint | heap peak | heap after `G` |
|---|---|---|---|---|
| before | 299–301 MiB | 211–213 | 146 | 78 |
| fix A | **311–314 MiB** | 208–209 | 138 | 53 |

It cut live heap and footprint but **raised** the gated RSS by about 12 MiB, repeatably. Likely cause: fixed 4.5 KB lane blocks in malloc's small region used to recycle cleanly, and variable tiny/small sizes fragment worse. Unconfirmed. Reverted: a change that worsens the gate isn't landing without understanding.

## Handoff (what remains)

Next step: **attribute churn, not live heap.**
1. Get full stack logging to start: `MallocStackLogging=1` came up in lite mode, so `malloc_history -highWaterMark` refused. Try `MallocStackLogging=malloc` or `MallocStackLoggingNoCompact=1`, or `xctrace record --template Allocations` with output to a file. Look for total bytes allocated per call site during `G`, and for what was live at the footprint high-water mark.
2. Candidate fixes, all without behaviour change, each measured on RSS **and** footprint, 3 runs:
   - (B) `Topological`: fold `indegree` / `expanded` / `emitted` into one per-commit state beside `Node` in `known`, give `parents` exact capacity, and drop entries once emitted and their parents are released. Est. −20–30 MiB live, cg and no cg.
   - (C) Promotion walk emits ids only, with no `hydrate`: it keeps just `commit.id` today. Removes 85k full-`Commit` decodes of churn, and its handle's cache fill. Needs a `graft-git` public item (MAP.md).
   - (D) Smaller object cache for the promotion handle (`SharedRepo::handle` has no size knob today; public item).
   - (A′) Retry fix A only alongside the others; judge on RSS.
3. Whether the gate should be `ru_maxrss` at all on macOS (it counts freed-reusable pages; footprint is 90 MiB lower) is a **spec question** (PROMPT §7.1 names `getrusage`). Raise it through the conductor. Don't change the measure silently.

Gotchas: one heavy job at a time; `prototypes/` profile builds need `--offline` (no network here); a threshold arg of `0` means "never stop".

## Continuation 1 (2026-09-13, builder-opus)

**Churn attribution.** `MallocStackLogging=malloc` (not `1`) gives full mode; `malloc_history <pid> -highWaterMark -callTree -invert -consolidateAllBySymbol` works. No cg, high water 149 MiB, **cumulative allocated 1.77 GB** (5.2M allocs). High-water holders match the table above: `Row.lanes` 36.8M; the promotion `hydrate` gix cache; stream `Dated` cache 14.6M; `Topological::discover`. Per-site churn totals were not extracted (`-allEvents` too large). Raw: `target/27b/prof2/hwm.txt`.

**Measurements** (release, 3 runs, `examples/budgets`; RSS = `getrusage` peak, footprint = `time -l`; script `target/27b/meas.sh`, raw `target/27b/meas/summary.txt`):

| | no cg RSS | no cg footprint | no cg G | cg RSS | cg footprint | cg G |
|---|---|---|---|---|---|---|
| before | 303.7–306.7 | 202.8–205.7 | 1.26 s | 255.3–255.8 | 159.0–162.6 | 0.54 s |
| **B landed** | **290.3–294.4** | **173.1–174.2** | 1.25 s | **222.6–225.7** | **129.7–131.9** | 0.53–0.57 s |

**Fix B (landed).** `Topological` keeps one `Entry` per commit (parents, time, generation, `u32` in-degree, expanded/emitted flags) instead of `known` + `indegree` + `expanded` + `emitted`; parents are taken, not cloned, on expand and freed on emit; `Node.parents` starts at capacity 1. Same emitted order (graft-git tests, incl. git-oracle tests). No cg is **still under 300 with only a ~6–10 MiB margin: not done.**

**Fix C (blocked, call site reverted).** `Walk::next_ids` / `IdChunk` added and tested (ids equal `next_chunk` ids across chunks; stop honoured); `SharedRepo::handle_with_object_cache` added (unused yet). MAP.md updated. Pointing `graft::history::topological_order` at `next_ids` **hangs the no-cg budget run**: 1 of 3 runs died, the next waited out PATIENCE (300 s). Stuck state: promoted window landed (`promoted: true`, `asked 3 == request 3`), `jump: None`, `promotion: None`, `cursor 0`, `laid_out 500`, `done false`. So the `G` press (`update.rs:926`, `Motion::Bottom`) either asked for nothing or had its request superseded by the promotion swap (`scroll.rs:103-157`, `promoted()` clears `jump` unconditionally at :131), and nothing re-asks. The faster promotion lands around the `G` press; the race exists before C but was unlikely. A scout also found `promoted()` clears `jump` but never `seeking` (update.rs `seek_landed` :837 then waits for ever on a search jump). **Fixing this changes app state logic: asked through the conductor, not done.**

### Next steps
1. User/conductor: OK to fix the promotion-vs-jump race in `graft::history::scroll` (`promoted()` should re-issue or preserve an in-flight jump/seek)? Then add a unit test for "`G` pressed while promotion Swapping reaches the bottom".
2. Re-apply C's call site (`order.extend(chunk.ids)` via `walk.next_ids`), measure 3×.
3. Wire D: promotion `store.shared()?.handle_with_object_cache(0)`; measure 3× (watch G time).
4. A′ only if still short of margin; judge on RSS.
5. Heavy-tier regression guard (scope item 3) and 27's results table / KNOWN_GAPS rows still open.

## NOTES
- No compaction. Stopped at the size seam (about 70k tokens): profile and finding done, no fix landed.
- Continuation 1: no compaction. Stopped at the race (needs the user), ~75k tokens.
