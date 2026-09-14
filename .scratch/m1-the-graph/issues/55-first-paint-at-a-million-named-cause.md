# m1-the-graph/55 — First paint at a million commits: name the cause by measurement, then fix it

Status: ready
Build with: **Opus** (high) — phase instrumentation inside the walk and the app's open path, then a fix in the rev-walk or the first-window path; correctness is numbers plus the existing order tests, so not Fable.
Needs: `Walk::next_links`, `CommitTable` (commit-handles-and-table); the walk reading the graph with dense state and its odb-read counter (walk-reads-the-commit-graph); rows hydrating on demand (rows-hydrate-on-demand **and rows-pass-through**, split 2026-09-14). Runs after all of them: their numbers are this ticket's baseline.
Blocked by: 49, 50, 52, 52b (rows-pass-through)

Why (user, 2026-09-14, ADR-0026 "User decisions on approval"): "The 1M first-screen miss gets its own ticket with a named cause, not just a number in a report." m1/46 measured 292–383 ms at 1M (100k: 26–85). ADR-0026 *names* the commit-graph path's in-degree pass as the likely cause; that is a hypothesis. **Both paths miss by about the same amount** (cg 292–346, no-cg 297–383), and the no-cg path has no in-degree pass before its date-order first window — so at least part of the cause is common to both paths and unnamed. This ticket names it by measurement, fixes what the design allows, and re-measures.

Budgets (ADR-0026, accepted): 1M is judged against §6.3's kernel-class **500 ms** and reported against **150 ms**. The 100k heavy-tier assertion at 150 ms stays.

## Read first
- ADR-0026 "Context" (first-paint sentence), "The gate at 1M" (first-paint bullet — the provisional-paint option "a later decision, flagged here, not taken"), "User decisions on approval"
- ADR-0008 (all); DESIGN.md §3.2 step 2 (both bullets; "This path is what meets the 150 ms first paint")
- Ticket 46 "Curve" table (first paint and "RSS at first paint" columns), ticket 27 lines 54–64 (what `budget_run` times and excludes), ticket 50 "Acceptance" (the first-paint finding it wrote, the odb counter)
- KNOWN_GAPS rows "First paint at 1M commits is 292–383 ms", "The first `advance` is a full 500-row chunk"
- `crates/graft-git/MAP.md` (`history`, `branches`, `repo`), `crates/graft/MAP.md` (`repos`, `repository`, `history` `stream`/`serve`, `refs`, `jobs`), `crates/graft-graph/MAP.md` (`layout::stream`)
- `crates/graft/examples/budgets.rs`: a scout summarises what it times and which jobs run concurrently during first paint (one page).

## Scope
### Phase A — name the cause (no fix yet)
- `examples/budgets` gains `--phases`: wall time, odb objects read and RSS delta per phase, serially on the loop's own thread, no concurrent jobs: `open` (`SharedRepo::open` + `facts`) · `refs` (`references`, `commit_tips`, `Branches::resolve`, `seeds_for`) · `table` (`CommitTable::open`) · `walk_new` (`Walk::all_refs` returns) · `first_link` (first `Link` out of `next_links`) · `first_chunk` (first `advance` returns) · `first_window` (rows sent). Then the same run with the jobs that normally start at open (`ReadRefs`, `ReadWorkingTree`, system git) enabled, to see contention.
- Run on the 1M small fixture, cg and no-cg, release, 3 runs each; 100k both paths once for the slope. Output to `target/<ticket>/phases.txt`.
- **Name the cause:** the phase (or concurrent job) holding ≥ 40 % of first paint on each path, with a per-commit cost (ns/commit, from 100k → 1M) and the mechanism — what runs once per commit and why (e.g. "the in-degree pass visits every graph commit down to the oldest tag's generation", "`Branches::resolve` rule 4 counts first-parent merges over the whole history", "gix parses every tip's commit for the date queue", "the first `advance` lays out 500 rows"). Confirm with one discriminating change (disable or stub it; the number drops by its share). Rank the hypotheses above; measure, don't guess.
- Write the phase table and the named cause(s) into this ticket before touching the fix. **Seam: stop here with a handoff note if over ~40k tokens.**

