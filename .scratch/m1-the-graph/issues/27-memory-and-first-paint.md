# m1-the-graph/27 — Peak memory and first paint, measured for real

Status: ready
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
