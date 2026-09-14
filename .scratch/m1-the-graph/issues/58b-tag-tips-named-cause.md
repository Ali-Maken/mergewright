# m1-the-graph/58b — Annotated tags at open: name what the 85 MiB holds, then peel inside the design (slope guard green or the floor named)

Status: done — stop rule fired (guard red loose AND packed; remainder is not tags)
Build with: **Opus** (high) — attribution across three candidate holders (OIDL pages, pack/idx pages per tag read, heap), a stop rule that needs honest judgment, and gix API navigation (`peeled`, cache-free handles); the bar is a number, so not Fable. 58's Sonnet pass mis-stated the per-tag arithmetic (13–19 KB is per *ref*; per tag it is ~80–130 KB) and skipped the 3× runs — this ticket must not.
Needs: `gated_in_child` / `ChildPeak` and the slope guard on it (57); `graft_fixtures::footprint` (57); `KnownCommits` and `RepoHandle::ref_snapshot(&dyn KnownCommits)` (58).
Blocked by: 58

## The finding, and the ruling

58 took the slope guard from 237 to **166 B/commit** (bar 150, ADR-0026; unchanged). The named residual: annotated tags (12–13 % of refs at every scale: 9/66, 83/650, 790/6498) are outside `CommitTable` by design and still pay a header read + peel. `ref_snapshot`'s peak-RSS rise after 58: **1.2 MiB (10k cg) / 8.3 MiB (100k) / 85.5 MiB (1M)**.

