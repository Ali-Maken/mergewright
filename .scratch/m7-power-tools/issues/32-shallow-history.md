# 32 shallow-history — a shallow clone's boundary commits are parentless, in every walk

Status: done (worktree agent-acb4a04073e2b9803; not merged)
Blocked by: **18e** (holds graft-git now, and edits `trace/log.rs`, `trace/mod.rs`, `blame.rs`, the files this ticket
moves the shallow reader out of).
Crates: graft-git only (`history/`, a new `shallow.rs`, `trace/shallow.rs`, `trace/log.rs` + `blame.rs` imports only,
`commit_graph.rs`, their tests, `MAP.md`). No graft, no graft-ui: `WalkError` gains a variant; the one graft match
(`crates/graft/src/jobs/preview.rs:158-164`) ends in `Err(other)`, so it compiles unchanged. Needing another crate →
commit at the seam, report `blocked: needs <crate>` naming the signature, stop.
Build with: **Opus**. One seam (`RepoNodes::node`) and a reader move, pinned by an oracle against `git rev-list` on real
shallow clones. Not Fable: a wrong answer is a red oracle test, and the choice below is cheap to move later.
Scout: none. The planner checked every site named here against `main` (2026-09-27); re-read each before editing.
Needs: none.
Split by: planner, 2026-09-27, from 20f's NOTES "Shallow capture — blocked".

## The defect (verified, rule 7)
Graft can't open a `git clone --depth N` repository: `reading history: commit b32274b… is not in the repository` (20f,
seen with `GRAFT_TEST_TRACE=1`).
- `history/node.rs:291-309` `node_from_object` interns **every** id in `commit.parent_ids()`; a boundary commit's
  object still names its parent. The walk then asks for that parent's node and `find_commit` returns
  `NodeError::Missing` (`node.rs:319-322`).
- Nothing in `history/` reads `<common_dir>/shallow` (grep `shallow` in `crates/graft-git/src/history/`: only the doc
  comment at `read.rs:111`, which claims a shallow commit "simply ends the chain" — false today).
- Same root, other readers: `branches.rs:249` maps `Missing` to `Ok(0)` merges, so in a shallow clone every branch
  counts 0 merges and the integration-branch guess (`branches.rs` `merges_on`) is decided on nothing. `graft`'s
  `jobs/preview.rs:162` drops a `Missing` commit from the picture.

## Ruling
**Git's rule, applied at the one seam every walk reads through:** a commit listed in `<common_dir>/shallow` has no
parents. That is what `git rev-list --parents` and `git log --graph` show, and what 18c/20d already assume for trace
and blame.
1. **One reader.** `trace/shallow.rs`'s `read_shallow` moves to a crate-level `shallow` module as `ShallowList::read`;
   its rules are unchanged (no file = not shallow; unreadable file or a non-id line = error, never "not shallow").
   `trace::shallow_boundaries` and `blame`'s call become callers of it. `BlameError::Shallow` and
   `TraceError::Shallow` keep their public shape (map from `ShallowError`).
