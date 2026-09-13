# m1-the-graph/27 — Peak memory and first paint, measured for real

Status: done — 100k budgets here; remainder completed in 27b, 27c, 27d
Build with: **Opus** (high) — measuring a concurrent streaming pipeline and deciding what to fix
Blocked by: 07, 09, 18, 31
Leaves runnable: yes — heavy tier asserts the 300 MB and 150 ms budgets.

Why: M1/06 built checkpoints, eviction and promotion but measured only an in-memory 100k history in a unit test. The real 100k repository, `getrusage`, the 300 MB gate and wall-clock first paint were deferred here (06's notes; KNOWN_GAPS "Nothing has been measured on the 100k fixture"). This is the retired ticket 16's core, text in 35bec3e.

## Read first
- PROMPT.md §1.2 (the two budget bullets), §6.3 "Performance budgets", §7.1 "Heavy tier" bullets
- DESIGN.md §3.6 (the budget and memory paragraphs only)
- ADR-0008 (decision and consequences), ADR-0015, ADR-0018
- KNOWN_GAPS.md rows: "No budget in the spec has ever been measured", "The heavy tier is a harness", "The emitted order grows with history", "Nothing has been measured on the 100k fixture", "Least-recently-used is the wrong policy"
- TESTING.md "The heavy tier" and "Getting `git/git`"
- Code: `MAP.md` of `graft-fixtures`, `graft-graph`, `graft`

## Scope
In:
- Heavy-tier assertions on the 100k fixture (`Scale::HEAVY`), **with and without the commit-graph file** (ADR-0008: two paths, two numbers):
  - peak RSS < 300 MB over open → first paint → `G` to the bottom → back to top, via `graft_fixtures::memory::measure`.
  - first paint ≤ 150 ms: process start to the first frame with rows.
- **git/git (85,649 commits), by hand:** no network in tests, so run the same two measurements once against the clone from `scripts/clone-git-git.sh` with an exact command, record the numbers in the ticket. Say which commit-graph state the clone had.
- Decide the two deferred questions from the numbers: does `EmittedOrder` need the compact index; does checkpoint LRU need thinning (measure a jump back into deep history on git/git). Fix inside M1 if the budget is missed; otherwise record the measurement.

Out:
- Input-to-frame, 60 fps scroll, criterion benches, 50k refs, the 1M claim (30).
- Search timing (26). New features: if a budget can only be met by changing behaviour, stop and ask.

## Signatures
Call (existing):
- `graft_fixtures::memory::measure<T>(impl FnOnce() -> T) -> Result<(T, Measured), MemoryError>`; `Measured { before, peak }`, `Measured::rise()`; `peak_rss()`
- `graft_fixtures::synthetic::borrow(Scale) -> Result<PathBuf, SyntheticError>`, `Scale::HEAVY`, `Scale::HEAVY.with_commit_graph()`
- `StreamingLayout::advance`, `resident() -> Resident`, `work() -> Work`, `checkpoint_rows()`
- `Walk::has_commit_graph()`, `Walk::full_topological`

Expose: none public. If a first-paint timestamp needs a seam in `graft`, keep it `pub(crate)` and test-only.

## Acceptance
- [ ] Results table in this ticket: budget · measured · fixture · commit-graph yes/no · machine. 100k ×2 and git/git ×2.
- [ ] Each assertion proven to fail by temporarily tightening its threshold — say so, then revert.
- [ ] Compact-index and checkpoint-policy decisions written with the numbers; the three KNOWN_GAPS rows closed or re-owned with a reason.
- [ ] Fast tier still under 3 minutes; one heavy job at a time.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`; the heavy tier once, alone, at the end

## Size
One context, under ~80k tokens. Read only what "Read first" names. If you compact, say so in your report; if a second compaction is coming, stop at a clean seam with a handoff note.

## Results (2026-09-13, builder-opus)

Machine: Apple M5, 24 GB, macOS (Darwin 25.6), git 2.50.1. Each run alone.
What is timed: `crates/graft/tests/it/budget_run.rs` — real `JobRunner` + real `update`, opened as `event_loop::opening_repository` does, 120×40. First paint = store created → first history window with rows. **Excludes** process exec, config, terminal detection and the draw (terminal half → ticket 30). Peak RSS = `getrusage` peak of a process that runs only this.

| Budget | Measured | Fixture | commit-graph | Build | How |
|---|---|---|---|---|---|
| first paint ≤ 150 ms | 65–76 ms | 100k synthetic | no | debug (heavy tier) | `heavy::budgets_hold_on_the_100k_fixture_without_a_commit_graph`, 3 runs |
| peak RSS < 300 MB | 179.4 MiB (at paint 28 MiB) | 100k synthetic | no | debug | same |
| first paint ≤ 150 ms | 64–76 ms | 100k synthetic | yes | debug | `heavy::…with_a_commit_graph`, 3 runs |
| peak RSS < 300 MB | 157.6 MiB (at paint 33 MiB) | 100k synthetic | yes | debug | same |
| first paint ≤ 150 ms | 46 ms warm, 82–113 ms cold | git/git 85,649 | yes (clone ships one) | release | `cargo run --release -p graft --example budgets -- "$(scripts/clone-git-git.sh --where)"`, 6 runs |
| peak RSS < 300 MB | 254.5–255.8 MiB (at paint 78 MiB) | git/git | yes | release | same |
| first paint ≤ 150 ms | 13.5 ms warm, 67 ms cold | git/git | no (copy in /tmp, `objects/info/commit-graph` removed) | release | same example on the copy, 3 runs |
| peak RSS < 300 MB (27, before fixes) | 300.5–305.9 MiB, missed | git/git | no | release | same |
| peak RSS < 300 MB | 223.6–225.3 MiB (after 27b B, 27c C, 27d D) | git/git | yes | release | `examples/budgets`, 3 runs (27d) |
| **peak RSS < 300 MB** | **219.1–221.5 MiB (after B, C, D)** | git/git | no | release | same, 3 runs (27d); per-fix table in ticket 27d |

Also seen: `G` 3.3 s (100k, cg, debug), 7.4 s (100k, no cg, debug), 0.52 s (git/git, cg, release), 1.25–1.33 s (git/git, no cg, release). `g` back to top after `G` ≈ 0.1 ms on all.

Each assertion proven to fail: `PEAK_RSS_LIMIT` set to 1 MiB → both tests fail ("peak RSS 157 MiB is over the 1 MiB budget"); `FIRST_PAINT_LIMIT` set to 1 ms → both fail ("first paint 74.062ms is over the 1ms budget"). Both reverted.

Fast tier after: 740 passed, 9 s wall.

## Decisions

- **Compact index for `EmittedOrder`: not now.** It is ~8–9 MB at 100k (m1-06), under 5 % of the 150–280 MiB `G` adds. Halving it cannot bring git/git back under 300 MB; the rise is elsewhere (see Handoff). Re-owned to the 1M-commit claim (30).
- **Checkpoint LRU thinning: not now.** 64 checkpoints × 2,000 rows = 128k rows, above both 100k and git/git's 85,649, so no checkpoint is ever evicted on either and the LRU policy never acts; `g` after `G` measured ≈ 0.1 ms. A jump into a checkpoint-less stretch only exists past 128k rows → re-owned to 30 (1M claim).
- **Background advance between frames: not now.** First paint is far under budget on all four; `G` on git/git is 0.5 s (cg) / 1.3 s (no cg) as a cancellable job. Leave on-demand.

## Handoff (what remains)

- Done: measurement harness (`tests/it/budget_run.rs`, `tests/it/heavy.rs`, `examples/budgets.rs`), both 100k budgets asserted and green, git/git measured.
- **Not done: git/git without a commit-graph peaks at 300.5–305.9 MiB**, over the 300 MB gate (it is also 254 MiB with one, too close). Nearly all of it arrives during `G` (+175 MiB with cg, +278 MiB without) while row chunks are bounded, so something holds ~2–3 KB per commit or a cache grows. Next step: find what, with a heap profile of `examples/budgets` on the no-cg copy (`cp -R` the clone, delete `objects/info/commit-graph`), then fix. Suspects below.
- Gotcha: zsh does not word-split `$VAR` commands; spell cargo commands out.

### Suspects (scout, from source, unmeasured — confirm with a heap profile first)

1. `graft-git/src/history/topological.rs` `Topological::{known, indegree, expanded, emitted}` never shrink: ~250–350 B per discovered commit for the walk's life. Without a commit-graph every generation is unknown, so the frontier heaps also hold ~all commits.
2. ADR-0008 promotion runs a second `Topological` (`graft/src/history/mod.rs` `topological_order`) and hydrates full `Commit`s (message, author, committer) only to take `commit.id`.
3. `Views::promote` builds the new layout (order, `Included`, cache, checkpoints) before dropping the old one: transient doubling.
4. `hydrate` churn on `G`: ~85k decoded commits of 500–800 B, freed but kept as allocator high-water.
5. `graft-git/src/repo.rs` 16 MiB object cache per `RepoHandle`, per thread (walk + promotion + refs).
6. `Included::{walked, shown}` + `EmittedOrder`: ~12 MB at 85k, by design.
Why git/git and not the synthetic 100k: real messages and identities are larger, so (2) and (4) scale with commit size.

## NOTES

- Context compacted once near the end (after commit b040bb8, before this section).
