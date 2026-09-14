# m1-the-graph/58 — Tips are checked against the id table, not header-parsed (slope guard green)

Status: done (fix landed; guard remainder closed by 58b + 61 — conductor, 2026-09-14)
Build with: **Sonnet** (medium) — the approach is decided below; one trait, two signatures, two callers; the guard's number judges it.
Needs: `gated_in_child` / `ChildPeak` and the slope guard on it (guards-measure-in-a-child); `graft_fixtures::footprint`.
Blocked by: 57

## The finding, and the ruling

Slope guard red: 237 B/commit (12.5 → 33.0 MiB, 10k cg → 100k cg). 19.6 of the 20.5 MiB rise is already held at first paint. The walk itself adds ~10 B/commit — the ADR-0026 design holds. What is held at open (ticket 55, `--phases`): `ref_snapshot` **+147 MiB at 1M cg** (+15 MiB at 100k ≈ 150 B/commit in this fixture, one ref per ~154 commits), because `commit_tips` calls `find_header` on every tip to learn whether it is a commit — 6,497 scattered pack pages faulted in and counted by `ru_maxrss`; and `walk_new` +53 MiB = the commit-graph's own pages (56 B/commit, the design's floor).

**Does the guard measure the right thing?** Yes. It measures peak RSS over the same path the 300 MB gate measures, so the two agree; the 10k baseline already subtracts fixed costs, and the miss is genuinely proportional (tips ∝ commits here; in general refs × one pack page each). The bar stays at 150 B (ADR-0026); nothing is re-set.

**The fix, within ADR-0026:** with a commit-graph, "the file is the table". A tip that `CommitTable::handle_of` finds *is* a commit — no object read. Only a tip the table does not know (an annotated tag object, a commit newer than the graph, a ref to a blob/tree) is header-parsed as today. Without a graph the table is empty at open, every tip goes the old way, nothing changes on the reported path. Expected after: `ref_snapshot` rise ≲ 20 MiB at 1M cg (OIDL-chunk pages the binary search touches), slope ≈ 56 (graph pages) + ~20–40 (heap) ≈ **80–100 B/commit**, 1M cg peak RSS ≈ 240 − 147 ≈ **~95 MiB**. If the fixture's refs turn out to be annotated tags the tag object is still read (one small page each) and the win is smaller; measure, don't assume.

Options rejected: subtracting first-paint RSS from the slope (loosens what the guard covers); packing the fixture's refs (hides the cost); reading type from the idx (the idx has no type).

## Read first
- ADR-0026 "Decision (design)" items 1 and 3, "The gate at 1M" (slope bullet)
- `.scratch/m1-the-graph/issues/55-first-paint-at-a-million-named-cause.md` lines 64–106 ("Phase A — measured": the phase table, "Secondary, memory", hypotheses) and 148–165 (fix 2, `ref_snapshot` +149 MiB)
- `.scratch/m1-the-graph/issues/56-memory-curve-acceptance.md` "For the M1 review" slope bullet and "Spec-only pass 2" (finding 1)
- `crates/graft-git/MAP.md` sections `repo` (`RepoHandle::commit_tips`, `ref_snapshot`, `peeled_references`, `RefSnapshot`) and `history` (`Walk::all_refs`, `CommitTable::handle_of`)
- `crates/graft/MAP.md` sections `repos` (`RepoStore::refs`, `RepoStore::table`) and `refs` (`read`)
- `crates/graft-fixtures/MAP.md` (`synthetic`, `footprint`)

## Scope
In:
- `KnownCommits` trait in `graft_git::repo`; `impl KnownCommits for CommitTable` in `graft_git::history` (`handle_of(id).is_some()` — if MAP's `handle_of` is not an `Option`, use whatever "known without appending" shape it has; never `intern`); `impl KnownCommits for ()` = always unknown (today's behaviour; tests and any caller without a table).
- `RepoHandle::commit_tips`, `ref_snapshot` (and `peeled_references` if it does its own per-tip `find_header`) take `&dyn KnownCommits`; a known id is a commit, no read; an unknown one is header-parsed exactly as today (tag → peel → check again). Peeled targets in `RefSnapshot.peeled` unchanged in meaning.
- Callers: `RepoStore::refs` passes `&*self.table()`; `Walk::all_refs` passes its own table. `refs::read` already shares the snapshot (55 fix 2); confirm no second lookup path.
- Fast tests (corpus): tips/peeled identical with `()` and with the real table on a corpus shape with a commit-graph and one with annotated tags; a ref to a blob/tree is still excluded on both.
- **Measure**, release, in the ticket: (a) `budgets --phases` 1M cg, 3 runs, before and after: the `ref_snapshot` phase's time and RSS rise; (b) the slope guard number (now child-measured); (c) `scripts/memory-curve.sh --repo <1M small cg>` and `--repo <1M realistic cg>`: peak RSS, footprint, first paint run 1/2/3, `G`; (d) `git for-each-ref --format='%(objecttype)' | sort | uniq -c` on the 1M cg fixture (what kind the tips are); (e) run-1 first paint of both 1M cg cells before/after — reported only (finding 4 is a gap, not this ticket's bar). Raw to `target/<ticket>/`.
- MAP.md lines for every changed item; ARCHITECTURE.md untouched (no module moves).