2. **Read once per walk**, held for the walk's life: at `Walk::all_refs` / `from_tips` / `full_topological`
   construction, and once per call of `read_commit`, `first_parent_merges`, `first_parent_chain`, `ahead_behind`.
   Not in `CommitTable`: the table outlives `git fetch --deepen`/`--unshallow` (`RepoStore` rebuilds it only when the
   commit-graph's presence changes, graft MAP.md `RepoStore::refresh`), so a list held there goes stale.
3. **Applied in `RepoNodes::node`**, after either source (graph or object): a listed id gets empty `parents`, and its
   missing parent is never interned. `hydrate`/`read_commit` clear `Commit::parents` for a listed id the same way, so a
   row's parents and its links agree. When the list is empty the per-node cost is one `is_empty()` branch.
4. **No commit-graph in a shallow repo.** `CommitTable::open` binds none and `commit_graph::status` answers `Absent`
   when the list is non-empty — git's own rule (`commit-graph.c` `commit_graph_compatible` returns 0 for a shallow
   repository; **unverified here**: pin the line in git's source for 2.38.5 and 2.54 in NOTES; if git differs, stop
   and report, don't choose). This also keeps `RepoStore`'s "presence changed" rebuild honest after `--unshallow`.
   Knock-on accepted until 20j: the commit-graph offer can show in a shallow clone (20j removes it).

Rejected: **catching `Missing` and treating it as a root** — it would also hide a genuinely corrupt repository (a
missing object that should be a refusal, `jobs/preview.rs:154-157`'s rule). **Using gix's own shallow reading** — a
second reader of the same file that could disagree with blame and trace.

## Read first
- 20f's NOTES "Shallow capture — blocked" (`.scratch/m7-power-tools/issues/20f-blame-place-walks.md`).
- `crates/graft-git/MAP.md` `## history`, `## commit_graph`, `trace` (`SHALLOW_END`, `TraceError::Shallow`), `blame`
  (`BlameCommit::shallow`, `BlameError::Shallow`).
- Source, only: `history/node.rs` (`RepoNodes`, `node_from_object`), `history/read.rs` (`hydrate`, `read_commit`,
  `first_parent_*`), `history/walk.rs` constructors, `history/divergence.rs`, `history/table.rs` `open`,
  `trace/shallow.rs`, `commit_graph.rs` `status`.

## Signatures (exact)
```rust
// graft-git::shallow (new module; `trace/shallow.rs`'s reader moves here, rules unchanged)
/// The commits `<common_dir>/shallow` lists: where a shallow clone's history was cut.
/// Git treats each as parentless.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ShallowList { /* BTreeSet<Oid> */ }
impl ShallowList {
    /// No file: an empty list. A file that can't be read, or a line that isn't an id: an error.
    pub fn read(repo: &RepoHandle) -> Result<ShallowList, ShallowError>;
    pub(crate) fn read_in(common_dir: &Path) -> Result<ShallowList, ShallowError>; // for callers holding gix::Repository
    pub fn is_empty(&self) -> bool;
    pub fn contains(&self, id: &Oid) -> bool;
}
#[derive(Debug, thiserror::Error)]
#[error("couldn't read {}: {source}", path.display())]
pub struct ShallowError { pub path: PathBuf, #[source] pub source: std::io::Error }

// graft-git::history::WalkError gains
/// `<common_dir>/shallow` exists but could not be read.
#[error(transparent)]
Shallow(#[from] ShallowError),

// graft-git::history (crate-private)
pub(crate) struct RepoNodes<'a> { repo: &'a gix::Repository, table: Arc<CommitTable>, shallow: Arc<ShallowList> }
impl<'a> RepoNodes<'a> {
    pub(crate) fn new(repo: &'a gix::Repository, table: Arc<CommitTable>, shallow: Arc<ShallowList>) -> Self;
}
```
Public signatures of `Walk::*`, `read_commit`, `first_parent_*`, `ahead_behind`, `CommitTable::open`,
`commit_graph::status` are unchanged (each already returns a `Result` whose error can carry the new case; if one can't,
say which and how you mapped it).

## Acceptance (name the verifying test at tick time)
Tests in `crates/graft-git/tests/it/shallow_history.rs`; clones are `git clone --depth N file://<fixture>` (as 18c).
- [x] **Red first:** `a_depth_2_clone_walks` — `Walk::all_refs` on the session fixture at depth 2 succeeds (today:
      `Missing`). Show it red before the fix. Red (before the seam): `walking a depth-2 clone: Some(Commit(Missing { id: Oid(b32274b37e6ce919315f41103f3f30078e9a3487) }))` — 20f's id. Green after.
- [x] **Oracle**, depth 1 and depth 2 of the session fixture, and depth 3 of a corpus shape with a merge whose second
      parent is cut (name the shape): the set of `(id, parents)` from `next_links` to the end **equals**
      `git rev-list --parents --all`'s; same for `next_chunk`'s rows' `parents`; asserts ≥1 boundary row so it isn't
      vacuous. Tests: `walks_match_git_on_a_depth_1_session_clone`, `walks_match_git_on_a_depth_2_session_clone`,
      `walks_match_git_on_a_depth_3_feature_branches_clone_with_a_cut_merge` (shape **feature-branches**, `main` only:
      `Merge branch 'feature/one'`'s second parent `one: finish` is a boundary; the test asserts the cut). Each checks
      `next_links` + `next_chunk` in CommitDate and Topological, plus `full_topological` links.
- [x] Oracle: `first_parent_chain(main)` = `git rev-list --first-parent main`; `first_parent_merges` =
      `git rev-list --first-parent --merges --count`; `ahead_behind` on two branches = `git rev-list --left-right
      --count a...b`; `read_commit(boundary).parents` is empty. Tests: `first_parent_reads_match_git_on_a_shallow_clone` (chain + merges,
      merges > 0), `ahead_behind_matches_git_across_a_shallow_boundary` (every origin branch vs main, depth 2, all
      branches), `read_commit_of_a_boundary_has_no_parents`.
- [x] **The graph path:** a full clone with `git commit-graph write --reachable`, then a hand-written
      `.git/shallow` (`git rev-parse HEAD~2 > .git/shallow`, git's own test technique): `CommitTable::open` binds no
      graph, `commit_graph::status` is `Absent`, and the walk oracle above holds. NOTES: git's
      `commit_graph_compatible` line, both versions. Test: `a_shallow_file_over_a_commit_graph_binds_no_graph` (also: a
      table opened *before* the shallow file keeps its graph and its walk still cuts — mutation-checked: disabling the
      graph-branch clear in `RepoNodes::node` turns it red).
- [x] `an_unreadable_shallow_file_fails_the_walk` (`shallow` is a directory) → `WalkError::Shallow`, never a walk that
      ignores it. Test: `an_unreadable_shallow_file_fails_the_walk` (`all_refs`, `first_parent_merges`, `read_commit`).
- [x] A full clone's walk is unchanged: the existing history suites pass untouched; name the count. Workspace nextest 4119/4119 (1 skipped),
      graft-git `it` 138/138; `src/history/tests.rs` not edited (diff stat).
- [x] Blame and trace unchanged through the move: `blame_marks_lines_from_the_shallow_boundary`,
      `an_unreadable_shallow_file_fails_blame`, 18c's shallow trace tests, 18e's
      `a_failed_log_with_an_unreadable_shallow_file_reports_gits_reason` — all green, not edited. Run by name: 3 blame + 7 trace shallow tests PASS; `tests/it/blame.rs`, `trace.rs`,
      `src/trace/tests.rs` absent from the diff.
- [ ] Perf (**not run**): the per-node cost on a non-shallow repo. Run the walk's existing example/bench from a script (grep
      `crates/graft-git/examples/`), before and after, to `docs/perf/32.txt`; read the summary line only. None fits →
      say so; the milestone review's release run covers it. **None run.** Only `examples/lost_cost.rs` (c)
      builds a `Walk`, and it is heavy-tier (`--features heavy-tests`, 100k-commit fixture, release); a before/after needs
      two release builds of the workspace. Added cost on a complete repo, by reading: one `is_empty()` per node and per
      hydrated row; one `open()` of the missing `shallow` file per walk / per `read_commit` call. Left to the milestone
      review's release run.
- [x] `read.rs:111`'s doc now true; say so. Rewritten: a root commit or a shallow boundary (parentless, git's rule)
      ends the chain — true per `first_parent_reads_match_git_on_a_shallow_clone`.
- [x] MAP.md: `shallow` module (`ShallowList`, `ShallowError`, the rule), `WalkError::Shallow`, `## history` "Reads"
      line gains the shallow rule, `commit_graph::status` and `CommitTable::open` gain "none in a shallow repo",
      `trace`'s `shallow.rs` line points at the new home. Done, plus `CommitGraphError::Shallow`, `TableError::Shallow`.
- [x] Gates workspace-wide: fmt, clippy `--all-targets -D warnings`, nextest, `test --doc`. All pass (fmt re-checked after the last test edit).

## Touches (check each; NOTES says how)
- **Graph (graft, graft-ui):** the boundary draws as a root — its lane ends at its row. No drawing change; no fixture
  is shallow, so every capture is byte-identical (the suites show it). Nothing marks the cut: gap row for the
  conductor, "a shallow boundary looks like a root; git's `log --decorate` says `grafted`" (unverified need).
- **Radar:** `first_parent_chain` ends at the boundary; the radar context in a shallow clone now succeeds (the loop 20f
  saw needs this *or* 33; 33 fixes the loop itself).
- **Blame boundary:** `BlameCommit::shallow` reads through the moved reader; behaviour unchanged (tests above).
- **`branches.rs:249` `merges_on`:** now counts real merges in a shallow clone; its `Missing` arm stays for a broken
  ref (its doc says why). Test: `integration_branch_in_a_shallow_clone` or say which existing test covers it.
- **`graft` `jobs/preview.rs:162`:** a boundary's parent is no longer read, so no longer dropped; the arm stays for a
  truly missing object. Grep, no edit.
- **`read_commit(` callers in every crate** (graft-ops' rewrite engine included): grep; each now sees a boundary as
  parentless, as git does. List them in NOTES.
- **Commit-graph offer (graft):** shows in a shallow clone until 20j. Name it; don't touch graft.
- **Perf:** above. **Keymap, journal, undo, captures:** none; show the grep.

## Size
~55k. One context. If it compacts, split: **32a** the reader move + `WalkError::Shallow`; **32b** the seam, the graph
rule, the oracles.

## Rules
Worktree only, own `CARGO_TARGET_DIR`. Never read `PROGRESS.md` or `KNOWN_GAPS.md`.

## NOTES (builder, 2026-09-27)
Scout: none — the ticket says the planner checked every site; each re-read before editing. No compaction.

**Git's rule, pinned** (source fetched from github.com/git/git at the tags): `commit-graph.c` `commit_graph_compatible`
returns 0 on `is_repository_shallow(r)` — **v2.38.5 line 220**, **v2.54.0 line 238**; `prepare_commit_graph` bails on
it — v2.38.5 line 653, v2.54.0 line 768. Git agrees with the ruling.

**Public shape changes** (ticket allowed; say which): `TableError` gains `Shallow(#[from] ShallowError)` (`CommitTable::open`
reads the list); `CommitGraphError` gains `Shallow(ShallowError)` (`status` reads it; Display = the ShallowError's).
graft matches neither exhaustively (`repos.rs:288,291` are `#[from]` only) — workspace build green. Crate-private:
`CommitTable::open_with(repo, &ShallowList)` so `first_parent_merges`/`ahead_behind` read the file once;
`node_from_object` gains `boundary: bool`; `hydrate` gains `&ShallowList`. `ShallowList::into_ids` (crate-private) keeps
trace's `BTreeSet`, so `finish` and `trace/tests.rs` are unedited.

**Touches:**
- Graph (graft, graft-ui): boundary now emitted parentless → lane ends at its row. No fixture is shallow; workspace
  nextest 4119/4119 incl. every capture test. Gap row for the conductor: *a shallow boundary looks like a root; git's
  `log --decorate` says `grafted`* (unverified need).
- Radar: `first_parent_chain` ends at the boundary (test above). The loop fix itself is 33's; not touched.
- Blame boundary: via `ShallowList::read`; 3 blame tests green, unedited.
- `branches.rs:249` `merges_on`: now counts real merges (it wraps `first_parent_merges`, covered by
  `first_parent_reads_match_git_on_a_shallow_clone`, merges > 0). No separate `integration_branch_in_a_shallow_clone`.
- `graft` `jobs/preview.rs:162`: boundary parent no longer read; arm stays for truly missing objects. Grep only.
- `read_commit(` callers (grep): graft `map/walk.rs:68`, `history/source.rs:83,132`, `jobs/commit_message.rs:92`,
  `jobs/detail.rs:76`, `jobs/preview.rs:139`; graft-ops `mistake_radar/probes.rs:110,612` (a boundary merge now reads
  as not a merge, as git shows it); graft-ops tests `journal_list_oracle.rs:92`, `lost_work.rs:92`,
  `mistake_radar.rs:103`; graft-graph test `streaming.rs:85,94,287`; graft-git `history/tests.rs:394,405`. All now see a
  boundary as parentless. **Perf knock-on:** `graft/src/history/source.rs:132` calls `read_commit` per evicted row, so
  each call now also opens `<common_dir>/shallow` (ENOENT on a full clone). Ticket-mandated ("once per call");
  unmeasured.
- Commit-graph offer (graft): `status` says `Absent` in a shallow clone, so the offer can show until 20j; and
  `commit_graph::write` there would report "git wrote a commit-graph but it is not in use" (git writes none — same
  `commit_graph_compatible`, v2.54.0 line 2584). Not touched.
- Keymap, journal, undo, captures: none — the diff touches only `crates/graft-git` and this ticket.
