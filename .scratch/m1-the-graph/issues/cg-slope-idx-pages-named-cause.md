# m1-the-graph — Commit-graph slope: name the `walk_new`/`first_chunk` page holders by measurement, cut only an avoidable gix read (bar 150 kept)

Status: draft — **file only if the user keeps ADR-0026's 150 B/commit and wants the cut attempted** (spec-only pass 3, planner-fable, 2026-09-14). The recommended branch is `slope-guard-bounds-heap-reports-pages.md`; its "The finding" section is this ticket's starting table.
Build with: **Opus** (high) — attribution by vmmap region across four holders and a trace into gix's index loading; a stop rule that needs honest judgment; the bar is a number, so not Fable.
Needs: `budget_run::gated_in_child` / `ChildPeak` (57); `budgets --phases` laps with footprint (57, 58b); the packed copies at `target/58b/packed-{1M,100k}-cg` if they still exist.
Blocked by: the user's decision; 58b (done).

## What the planner expects, said up front
The remainder above 150 is, by arithmetic (see the other spec's table), commit-graph pages (~56 B/commit, the design's own floor) + pack-index pages (~68 on these fixtures: 20 B/object sha table saturated by 1,500 hydration lookups, plus a 4 B/object offset table faulted whole on the first lookup) + heap (~35). The **only in-design cut is the offset-table read, ≤ ~11 B/commit**, and only if gix reads it through a call Graft can avoid with a public API. Expected end state: **red at ~155 loose / ~145 packed.** This ticket exists to replace that arithmetic with a measurement and to take the 11 if it is there — not to make the guard green by re-tuning hydration breadth, packing the fixtures, or a private reader.

## Read first
- `slope-guard-bounds-heap-reports-pages.md` "The finding" (the table this ticket must confirm or correct).
- `.scratch/m1-the-graph/issues/58b-tag-tips-named-cause.md` "Holder table" (lines 134–150: method — vmmap per region inside the process before/after a lap; identify mappings by size) and "Phase B result" (lines 161–199).
- ADR-0026 "Decision (design)" items 1, 3, 4; "The gate at 1M" slope bullet; "Options considered per grower → Pack pages" (what is rejected).
- `crates/graft-git/MAP.md` `history` (`Walk`, `CommitTable::{handle_of, id_of}`), `repo` (`SharedRepo::{handle, handle_with_object_cache}`).
- `crates/graft/MAP.md` `examples/budgets` line; `crates/graft-fixtures/MAP.md` (`footprint`, `synthetic`).

## Scope
### Phase A — measure the holders (no fix)
- On the guard's own fixtures (10k cg, 100k cg, loose) and their `pack-refs --all` copies: vmmap per region before and after the `walk_new` and `first_chunk` laps of `budgets --phases`, 3 runs each, release, raw to `target/<ticket>/`. Regions by size: commit-graph, `.idx`, `.pack`, heap (footprint).
- **Holder table** at 10k and 100k, loose and packed: OIDL / CDAT+EDGE / idx sha table / idx offset table / pack pages / heap — MiB, B/commit over the 90k, kept or transient. The 100k − 10k sums must land within 10 % of the guard's rise (~14–15 MiB); if not, `vmmap -v` page-level deltas on the paused process.
- **Trace the offset-table read:** which gix call on the first object lookup faults a whole 4 B/object idx table (candidates: index open validating the trailer/large-offset table; `lookup_prefix`; the object cache). A Haiku/Sonnet scout may read gix's source and return ≤ 1 page; the builder does not. Say whether a public gix API avoids it without changing what Graft reads.
- Name the cause in one line and split it: **floor** (graph pages, idx sha-table pages from hydration lookups, heap) vs **avoidable** (whatever the trace names).
- **Seam:** past ~45k tokens after the table, stop with a handoff (table, trace result, Phase B item list).

### Phase B — cut only what Phase A named avoidable
- Only a gix call Graft can route around with a public API (e.g. a different lookup entry point or opening the index differently). Nothing else: no hydration re-tune, no fixture change, no bar change, no private reader, no `madvise`, no change to what the walk emits.
- Re-measure the guard **3 times alone**, loose; the packed copies' slope reported beside it.
- **Stop rule:** if Phase A's avoidable share is < 16 B/commit (the loose gap above the bar) — the expected case — **do not start Phase B for the guard's sake**; report the floor with the user question from the other spec, unchanged. Take the cut anyway only if it is a one-line API change with a measured win; otherwise leave it as a KNOWN_GAPS line.

Out: the no-cg path (59); the 300 MB gate; `ref_snapshot` (58b closed it); anything the ADR rejects under "Pack pages".

## Signatures
None new and public. If the trace leads to a different gix entry point inside `graft-git`, the change is private to `history`/`repo`; MAP.md lines change only if a public item's meaning does. Calls: `graft_fixtures::{footprint, peak_rss}`, `budgets --phases`, `gated_in_child`.

## Acceptance
- [ ] Holder table at 10k/100k, loose and packed, 3 runs, sums within 10 % of the guard's rise; cause in one line; floor vs avoidable in B/commit.
- [ ] The offset-table read traced to a named gix call, with the verdict (avoidable via public API: yes/no).
- [ ] Guard ×3 alone, loose: numbers in the ticket — green at ≤ 150, **or** the stop rule with the floor named and the packed slope beside it; nothing loosened.
- [ ] KNOWN_GAPS "m1-the-graph/58" section rewritten from the measured table (replaces the arithmetic).
- [ ] Order tests and corpus tests untouched and green; fast gates and heavy clippy green.
- [ ] Report: cause or symptom per change; what it costs the user (expected: nothing); raw paths only.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`; heavy: the guard ×3, release, alone.

## Size
One context for Phase A (~45k); Phase B, if any, a second under 30k. 100k runs are ~1 s; output to files.
