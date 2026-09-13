# Memory at a million commits: handles instead of ids, the walk reads the commit-graph, and what the 300 MB gate covers at 1M

Status: **Accepted** (user, 2026-09-14; planner-fable, on m1-the-graph/46's finding). Lands in M1.

## Context

PROMPT.md §1.2 promises "memory for rendering grows with the viewport, not with history size" and 60 fps on 1M+ commits; §7.1 gates 300 MB peak RSS (`getrusage`) on the 100k fixture. m1/46 measured the curve (release, 3 runs): 10k 24–37 MiB, 100k 104–129, **1M 881–949**, always at `G`. The rendering half holds (rows ~7 MiB, flat). What grows is everything keyed by a 33-byte `Oid`, plus the pack pages the walk reads:

| Holder (1M) | Measured | Per commit |
|---|---|---|
| Mapped pack + commit-graph pages the full walk touched | 275–340 MiB | ≈ commit object bytes |
| `Topological.known` (`HashMap<Oid, Entry>`; kept for the view with a cg) | 178 kept / 267 peak | 178–187 B |
| `EmittedOrder` (`Vec<Oid>` + `HashMap<Oid, usize>`) | 131 (cg) / 222 (no cg) | 137 / 233 B |
| `Included.walked/shown` (`HashSet<Oid>`) | 23–45 | 35–48 B |
| `Views.promoted` (`Arc<[Oid]>`) | 31 | 33 B |
| malloc-retained churn (1M `Commit` hydrations per `G`) | 148–229 after `G` | — |

First paint at 1M: 292–383 ms (100k: 26–85). RSS at first paint 162–211 MiB: something linear in history runs before the first window.

## Decision (design)

**One shared id table; every history-sized structure holds a `u32` handle, never an `Oid`; the walk reads the commit-graph file, not commit objects; rows are hydrated only near the viewport.**

1. **`CommitHandle(u32)`** (`graft-core`). With a commit-graph, a handle *is* the commit's position in the commit-graph file: the file is the table, `id_of` is one mmap'd read, `handle_of` a fanout binary search; nothing is copied to the heap. Commits absent from the graph (fetched since it was written) and every commit when there is no graph go to an **overflow interner** (`graft-git::history::CommitTable`): raw id bytes at the repo's hash length (20 B for SHA-1) plus a `u64`-prefix → handle map (~20–40 B), verified against the stored id on every hit, with a full-`Oid` side map for genuine prefix collisions. One `Arc<CommitTable>` per open repository, shared by the walk, promotion, both views and search.
2. **Order state on handles.** `EmittedOrder` = `Vec<CommitHandle>` (row → handle) + dense `Vec<u32>` (handle → row). `Included.walked/shown`, `Dated.seen`, `Topological`'s flags = `HandleSet` bitsets (1 bit per commit). `Views.promoted` = `Arc<[CommitHandle]>`. `LayoutCursor.waiting` keyed by handle.
3. **The walk reads the graph.** On the commit-graph path `Topological` takes parents, commit time and generation from the commit-graph and never opens a commit object; a commit outside the graph is treated as generation infinity (git's rule) and header-parsed. Per-commit walk state is a dense `Vec<u32>` in-degree indexed by handle plus two bitsets; no entry is ever hashed or kept after emission. The no-commit-graph promotion sort uses the same dense arrays, freed when the sort ends.
4. **Rows hydrate on demand.** `CommitSource::pull` yields `Link { commit, parents }` (handles only); `LayoutCursor::extend` lays links out into `RowShape`s. `StreamingLayout::advance` materialises a `Row` (reads the `Commit`, assembles it) only for chunks intersecting the viewport ± buffer; every other chunk passes through — lane state advances, order and checkpoints are recorded, a shape fingerprint is kept, no commit object is read. `rows()` over a passed-through range rebuilds from the nearest checkpoint as today, hydrating only that range. `Row` keeps `commit: Commit` with the parents the view shows (trimmed under `Included`), so today's rows and snapshots are unchanged.

Expected at 1M, from the per-commit costs: commit-graph path heap ≈ flat ~50 MiB + ~20 MiB (8 order per view, 4 promoted, 5 walk, bitsets); RSS ≈ heap + ~56 MiB of commit-graph pages + the pack pages of rows shown → **~130–160 MiB**. No-commit-graph path heap ≈ 50 + ~100 (the interner is the one unavoidable table) ≈ 150 MiB; RSS ≈ 150 + the pack pages of every commit parsed (~300 MiB on the fixture).

What it costs: a deep jump still replays ≤ 2,000 links from a checkpoint (cheaper than today: links come from the graph, no hydration) then reads one chunk of commits (~500 objects, a few ms); first paint loses the hashing and `Oid` copying; one new table, one new trait shape, one assembly step (`RowShape` + `Commit` → `Row`).

## Options considered per grower

- **Pack pages.** (a) Keep parsing every commit; (b) walk from the commit-graph (chosen — it is what git does); (c) `madvise` the pack mapping between chunks — needs gix to expose the map, one more `unsafe` libc call, unverified on macOS `ru_maxrss`; (d) read the pack with `pread` into a bounded buffer — a private object reader, out of the question; (e) spawn `git rev-list --topo-order --parents` for the no-graph sort — moves the pages into git's process, which hides the cost from the gate without removing it from the laptop, and ADR-0011 makes gix the reader. Rejected.
- **Order and sets.** (a) Compact `Oid` keys (20 B, `u64`-prefix maps) in each of the four structures — halves them, still four tables; (b) one shared table, handles everywhere (chosen); (c) spill the order to disk — latency on every far scroll for a structure that is 8 MB at 1M once it holds handles. 
- **`Topological.known`.** (a) Drop entries on emission (in topological order no later commit names an emitted one as a parent, so the entry is dead); (b) dense arrays by handle (chosen; (a) falls out of it — nothing is kept).
- **Churn.** (a) A smaller object cache / arena — treats the symptom; (b) do not hydrate what is never shown (chosen).

## The gate at 1M (Proposed change to PROMPT.md §1.2/§7.1 wording; not a new number)

PROMPT.md names 300 MB only for the 100k repo and gives two first-paint numbers: 150 ms "typical repo", 500 ms "Linux kernel" (1.3M commits). The 1M claim in §1.2 is stated in terms of viewport-bound rendering memory, with no MB figure. m1/46 read 300 MB as the 1M budget, as the user did. Proposal, so the claim is checkable rather than implied:

- **The gate stays `getrusage` peak RSS, 300 MB, both builds** (ADR-0021). Unchanged.
- **With a commit-graph, 300 MB peak RSS at 1M is a hard target** the design above must meet; it is the common case (`gc.writeCommitGraph` defaults on) and the case Graft offers to create (ADR-0008, R2-4 in M2). `scripts/memory-curve.sh` at 1M with a commit-graph asserts it at every review.
- **Without a commit-graph, peak RSS at 1M is reported, not gated.** The walk must parse every commit object, gix maps the pack, and clean file-backed pages count in `ru_maxrss` even though the kernel reclaims them first under pressure and Activity Monitor's "Memory" (footprint) excludes them. The design bounds what Graft itself allocates: **≤ 100 B of heap per commit** on that path (footprint under 300 MB at 1M), asserted by the curve. The fix for the pages is the commit-graph offer (M2), not a tuned cap. The 100k no-graph gate stays as it is (passes at 118–129 MiB).
- **First paint at 1M is judged against §6.3's kernel-class 500 ms** and reported against 150 ms. A 1M history is the kernel class, not the "typical repo". On the commit-graph path git's own in-degree pass runs down to the generation of the oldest tip before the first emission (old tags make that the whole range, in git as in Graft); if the redesigned walk still misses 150 ms at 1M, the remaining option is ADR-0008's provisional paint plus promotion on the commit-graph path too — a later decision, flagged here, not taken.
- **A slope guard joins the heavy tier:** peak RSS rise between the 10k and 100k commit-graph fixtures ≤ 150 B per commit (today 0.85–0.93 KiB; expected ~60–80 after this design). The 1M run stays manual at reviews (a 1M fixture is 300–360 MiB on disk; PROMPT §7.1 names the 100k repo for the tier).

This is not a loosening of 300 MB: it says what the number covers at 1M and names the one case it cannot cover with the reader PROMPT.md chose.

## Scope: M1

Land before the M1 checkpoint (five tickets, three on Opus, one Sonnet, the acceptance on Fable). Why not defer: the handle table and the `CommitSource` shape are structural — M2's detail pane ("contained in", the reachability index KNOWN_GAPS names) wants exactly the commit-graph-position table, and Mistake Radar scans the same walk; retrofitting after M2 touches every consumer twice. Cost to the user: M2's daily-value work starts about five tickets later. He does not open 1M-commit repos daily; he named the curve and the Linux kernel is PROMPT's own manual check, so a 3× miss on the record at the "performance budgets" milestone is worse than the delay. Deferred with a named expiry: the no-graph pack-page residual (M2, the offer); the 1M `frames::` scroll numbers (M1 review's release perf run, once the curve passes).

## The 2× no-commit-graph `EmittedOrder`

The no-graph run's one hash table had 2^22 buckets against the graph run's 2^21: more than 1.83M distinct keys were inserted for 1M commits. With the order on handles that table is gone, but the *cause* would reappear as an oversized interner. So it is part of the design, not a separate investigation: the order ticket asserts on the 1M no-graph fixture that `CommitTable::len() == commits` after `G` and after promotion, and stops and reports if it is not.

## Consequences

- `graft-core` gains `CommitHandle`, `Link`, `Parents`, `HandleSet` (plain data and one container; no domain rule — within ADR-0007's line, stated so here).
- `CommitSource` changes shape (`pull` yields links; `links`, `read`, `handle_of`, `id_of`); `Walk` grows `next_links`; `next_ids`/`IdChunk` (27c) go.
- Search (`search::scan`) still reads every commit whole — it matches on messages, so it must; a 1M search touches the pack once, off-thread. Unchanged, noted.
- `Row.lanes` sized to the widest column ever seen (4.5 KB/row on git/git, 27b) is flat in history and stays as it is.
- DESIGN.md §3.1 `EmittedOrder` (~90 B/commit) and §3.2 walker_state, §3.6 "re-reading their commits from the retained order" are amended by the acceptance ticket once the numbers are in.

## User decisions on approval (2026-09-14)

- **The gate at 1M: yes, with two conditions.**
  1. Wherever memory is reported rather than failed (1M without a commit-graph file), the numbers must be **visible in the milestone review and in KNOWN_GAPS.md**, not only in a results file.
  2. When a repo has **no commit-graph file, Graft offers to build one**, as ADR-0008 already promised. The fallback path must not be silent.
- **Lands in M1**, not M2: it is the foundation, and M2's reachability work wants the same table; doing it later means rewriting M2's work.
- **The 1M first-screen miss (over 500 ms) gets its own ticket with a named cause**, not just a number in a report.
- `CommitHandle`/`Link`/`Parents`/`HandleSet` in graft-core: fine (plain data, no behaviour, per ADR-0007). Reshaping `CommitSource` follows from it: fine.