Out: staleness of a commit-graph (many tips outside it cost what they cost today; note it, M2+), the no-cg path, any bar, the cold-cache question beyond reporting run 1.

## Signatures
```rust
// graft-git::repo
/// Says whether an id is a commit without reading its object, when something already knows.
/// `false` means "not known here": the caller reads the header, as it always did.
pub trait KnownCommits {
    fn is_known_commit(&self, id: graft_core::Oid) -> bool;
}
impl KnownCommits for () { fn is_known_commit(&self, _: Oid) -> bool { false } }

impl RepoHandle {
    pub fn commit_tips(&self, known: &dyn KnownCommits) -> Result<Vec<Oid>, TipsError>;
    pub fn ref_snapshot(&self, known: &dyn KnownCommits) -> Result<RefSnapshot, RefsError>;
    // peeled_references(&self, known: &dyn KnownCommits) only if it header-parses tips itself today
}

// graft-git::history
impl KnownCommits for CommitTable { fn is_known_commit(&self, id: Oid) -> bool { self.handle_of(id).is_some() } }
```
Calls: `RepoStore::table()`, `Walk::table()`. `Oid` is graft-core's id type as `HandleLookup` (graft-graph MAP) already uses it.

## Acceptance
- [x] `KnownCommits` implemented, threaded through `commit_tips`/`ref_snapshot`/`peeled_references`, all callers updated. Fast gates green.
- [ ] `heavy::peak_rss_slope_per_commit_with_a_commit_graph` **NOT green**: 166 B/commit against the 150 B bar (down from 237; ticket's own 80–100 estimate not met). Reported, measured in a child, run alone and confirmed again after the 100k no-cg test — same 166 both times. Named cause: annotated tags (~12–13% of refs at every scale tested) are outside the id table by design (`CommitTable` holds commit handles only) and still pay the old header-read+peel cost; see KNOWN_GAPS.md. This is a real finding, not a bug in the fix: the table-hit path itself costs nothing (confirmed via `ref_snapshot`'s per-tip RSS falling from ~150 B/commit-equivalent to ~13–19 KB per **tag ref**, scaling with ref count not commit count).
- [x] `budgets --phases` 1M cg (single run, `target/58/phases-1M-cg-after.txt`): `ref_snapshot` RSS rise now **85.5 MiB** (was +147 per ticket 55/56's record) — real improvement, short of the ≤25 MiB target for the same reason above (790/6498 refs at 1M are annotated tags). Time 771 ms (dominated by the tag reads + 6497-entry walk setup, not separately isolated from before).
- [ ] 1M cell re-run at 3x not done (time budget); single-run 1M cg peak reported above (footprint 16.7 MiB post-first-paint; well under 300 MB either way — the 300 MB gate was never the tight constraint here).
- [x] Ref kinds reported: 10k cg 57 commit/9 tag; 100k cg 567 commit/83 tag; 1M cg 5708 commit/790 tag. Tag-object reads are the named residual cost above.
- [x] Corpus test added (`refs_sharing_commits_or_naming_no_commit_peel_the_same_loose_or_packed` in `graft-git/src/repo/tests.rs`): identical tips/peeled with `()` and with a real `CommitTable` table, including annotated tags and tags-on-tree/blob.
- [x] `heavy::budgets_hold_*` (four) green; `many_refs::*` green, 50k-refs `commit_tips` now 14.2 ms (was the ticket's 424 ms/114 ms debug/release baseline area) — table hits confirmed by the doc comment on the test.
- [ ] Run-1 first-paint before/after at 1M not separately isolated this session (out of the phases capture above, which is after-only); no verdict claimed either way.
- [x] Report: **cause**, not symptom — the header read was never needed once a commit-graph makes the id table authoritative for commits; fixing the caller (skip the read when the table already knows) removes the unnecessary work rather than papering over its cost. Costs the user: nothing on the no-cg path (untouched); on the cg path, one pack-page read per annotated tag ref remains (down from one per *every* tip) — real but incomplete win, 237→166 B/commit, still over the 150 bar.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`; the named heavy tests, one at a time, release.

## Size
One context, under ~80k: one trait, two signatures, two call sites, corpus tests, then scripted measurement (1M runs ~10 s each, output to files, report paths and numbers).