### Phase B — fix what the design allows, re-measure
- Anything the first window does not need moves after it or into a job that already exists (refs/branch work, counting, the working-tree read), or is bounded (the first `advance` sized to the viewport plus `buffer_rows`, then the usual chunk). Same emitted order, same rows: every order and snapshot test unchanged.
- Re-measure 1M cg and no-cg (3 runs each) and 100k both paths; write before/after in this ticket, TESTING.md's curve section (one row: first paint per cell), and close or rewrite the two KNOWN_GAPS rows with the numbers.
- **If the named cause on the cg path is the strict-topo in-degree pass itself** (inherent to emitting strict order before the oldest tip's generation) and 1M cg is still over 150 ms after everything else is fixed: **do not build the fallback.** Write the number, the cost per commit, and the recommendation below into the ticket and stop. It changes DESIGN.md §3.2's sentence "This path is what meets the 150 ms first paint" and needs the user's OK at the checkpoint.

### Pre-decided design for the fallback (built only on the user's OK, as its own ticket)
Date-order provisional paint plus promotion on the commit-graph path too — ADR-0008's no-cg machinery, already built (`WalkFacts::is_provisional`, `OrderNotice::Sorting`, `PromoteHistory`, `Views::promote`), gated by a deterministic count, never by elapsed time:
```rust
// graft-git::history
pub const DIRECT_TOPO_LIMIT: u32;   // graph commits the in-degree pass may visit before the first emission; above it the walk starts in date order, provisional, and promotion runs. Value from the measured ns/commit so the pass fits ~100 ms; the measurement in its doc comment.
impl Walk {
    pub fn pass_depth(&self) -> Option<u32>;   // commits with generation ≥ the lowest tip's generation: one scan of the graph's generation column; None without a graph
    pub fn all_refs_with_limit(repo: &RepoHandle, order: Order, table: Arc<CommitTable>, limit: u32) -> Result<Self, WalkError>;   // `all_refs` = this with DIRECT_TOPO_LIMIT
}
```
`full_topological` on the cg path reads the graph (ticket 50), so promotion is the 1M `G` number, not a full object walk. Test: the 5k cg synthetic pair with `limit = 100` goes provisional and the promoted order equals the direct topo order; `limit = u32::MAX` never goes provisional. Cost to the user: on a kernel-sized repo with a graph, one reshuffle a few seconds after open (the notice says so), instead of a 300 ms blank.

Out: memory (memory-curve-acceptance); the walk's emitted order; building the fallback without the user's OK; the 100k assertion's limit.

## Signatures
No new public items in Phase A beyond `examples/budgets --phases`. Phase B's are whatever the named cause needs; if a public item changes, its MAP.md line changes with it and the reviewer checks the order tests. The fallback's signatures are above, for the follow-up ticket.

## Acceptance
- [ ] Phase table in this ticket: 7 phases × {1M cg, 1M no-cg} × 3 runs, plus 100k both paths, plus the concurrent-jobs run; `target/<ticket>/phases.txt` referenced.
- [ ] Each path's cause named with mechanism, ns/commit and the discriminating measurement that confirmed it. "The in-degree pass" is accepted only with its measured share; "something linear" is not a name.
- [ ] After the fix: 1M cg and no-cg first paint < 500 ms on every run (the judged budget); each number written against 150 ms with yes/no and, for a miss, the residual's named cause.
- [ ] 100k both paths still ≤ 150 ms in `heavy::budgets_hold_*`; every `graft-git` order test and every corpus snapshot unchanged.
- [ ] If the residual is the in-degree pass: the recommendation section filled in with numbers; no fallback code in the diff.
- [ ] TESTING.md curve section and KNOWN_GAPS rows updated with numbers; MAP.md for any crate whose public items moved.
- [ ] Report: cause vs symptom stated per fix; what was measured and what was not (a real kernel clone is not required; say so).

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`; heavy tier once, alone, at the end.

## Size
One context, under ~80k tokens: 1M runs are ~10 s each. Seam between Phase A and Phase B; stop there with a handoff note rather than compacting.