What the numbers already say, before anyone measures more:
- **It is not retained heap.** Footprint (anonymous pages) peaks at 36.8–38.1 MiB for the *whole* 1M cg run (56's table); the `ref_snapshot` phase alone raises RSS by 85.5 MiB. A decoded tag is ~200 B; 790 of them cannot be 65 MiB of heap. Expect the rise to be **mapped file pages** faulted in by the phase: (a) commit-graph **OIDL pages** the `handle_of` binary search touches for every tip — the walk itself reads CDAT/EDGE by position and touches OIDL only on `id_of` for the ~1,500 hydrated rows, so these pages are new at peak (≈ 2 MiB at 100k, ≈ 20 MiB at 1M, ~all of the chunk once lookups are in the hundreds); (b) per annotated tag, the **pack `.idx` page** of its lookup plus the **pack pages of its delta chain** (synthetic tags with near-identical text deltify against each other; `find_header` on a packed delta walks the chain to the base to learn the kind, then the decode walks it again — same pages, faulted once). 82 KB ≈ 5 pages per tag at 1M fits (b); 110 KB at 10k fits too.
- The bar arithmetic at 100k: 166 B × 90k = 14.9 MiB rise between 10k and 100k; `ref_snapshot`'s share is 8.3 − 1.2 = **7.1 MiB ≈ 79 B/commit**; commit-graph pages are the ADR's 56 B floor; heap ~30 B. Green needs `ref_snapshot`'s share ≤ ~63 B/commit ≈ **5.7 MiB at 100k** — 1.4 MiB off, not a rewrite.

**The ruling, inside ADR-0026:** the table stays commits-only. A tag ref is resolved by git's own two answers, in this order: **(1) the `packed-refs` peeled line** (`^<id>` after a packed tag; gix exposes it as `Reference::peeled`), which costs no object read at all; **(2) otherwise one decode of the tag object** — not a header read *and* a decode — keeping only its target id, which is then judged by the table (a known target is a commit, no read; an unknown one is read as today: tag-of-tag, commit outside the graph, or blob/tree). Nothing decoded is kept. That is the design floor for loose annotated tags: one object read per tag. If the floor alone still leaves the guard red on the loose-ref fixture, that is a finding for the user (below), not a number to adjust.

Options rejected: table entries for tag targets (the target is only known *after* the read, so it saves nothing; and 58 already skips the target's read via `KnownCommits`); a private `packed-refs`/pack reader (ADR-0011: gix is the reader); packing the *guard's* fixture inside this ticket (58's ruling stands until the user changes it; this ticket only *reports* the packed number beside the loose one); subtracting first-paint RSS from the slope.

## Order

Before 59. Smaller (one seam at most); closes the only red heavy test; its `--phases` laps land before 59 extends the same file with no-cg stages; both run 1M fixtures serially anyway (one heavy job at a time). 59's Phase B touches `Walk::full_topological` only — no overlap with `repo`.

## Read first
- This spec, whole.
- `.scratch/m1-the-graph/issues/58-tips-from-the-table.md`: "The finding, and the ruling" (lines 8–16) and Acceptance lines 60–67 (the measured residual).
- `.scratch/m1-the-graph/issues/55-first-paint-at-a-million-named-cause.md` lines 82–99: refs all loose at 1M; the packed-copy discriminator (`target/55/packed-1M-cg`); "+145 MiB … present packed too" (the ref files are not the holder).
- `.scratch/m1-the-graph/issues/57-guards-measure-in-a-child.md` "Result" (footprint vs `time -l`; how the guard runs).
- `.scratch/m1-the-graph/issues/56-memory-curve-acceptance.md` "For the M1 review" table (the 1M cg cells: footprint 36.8–38.1 MiB is the heap ceiling this ticket reasons from).
- ADR-0026 "Decision (design)" item 1; "The gate at 1M" slope bullet.
- `KNOWN_GAPS.md` section "m1-the-graph/58 — tips checked against the id table".
- `crates/graft-git/MAP.md` `repo` (`RepoHandle::{ref_snapshot, commit_tips, peeled_references, handle_with_object_cache}`, `KnownCommits`, `RefSnapshot`, `SharedRepo::OBJECT_CACHE_BYTES`) and `history` (`CommitTable::handle_of`).
- `crates/graft/MAP.md` `repos` (`RepoStore::refs`, `table`), `examples/budgets` line.
- `crates/graft-fixtures/MAP.md` (`footprint`, `Footprint`, `synthetic`).
- TESTING.md lines 74–87 (slope guards in a child) and line 141 (`scripts/memory-curve.sh`).

## Scope

### Phase A — name the holder (no fix)
- **`TipStats`** on `RefSnapshot` (signature below), filled by the one pass: refs seen, tips the table answered, annotated tags met, tags resolved from a packed peeled id, objects decoded, `find_header` calls. This is the instrumentation, kept — not a temporary edit.
- **`budgets --phases`** prints, on the `ref_snapshot` line: `RSS +<MiB> footprint +<MiB>` (footprint delta is 57's reading; it is the heap-vs-pages discriminator) and the `TipStats` fields.
- **Discriminator runs**, release, 1M cg and 100k cg, 3 runs each, raw to `target/<ticket>/`:
  1. as-is (the baseline this ticket starts from; must reproduce 58's 85.5 / 8.3 MiB within 10 %);
  2. tag refs skipped (temporary local edit, reverted): the rise without any tag read = the OIDL/table cost;
  3. `ref_snapshot` on `handle_with_object_cache(0)` (temporary): the retained-decode test — expect no change if the rise is pages;
  4. the packed copy (`target/55/packed-1M-cg` if it still exists, else `cp -c` the 1M cg fixture + `git pack-refs --all`): the rise when peeled ids are available — this is the post-fix expectation for a gc'd repo, measured before writing the fix;
  5. `git verify-pack -v <pack.idx> | awk '$2=="tag"'` on the 100k and 1M cg fixtures: are the tags packed or loose, and their delta-chain depth (count, mean, max); plus `git count-objects -v`.
- **Holder table** (like ADR-0026's): holder — OIDL pages / idx pages / pack pages of tag reads / heap — MiB at 1M and 100k, per tag or per ref, kept or transient. Must sum within 10 % of the measured rise at both scales; if arithmetic will not close it, `vmmap`/`footprint` per region on the paused process (report what was used). Name the cause in one line.
- **Seam.** Past ~45k tokens after the table: stop with a handoff (table, cause, the Phase B item list); the conductor files Phase B on this spec.

### Phase B — fix within the design, re-measure
1. **Packed peeled ids.** A ref whose gix `Reference` carries `peeled: Some(id)` (the `packed-refs` `^` line) resolves to `id` with no object read; `id` is judged by the table like any tip (known → commit; unknown → read as today). If the gix version in `Cargo.lock` does not expose `peeled`, say so and stop on this item — no private `packed-refs` parser.
2. **One decode per unknown tip, no header read.** An unknown tip is read once with a full `find` (the decode says the kind; for a tag it says the target); the separate `find_header` goes. The tag's target: known → done; unknown → read once more (tag-of-tag continues; blob/tree excluded as today). `find_header` count in `TipStats` becomes 0 on the cg path.
3. **Nothing decoded is kept.** Only if Phase A run 3 moved the footprint: read tips on `handle_with_object_cache(0)` (as 27d's fix D did for promotion). If run 3 showed nothing, do not change the handle — a change without a measured cause is a symptom fix.
4. Anything else Phase A's table names, within: no second reader, no fixture change, no bar change, no change to what `tips`/`peeled` mean, no-cg path untouched in meaning (stats only).
- **Fast tests** (corpus, `graft-git/src/repo/tests.rs`): extend `refs_sharing_commits_or_naming_no_commit_peel_the_same_loose_or_packed`: packed variant → `stats.peeled_packed == stats.tags` and `stats.objects_read == 0` when every tag target is in the table; loose variant → `stats.objects_read == stats.tags + <unknown targets>`, `stats.find_header == 0` with a real table; `tips`/`peeled` byte-identical loose vs packed, `()` vs real table. Tag-of-tag and tag-on-blob cases kept.
- **Measure**, release, raw to `target/<ticket>/`:
  - the slope guard (`heavy::peak_rss_slope_per_commit_with_a_commit_graph`) **3 times**, alone; the number and both peaks each run;
  - `budgets --phases` 1M cg **3 runs**: `ref_snapshot` RSS/footprint rise and `TipStats`; the same on 100k cg;
  - `scripts/memory-curve.sh --repo` on 1M small cg and 1M realistic cg, **3 runs** each: peak RSS, footprint, first paint 1/2/3, `G`;
  - **reported, not gated:** the guard's slope on `git pack-refs --all` copies of the 10k cg and 100k cg fixtures (one `cp -c` each, deleted after), 3 runs — the number the user needs if the loose floor is red.
- MAP.md lines for `RefSnapshot`, `TipStats`, `budgets --phases`; KNOWN_GAPS.md "m1-the-graph/58" section updated (closed, or the floor with its numbers); ARCHITECTURE.md untouched.

Out: the no-cg heap (59); any bar or fixture change; commit-graph staleness (tips outside the graph, M2+); the cold-cache question; `refs::read`'s badges beyond consuming the same snapshot (confirm no second lookup path, as 58 did).

## Signatures
```rust
// graft-git::repo
/// How one ref-store pass learned its tips — what the table answered, what had to be read.
/// For the memory record (`budgets --phases`) and the corpus tests; never a decision input.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TipStats {
    pub refs: u32,          // refs enumerated
    pub known: u32,         // tips the `KnownCommits` table answered without a read
    pub tags: u32,          // annotated tag objects met (peeled from packed-refs or decoded)
    pub peeled_packed: u32, // tags resolved from a packed-refs peeled id: no object read
    pub objects_read: u32,  // objects decoded whole (tags, and unknown targets)
    pub find_header: u32,   // header-only reads (0 on the commit-graph path after Phase B)
}

pub struct RefSnapshot {
    // refs, tips, peeled: unchanged in type and meaning (55, 58)
    pub stats: TipStats,
}

impl RepoHandle {
    // unchanged shapes; `ref_snapshot` fills `stats`, `commit_tips`/`peeled_references` keep delegating
    pub fn ref_snapshot(&self, known: &dyn KnownCommits) -> Result<RefSnapshot, RefsError>;
}
```
`budgets --phases` line: `ref_snapshot <ms> ms  RSS +<MiB>  footprint +<MiB>  refs=<n> known=<n> tags=<n> peeled_packed=<n> objects_read=<n> find_header=<n>`.
Calls: `CommitTable::handle_of` through `KnownCommits`; gix `Reference::peeled` (the `^` line); `SharedRepo::handle_with_object_cache(0)` (Phase B item 3 only); `graft_fixtures::{footprint, peak_rss}`; `gated_in_child`.

## Acceptance
- [x] Phase A holder table at 1M cg and 100k cg (3 runs each), sums within 10 % of the measured `ref_snapshot` rise; footprint delta on the line; runs 1–5 in the ticket with raw paths; cause named in one line; the loose-tag floor named in pages per tag and B/commit at 100k.
- [x] `TipStats` on `RefSnapshot`, printed by `--phases`; MAP.md lines.
- [ ] Phase B items 1 and 2 landed (3 only with its measured cause); `find_header == 0` on the cg path; corpus tests as scoped, green; `tips`/`peeled` identical loose vs packed, `()` vs table.
- [ ] `heavy::peak_rss_slope_per_commit_with_a_commit_graph` run 3 times alone: **≤ 150 B/commit all three**, numbers in the ticket — **or** the stop rule: if after items 1–3 the guard is still red and Phase A's table shows the remainder is the loose-tag floor (one decode's pages per annotated tag) plus the OIDL pages, **stop and report**: the slope (3 runs), the floor per tag, and the packed-copy slope beside it. No bar, no fixture change here.
- [ ] `budgets --phases` 1M cg, 3 runs: `ref_snapshot` rise before/after in the ticket (before = this ticket's run 1, not 58's record).
- [ ] `scripts/memory-curve.sh --repo` 1M small cg and 1M realistic cg, 3 runs each: peak RSS, footprint, first paint 1/2/3, `G` — for 56's re-run table.
- [ ] Packed-copy guard slope (10k cg / 100k cg, `pack-refs --all`), 3 runs, reported.
- [ ] KNOWN_GAPS.md "m1-the-graph/58" section: closed with the numbers, or rewritten as the floor finding with the loose and packed numbers side by side and the user decision named.
- [ ] Report: cause or symptom for each change; what it costs the user (expected: nothing on the no-cg path; on the cg path a gc'd repo reads no tag objects at open, a post-fetch repo with loose tags reads each once); raw paths only.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`; heavy: the slope guard ×3 and `budgets_hold_*` (four) once, release, one at a time.

## Size
One context targeted (stats struct, one decode path, one `peeled` branch, corpus tests, scripted runs at ~10 s per 1M run); seam after Phase A if past ~45k. Output to files, report numbers and paths.

## For the user (only if the stop rule fires)
The 10k/100k cg guard fixtures have a commit-graph but 66/650 **loose** refs — a state `git gc` never produces (gc writes the graph *and* packs refs; a fresh clone has `packed-refs` too). If the loose floor is > 150 while the packed copy is green: **recommend the guard's cg fixtures get `pack-refs`** (the repo gc makes; the design lever is git's own), with the loose number kept as a reported cell in the curve and in KNOWN_GAPS. Alternative: keep the loose fixture and the bar, red stays with a gap row. Bar unchanged either way.

## Phase A result (2026-09-14, builder-opus)

Instrumentation landed: `TipStats` on `RefSnapshot`; `--phases` prints `footprint +<MiB>` on every lap and the stats on `refs.ref_snapshot`. **Caveat for Phase B:** the pre-fix path counts `objects_read` as one per tag peel; the peel actually decodes the tag *and* its commit (2). Phase B's single-decode path makes the count exact.

Fixtures (`target/58b/fixtures.txt`): 1M cg `synthetic-1000000-commit-graph-511a25b425758b13`, 100k cg `…-9a592fc3353d5ab5`.

### Runs (release, `budgets --phases`, `refs.ref_snapshot` line; 3 runs each unless noted)
| run | 1M cg RSS / footprint (MiB) | 100k cg RSS / footprint | raw |
|---|---|---|---|
| 1 as-is | 85.5, 85.6, 85.8 / 5.0, 5.0, 5.2 | 8.3, 8.3, 8.4 / 0.9 ×3 | `target/58b/run1-asis-{1M,100k}-{1,2,3}.txt` |
| 2 unknown tips skipped (no tag read) | 34.7, 34.6, 34.6 / 4.4, 4.3, 4.3 | 4.0 ×3 / 0.7 ×3 | `run2-skiptags-*` |
| 3 `handle_with_object_cache(0)` | 85.0 ×3 / 4.5 ×3 | 8.2 ×3 / 0.7–0.8 | `run3-cache0-*` |
| 4 packed copy (`cp -c` + `pack-refs --all`), **pre-fix code** | 84.8–84.9 / 3.8–3.9 | 8.1–8.2 / 0.6–0.8 | `run4-packed-*`; copies kept at `target/58b/packed-{1M,100k}-cg` |

Stats every run 1/3/4: 1M `refs=6498 known=5708 tags=790 find_header=790`; 100k `refs=650 known=567 tags=83 find_header=83`.
- Run 1 reproduces 58 (85.5 / 8.3) within 1 %.
- Run 3 flat: **no retained decode**. Phase B item 3 must not be done.
- Run 4 flat: ref files are not the holder. Pre-fix code ignores `peeled`, so this is **not** the post-fix packed number; run 2 is its upper bound (no tag reads at all): **34.6 / 4.0 MiB**.
- Run 5 (`git verify-pack -v`, `count-objects -v`; `target/58b/run5-verify-pack-1M.txt`): all objects packed (1 pack, 0 loose; 2,862,088 / 286,213 in pack). Tags 790 / 83, **none deltified** (depth 0). The delta-chain hypothesis (b) is wrong: a tag read is one idx lookup + one undeltified pack entry.

### Holder table (vmmap per region, taken inside the process before and after `ref_snapshot`; `target/58b/vmmap-{1M,100k}-{asis,skip}-{before,after}.txt`, laps in `target/58b/vm-*.txt`)
Mappings by size: commit-graph 57.2 M / 5.9 M, pack `.idx` 76.4 M / 7.8 M, `.pack` 221 M / 22 M. All mapped file pages are kept for the life of the handle.

| holder | 1M cg | 100k cg | per | kept / transient |
|---|---|---|---|---|
| commit-graph pages (`handle_of` for the known tips; ≈ the whole OIDL chunk, 1M × 20 B = 19.1 MiB) | 19.1 | 1.9 | per open, ∝ commits | kept |
| `.idx` pages from the pass's first object read (HEAD's peel; present with tags skipped) | 11.0 | 1.1 | per open, ≈ 14 % of the idx at both scales | kept |
| `.idx` pages of tag reads (tag lookup + target-commit lookup) | 37.8 → **47.8 KB/tag** | 2.8 → **34.7 KB/tag** | per tag | kept |
| `.pack` pages of tag reads (tag entry + target commit entry) | 12.6 → **16.3 KB/tag** | 1.3 → **16.2 KB/tag** | per tag | kept |
| heap (footprint peak) | 5.0 (4.4 without tags) | 0.9 | per ref | transient |
| **sum** | **85.5** (measured 85.5–85.8) | **8.1** (measured 8.3–8.4, −3 %) | | |

**Cause:** the rise is mapped file pages, not heap: each annotated tag pays two object lookups (the tag, then the commit it peels to), ≈ 64 KB/tag at 1M and 51 KB/tag at 100k (≈ 4 and ≈ 3 16-KiB pages); the rest is the OIDL chunk for the table lookups and ~14 % of the `.idx` from the pass's first object read.

Unexplained, not traced: why one object read faults ~14 % of the idx (≈ the size of one of its 4-byte-per-object tables). It is per open, not per tag. The walk's `first_chunk` reads commit objects anyway, so moving HEAD's read out of `ref_snapshot` would only move this peak. Not a Phase B item.

**Loose-tag floor, estimate (not measured):** the design floor keeps one lookup and one pack entry per tag and drops the target-commit read. That is ≈ half the per-tag pages: ~25–32 KB/tag (≈ 2 pages). At 100k the tags now cost 4.1 MiB ≈ 43 B/commit. After item 2, ≈ 2.0 MiB ≈ 21 B/commit, which puts the guard at roughly 145–150 B/commit: **borderline**, so the stop rule may fire. Packed (item 1, no tag read) ≈ 0 B/commit: roughly 120–125. The 10k fixture's share (9 tags) is not subtracted in these estimates. The ticket's green condition, ≤ 5.7 MiB for `ref_snapshot` at 100k, has run 2 at 4.0 (packed post-fix bound) and loose after item 2 at ≈ 6.0 (est.).

## Handoff: Phase B (next builder, Opus)
- Done and committed: `TipStats` struct + `RefSnapshot::stats`, counted in the *old* path (`tip_of` in `crates/graft-git/src/repo.rs`); `--phases` footprint and stats (`crates/graft/examples/budgets/phases.rs`); MAP.md lines. Gates green.
- Next, exactly:
  1. `tip_of`: if `reference.inner.peeled` is `Some(id)` (gix 0.87.1 / gix-ref 0.67.1 fill it from the packed-refs `^` line: `raw.rs` `From<packed::Reference>`), count `tags += 1` and `peeled_packed += 1`, then judge `id` by the table (known → tip; unknown → the read path below). No object read.
  2. Replace `find_header` + `peel_commit` for unknown `Object` targets with a loop: `self.repo.find_object(id)` (`objects_read += 1`). `Commit` → tip. `Tag` → `tags += 1`, `obj.to_tag_ref_iter().target_id()`; a known target → tip, else loop (tag-of-tag). Tree/blob → `None`. Map errors to `TipsError::Read`, including a missing object, as `peel_commit` did. Keep `peel_commit` only for symbolic refs.
  3. Do **not** do item 3 (run 3 flat).
  4. Tests and measurements as scoped. Packed copies for the phases runs are at `target/58b/packed-{1M,100k}-cg`. The guard's packed-copy slope needs 10k/100k copies of the guard's own fixtures, made by the guard's path.
- Gotchas: the Bash tool's shell is zsh, so `for a in "100000 --commit-graph"` does not word-split (use `eval`); vmmap truncates paths, so identify mappings by size; one object read maps ~14 % of the idx per open, so don't attribute it to tags.

## Phase B result (2026-09-14, builder-opus)

Code: `bfa9b8f`. Items 1 and 2 landed; item 3 not done (Phase A run 3 flat).
- **Item 1:** a ref whose gix `Reference::inner.peeled` is `Some` (packed-refs `^` line) resolves to that id with no object read; the id is judged by the table like any tip.
- **Item 2:** `commit_at` loop: table first; else one whole `find_object` (commit → tip, tag → judge its target, tree/blob → none). Every id judged goes into the per-pass cache, so a shared tag target is read at most once. `find_header` is 0 on the cg path.
- **One addition not in the spec:** `KnownCommits::covers_the_graph(&self) -> bool`, a provided method, default `false`; `CommitTable` returns `has_commit_graph()`. Without it, the `()`/no-cg path would decode every commit tip whole, which is the 30d regression. When it is `false` the header is read first, as before. It changes what is read, never the answer.
- **`objects_read` undercount fixed:** every whole read is counted. At 1M loose: 790 = tags, 0 other.
- **Corpus test** (`refs_sharing_commits_or_naming_no_commit_peel_the_same_loose_or_packed`): added a tag-of-tag (`outer` → `release` → parent). Exact `TipStats` for `()` loose/packed: 7/4 headers, 5/2 reads, tags 3, peeled 0/3. With the table: `find_header == 0`, `known == 2`; loose `objects_read == tags + 2` (tree, blob); packed `peeled_packed == tags` and `objects_read == 2` (tree and blob only, no tag object read). Tips and peeled are identical loose vs packed, and `()` vs table.

### Measured (release, raw in `target/58b/`)
`budgets --phases`, `refs.ref_snapshot` lap, 3 runs each (`after-{1M,100k,packed-1M,packed-100k}-{1,2,3}.txt`):

| fixture | before (run 1) RSS / footprint | after RSS / footprint | stats after |
|---|---|---|---|
| 1M cg loose | 85.5–85.8 / 5.0–5.2 | **73.9–74.2** / 4.5–4.7 | known=6497 tags=790 peeled_packed=0 objects_read=790 find_header=0 |
| 100k cg loose | 8.3–8.4 / 0.9 | **7.1** / 0.7–0.8 | known=649 tags=83 objects_read=83 find_header=0 |
| 1M cg packed copy | 84.8–84.9 (pre-fix) | **33.8–33.9** / 3.2–3.3 | peeled_packed=790 objects_read=0 |
| 100k cg packed copy | 8.1–8.2 (pre-fix) | **3.8** / 0.5–0.6 | peeled_packed=83 objects_read=0 |

**Slope guard, 3 runs alone:**
- loose fixtures: **166, 167, 166 B/commit** (peaks 12 / 27 MiB), `guard-loose-{1,2,3}.txt`.
- **reported:** `pack-refs --all` copies swapped in for the 10k and 100k cg fixtures, originals restored after: **157, 156, 156** (12 / 26 MiB), `guard-packed-{1,2,3}.txt`.

**Why the guard did not move on the loose fixtures:** the pages leave `ref_snapshot` but are faulted later anyway. At 100k, `ref_snapshot` fell 1.2 MiB and `first_chunk` rose 1.0 MiB (8.3/4.3 before, 7.1/5.3 after; `run1-asis-100k-1.txt` vs `after-100k-1.txt`). The tag targets are walk tips, and the first chunk reads those commits, touching the idx and pack pages the old peel had touched. The process peak fell only 0.2 MiB.

**Loose-tag floor** (after minus packed): 1M (74.0 − 33.9) / 790 = **51 KB/tag** (≈ 3 16-KiB pages); 100k (7.1 − 3.8) / 83 = **41 KB/tag** (≈ 2.5 pages). In the whole-process slope it is worth **~10 B/commit** (loose 166 vs packed 156), not the ~35 B/commit the lap alone suggests.

**Stop rule, finding for the user:** the ticket's premise does not hold. Packing refs (git's own lever) removes every tag read and the guard is **still red at 156**. So the remainder is not the loose-tag floor. At 100k, per the laps: OIDL pages for `handle_of` (~1.9 MiB, Phase A), idx pages from the first object read (~1.1 MiB), `walk_new` +3.4 MiB, `first_chunk` +5.3 MiB. The recommendation in "For the user" (pack the guard's fixtures) would **not** make it green. Nothing was changed in the bar or the fixtures. For the planner: name the ~6 B/commit above the bar on the packed path, in `walk_new`/`first_chunk` pages, or re-derive ADR-0026's 150 from those phases.

`scripts/memory-curve.sh --repo`, 3 runs each (`target/58b/58b-1M-{small,realistic}-cg.txt`):

| cell | peak RSS at paint | after G | footprint | first paint 1/2/3 | G 1/2/3 |
|---|---|---|---|---|---|
| 1M small cg | 128.1 / 127.7 / 127.9 MiB | 168.0 / 167.5 / 169.2 | 36.3 / 35.8 / 37.5 | 134 / 87 / 89 ms | 554 / 504 / 566 ms |
| 1M realistic cg | 129.0 / 128.7 / 128.7 MiB | 172.5 / 173.1 / 170.6 | 40.8 / 41.4 / 38.9 | 818 / 86 / 86 ms | 545 / 507 / 509 ms |

Run 1 first paint is cold (ADR-0027 times warm).

`budgets_hold_*`, release, once (`target/58b/budgets-hold.txt`): all 5 pass (the four plus 50k refs). 100k cg peak 28.1 MiB; realistic cg 32.2; no-cg 74.0; realistic no-cg 80.8; 50k refs 113.8.

Gates: fmt, clippy (with and without `heavy-tests`), `cargo test` 858/858.

### Acceptance (Phase B)
- [x] Items 1 and 2 landed; item 3 not done (no measured cause); `find_header == 0` on the cg path; corpus tests green; tips/peeled identical.
- [x] Slope guard ×3: **red, 166/167/166**. Stop rule fired, with the packed copy red too (156/156/157); finding above.
- [x] `budgets --phases` 1M cg ×3, before/after in the table.
- [x] memory-curve 1M small/realistic cg ×3.
- [x] Packed-copy guard slope ×3, reported.
- [x] KNOWN_GAPS "m1-the-graph/58" rewritten as the finding.
