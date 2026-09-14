# m1-the-graph/59 — No-commit-graph heap at 1M: name the holder, then fix what the design allows

Status: ready
Build with: **Opus** (high) — attribution needs judgment (which holder, at which stage, by how much) and the fix is inside the walk's topological sort in `graft-git`; the bar is a number, so it is not a Fable ticket.
Needs: `graft_fixtures::footprint` and `budgets --gate` / `--phases` footprint lines (guards-measure-in-a-child).
Blocked by: 57, 58b

## The finding

ADR-0026 bounds what Graft allocates on the no-commit-graph path at **≤ 100 B of heap per commit** (footprint, so pack pages don't count). 56 measured **~176 B/commit at peak** (footprint 37.2 MiB at 100k → 188.3 MiB at 1M, no cg, small), footprint < 300 MB holds, and left the holder unattributed. Note the curve is not linear: 10k → 100k is 303 B/commit, 100k → 1M is 176; something scale-dependent (hash-table doubling, slack) is in there.

What the design says holds heap per commit without a graph: the interner (`CommitTable` overflow: 20 B raw id + a `u64`-prefix → handle map; ADR's own estimate "~100", 56's "~40"), `EmittedOrder` 9–12 B (measured), `Included` bitsets, `Views.promoted` 4 B, and — **transient at peak** — `Walk::full_topological`'s dense per-handle arrays for the promotion sort (parents per commit; if stored as a `Vec` per commit that is ~24 B header + allocation + malloc overhead ≈ 50–60 B/commit) plus the old and new `EmittedOrder` both alive while `Views::promote` lays the promoted layout out to the anchor (ADR-0008, by design). Those add up to the neighbourhood of 176. Which ones actually dominate, and at which stage, is what Phase A must say with numbers, not what this spec guesses.

## Read first
- ADR-0026: "Context" table, "Decision (design)" items 1–3, "Expected at 1M" paragraph, "The gate at 1M" no-cg bullet, "The 2× no-commit-graph `EmittedOrder`"
- `.scratch/m1-the-graph/issues/56-memory-curve-acceptance.md`: the review table's two no-cg rows, the "No-cg heap per commit" bullet (line 73), "Out" (line 29: the heap-profile prototype and `owners.py` exist, from 27b; use them if they still run), "Spec-only pass 2" (finding 2)
- `crates/graft-git/MAP.md` `history` (`Walk`, `full_topological`, `CommitTable::*`, `approximate_bytes`)
- `crates/graft/MAP.md` `history` (`topological_order`, `Promote`, `Promotion`, `Views::promote`), `examples/budgets` line
- `crates/graft-graph/MAP.md` `layout::order` (`EmittedOrder`, `approximate_bytes`), `layout::stream` (`Resident`, `Work`, `StreamingLayout::resident`)
- `crates/graft-fixtures/MAP.md` (`footprint`, `synthetic`)

## Scope
**Phase A — name the holder (no fix).**
- `budgets --phases` on the no-cg path gains the stages the no-cg peak passes through, each with `footprint now` and the introspection at hand: `open` (interner bytes), `first paint`, `date walk to end` (= `G` without promotion: interner bytes, `EmittedOrder::approximate_bytes`, `Resident` bytes), `topological_order` alone (footprint before / peak during / after — the sort's transient arrays are this delta; also `CommitTable::len`), `promote swap` (both layouts alive: the delta), `g`. Serial, one thread, like the existing `--phases`.
- Run it at 100k no-cg and 1M no-cg (small), 3 runs each, release; raw to `target/<ticket>/`. Write a holder table like ADR-0026's: holder, MiB at 1M, B/commit, stage where it peaks, transient or kept. The sum must explain the 176; the unexplained remainder ≤ 15 B/commit or the table is not done (use the heap profiler if arithmetic won't close it).
- Name the cause in one line, and say which holders the design allows to shrink and which are the floor (interner + one order + bitsets).
- **Seam.** If Phase A ends past ~45k tokens, stop with a handoff (table, cause, the fix list); the conductor files Phase B as its own ticket on this spec.

**Phase B — fix within ADR-0026, re-measure.**
- Expected candidates, in order of likely yield (do the ones the table names, not all of them): the sort's per-commit parent storage → CSR (`offsets: Vec<u32>` + `parents: Vec<CommitHandle>`, ~8 B/commit) and per-commit time only as wide as the sort needs; interner slack (reserve/shrink where the count is known — after the date walk it is; never a second table); anything transient that outlives its stage (drop the sort's arrays before handing the order to the app; the promoted `Vec` becoming the `Arc<[CommitHandle]>` without a copy).
- Not allowed: a second reader, spilling to disk, changing what the walk emits or its order (the order tests are the guard), touching the cg path, any bar.
- Re-measure: the curve's no-cg cells at 100k and 1M via `scripts/memory-curve.sh --repo`, 3 runs, plus `--phases` after. Numbers into the ticket for 56's re-run.

Out: the pack pages (the offer is their fix), a heavy-tier no-cg slope guard (the 100k regime reads 303 B/commit today and is not the 1M regime; a guard needs its own understanding — name it as a follow-up in the report if Phase A's table makes one obvious), any RSS number.

## Signatures
None new and public in Phase A beyond `budgets --phases` output lines. Phase B changes `Walk::full_topological`'s internals only; if a `CommitTable` or `Walk` public item changes shape, its MAP.md line changes with it and the order tests (`graft-git` corpus, `graft` promotion) must stay green untouched. Calls: `graft_fixtures::footprint`, `CommitTable::{len, overflow_len, approximate_bytes}`, `EmittedOrder::approximate_bytes`, `StreamingLayout::{resident, work}`, `graft::history::topological_order`.

## Acceptance
- [ ] Holder table at 1M no-cg (and 100k) in the ticket, 3 runs, sums within 15 B/commit of the measured slope; the cause named in one line; the design floor named in B/commit.
- [ ] Phase B: `(footprint_peak(1M no-cg) − footprint_peak(100k no-cg)) / 900,000 ≤ 100 B`, from `budgets --gate` in fresh processes, all 3 runs; `footprint_after_g` reported beside it (the ADR's "after `G`" reading). Realistic no-cg cell reported too.
- [ ] If the floor alone is > 100 B/commit: **stop after Phase A**, report the floor with its numbers — that is a finding for the user (the bar was set on the ADR's ~100 B interner estimate), not a number to adjust here.
- [ ] Order tests untouched and green; `overflow_len == commits` at 1M no-cg after `G` and after promotion (ADR-0026's 2× check) still holds.
- [ ] `G` time on 1M no-cg not worse than 56's 5.3–6.0 s; `topological_order` time reported before/after.
- [ ] Report: cause or symptom for each change, what it costs the user; raw paths only.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`; heavy: `budgets_hold_*` (four) once, release, one at a time.

## Size
Two contexts at most, seam after Phase A (see above); each under ~80k. 1M no-cg runs are ~10 s each; output to files.
